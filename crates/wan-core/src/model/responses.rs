//! OpenAI Responses adapter (`POST {base_url}/responses`).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::blocks::Blocks;
use super::sse::SseParser;
use super::transport::{HttpResponse, Transport};
use super::types::{
    AssistantMessage, ContentBlock, Message, ModelRequest, Pricing, StopReason, StreamEvent,
    ToolCall, Usage,
};
use super::{
    Decoder, EndpointConfig, ModelError, Out, event_stream, parse_arguments, post_json, str_field,
    unwrap_data,
};
use crate::ports::{Model, ModelStream, SecretSource};

#[derive(Debug)]
pub struct Responses<T, S> {
    config: EndpointConfig,
    transport: T,
    secrets: S,
}

impl<T: Transport, S: SecretSource> Responses<T, S> {
    pub fn new(config: EndpointConfig, transport: T, secrets: S) -> Self {
        Self {
            config,
            transport,
            secrets,
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    async fn send(&self, request: &ModelRequest, stream: bool) -> Result<HttpResponse, ModelError> {
        post_json(
            &self.transport,
            &self.secrets,
            &self.config,
            "/responses",
            request_body(request, stream),
        )
        .await
    }
}

impl<T: Transport, S: SecretSource> Model for Responses<T, S> {
    async fn stream(&self, request: ModelRequest) -> Result<ModelStream, ModelError> {
        let response = self.send(&request, true).await?;
        Ok(event_stream(
            response.body,
            ResponsesDecoder::new(self.config.pricing),
        ))
    }

    async fn complete(&self, request: ModelRequest) -> Result<AssistantMessage, ModelError> {
        let response = self.send(&request, false).await?;
        let body = response.collect().await?;
        parse_completion(&body, &self.config.pricing)
    }
}

fn request_body(request: &ModelRequest, stream: bool) -> Value {
    let mut input = Vec::new();
    for message in &request.messages {
        match message {
            Message::User { text } => input.push(json!({"role": "user", "content": text})),
            Message::Assistant { content } => {
                for block in content {
                    input.push(match block {
                        ContentBlock::Text { text } => {
                            json!({"role": "assistant", "content": text})
                        }
                        ContentBlock::ToolCall(call) => json!({
                            "type": "function_call",
                            "call_id": call.id,
                            "name": call.name,
                            "arguments": call.arguments.to_string(),
                        }),
                    });
                }
            }
            Message::ToolResult {
                call_id, content, ..
            } => input.push(json!({
                "type": "function_call_output",
                "call_id": call_id,
                "output": content,
            })),
        }
    }

    let mut body = json!({"model": request.model, "input": input, "stream": stream});
    if let Some(system) = &request.system {
        body["instructions"] = json!(system);
    }
    if let Some(max) = request.max_output_tokens {
        body["max_output_tokens"] = json!(max);
    }
    if !request.tools.is_empty() {
        // `strict` is a required key of a Responses function tool.
        body["tools"] = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                    "strict": false,
                })
            })
            .collect();
    }
    body
}

fn parse_completion(body: &[u8], pricing: &Pricing) -> Result<AssistantMessage, ModelError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| ModelError::Protocol(format!("response is not JSON: {error}")))?;
    let value = unwrap_data(value);
    if let Some(message) = value.pointer("/error/message").and_then(Value::as_str) {
        return Err(ModelError::Protocol(message.to_owned()));
    }

    let mut content = Vec::new();
    let mut has_calls = false;
    for item in value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match item.get("type").and_then(Value::as_str) {
            Some("message") => {
                let text: String = item
                    .get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|part| part.get("type").and_then(Value::as_str) == Some("output_text"))
                    .filter_map(|part| part.get("text").and_then(Value::as_str))
                    .collect();
                if !text.is_empty() {
                    content.push(ContentBlock::Text { text });
                }
            }
            Some("function_call") => {
                has_calls = true;
                content.push(ContentBlock::ToolCall(ToolCall {
                    id: str_field(item, "/call_id")?,
                    name: str_field(item, "/name")?,
                    arguments: parse_arguments(
                        item.get("arguments")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                    )?,
                }));
            }
            _ => {}
        }
    }

    let usage = parse_usage(value.get("usage"));
    Ok(AssistantMessage {
        content,
        stop_reason: stop_reason(&value, has_calls),
        usage,
        cost: usage.cost(pricing),
        response_id: value.get("id").and_then(Value::as_str).map(str::to_owned),
    })
}

/// Responses has no tool-call finish reason, so tool use is inferred from the output.
fn stop_reason(response: &Value, has_calls: bool) -> StopReason {
    match response
        .pointer("/incomplete_details/reason")
        .and_then(Value::as_str)
    {
        Some("max_output_tokens") => StopReason::Length,
        Some("content_filter") => StopReason::ContentFilter,
        _ if has_calls => StopReason::ToolUse,
        _ => StopReason::Stop,
    }
}

/// `input_tokens` includes cached tokens; `Usage::input` counts only the uncached part.
fn parse_usage(usage: Option<&Value>) -> Usage {
    let Some(usage) = usage else {
        return Usage::default();
    };
    let count = |pointer: &str| usage.pointer(pointer).and_then(Value::as_u64).unwrap_or(0);
    let cached = count("/input_tokens_details/cached_tokens");
    let written = count("/input_tokens_details/cache_write_tokens");
    Usage {
        input: count("/input_tokens").saturating_sub(cached + written),
        output: count("/output_tokens"),
        cache_read: cached,
        cache_write: written,
    }
}

struct ResponsesDecoder {
    parser: SseParser,
    pricing: Pricing,
    blocks: Blocks,
    /// Provider `output_index` -> block index. Reasoning items get no block.
    items: BTreeMap<u64, usize>,
    response_id: Option<String>,
    done: bool,
}

