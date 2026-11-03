//! OpenAI Chat Completions adapter (`POST {base_url}/chat/completions`).

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
pub struct ChatCompletions<T, S> {
    config: EndpointConfig,
    transport: T,
    secrets: S,
}

impl<T: Transport, S: SecretSource> ChatCompletions<T, S> {
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
            "/chat/completions",
            &[],
            request_body(request, stream),
        )
        .await
    }
}

impl<T: Transport, S: SecretSource> Model for ChatCompletions<T, S> {
    async fn stream(&self, request: ModelRequest) -> Result<ModelStream, ModelError> {
        let response = self.send(&request, true).await?;
        Ok(event_stream(
            response.body,
            ChatDecoder::new(self.config.pricing),
        ))
    }

    async fn complete(&self, request: ModelRequest) -> Result<AssistantMessage, ModelError> {
        let response = self.send(&request, false).await?;
        let body = response.collect().await?;
        parse_completion(&body, &self.config.pricing)
    }
}

fn request_body(request: &ModelRequest, stream: bool) -> Value {
    let mut messages = Vec::new();
    if let Some(system) = &request.system {
        messages.push(json!({"role": "system", "content": system}));
    }
    for message in &request.messages {
        messages.push(match message {
            Message::User { text } => json!({"role": "user", "content": text}),
            Message::Assistant { content } => assistant_message(content),
            Message::ToolResult {
                call_id, content, ..
            } => json!({"role": "tool", "tool_call_id": call_id, "content": content}),
        });
    }

    let mut body = json!({"model": request.model, "messages": messages, "stream": stream});
    if stream {
        body["stream_options"] = json!({"include_usage": true});
    }
    if let Some(max) = request.max_output_tokens {
        body["max_completion_tokens"] = json!(max);
    }
    if !request.tools.is_empty() {
        body["tools"] = request
            .tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters,
                    },
                })
            })
            .collect();
    }
    body
}

fn assistant_message(content: &[ContentBlock]) -> Value {
    let text: String = content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            ContentBlock::ToolCall(_) => None,
        })
        .collect();
    let calls: Vec<Value> = content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolCall(call) => Some(json!({
                "id": call.id,
                "type": "function",
                "function": {"name": call.name, "arguments": call.arguments.to_string()},
            })),
            ContentBlock::Text { .. } => None,
        })
        .collect();

    let mut message = json!({"role": "assistant", "content": text});
    if !calls.is_empty() {
        message["tool_calls"] = Value::Array(calls);
    }
    message
}

fn parse_completion(body: &[u8], pricing: &Pricing) -> Result<AssistantMessage, ModelError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| ModelError::Protocol(format!("response is not JSON: {error}")))?;
    let value = unwrap_data(value);
    let choice = value
        .pointer("/choices/0")
        .ok_or_else(|| ModelError::Protocol("response has no choices".to_owned()))?;

    let mut content = Vec::new();
    if let Some(text) = choice.pointer("/message/content").and_then(Value::as_str)
        && !text.is_empty()
    {
        content.push(ContentBlock::Text {
            text: text.to_owned(),
        });
    }
    for call in choice
        .pointer("/message/tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        content.push(ContentBlock::ToolCall(ToolCall {
            id: str_field(call, "/id")?,
            name: str_field(call, "/function/name")?,
            arguments: parse_arguments(
                call.pointer("/function/arguments")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            )?,
        }));
    }

    let usage = parse_usage(value.get("usage"));
    Ok(AssistantMessage {
        content,
        stop_reason: stop_reason(choice.get("finish_reason").and_then(Value::as_str)),
        usage,
        cost: usage.cost(pricing),
        response_id: value.get("id").and_then(Value::as_str).map(str::to_owned),
    })
}

fn stop_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("length") => StopReason::Length,
        Some("tool_calls" | "function_call") => StopReason::ToolUse,
        Some("content_filter") => StopReason::ContentFilter,
        _ => StopReason::Stop,
    }
}

/// `prompt_tokens` includes cached tokens; `Usage::input` counts only the uncached part so the
/// four buckets can be priced independently.
fn parse_usage(usage: Option<&Value>) -> Usage {
    let Some(usage) = usage else {
        return Usage::default();
    };
    let count = |pointer: &str| usage.pointer(pointer).and_then(Value::as_u64).unwrap_or(0);
    let cached = count("/prompt_tokens_details/cached_tokens");
    let written = count("/prompt_tokens_details/cache_write_tokens");
    Usage {
        input: count("/prompt_tokens").saturating_sub(cached + written),
        output: count("/completion_tokens"),
        cache_read: cached,
        cache_write: written,
    }
}

