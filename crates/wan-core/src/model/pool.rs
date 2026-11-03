//! Model pool configuration. Settings hold the pool and `vault_ref`s only; a bot refers to
//! pool entries by name for its main, review, and memory roles, and one optional global Jev
//! classifier is shared. There is no automatic fallback between models in v1.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::EndpointConfig;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Responses,
    ChatCompletions,
    AnthropicMessages,
    SystemOne,
}

impl Protocol {
    /// Chat protocols stream messages and call tools; System One only classifies.
    pub fn is_chat(self) -> bool {
        !matches!(self, Self::SystemOne)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    pub protocol: Protocol,
    /// Model id sent to the service.
    pub model: String,
    pub endpoint: EndpointConfig,
    pub max_output_tokens: Option<u32>,
}

/// The pool names a bot uses for each role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotModels {
    pub main: String,
    pub review: String,
    pub memory: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelPool {
    pub models: BTreeMap<String, ModelSpec>,
    /// Pool name of the global Jev classifier, when one is configured.
    pub jev: Option<String>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum PoolError {
    #[error("model {0} is not in the pool")]
    Unknown(String),
    #[error("the {role} model {name} must use a chat protocol")]
    NotChat { role: &'static str, name: String },
    #[error("the Jev model {0} must use the system_one protocol")]
    JevNotSystemOne(String),
}

impl ModelPool {
    pub fn get(&self, name: &str) -> Result<&ModelSpec, PoolError> {
        self.models
            .get(name)
            .ok_or_else(|| PoolError::Unknown(name.to_owned()))
    }

    /// Checks that every role of a bot names a chat model that exists in the pool.
    pub fn validate_bot(&self, bot: &BotModels) -> Result<(), PoolError> {
        for (role, name) in [
            ("main", &bot.main),
            ("review", &bot.review),
            ("memory", &bot.memory),
        ] {
            if !self.get(name)?.protocol.is_chat() {
                return Err(PoolError::NotChat {
                    role,
                    name: name.clone(),
                });
            }
        }
        Ok(())
    }

    /// Returns the Jev classifier, if configured, after checking its protocol.
    pub fn jev(&self) -> Result<Option<&ModelSpec>, PoolError> {
        let Some(name) = &self.jev else {
            return Ok(None);
        };
        let spec = self.get(name)?;
        if spec.protocol != Protocol::SystemOne {
            return Err(PoolError::JevNotSystemOne(name.clone()));
        }
        Ok(Some(spec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pool() -> ModelPool {
        serde_json::from_value(json!({
            "models": {
                "claude": {
                    "protocol": "anthropic_messages",
                    "model": "claude-test",
                    "endpoint": {
                        "base_url": "https://api.anthropic.com/v1",
                        "vault_ref": "vault://models/claude",
                        "pricing": {"input": 3.0, "output": 15.0, "cache_read": 0.3, "cache_write": 3.75}
                    },
                    "max_output_tokens": 8192
                },
                "gpt": {
                    "protocol": "responses",
                    "model": "gpt-test",
                    "endpoint": {"base_url": "https://gateway.example/v1", "vault_ref": "vault://models/gpt"}
                },
                "jev": {
                    "protocol": "system_one",
                    "model": "jev-latest",
                    "endpoint": {"base_url": "https://api.typesafe.ai/v1/", "vault_ref": "vault://models/jev"}
                }
            },
            "jev": "jev"
        }))
        .expect("pool config deserializes")
    }

    fn bot(main: &str, review: &str, memory: &str) -> BotModels {
        BotModels {
            main: main.to_owned(),
            review: review.to_owned(),
            memory: memory.to_owned(),
        }
    }

    #[test]
    fn pool_holds_only_vault_refs_and_defaults_pricing() {
        let pool = pool();

        let gpt = pool.get("gpt").expect("gpt is in the pool");
        assert_eq!(gpt.endpoint.vault_ref, "vault://models/gpt");
        assert_eq!(gpt.endpoint.pricing, Default::default());
        assert_eq!(gpt.max_output_tokens, None);
        let serialized = serde_json::to_string(&pool).expect("pool serializes");
        assert!(!serialized.contains("api_key") && !serialized.contains("secret"));
    }

    #[test]
    fn bot_roles_must_name_chat_models_in_the_pool() {
        let pool = pool();

        assert_eq!(pool.validate_bot(&bot("claude", "gpt", "claude")), Ok(()));
        assert_eq!(
            pool.validate_bot(&bot("claude", "missing", "gpt")),
            Err(PoolError::Unknown("missing".to_owned()))
        );
        assert_eq!(
            pool.validate_bot(&bot("claude", "gpt", "jev")),
            Err(PoolError::NotChat {
                role: "memory",
                name: "jev".to_owned()
            })
        );
    }

    #[test]
    fn jev_is_optional_and_must_use_system_one() {
        let mut pool = pool();
        assert_eq!(
            pool.jev()
                .expect("jev resolves")
                .map(|spec| spec.model.as_str()),
            Some("jev-latest")
        );

        pool.jev = Some("gpt".to_owned());
        assert_eq!(
            pool.jev(),
            Err(PoolError::JevNotSystemOne("gpt".to_owned()))
        );

        pool.jev = None;
        assert_eq!(pool.jev(), Ok(None));
    }
}
