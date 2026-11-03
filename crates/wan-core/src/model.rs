//! Model layer: protocol adapters, not provider adapters. Any service that speaks one of the
//! supported wire protocols works once the user supplies a base URL and a `vault_ref`.

pub(crate) mod blocks;
pub mod chat_completions;
pub mod responses;
pub mod sse;
pub mod transport;
pub mod types;

use std::collections::VecDeque;

use futures_util::StreamExt;
use futures_util::stream;
use serde_json::Value;
use thiserror::Error;

use crate::ports::{ModelStream, SecretError, SecretSource};
use transport::{BodyStream, HttpRequest, HttpResponse, Transport};
use types::{Pricing, StreamEvent};

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("credential error: {0}")]
    Secret(#[from] SecretError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("model service returned status {status}: {body}")]
    Status { status: u16, body: String },
    #[error("invalid response: {0}")]
    Protocol(String),
    #[error("timed out waiting for the model response")]
    Timeout,
}

/// Connection settings for one protocol endpoint. Only the `vault_ref` is stored; the key is
/// resolved through `SecretSource` for every request.
#[derive(Debug, Clone, PartialEq)]
pub struct EndpointConfig {
    pub base_url: String,
    pub vault_ref: String,
    pub pricing: Pricing,
}

pub(crate) type Out = Vec<Result<StreamEvent, ModelError>>;

/// Turns response body chunks into stream events without owning any I/O.
pub(crate) trait Decoder: Send + 'static {
    fn push(&mut self, chunk: &[u8]) -> Out;
    /// Called once when the body ends without the decoder having produced `Done`.
    fn finish(&mut self) -> Out;
}

/// Drives a decoder over a response body. The stream stops after `Done` or the first error.
pub(crate) fn event_stream(body: BodyStream, decoder: impl Decoder) -> ModelStream {
    struct State<D> {
        body: BodyStream,
        decoder: D,
        queue: VecDeque<Result<StreamEvent, ModelError>>,
        stopped: bool,
    }

    let state = State {
        body,
        decoder,
        queue: VecDeque::new(),
        stopped: false,
    };
    stream::unfold(state, |mut state| async move {
        loop {
            if let Some(item) = state.queue.pop_front() {
                if matches!(item, Err(_) | Ok(StreamEvent::Done(_))) {
                    state.stopped = true;
                    state.queue.clear();
                }
                return Some((item, state));
            }
            if state.stopped {
                return None;
            }
            match state.body.next().await {
                Some(Ok(chunk)) => {
                    let out = state.decoder.push(&chunk);
                    state.queue.extend(out);
                }
                Some(Err(error)) => state.queue.push_back(Err(error)),
                None => {
                    state.stopped = true;
                    let out = state.decoder.finish();
                    state.queue.extend(out);
                }
            }
        }
    })
    .boxed()
}

/// Resolves the credential, posts `body` to `{base_url}{path}`, and turns non-2xx into errors.
/// No retry happens here; the retry policy belongs to M4-07.
pub(crate) async fn post_json(
    transport: &impl Transport,
    secrets: &impl SecretSource,
    config: &EndpointConfig,
    path: &str,
    body: Value,
) -> Result<HttpResponse, ModelError> {
    let bearer = secrets.resolve(&config.vault_ref).await?;
    let url = format!("{}{path}", config.base_url.trim_end_matches('/'));
    let response = transport.post(HttpRequest { url, bearer, body }).await?;
    if response.status / 100 != 2 {
        let status = response.status;
        let body = String::from_utf8_lossy(&response.collect().await?).into_owned();
        return Err(ModelError::Status { status, body });
    }
    Ok(response)
}

/// Parses accumulated tool-call arguments; an empty string means "no arguments".
pub(crate) fn parse_arguments(raw: &str) -> Result<Value, ModelError> {
    if raw.trim().is_empty() {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str(raw)
        .map_err(|error| ModelError::Protocol(format!("tool arguments are not JSON: {error}")))
}

pub(crate) fn str_field(value: &Value, pointer: &str) -> Result<String, ModelError> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ModelError::Protocol(format!("missing {pointer}")))
}

/// Some OpenAI-compatible gateways wrap non-streaming responses as `{"data": {...}}`.
pub(crate) fn unwrap_data(value: Value) -> Value {
    match value {
        Value::Object(mut object) if object.len() == 1 && object.contains_key("data") => {
            object.remove("data").unwrap_or(Value::Null)
        }
        other => other,
    }
}
