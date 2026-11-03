//! Thin HTTP seam under the protocol adapters. `ReqwestTransport` talks to the network;
//! `ReplayTransport` serves recorded response bodies so adapter tests never need a network.

use std::collections::VecDeque;
use std::future::Future;
use std::sync::Mutex;

use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;

use super::ModelError;
use crate::timeouts;

pub type BodyStream = BoxStream<'static, Result<Vec<u8>, ModelError>>;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub bearer: SecretString,
    pub body: Value,
}

pub struct HttpResponse {
    pub status: u16,
    pub body: BodyStream,
}

impl HttpResponse {
    /// Reads the whole body; used for non-streaming responses and error bodies.
    pub async fn collect(mut self) -> Result<Vec<u8>, ModelError> {
        let mut all = Vec::new();
        while let Some(chunk) = self.body.next().await {
            all.extend(chunk?);
        }
        Ok(all)
    }
}

pub trait Transport: Send + Sync {
    fn post(
        &self,
        request: HttpRequest,
    ) -> impl Future<Output = Result<HttpResponse, ModelError>> + Send;
}

#[derive(Debug, Clone, Default)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transport for ReqwestTransport {
    async fn post(&self, request: HttpRequest) -> Result<HttpResponse, ModelError> {
        let send = self
            .client
            .post(&request.url)
            .bearer_auth(request.bearer.expose_secret())
            .json(&request.body)
            .send();
        let response = tokio::time::timeout(timeouts::MODEL_RESPONSE_HEADER, send)
            .await
            .map_err(|_| ModelError::Timeout)??;
        let status = response.status().as_u16();

        let chunks = response.bytes_stream();
        let body = stream::unfold(chunks, |mut chunks| async move {
            match tokio::time::timeout(timeouts::MODEL_STREAM_IDLE, chunks.next()).await {
                Err(_) => Some((Err(ModelError::Timeout), chunks)),
                Ok(None) => None,
                Ok(Some(Err(error))) => Some((Err(ModelError::Http(error)), chunks)),
                Ok(Some(Ok(bytes))) => Some((Ok(bytes.to_vec()), chunks)),
            }
        })
        .boxed();
        Ok(HttpResponse { status, body })
    }
}

/// A recorded response: status plus the body split into the chunks it arrived in.
#[derive(Debug, Clone)]
pub struct RecordedResponse {
    pub status: u16,
    pub chunks: Vec<Vec<u8>>,
}

impl RecordedResponse {
    pub fn new(status: u16, body: &str) -> Self {
        Self {
            status,
            chunks: vec![body.as_bytes().to_vec()],
        }
    }

    /// Splits the body into `size`-byte chunks to exercise incremental parsing.
    pub fn chunked(status: u16, body: &str, size: usize) -> Self {
        Self {
            status,
            chunks: body.as_bytes().chunks(size).map(<[u8]>::to_vec).collect(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ReplayTransport {
    responses: Mutex<VecDeque<RecordedResponse>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl ReplayTransport {
    pub fn with_response(self, response: RecordedResponse) -> Self {
        self.responses
            .lock()
            .expect("replay lock is not poisoned")
            .push_back(response);
        self
    }

    pub fn requests(&self) -> Vec<HttpRequest> {
        self.requests
            .lock()
            .expect("replay lock is not poisoned")
            .clone()
    }
}

impl Transport for ReplayTransport {
    async fn post(&self, request: HttpRequest) -> Result<HttpResponse, ModelError> {
        self.requests
            .lock()
            .expect("replay lock is not poisoned")
            .push(request);
        let recorded = self
            .responses
            .lock()
            .expect("replay lock is not poisoned")
            .pop_front()
            .ok_or_else(|| ModelError::Protocol("no recorded response left".to_owned()))?;
        Ok(HttpResponse {
            status: recorded.status,
            body: stream::iter(recorded.chunks.into_iter().map(Ok)).boxed(),
        })
    }
}
