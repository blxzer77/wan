//! One suite, all four protocol adapters. Each case runs against every `Protocol`; the
//! `match` statements are exhaustive, so adding a protocol fails to compile until it is covered.

use std::collections::BTreeMap;

use secrecy::ExposeSecret;
use serde_json::json;
use wan_core::model::anthropic::AnthropicMessages;
use wan_core::model::chat_completions::ChatCompletions;
use wan_core::model::classifier::{BoolCriteria, ClassifierRequest, Question};
use wan_core::model::pool::Protocol;
use wan_core::model::responses::Responses;
use wan_core::model::system_one::SystemOne;
use wan_core::model::transport::{HttpRequest, RecordedResponse, ReplayTransport};
use wan_core::model::types::{Message, ModelRequest, Pricing};
use wan_core::model::{EndpointConfig, ModelError};
use wan_core::ports::{Classifier, FakeSecretSource, Model};

const ALL: [Protocol; 4] = [
    Protocol::Responses,
    Protocol::ChatCompletions,
    Protocol::AnthropicMessages,
    Protocol::SystemOne,
];

const SECRET: &str = "sk-conformance-secret";
const VAULT_REF: &str = "vault://model/conformance";

struct Outcome {
    result: Result<(), ModelError>,
    requests: Vec<HttpRequest>,
}

fn path(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Responses => "/responses",
        Protocol::ChatCompletions => "/chat/completions",
        Protocol::AnthropicMessages => "/messages",
        Protocol::SystemOne => "/systemone",
    }
}

fn success_body(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Responses => include_str!("fixtures/model/responses_complete.json"),
        Protocol::ChatCompletions => include_str!("fixtures/model/chat_complete.json"),
        Protocol::AnthropicMessages => include_str!("fixtures/model/anthropic_complete.json"),
        Protocol::SystemOne => include_str!("fixtures/model/system_one_answers.json"),
    }
}

fn chat_request() -> ModelRequest {
    ModelRequest {
        model: "model-under-test".to_owned(),
        system: None,
        messages: vec![Message::User {
            text: "hello".to_owned(),
        }],
        tools: Vec::new(),
        max_output_tokens: Some(64),
    }
}

fn classifier_request() -> ClassifierRequest {
    ClassifierRequest {
        model: "model-under-test".to_owned(),
        state: json!({"message": "hello"}),
        questions: BTreeMap::from([(
            "approved".to_owned(),
            Question::Bool {
                instructions: "Does the user approve?".to_owned(),
                criteria: BoolCriteria {
                    when_true: "Approval".to_owned(),
                    when_false: "No approval".to_owned(),
                },
            },
        )]),
    }
}

/// Makes one non-streaming call through the adapter for `protocol`.
async fn call(
    protocol: Protocol,
    base_url: &str,
    stored_refs: &[(&str, &str)],
    response: RecordedResponse,
) -> Outcome {
    let config = EndpointConfig {
        base_url: base_url.to_owned(),
        vault_ref: VAULT_REF.to_owned(),
        pricing: Pricing::default(),
    };
    let transport = ReplayTransport::default().with_response(response);
    let secrets = stored_refs.iter().fold(
        FakeSecretSource::default(),
        |secrets, (vault_ref, secret)| secrets.with(vault_ref, secret),
    );

    match protocol {
        Protocol::Responses => {
            let model = Responses::new(config, transport, secrets);
            let result = model.complete(chat_request()).await.map(|_| ());
            Outcome {
                result,
                requests: model.transport().requests(),
            }
        }
        Protocol::ChatCompletions => {
            let model = ChatCompletions::new(config, transport, secrets);
            let result = model.complete(chat_request()).await.map(|_| ());
            Outcome {
                result,
                requests: model.transport().requests(),
            }
        }
        Protocol::AnthropicMessages => {
            let model = AnthropicMessages::new(config, transport, secrets);
            let result = model.complete(chat_request()).await.map(|_| ());
            Outcome {
                result,
                requests: model.transport().requests(),
            }
        }
        Protocol::SystemOne => {
            let model = SystemOne::new(config, transport, secrets);
            let result = model.classify(classifier_request()).await.map(|_| ());
            Outcome {
                result,
                requests: model.transport().requests(),
            }
        }
    }
}

