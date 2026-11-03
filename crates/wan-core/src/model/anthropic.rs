//! Anthropic Messages adapter (`POST {base_url}/messages`).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::blocks::Blocks;
use super::sse::SseParser;
use super::transport::{HttpResponse, Transport};
use super::types::{
    AssistantMessage, ContentBlock, Message, ModelRequest, Pricing, StopReason, StreamEvent,
    ToolCall, Usage,
};
use super::{Decoder, EndpointConfig, ModelError, Out, event_stream, post_json, str_field};
use crate::ports::{Model, ModelStream, SecretSource};

/// Sent as `anthropic-version`; the API requires the header on every request.
pub const API_VERSION: &str = "2026-10-21";

#[derive(Debug)]
pub struct AnthropicMessages<T, S> {
    config: EndpointConfig,
    transport: T,
    secrets: S,
}

impl<T: Transport, S: SecretSource> AnthropicMessages<T, S> {
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

    async fn send(&self, body: Value) -> Result<HttpResponse, ModelError> {
        post_json(
            &self.transport,
            &self.secrets,
            &self.config,
            "/messages",
            &[("anthropic-version", API_VERSION)],
            body,
        )
        .await
    }
}

impl<T: Transport, S: SecretSource> Model for AnthropicMessages<T, S> {
    async fn stream(&self, request: ModelRequest) -> Result<ModelStream, ModelError> {
        let response = self.send(request_body(&request, true)?).await?;
        Ok(event_stream(
            response.body,
            AnthropicDecoder::new(self.config.pricing),
        ))
    }

    async fn complete(&self, request: ModelRequest) -> Result<AssistantMessage, ModelError> {
        let response = self.send(request_body(&request, false)?).await?;
        let body = response.collect().await?;
        parse_completion(&body, &self.config.pricing)
    }
}

fn request_body(request: &ModelRequest, stream: bool) -> Result<Value, ModelError> {
    // The API has no default for `max_tokens`; inventing one here would hide a config gap.
    let max_tokens = request.max_output_tokens.ok_or_else(|| {
        ModelError::InvalidRequest("Anthropic Messages requires max_output_tokens".to_owned())
    })?;

    let mut messages: Vec<Value> = Vec::new();
    for message in &request.messages {
        match message {
            Message::User { text } => messages.push(json!({"role": "user", "content": text})),
            Message::Assistant { content } => {
                messages.push(json!({"role": "assistant", "content": assistant_blocks(content)}))
            }
            Message::ToolResult {
                call_id,
                content,
                is_error,
            } => push_tool_result(
                &mut messages,
                json!({
                    "type": "tool_result",
                    "tool_use_id": call_id,
                    "content": content,
                    "is_error": is_error,
                }),
            ),
        }
    }

    let mut body = json!({
        "model": request.model,
        "max_tokens": max_tokens,
        "messages": messages,
        "stream": stream,
    });
    if let Some(system) = &request.system {
        body["system"] = json!(system);
    }
    if !request.tools.is_empty() {
        body["tools"] = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.parameters,
                })
            })
            .collect();
    }
    Ok(body)
}

fn assistant_blocks(content: &[ContentBlock]) -> Vec<Value> {
    content
        .iter()
        .filter_map(|block| match block {
            // The API rejects empty text blocks.
            ContentBlock::Text { text } if text.is_empty() => None,
            ContentBlock::Text { text } => Some(json!({"type": "text", "text": text})),
            ContentBlock::ToolCall(call) => Some(json!({
                "type": "tool_use",
                "id": call.id,
                "name": call.name,
                "input": call.arguments,
            })),
        })
        .collect()
}

/// Results for one assistant turn must share a single user message.
fn push_tool_result(messages: &mut Vec<Value>, block: Value) {
    if let Some(content) = messages.last_mut().and_then(open_tool_results) {
        content.push(block);
        return;
    }
    messages.push(json!({"role": "user", "content": [block]}));
}

fn open_tool_results(message: &mut Value) -> Option<&mut Vec<Value>> {
    let holds_results = message["role"] == "user"
        && message["content"]
            .as_array()
            .and_then(|content| content.first())
            .is_some_and(|block| block["type"] == "tool_result");
    if !holds_results {
        return None;
    }
    message["content"].as_array_mut()
}

fn parse_completion(body: &[u8], pricing: &Pricing) -> Result<AssistantMessage, ModelError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| ModelError::Protocol(format!("response is not JSON: {error}")))?;
    if value.get("type").and_then(Value::as_str) == Some("error") {
        return Err(error_event(&value));
    }

    let mut content = Vec::new();
    for block in value
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                let text = block
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !text.is_empty() {
                    content.push(ContentBlock::Text {
                        text: text.to_owned(),
                    });
                }
            }
            Some("tool_use") => content.push(ContentBlock::ToolCall(ToolCall {
                id: str_field(block, "/id")?,
                name: str_field(block, "/name")?,
                arguments: block.get("input").cloned().unwrap_or_else(|| json!({})),
            })),
            _ => {}
        }
    }

    let mut usage = Usage::default();
    merge_usage(&mut usage, value.get("usage"));
    Ok(AssistantMessage {
        content,
        stop_reason: stop_reason(value.get("stop_reason").and_then(Value::as_str)),
        usage,
        cost: usage.cost(pricing),
        response_id: value.get("id").and_then(Value::as_str).map(str::to_owned),
    })
}

fn error_event(value: &Value) -> ModelError {
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("model service reported an error");
    ModelError::Protocol(message.to_owned())
}

fn stop_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("max_tokens" | "model_context_window_exceeded") => StopReason::Length,
        Some("tool_use") => StopReason::ToolUse,
        Some("refusal") => StopReason::ContentFilter,
        _ => StopReason::Stop,
    }
}

