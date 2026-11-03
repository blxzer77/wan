use std::collections::BTreeMap;

use secrecy::ExposeSecret;
use serde_json::json;
use wan_core::model::classifier::{Answer, BoolCriteria, ClassifierRequest, Question};
use wan_core::model::system_one::SystemOne;
use wan_core::model::transport::{RecordedResponse, ReplayTransport};
use wan_core::model::types::{Pricing, Usage};
use wan_core::model::{EndpointConfig, ModelError};
use wan_core::ports::{Classifier, FakeSecretSource};

const ANSWERS: &str = include_str!("fixtures/model/system_one_answers.json");

fn adapter(response: RecordedResponse) -> SystemOne<ReplayTransport, FakeSecretSource> {
    let config = EndpointConfig {
        base_url: "https://api.typesafe.ai/v1/".to_owned(),
        vault_ref: "vault://model/jev".to_owned(),
        pricing: Pricing {
            input: 1.0,
            output: 4.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
    };
    SystemOne::new(
        config,
        ReplayTransport::default().with_response(response),
        FakeSecretSource::default().with("vault://model/jev", "ts-test"),
    )
}

fn request() -> ClassifierRequest {
    let mut questions = BTreeMap::new();
    questions.insert(
        "category".to_owned(),
        Question::Choice {
            instructions: "Classify the message.".to_owned(),
            criteria: BTreeMap::from([
                ("success".to_owned(), "The change works".to_owned()),
                ("failure".to_owned(), "The change is broken".to_owned()),
            ]),
        },
    );
    questions.insert(
        "satisfaction".to_owned(),
        Question::Score {
            instructions: "Score user satisfaction.".to_owned(),
            criteria: vec![
                "dissatisfied".to_owned(),
                "neutral".to_owned(),
                "satisfied".to_owned(),
            ],
        },
    );
    questions.insert(
        "approved".to_owned(),
        Question::Bool {
            instructions: "Does the user approve?".to_owned(),
            criteria: BoolCriteria {
                when_true: "Approval".to_owned(),
                when_false: "No approval".to_owned(),
            },
        },
    );
    ClassifierRequest {
        model: "jev-latest".to_owned(),
        state: json!({"message": "The change works perfectly, thanks."}),
        questions,
    }
}

#[tokio::test]
async fn sends_bool_questions_as_noul_to_the_systemone_endpoint() {
    let model = adapter(RecordedResponse::new(200, ANSWERS));

    model.classify(request()).await.expect("classifies");

    let sent = model.transport().requests();
    assert_eq!(sent[0].url, "https://api.typesafe.ai/v1/systemone");
    assert_eq!(sent[0].bearer.expose_secret(), "ts-test");
    assert!(sent[0].headers.is_empty());
    let body = &sent[0].body;
    assert_eq!(body["model"], json!("jev-latest"));
    assert_eq!(
        body["state"],
        json!({"message": "The change works perfectly, thanks."})
    );
    assert_eq!(
        body["questions"]["approved"],
        json!({
            "type": "noul",
            "instructions": "Does the user approve?",
            "criteria": {"true": "Approval", "false": "No approval"},
        })
    );
    assert_eq!(body["questions"]["category"]["type"], json!("choice"));
    assert_eq!(
        body["questions"]["category"]["criteria"]["success"],
        json!("The change works")
    );
    assert_eq!(
        body["questions"]["satisfaction"]["criteria"],
        json!(["dissatisfied", "neutral", "satisfied"])
    );
    assert!(!format!("{:?}", sent[0]).contains("ts-test"));
}

#[tokio::test]
async fn maps_noul_answers_to_bool_and_prices_the_reported_usage() {
    let model = adapter(RecordedResponse::new(200, ANSWERS));

    let result = model.classify(request()).await.expect("classifies");

    assert_eq!(
        result.answers["approved"],
        Answer::Bool { probability: 0.95 }
    );
    assert_eq!(
        result.answers["satisfaction"],
        Answer::Score {
            score: 2.0,
            confidence: 0.7
        }
    );
    assert_eq!(
        result.answers["category"],
        Answer::Choice {
            choice: "success".to_owned(),
            probabilities: BTreeMap::from([
                ("failure".to_owned(), 0.1),
                ("success".to_owned(), 0.9),
            ]),
            confidence: 0.8,
        }
    );
    assert_eq!((result.usage.input, result.usage.output), (308, 23));
    let expected = 308.0 / 1e6 + 23.0 * 4.0 / 1e6;
    assert!((result.cost.total - expected).abs() < 1e-12);
}

#[tokio::test]
async fn usage_is_zero_when_the_service_reports_none() {
    let body = ANSWERS.replace(",\"usage\":{\"input_tokens\":308,\"output_tokens\":23}", "");
    let model = adapter(RecordedResponse::new(200, &body));

    let result = model.classify(request()).await.expect("classifies");

    assert_eq!(result.usage, Usage::default());
    assert_eq!(result.cost.total, 0.0);
}

#[tokio::test]
async fn malformed_answers_still_report_the_billed_usage() {
    let body = ANSWERS.replace(
        "\"approved\":{\"type\":\"noul\",\"noul\":0.95}",
        "\"other\":{}",
    );
    let model = adapter(RecordedResponse::new(200, &body));

    let error = model
        .classify(request())
        .await
        .expect_err("answer is missing");

    let ModelError::MalformedAnswers { message, usage } = error else {
        panic!("expected malformed answers, got {error:?}");
    };
    assert!(
        message.contains("did not return an answer for approved"),
        "{message}"
    );
    assert_eq!(
        usage.map(|usage| (usage.input, usage.output)),
        Some((308, 23))
    );
}

#[tokio::test]
async fn wrong_answer_type_is_rejected() {
    let body = ANSWERS.replace(
        "\"approved\":{\"type\":\"noul\",\"noul\":0.95}",
        "\"approved\":{\"type\":\"score\",\"score\":1,\"confidence\":0.5}",
    );
    let model = adapter(RecordedResponse::new(200, &body));

    let error = model.classify(request()).await.expect_err("type mismatch");

    assert!(matches!(
        error,
        ModelError::MalformedAnswers { message, .. } if message.contains("bool answer for approved")
    ));
}

#[tokio::test]
async fn non_numeric_probability_is_rejected() {
    let body = ANSWERS.replace("\"noul\":0.95", "\"noul\":\"high\"");
    let model = adapter(RecordedResponse::new(200, &body));

    let error = model.classify(request()).await.expect_err("bad number");

    assert!(matches!(
        error,
        ModelError::MalformedAnswers { message, .. } if message.contains("invalid probability for approved")
    ));
}

#[tokio::test]
async fn non_json_body_is_a_protocol_error() {
    let model = adapter(RecordedResponse::new(200, "<html>gateway</html>"));

    let error = model.classify(request()).await.expect_err("not json");

    assert!(matches!(error, ModelError::Protocol(_)));
}