#[tokio::test]
async fn every_adapter_accepts_its_recorded_success_response() {
    for protocol in ALL {
        let outcome = call(
            protocol,
            "https://gateway.example/v1",
            &[(VAULT_REF, SECRET)],
            RecordedResponse::new(200, success_body(protocol)),
        )
        .await;

        assert!(outcome.result.is_ok(), "{protocol:?}: {:?}", outcome.result);
        assert_eq!(outcome.requests.len(), 1, "{protocol:?}");
    }
}

#[tokio::test]
async fn every_adapter_posts_to_its_path_under_the_user_base_url() {
    for protocol in ALL {
        for base_url in ["https://gateway.example/v1", "https://gateway.example/v1/"] {
            let outcome = call(
                protocol,
                base_url,
                &[(VAULT_REF, SECRET)],
                RecordedResponse::new(200, success_body(protocol)),
            )
            .await;

            assert_eq!(
                outcome.requests[0].url,
                format!("https://gateway.example/v1{}", path(protocol)),
                "{protocol:?} with base url {base_url}"
            );
        }
    }
}

#[tokio::test]
async fn every_adapter_gets_its_key_only_through_the_vault_ref() {
    for protocol in ALL {
        let outcome = call(
            protocol,
            "https://gateway.example/v1",
            &[(VAULT_REF, SECRET), ("vault://other", "sk-someone-elses")],
            RecordedResponse::new(200, success_body(protocol)),
        )
        .await;

        let sent = &outcome.requests[0];
        assert_eq!(sent.bearer.expose_secret(), SECRET, "{protocol:?}");
        assert!(
            !format!("{sent:?}").contains(SECRET),
            "{protocol:?}: Debug output must not reveal the key"
        );
        assert!(
            !sent.body.to_string().contains(SECRET),
            "{protocol:?}: the key must not appear in the request body"
        );
    }
}

#[tokio::test]
async fn every_adapter_fails_before_sending_when_the_vault_ref_is_unknown() {
    for protocol in ALL {
        let outcome = call(
            protocol,
            "https://gateway.example/v1",
            &[("vault://other", "sk-someone-elses")],
            RecordedResponse::new(200, success_body(protocol)),
        )
        .await;

        assert!(
            matches!(outcome.result, Err(ModelError::Secret(_))),
            "{protocol:?}: {:?}",
            outcome.result
        );
        assert!(outcome.requests.is_empty(), "{protocol:?}");
    }
}

#[tokio::test]
async fn every_adapter_surfaces_http_errors_without_retrying() {
    for protocol in ALL {
        let outcome = call(
            protocol,
            "https://gateway.example/v1",
            &[(VAULT_REF, SECRET)],
            RecordedResponse::new(500, "{\"error\":\"boom\"}"),
        )
        .await;

        assert!(
            matches!(
                &outcome.result,
                Err(ModelError::Status { status: 500, body }) if body.contains("boom")
            ),
            "{protocol:?}: {:?}",
            outcome.result
        );
        assert_eq!(outcome.requests.len(), 1, "{protocol:?} must not retry");
    }
}

#[tokio::test]
async fn every_adapter_rejects_a_success_status_with_a_non_json_body() {
    for protocol in ALL {
        let outcome = call(
            protocol,
            "https://gateway.example/v1",
            &[(VAULT_REF, SECRET)],
            RecordedResponse::new(200, "<html>gateway error page</html>"),
        )
        .await;

        assert!(
            matches!(outcome.result, Err(ModelError::Protocol(_))),
            "{protocol:?}: {:?}",
            outcome.result
        );
    }
}
