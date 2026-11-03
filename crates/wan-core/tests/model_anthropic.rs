use futures_util::StreamExt;
use secrecy::ExposeSecret;
use serde_json::json;
use wan_core::model::anthropic::{API_VERSION, AnthropicMessages};
use wan_core::model::transport::{RecordedResponse, ReplayTransport};
use wan_core::model::types::{
    ContentBlock, Message, ModelRequest, Pricing, StopReason, StreamEvent, ToolCall, ToolSpec,
};
use wan_core::model::{EndpointConfig, ModelError};
use wan_core::ports::{FakeSecretSource, Model};

const STREAM_TOOL: &str = include_str!("fixtures/model/anthropic_stream_tool.sse");
const COMPLETE: &str = include_str!("fixtures/model/anthropic_complete.json");
const COMPLETE_TOOL: &str = include_str!("fixtures/model/anthropic_complete_tool.json");

fn adapter(response: RecordedResponse) -> AnthropicMessages<ReplayTransport, FakeSecretSource> {
    let config = EndpointConfig {
        base_url: "https://api.anthropic.com/v1/".to_owned(),
        vault_ref: "vault://model/main".to_owned(),
        pricing: Pricing {
            input: 2.0,
            output: 10.0,
            cache_read: 0.5,
            cache_write: 4.0,
        },
    };
    AnthropicMessages::new(
        config,
        ReplayTransport::default().with_response(response),
        FakeSecretSource::default().with("vault://model/main", "sk-test"),
    )
}

fn request() -> ModelRequest {
    ModelRequest {
        model: "claude-test".to_owned(),
        system: Some("be brief".to_owned()),
        messages: vec![
            Message::User {
                text: "read both files".to_owned(),
            },
            Message::Assistant {
                content: vec![
                    ContentBlock::Text {
                        text: String::new(),
                    },
                    ContentBlock::ToolCall(ToolCall {
                        id: "toolu_a".to_owned(),
                        name: "read_file".to_owned(),
                        arguments: json!({"path": "a.txt"}),
                    }),
                    ContentBlock::ToolCall(ToolCall {
                        id: "toolu_b".to_owned(),
                        name: "read_file".to_owned(),
                        arguments: json!({"path": "b.txt"}),
                    }),
                ],
            },
            Message::ToolResult {
                call_id: "toolu_a".to_owned(),
                content: "alpha".to_owned(),
                is_error: false,
            },
            Message::ToolResult {
                call_id: "toolu_b".to_owned(),
                content: "no such file".to_owned(),
                is_error: true,
            },
        ],
        tools: vec![ToolSpec {
            name: "read_file".to_owned(),
            description: "Read a file".to_owned(),
            parameters: json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        }],
        max_output_tokens: Some(256),
    }
}

