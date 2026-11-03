use std::sync::Arc;

use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;

const MASK: &str = "[REDACTED]";

/// Masks known secret values in JSON before it reaches durable storage.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    secrets: Arc<[SecretString]>,
}

impl Redactor {
    pub fn new(secrets: impl IntoIterator<Item = SecretString>) -> Self {
        let mut secrets: Vec<SecretString> = secrets
            .into_iter()
            .filter(|secret| !secret.expose_secret().is_empty())
            .collect();
        // Longest first, so a secret that contains another one is masked as a whole.
        secrets.sort_by_key(|secret| std::cmp::Reverse(secret.expose_secret().len()));
        Self {
            secrets: secrets.into(),
        }
    }

    pub fn redact(&self, value: &Value) -> Value {
        match value {
            Value::String(text) => Value::String(self.redact_str(text)),
            Value::Array(items) => {
                Value::Array(items.iter().map(|item| self.redact(item)).collect())
            }
            Value::Object(entries) => Value::Object(
                entries
                    .iter()
                    .map(|(key, entry)| (self.redact_str(key), self.redact(entry)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn redact_str(&self, text: &str) -> String {
        self.secrets.iter().fold(text.to_owned(), |text, secret| {
            text.replace(secret.expose_secret(), MASK)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn redactor(secrets: &[&str]) -> Redactor {
        Redactor::new(secrets.iter().map(|secret| SecretString::from(*secret)))
    }

    #[test]
    fn masks_secrets_in_nested_values_and_keys() {
        let redactor = redactor(&["sk-live-abc"]);
        let redacted = redactor.redact(&json!({
            "header": "Bearer sk-live-abc",
            "list": [{"inner": "sk-live-abc"}, 7, null],
            "sk-live-abc": true,
        }));

        assert_eq!(
            redacted,
            json!({
                "header": "Bearer [REDACTED]",
                "list": [{"inner": "[REDACTED]"}, 7, null],
                "[REDACTED]": true,
            })
        );
    }

    #[test]
    fn masks_longer_secret_before_the_one_it_contains() {
        let redacted = redactor(&["abc", "abcdef"]).redact(&json!("key=abcdef"));

        assert_eq!(redacted, json!("key=[REDACTED]"));
    }

    #[test]
    fn ignores_empty_secrets() {
        let redacted = redactor(&[""]).redact(&json!("unchanged"));

        assert_eq!(redacted, json!("unchanged"));
    }
}
