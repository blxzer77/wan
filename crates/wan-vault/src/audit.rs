//! Audit records hold metadata only: when, which item and field, where it was used, and what
//! triggered it. They never hold a secret value. The fields map to columns.

use std::time::SystemTime;

use crate::item::SealedItem;

/// What caused a secret to be used.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Trigger {
    Thread(String),
    Routine(String),
    /// Used inside the daemon with no conversation, such as a model API key for a request.
    Daemon,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AuditRecord {
    pub at: SystemTime,
    pub item_id: String,
    pub field: String,
    /// The origin the secret was used for; empty when the item has none.
    pub target_origin: String,
    pub trigger: Trigger,
}

impl AuditRecord {
    /// Builds the record for one use of `item`. It copies identifiers only, never plaintext.
    pub fn for_use(item: &SealedItem, target_origin: &str, trigger: Trigger) -> Self {
        Self {
            at: SystemTime::now(),
            item_id: item.item_id.clone(),
            field: item.field.clone(),
            target_origin: target_origin.to_owned(),
            trigger,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ItemType, seal_item};
    use crate::key::generate_dek;

    #[test]
    fn record_carries_metadata_and_no_secret() {
        let dek = generate_dek();
        let item = seal_item(
            &dek,
            1,
            "item-1",
            "password",
            "https://example.com",
            ItemType::Login,
            b"correct-horse-battery-staple",
        )
        .expect("seals");

        let record = AuditRecord::for_use(
            &item,
            "https://example.com",
            Trigger::Routine("daily-report".to_owned()),
        );

        assert_eq!(record.item_id, "item-1");
        assert_eq!(record.field, "password");
        assert_eq!(record.target_origin, "https://example.com");
        assert_eq!(record.trigger, Trigger::Routine("daily-report".to_owned()));
        assert!(!format!("{record:?}").contains("correct-horse-battery-staple"));
    }
}