#[tokio::test]
async fn replays_streaming_text_and_tool_call_for_every_chunk_size() {
    for size in [1, 7, 64, STREAM_TOOL.len()] {
        let model = adapter(RecordedResponse::chunked(200, STREAM_TOOL, size));

        let events = model
            .stream(request())
            .await
            .expect("stream starts")
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .expect("stream has no errors");

        // The thinking block gets no block, so text is block 0 and the call is block 1.
        let deltas: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::TextDelta { index, delta } => Some((*index, delta.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, [(0, "Checking.")], "chunk size {size}");
        assert!(events.iter().any(|event| matches!(
            event,
            StreamEvent::ToolCallStart { index: 1, id, name } if id == "toolu_1" && name == "read_file"
        )));
        let argument_deltas: String = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::ToolCallDelta { index: 1, delta } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(argument_deltas, "{\"path\":\"a.txt\"}");

        let Some(StreamEvent::Done(message)) = events.last() else {
            panic!("stream must end with Done (chunk size {size})");
        };
        let call = ToolCall {
            id: "toolu_1".to_owned(),
            name: "read_file".to_owned(),
            arguments: json!({"path": "a.txt"}),
        };
        assert_eq!(
            message.content,
            vec![
                ContentBlock::Text {
                    text: "Checking.".to_owned()
                },
                ContentBlock::ToolCall(call.clone()),
            ]
        );
        assert!(events.iter().any(
            |event| matches!(event, StreamEvent::ToolCallEnd { index: 1, call: c } if *c == call)
        ));
        assert_eq!(message.stop_reason, StopReason::ToolUse);
        assert_eq!(message.response_id.as_deref(), Some("msg_1"));

        // message_start carries the input counters and message_delta the final output count.
        assert_eq!(message.usage.input, 200);
        assert_eq!(message.usage.cache_read, 1000);
        assert_eq!(message.usage.output, 30);
        let expected = 200.0 * 2.0 / 1e6 + 30.0 * 10.0 / 1e6 + 1000.0 * 0.5 / 1e6;
        assert!((message.cost.total - expected).abs() < 1e-12);
    }
}

#[tokio::test]
async fn sends_version_header_bearer_key_and_the_messages_body() {
    let model = adapter(RecordedResponse::new(200, COMPLETE));

    model.complete(request()).await.expect("completes");

    let sent = model.transport().requests();
    assert_eq!(sent[0].url, "https://api.anthropic.com/v1/messages");
    assert_eq!(sent[0].bearer.expose_secret(), "sk-test");
    assert_eq!(
        sent[0].headers,
        [("anthropic-version".to_owned(), API_VERSION.to_owned())]
    );
    assert_eq!(API_VERSION, "2026-10-21");

    let body = &sent[0].body;
    assert_eq!(body["model"], json!("claude-test"));
    assert_eq!(body["max_tokens"], json!(256));
    assert_eq!(body["stream"], json!(false));
    assert_eq!(body["system"], json!("be brief"));
    assert_eq!(body["tools"][0]["name"], json!("read_file"));
    assert_eq!(body["tools"][0]["input_schema"]["type"], json!("object"));
    // API version 2026-10-21 answers 400 to these sampling parameters, so they must never be sent.
    for removed in ["temperature", "top_p", "top_k"] {
        assert!(body.get(removed).is_none(), "{removed} must not be sent");
    }
    assert!(!format!("{:?}", sent[0]).contains("sk-test"));
}

#[tokio::test]
async fn tool_results_share_one_user_message_and_empty_text_is_dropped() {
    let model = adapter(RecordedResponse::new(200, COMPLETE));

    model.complete(request()).await.expect("completes");

    let messages = model.transport().requests()[0].body["messages"].clone();
    let messages = messages.as_array().expect("messages is an array");
    assert_eq!(messages.len(), 3);
    assert_eq!(
        messages[0],
        json!({"role": "user", "content": "read both files"})
    );
    let assistant = messages[1]["content"].as_array().expect("assistant blocks");
    assert_eq!(assistant.len(), 2, "the empty text block is dropped");
    assert_eq!(assistant[0]["type"], json!("tool_use"));
    assert_eq!(assistant[0]["input"], json!({"path": "a.txt"}));
    assert_eq!(
        messages[2],
        json!({
            "role": "user",
            "content": [
                {"type": "tool_result", "tool_use_id": "toolu_a", "content": "alpha", "is_error": false},
                {"type": "tool_result", "tool_use_id": "toolu_b", "content": "no such file", "is_error": true},
            ]
        })
    );
}

#[tokio::test]
async fn max_output_tokens_is_required_and_nothing_is_sent_without_it() {
    let model = adapter(RecordedResponse::new(200, COMPLETE));
    let mut without = request();
    without.max_output_tokens = None;

    let error = model
        .complete(without)
        .await
        .expect_err("max_tokens is required");

    assert!(matches!(error, ModelError::InvalidRequest(_)));
    assert!(model.transport().requests().is_empty());
}

#[tokio::test]
async fn replays_non_streaming_text_response_and_skips_thinking() {
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
    assert_eq!(message.response_id.as_deref(), Some("msg_2"));
}

#[tokio::test]
async fn replays_non_streaming_tool_call_with_cache_counters() {
    let model = adapter(RecordedResponse::new(200, COMPLETE_TOOL));

    let message = model.complete(request()).await.expect("completes");

    assert_eq!(
        message.content,
        vec![ContentBlock::ToolCall(ToolCall {
            id: "toolu_3".to_owned(),
            name: "read_file".to_owned(),
            arguments: json!({"path": "b.txt"}),
        })]
    );
    assert_eq!(message.stop_reason, StopReason::ToolUse);
    assert_eq!(message.usage.input, 5);
    assert_eq!(message.usage.cache_read, 100);
    assert_eq!(message.usage.cache_write, 40);
    let expected = 5.0 * 2.0 / 1e6 + 2.0 * 10.0 / 1e6 + 100.0 * 0.5 / 1e6 + 40.0 * 4.0 / 1e6;
    assert!((message.cost.total - expected).abs() < 1e-12);
}

#[tokio::test]
async fn http_errors_surface_status_and_body_without_retrying() {
    let model = adapter(RecordedResponse::new(
        529,
        "{\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}",
    ));

    let error = model
        .complete(request())
        .await
        .expect_err("529 is an error");

    assert!(matches!(error, ModelError::Status { status: 529, .. }));
    assert_eq!(model.transport().requests().len(), 1);
}

#[tokio::test]
async fn error_event_in_the_stream_is_an_error() {
    let body = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_9\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\nevent: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n";
    let model = adapter(RecordedResponse::new(200, body));

    let results: Vec<_> = model
        .stream(request())
        .await
        .expect("stream starts")
        .collect()
        .await;

    assert_eq!(results.len(), 1);
    assert!(matches!(&results[0], Err(ModelError::Protocol(message)) if message == "Overloaded"));
}

#[tokio::test]
async fn truncated_stream_is_an_error_not_a_silent_success() {
    let cut = STREAM_TOOL
        .split("event: message_stop")
        .next()
        .expect("fixture has a message_stop event");
    let model = adapter(RecordedResponse::new(200, cut));

    let results: Vec<_> = model
        .stream(request())
        .await
        .expect("stream starts")
        .collect()
        .await;

    assert!(matches!(results.last(), Some(Err(ModelError::Protocol(_)))));
    assert!(
        !results
            .iter()
            .any(|item| matches!(item, Ok(StreamEvent::Done(_))))
    );
}