/// Anthropic reports uncached input, cache reads, and cache writes as separate counters, so
/// they map straight onto `Usage`. A field that is absent keeps its previous value, because
/// `message_start` and `message_delta` each carry only part of the totals.
fn merge_usage(usage: &mut Usage, reported: Option<&Value>) {
    let Some(reported) = reported else {
        return;
    };
    let read = |field: &str| reported.get(field).and_then(Value::as_u64);
    if let Some(tokens) = read("input_tokens") {
        usage.input = tokens;
    }
    if let Some(tokens) = read("output_tokens") {
        usage.output = tokens;
    }
    if let Some(tokens) = read("cache_read_input_tokens") {
        usage.cache_read = tokens;
    }
    if let Some(tokens) = read("cache_creation_input_tokens") {
        usage.cache_write = tokens;
    }
}

struct AnthropicDecoder {
    parser: SseParser,
    pricing: Pricing,
    blocks: Blocks,
    /// Provider block index -> block index. Thinking and other blocks the core does not model
    /// get no entry, so the numbering matches the final message content.
    mapped: BTreeMap<u64, usize>,
    usage: Usage,
    stop_reason: Option<String>,
    response_id: Option<String>,
    done: bool,
}

impl AnthropicDecoder {
    fn new(pricing: Pricing) -> Self {
        Self {
            parser: SseParser::default(),
            pricing,
            blocks: Blocks::default(),
            mapped: BTreeMap::new(),
            usage: Usage::default(),
            stop_reason: None,
            response_id: None,
            done: false,
        }
    }

    fn accept(&mut self, event: Option<&str>, data: &str, out: &mut Out) {
        let value: Value = match serde_json::from_str(data) {
            Ok(value) => value,
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
        let provider_index = value.get("index").and_then(Value::as_u64).unwrap_or(0);
        match kind {
            "message_start" => {
                if let Some(id) = value.pointer("/message/id").and_then(Value::as_str) {
                    self.response_id = Some(id.to_owned());
                }
                merge_usage(&mut self.usage, value.pointer("/message/usage"));
            }
            "content_block_start" => self.block_start(
                provider_index,
                value.get("content_block").unwrap_or(&Value::Null),
                out,
            ),
            "content_block_delta" => self.block_delta(
                provider_index,
                value.get("delta").unwrap_or(&Value::Null),
                out,
            ),
            "content_block_stop" => {
                if let Some(&index) = self.mapped.get(&provider_index) {
                    self.blocks.end_call(index, out);
                }
            }
            "message_delta" => {
                if let Some(reason) = value.pointer("/delta/stop_reason").and_then(Value::as_str) {
                    self.stop_reason = Some(reason.to_owned());
                }
                merge_usage(&mut self.usage, value.get("usage"));
            }
            "message_stop" => self.complete(out),
            "error" => out.push(Err(error_event(&value))),
            _ => {}
        }
    }

    fn block_start(&mut self, provider_index: u64, block: &Value, out: &mut Out) {
        match block.get("type").and_then(Value::as_str) {
            Some("tool_use") => {
                let id = block
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let name = block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let index = self.blocks.push_call(id.clone(), name.clone());
                self.mapped.insert(provider_index, index);
                out.push(Ok(StreamEvent::ToolCallStart { index, id, name }));
            }
            Some("text") => {
                let text = block
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                self.text_delta(provider_index, text, out);
            }
            _ => {}
        }
    }

    fn block_delta(&mut self, provider_index: u64, delta: &Value, out: &mut Out) {
        match delta.get("type").and_then(Value::as_str) {
            Some("text_delta") => {
                let text = delta
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                self.text_delta(provider_index, text, out);
            }
            Some("input_json_delta") => {
                let partial = delta
                    .get("partial_json")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match self.mapped.get(&provider_index) {
                    Some(&index) if !partial.is_empty() => {
                        self.blocks.append_arguments(index, partial);
                        out.push(Ok(StreamEvent::ToolCallDelta {
                            index,
                            delta: partial.to_owned(),
                        }));
                    }
                    Some(_) => {}
                    None => out.push(Err(ModelError::Protocol(
                        "arguments delta for an unknown block".to_owned(),
                    ))),
                }
            }
            _ => {}
        }
    }

    /// Creates the text block on the first non-empty text so the content never holds an empty one.
    fn text_delta(&mut self, provider_index: u64, text: &str, out: &mut Out) {
        if text.is_empty() {
            return;
        }
        let index = match self.mapped.get(&provider_index) {
            Some(&index) => index,
            None => {
                let index = self.blocks.push_text();
                self.mapped.insert(provider_index, index);
                index
            }
        };
        if let Some(buffer) = self.blocks.text_mut(index) {
            buffer.push_str(text);
        }
        out.push(Ok(StreamEvent::TextDelta {
            index,
            delta: text.to_owned(),
        }));
    }

    fn complete(&mut self, out: &mut Out) {
        if std::mem::replace(&mut self.done, true) {
            return;
        }
        let mut blocks = std::mem::take(&mut self.blocks);
        let before = out.len();
        blocks.end_all_calls(out);
        if out[before..].iter().any(Result::is_err) {
            return;
        }
        let content = match blocks.into_content() {
            Ok((content, _)) => content,
            Err(error) => {
                out.push(Err(error));
                return;
            }
        };
        out.push(Ok(StreamEvent::Done(AssistantMessage {
            content,
            stop_reason: stop_reason(self.stop_reason.as_deref()),
            usage: self.usage,
            cost: self.usage.cost(&self.pricing),
            response_id: self.response_id.take(),
        })));
    }
}

impl Decoder for AnthropicDecoder {
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