struct ChatDecoder {
    parser: SseParser,
    pricing: Pricing,
    blocks: Blocks,
    text_block: Option<usize>,
    /// Provider tool-call index -> block index.
    calls: BTreeMap<usize, usize>,
    finish_reason: Option<String>,
    usage: Option<Usage>,
    response_id: Option<String>,
    done: bool,
}

impl ChatDecoder {
    fn new(pricing: Pricing) -> Self {
        Self {
            parser: SseParser::default(),
            pricing,
            blocks: Blocks::default(),
            text_block: None,
            calls: BTreeMap::new(),
            finish_reason: None,
            usage: None,
            response_id: None,
            done: false,
        }
    }

    fn accept(&mut self, data: &str, out: &mut Out) {
        if data.trim() == "[DONE]" {
            self.close(out);
            return;
        }
        let value: Value = match serde_json::from_str(data) {
            Ok(value) => unwrap_data(value),
            Err(error) => {
                out.push(Err(ModelError::Protocol(format!(
                    "bad stream chunk: {error}"
                ))));
                return;
            }
        };
        if let Some(message) = value.pointer("/error/message").and_then(Value::as_str) {
            out.push(Err(ModelError::Protocol(message.to_owned())));
            return;
        }
        if let Some(id) = value.get("id").and_then(Value::as_str) {
            self.response_id.get_or_insert_with(|| id.to_owned());
        }
        if value.get("usage").is_some_and(|usage| !usage.is_null()) {
            self.usage = Some(parse_usage(value.get("usage")));
        }
        let Some(choice) = value.pointer("/choices/0") else {
            return;
        };
        if let Some(reason) = choice.get("finish_reason").and_then(Value::as_str) {
            self.finish_reason = Some(reason.to_owned());
        }
        if let Some(text) = choice.pointer("/delta/content").and_then(Value::as_str)
            && !text.is_empty()
        {
            let index = match self.text_block {
                Some(index) => index,
                None => {
                    let index = self.blocks.push_text();
                    self.text_block = Some(index);
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
        for call in choice
            .pointer("/delta/tool_calls")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            self.accept_tool_call(call, out);
        }
    }

    fn accept_tool_call(&mut self, call: &Value, out: &mut Out) {
        let position = call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
        let id = call.get("id").and_then(Value::as_str);
        let name = call.pointer("/function/name").and_then(Value::as_str);
        let index = match self.calls.get(&position) {
            Some(index) => {
                self.blocks.set_call_identity(*index, id, name);
                *index
            }
            None => {
                let (id, name) = (id.unwrap_or_default(), name.unwrap_or_default());
                let index = self.blocks.push_call(id.to_owned(), name.to_owned());
                self.calls.insert(position, index);
                out.push(Ok(StreamEvent::ToolCallStart {
                    index,
                    id: id.to_owned(),
                    name: name.to_owned(),
                }));
                index
            }
        };
        if let Some(delta) = call
            .pointer("/function/arguments")
            .and_then(Value::as_str)
            .filter(|delta| !delta.is_empty())
        {
            self.blocks.append_arguments(index, delta);
            out.push(Ok(StreamEvent::ToolCallDelta {
                index,
                delta: delta.to_owned(),
            }));
        }
    }

    fn close(&mut self, out: &mut Out) {
        if std::mem::replace(&mut self.done, true) {
            return;
        }
        let mut blocks = std::mem::take(&mut self.blocks);
        let first_error = out.len();
        blocks.end_all_calls(out);
        if out[first_error..].iter().any(Result::is_err) {
            return;
        }
        let content = match blocks.into_content() {
            Ok((content, _)) => content,
            Err(error) => {
                out.push(Err(error));
                return;
            }
        };
        let usage = self.usage.unwrap_or_default();
        out.push(Ok(StreamEvent::Done(AssistantMessage {
            content,
            stop_reason: stop_reason(self.finish_reason.as_deref()),
            usage,
            cost: usage.cost(&self.pricing),
            response_id: self.response_id.take(),
        })));
    }
}

impl Decoder for ChatDecoder {
    fn push(&mut self, chunk: &[u8]) -> Out {
        let mut out = Vec::new();
        for sse in self.parser.push(chunk) {
            self.accept(&sse.data, &mut out);
        }
        out
    }

    fn finish(&mut self) -> Out {
        let mut out = Vec::new();
        if let Some(sse) = self.parser.finish() {
            self.accept(&sse.data, &mut out);
        }
        if !self.done {
            if self.finish_reason.is_some() {
                self.close(&mut out);
            } else {
                out.push(Err(ModelError::Protocol(
                    "stream ended before the response completed".to_owned(),
                )));
            }
        }
        out
    }
}
