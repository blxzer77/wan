use futures_util::StreamExt;
use secrecy::ExposeSecret;
use serde_json::json;
use wan_core::model::chat_completions::ChatCompletions;
use wan_core::model::transport::{RecordedResponse, ReplayTransport};
use wan_core::model::types::{
    ContentBlock, Message, ModelRequest, Pricing, StopReason, StreamEvent, ToolSpec,
};
use wan_core::model::{EndpointConfig, ModelError};
use wan_core::ports::{FakeSecretSource, Model};

const STREAM_TOOL: &str = include_str!("fixtures/model/chat_stream_tool.sse");
const COMPLETE: &str = include_str!("fixtures/model/chat_complete.json");
const COMPLETE_WRAPPED: &str = include_str!("fixtures/model/chat_complete_wrapped.json");

fn adapter(response: RecordedResponse) -> ChatCompletions<ReplayTransport, FakeSecretSource> {
    let config = EndpointConfig {
        base_url: "https://gateway.example/v1/".to_owned(),
        vault_ref: "vault://model/main".to_owned(),
        pricing: Pricing {
            input: 2.0,
            output: 10.0,
            cache_read: 0.5,
            cache_write: 0.0,
        },
    };
    ChatCompletions::new(
        config,
        ReplayTransport::default().with_response(response),
        FakeSecretSource::default().with("vault://model/main", "sk-test"),
    )
}

fn request() -> ModelRequest {
    ModelRequest {
        model: "gpt-test".to_owned(),
        system: Some("be brief".to_owned()),
        messages: vec![Message::User {
            text: "read a.txt".to_owned(),
        }],
        tools: vec![ToolSpec {
            name: "read_file".to_owned(),
            description: "Read a file".to_owned(),
            parameters: json!({"type": "object"}),
        }],
        max_output_tokens: Some(256),
    }
}

#[tokio::test]
async fn replays_streaming_text_and_tool_call_for_every_chunk_size() {
    for size in [1, 7, 64, STREAM_TOOL.len()] {
        let model = adapter(RecordedResponse::chunked(200, STREAM_TOOL, size));

        let events: Vec<_> = model
            .stream(request())
            .await
            .expect("stream starts")
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<_, _>>()
            .expect("stream has no errors");

        let deltas: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, ["Checking."], "chunk size {size}");

        let Some(StreamEvent::Done(message)) = events.last() else {
            panic!("stream must end with Done (chunk size {size})");
        };
        assert_eq!(message.stop_reason, StopReason::ToolUse);
        assert_eq!(message.response_id.as_deref(), Some("chatcmpl-1"));
        let ContentBlock::ToolCall(call) = &message.content[1] else {
            panic!("second block is the tool call");
        };
        assert_eq!(call.id, "call_1");
        assert_eq!(call.arguments, json!({"path": "a.txt"}));
        assert!(events.iter().any(
            |event| matches!(event, StreamEvent::ToolCallEnd { index: 1, call: c } if c == call)
        ));

        // 1200 prompt tokens, 1000 of them cached: 200 uncached input.
        assert_eq!(message.usage.input, 200);
        assert_eq!(message.usage.cache_read, 1000);
        assert_eq!(message.usage.output, 30);
        let expected = 200.0 * 2.0 / 1e6 + 30.0 * 10.0 / 1e6 + 1000.0 * 0.5 / 1e6;
        assert!((message.cost.total - expected).abs() < 1e-12);
    }
}

#[tokio::test]
async fn sends_the_request_to_the_configured_base_url_with_the_resolved_key() {
    let model = adapter(RecordedResponse::new(200, COMPLETE));

    model.complete(request()).await.expect("completes");

    let sent = model.transport().requests();
    assert_eq!(sent[0].url, "https://gateway.example/v1/chat/completions");
    assert_eq!(sent[0].bearer.expose_secret(), "sk-test");
    assert_eq!(sent[0].body["stream"], json!(false));
    assert_eq!(sent[0].body["messages"][0]["role"], json!("system"));
    assert_eq!(
        sent[0].body["tools"][0]["function"]["name"],
        json!("read_file")
    );
    assert_eq!(sent[0].body["max_completion_tokens"], json!(256));
    assert!(!format!("{:?}", sent[0]).contains("sk-test"));
}

#[tokio::test]
async fn replays_non_streaming_response() {
    let model = adapter(RecordedResponse::new(200, COMPLETE));

    let message = model.complete(request()).await.expect("completes");

    assert_eq!(
        message.content,
        vec![ContentBlock::Text {
            text: "Hello there.".to_owned()
        }]
    );
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!((message.usage.input, message.usage.output), (10, 3));
    assert_eq!(message.response_id.as_deref(), Some("chatcmpl-2"));
}

#[tokio::test]
async fn tolerates_gateway_data_wrapper_on_non_streaming_responses() {
    let model = adapter(RecordedResponse::new(200, COMPLETE_WRAPPED));

    let message = model.complete(request()).await.expect("completes");

    assert_eq!(
        message.content,
        vec![ContentBlock::Text {
            text: "Wrapped.".to_owned()
        }]
    );
    assert_eq!(message.usage.output, 2);
}

#[tokio::test]
async fn http_errors_surface_status_and_body_without_retrying() {
    let model = adapter(RecordedResponse::new(429, "{\"error\":\"slow down\"}"));

    let error = model
        .complete(request())
        .await
        .expect_err("429 is an error");

    assert!(matches!(error, ModelError::Status { status: 429, .. }));
    assert_eq!(model.transport().requests().len(), 1);
}

#[tokio::test]
async fn truncated_stream_is_an_error_not_a_silent_success() {
    let cut = STREAM_TOOL.split("data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\"").next().unwrap();
    let model = adapter(RecordedResponse::new(200, cut));

    let results: Vec<_> = model
        .stream(request())
        .await
        .expect("stream starts")
        .collect()
        .await;

    assert!(matches!(results.last(), Some(Err(ModelError::Protocol(_)))));
}

#[tokio::test]
async fn missing_vault_ref_fails_before_any_request_is_sent() {
    let config = EndpointConfig {
        base_url: "https://gateway.example/v1".to_owned(),
        vault_ref: "vault://missing".to_owned(),
        pricing: Pricing::default(),
    };
    let model = ChatCompletions::new(
        config,
        ReplayTransport::default().with_response(RecordedResponse::new(200, COMPLETE)),
        FakeSecretSource::default(),
    );

    let error = model.complete(request()).await.expect_err("no credential");

    assert!(matches!(error, ModelError::Secret(_)));
    assert!(model.transport().requests().is_empty());
}
