//! System One adapter (`POST {base_url}/systemone`), following pi-ai's `typesafe-system-one`.
//! It backs the Jev classifier: typed questions in, typed answers out, no streaming and no
//! tool calls. Image input is not supported.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::classifier::{Answer, ClassifierRequest, ClassifierResult, Question};
use super::transport::Transport;
use super::types::{Pricing, Usage};
use super::{EndpointConfig, ModelError, post_json};
use crate::ports::{Classifier, SecretSource};

const LABEL: &str = "System One API";

#[derive(Debug)]
pub struct SystemOne<T, S> {
    config: EndpointConfig,
    transport: T,
    secrets: S,
}

impl<T: Transport, S: SecretSource> SystemOne<T, S> {
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
}

impl<T: Transport, S: SecretSource> Classifier for SystemOne<T, S> {
    async fn classify(&self, request: ClassifierRequest) -> Result<ClassifierResult, ModelError> {
        let response = post_json(
            &self.transport,
            &self.secrets,
            &self.config,
            "/systemone",
            &[],
            wire_request(&request),
        )
        .await?;
        let body = response.collect().await?;
        parse_response(&body, &request, &self.config.pricing)
    }
}

fn wire_request(request: &ClassifierRequest) -> Value {
    let questions: serde_json::Map<String, Value> = request
        .questions
        .iter()
        .map(|(id, question)| (id.clone(), wire_question(question)))
        .collect();
    json!({"model": request.model, "state": request.state, "questions": questions})
}

fn wire_question(question: &Question) -> Value {
    match question {
        Question::Choice {
            instructions,
            criteria,
        } => json!({"type": "choice", "instructions": instructions, "criteria": criteria}),
        Question::Score {
            instructions,
            criteria,
        } => json!({"type": "score", "instructions": instructions, "criteria": criteria}),
        // System One calls the boolean question type `noul` on the wire.
        Question::Bool {
            instructions,
            criteria,
        } => json!({
            "type": "noul",
            "instructions": instructions,
            "criteria": {"true": criteria.when_true, "false": criteria.when_false},
        }),
    }
}

fn parse_response(
    body: &[u8],
    request: &ClassifierRequest,
    pricing: &Pricing,
) -> Result<ClassifierResult, ModelError> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|error| ModelError::Protocol(format!("response is not JSON: {error}")))?;
    if !value.is_object() {
        return Err(ModelError::Protocol(format!(
            "{LABEL} returned an unexpected response"
        )));
    }

    let reported = value.get("usage").filter(|usage| {
        usage.get("input_tokens").is_some() || usage.get("output_tokens").is_some()
    });
    let usage = reported.map_or_else(Usage::default, |usage| Usage {
        input: token_count(usage.get("input_tokens")),
        output: token_count(usage.get("output_tokens")),
        cache_read: 0,
        cache_write: 0,
    });

    // A request answered with malformed answers was still billed, so its usage travels along.
    let answers = parse_answers(value.get("answers"), request).map_err(|message| {
        ModelError::MalformedAnswers {
            message,
            usage: reported.map(|_| usage),
        }
    })?;
    Ok(ClassifierResult {
        answers,
        usage,
        cost: usage.cost(pricing),
    })
}

fn token_count(value: Option<&Value>) -> u64 {
    value
        .and_then(Value::as_f64)
        .filter(|tokens| tokens.is_finite() && *tokens > 0.0)
        .map_or(0, |tokens| tokens as u64)
}

fn parse_answers(
    answers: Option<&Value>,
    request: &ClassifierRequest,
) -> Result<BTreeMap<String, Answer>, String> {
    let Some(answers) = answers.and_then(Value::as_object) else {
        return Err(format!("{LABEL} returned an unexpected response"));
    };

    let mut parsed = BTreeMap::new();
    for (id, question) in &request.questions {
        let answer = answers
            .get(id)
            .filter(|answer| answer.is_object())
            .ok_or_else(|| format!("{LABEL} did not return an answer for {id}"))?;
        let kind = answer.get("type").and_then(Value::as_str);
        let wanted = |expected: &str, name: &str| {
            if kind == Some(expected) {
                Ok(())
            } else {
                Err(format!("{LABEL} did not return a {name} answer for {id}"))
            }
        };

        let parsed_answer = match question {
            Question::Choice { .. } => {
                wanted("choice", "choice")?;
                let choice = answer
                    .get("choice")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{LABEL} did not return a choice answer for {id}"))?;
                let probabilities = answer
                    .get("probabilities")
                    .and_then(Value::as_object)
                    .ok_or_else(|| format!("{LABEL} returned invalid probabilities for {id}"))?
                    .iter()
                    .map(|(key, probability)| {
                        number(Some(probability), &format!("probability for {id}.{key}"))
                            .map(|probability| (key.clone(), probability))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?;
                Answer::Choice {
                    choice: choice.to_owned(),
                    probabilities,
                    confidence: number(answer.get("confidence"), &format!("confidence for {id}"))?,
                }
            }
            Question::Score { .. } => {
                wanted("score", "score")?;
                Answer::Score {
                    score: number(answer.get("score"), &format!("score for {id}"))?,
                    confidence: number(answer.get("confidence"), &format!("confidence for {id}"))?,
                }
            }
            Question::Bool { .. } => {
                wanted("noul", "bool")?;
                Answer::Bool {
                    probability: number(answer.get("noul"), &format!("probability for {id}"))?,
                }
            }
        };
        parsed.insert(id.clone(), parsed_answer);
    }
    Ok(parsed)
}

fn number(value: Option<&Value>, field: &str) -> Result<f64, String> {
    value
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{LABEL} returned an invalid {field}"))
}
