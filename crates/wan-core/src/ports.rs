//! Port traits owned by wan-core. Adapter crates never depend on this module; `wand` wraps
//! each adapter in a local newtype that implements the matching port (ADR 0002).

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::sync::Mutex;

use futures_util::stream::{self, BoxStream};
use secrecy::SecretString;
use thiserror::Error;

use crate::model::ModelError;
use crate::model::classifier::{ClassifierRequest, ClassifierResult};
use crate::model::types::{AssistantMessage, ModelRequest, StreamEvent};

pub type ModelStream = BoxStream<'static, Result<StreamEvent, ModelError>>;

pub trait Model: Send + Sync {
    /// Starts a streaming completion. The stream ends with `StreamEvent::Done`.
    fn stream(
        &self,
        request: ModelRequest,
    ) -> impl Future<Output = Result<ModelStream, ModelError>> + Send;

    /// Runs a non-streaming completion.
    fn complete(
        &self,
        request: ModelRequest,
    ) -> impl Future<Output = Result<AssistantMessage, ModelError>> + Send;
}

/// Answers typed questions about a JSON state (System One / Jev). Not a chat model: no
/// streaming and no tool calls.
pub trait Classifier: Send + Sync {
    fn classify(
        &self,
        request: ClassifierRequest,
    ) -> impl Future<Output = Result<ClassifierResult, ModelError>> + Send;
}

#[derive(Debug, Error)]
pub enum SecretError {
    #[error("no secret is stored for vault_ref {0}")]
    NotFound(String),
    #[error("the vault is locked")]
    Locked,
}

/// Resolves a `vault_ref` to the secret it names. It is the only way model adapters obtain
/// credentials; the value is never stored in settings, logs, or step records.
pub trait SecretSource: Send + Sync {
    fn resolve(
        &self,
        vault_ref: &str,
    ) -> impl Future<Output = Result<SecretString, SecretError>> + Send;
}

/// In-memory `SecretSource` for tests.
#[derive(Debug, Default)]
pub struct FakeSecretSource {
    secrets: HashMap<String, String>,
}

impl FakeSecretSource {
    pub fn with(mut self, vault_ref: &str, secret: &str) -> Self {
        self.secrets.insert(vault_ref.to_owned(), secret.to_owned());
        self
    }
}

impl SecretSource for FakeSecretSource {
    async fn resolve(&self, vault_ref: &str) -> Result<SecretString, SecretError> {
        self.secrets
            .get(vault_ref)
            .map(|secret| SecretString::from(secret.as_str()))
            .ok_or_else(|| SecretError::NotFound(vault_ref.to_owned()))
    }
}

/// In-memory `Classifier` for tests: replays queued results and records the requests it saw.
#[derive(Debug, Default)]
pub struct FakeClassifier {
    results: Mutex<VecDeque<ClassifierResult>>,
    requests: Mutex<Vec<ClassifierRequest>>,
}

impl FakeClassifier {
    pub fn with_result(self, result: ClassifierResult) -> Self {
        self.results
            .lock()
            .expect("fake classifier lock is not poisoned")
            .push_back(result);
        self
    }

    pub fn requests(&self) -> Vec<ClassifierRequest> {
        self.requests
            .lock()
            .expect("fake classifier lock is not poisoned")
            .clone()
    }
}

impl Classifier for FakeClassifier {
    async fn classify(&self, request: ClassifierRequest) -> Result<ClassifierResult, ModelError> {
        self.requests
            .lock()
            .expect("fake classifier lock is not poisoned")
            .push(request);
        self.results
            .lock()
            .expect("fake classifier lock is not poisoned")
            .pop_front()
            .ok_or_else(|| ModelError::Protocol("fake classifier has no queued result".to_owned()))
    }
}

/// In-memory `Model` for tests: replays queued responses and records the requests it saw.
#[derive(Debug, Default)]
pub struct FakeModel {
    responses: Mutex<VecDeque<Vec<StreamEvent>>>,
    requests: Mutex<Vec<ModelRequest>>,
}

impl FakeModel {
    pub fn with_response(self, events: Vec<StreamEvent>) -> Self {
        self.responses
            .lock()
            .expect("fake model lock is not poisoned")
            .push_back(events);
        self
    }

    pub fn requests(&self) -> Vec<ModelRequest> {
        self.requests
            .lock()
            .expect("fake model lock is not poisoned")
            .clone()
    }

    fn next_response(&self, request: ModelRequest) -> Result<Vec<StreamEvent>, ModelError> {
        self.requests
            .lock()
            .expect("fake model lock is not poisoned")
            .push(request);
        self.responses
            .lock()
            .expect("fake model lock is not poisoned")
            .pop_front()
            .ok_or_else(|| ModelError::Protocol("fake model has no queued response".to_owned()))
    }
}

impl Model for FakeModel {
    async fn stream(&self, request: ModelRequest) -> Result<ModelStream, ModelError> {
        let events = self.next_response(request)?;
        Ok(Box::pin(stream::iter(events.into_iter().map(Ok))))
    }

    async fn complete(&self, request: ModelRequest) -> Result<AssistantMessage, ModelError> {
        self.next_response(request)?
            .into_iter()
            .find_map(|event| match event {
                StreamEvent::Done(message) => Some(message),
                _ => None,
            })
            .ok_or_else(|| ModelError::Protocol("fake response has no final message".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::types::{ContentBlock, Cost, StopReason, Usage};
    use futures_util::StreamExt;
    use secrecy::ExposeSecret;

    fn request() -> ModelRequest {
        ModelRequest {
            model: "test-model".to_owned(),
            system: None,
            messages: Vec::new(),
            tools: Vec::new(),
            max_output_tokens: None,
        }
    }

    fn done() -> AssistantMessage {
        AssistantMessage {
            content: vec![ContentBlock::Text {
                text: "hi".to_owned(),
            }],
            stop_reason: StopReason::Stop,
            usage: Usage::default(),
            cost: Cost::default(),
            response_id: None,
        }
    }

    #[tokio::test]
    async fn fake_secret_source_resolves_only_known_refs() {
        let source = FakeSecretSource::default().with("vault://model/main", "sk-test");

        let secret = source
            .resolve("vault://model/main")
            .await
            .expect("known ref resolves");
        assert_eq!(secret.expose_secret(), "sk-test");
        assert!(matches!(
            source.resolve("vault://missing").await,
            Err(SecretError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn fake_model_replays_queued_events_and_records_requests() {
        let model = FakeModel::default()
            .with_response(vec![
                StreamEvent::TextDelta {
                    index: 0,
                    delta: "hi".to_owned(),
                },
                StreamEvent::Done(done()),
            ])
            .with_response(vec![StreamEvent::Done(done())]);

        let events: Vec<_> = model
            .stream(request())
            .await
            .expect("stream starts")
            .collect()
            .await;
        assert_eq!(events.len(), 2);
        assert_eq!(model.complete(request()).await.expect("completes"), done());
        assert_eq!(model.requests().len(), 2);
        assert!(model.complete(request()).await.is_err());
    }
}