impl ResponsesDecoder {
    fn new(pricing: Pricing) -> Self {
        Self {
            parser: SseParser::default(),
            pricing,
            blocks: Blocks::default(),
            items: BTreeMap::new(),
            response_id: None,
            done: false,
        }
    }

    fn accept(&mut self, event: Option<&str>, data: &str, out: &mut Out) {
        if data.trim() == "[DONE]" {
            return;
        }
        let value: Value = match serde_json::from_str(data) {
            Ok(value) => unwrap_data(value),
            Err(error) => {
                out.push(Err(ModelError::Protocol(format!(
                    "bad stream event: {error}"
                ))));
                return;
            }
        };
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .or(event)
            .unwrap_or_default();
        let output_index = value
            .get("output_index")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        match kind {
            "response.created" | "response.in_progress" => {
                if let Some(id) = value.pointer("/response/id").and_then(Value::as_str) {
                    self.response_id.get_or_insert_with(|| id.to_owned());
                }
            }
            "response.output_item.added" => {
                if let Some(item) = value.get("item") {
                    self.ensure_call(output_index, item, out);
                }
            }
            "response.output_text.delta" => self.text_delta(output_index, &value, out),
            "response.function_call_arguments.delta" => {
                let delta = value
                    .get("delta")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match self.items.get(&output_index) {
                    Some(&index) if !delta.is_empty() => {
                        self.blocks.append_arguments(index, delta);
                        out.push(Ok(StreamEvent::ToolCallDelta {
                            index,
                            delta: delta.to_owned(),
                        }));
                    }
                    Some(_) => {}
                    None => out.push(Err(ModelError::Protocol(
                        "arguments delta for an unknown item".to_owned(),
                    ))),
                }
            }
            "response.function_call_arguments.done" => {
                if let Some(&index) = self.items.get(&output_index) {
                    if let Some(arguments) = value.get("arguments").and_then(Value::as_str) {
                        self.blocks.set_arguments(index, arguments);
                    }
                    self.blocks.end_call(index, out);
                }
            }
            "response.output_item.done" => {
                let Some(item) = value.get("item") else {
                    return;
                };
                if let Some(index) = self.ensure_call(output_index, item, out) {
                    if let Some(arguments) = item.get("arguments").and_then(Value::as_str) {
                        self.blocks.set_arguments(index, arguments);
                    }
                    self.blocks.end_call(index, out);
                }
            }
            "response.completed" | "response.incomplete" => self.complete(&value, out),
            "response.failed" => {
                let message = value
                    .pointer("/response/error/message")
                    .and_then(Value::as_str)
                    .unwrap_or("response failed");
                out.push(Err(ModelError::Protocol(message.to_owned())));
            }
            "error" => {
                let message = value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("model service reported an error");
                out.push(Err(ModelError::Protocol(message.to_owned())));
            }
            _ => {}
        }
    }

    /// Registers a `function_call` item the first time it is seen and returns its block index.
    fn ensure_call(&mut self, output_index: u64, item: &Value, out: &mut Out) -> Option<usize> {
        if item.get("type").and_then(Value::as_str) != Some("function_call") {
            return None;
        }
        if let Some(&index) = self.items.get(&output_index) {
            return Some(index);
        }
        let id = item
            .get("call_id")
            .or_else(|| item.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let index = self.blocks.push_call(id.clone(), name.clone());
        self.items.insert(output_index, index);
        out.push(Ok(StreamEvent::ToolCallStart { index, id, name }));
        Some(index)
    }

    fn text_delta(&mut self, output_index: u64, value: &Value, out: &mut Out) {
        let Some(delta) = value
            .get("delta")
            .and_then(Value::as_str)
            .filter(|delta| !delta.is_empty())
        else {
            return;
        };
        let index = match self.items.get(&output_index) {
            Some(&index) => index,
            None => {
                let index = self.blocks.push_text();
                self.items.insert(output_index, index);
                index
            }
        };
        if let Some(buffer) = self.blocks.text_mut(index) {
            buffer.push_str(delta);
        }
        out.push(Ok(StreamEvent::TextDelta {
            index,
            delta: delta.to_owned(),
        }));
    }

    fn complete(&mut self, value: &Value, out: &mut Out) {
        if std::mem::replace(&mut self.done, true) {
            return;
        }
        let response = value.get("response").unwrap_or(&Value::Null);
        let mut blocks = std::mem::take(&mut self.blocks);
        let before = out.len();
        blocks.end_all_calls(out);
        if out[before..].iter().any(Result::is_err) {
            return;
        }
        let (content, has_calls) = match blocks.into_content() {
            Ok(parts) => parts,
            Err(error) => {
                out.push(Err(error));
                return;
            }
        };
        let usage = parse_usage(response.get("usage"));
        out.push(Ok(StreamEvent::Done(AssistantMessage {
            content,
            stop_reason: stop_reason(response, has_calls),
            usage,
            cost: usage.cost(&self.pricing),
            response_id: response
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| self.response_id.take()),
        })));
    }
}

impl Decoder for ResponsesDecoder {
    fn push(&mut self, chunk: &[u8]) -> Out {
        let mut out = Vec::new();
        for sse in self.parser.push(chunk) {
            self.accept(sse.event.as_deref(), &sse.data, &mut out);
        }
        out
    }

    fn finish(&mut self) -> Out {
        let mut out = Vec::new();
        if let Some(sse) = self.parser.finish() {
            self.accept(sse.event.as_deref(), &sse.data, &mut out);
        }
        if !self.done && out.iter().all(Result::is_ok) {
            out.push(Err(ModelError::Protocol(
                "stream ended before the response completed".to_owned(),
            )));
        }
        out
    }
}
