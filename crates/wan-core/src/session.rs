use serde_json::Value;
use sqlx::Row;

use crate::store::{Store, StoreError};

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub seq: i64,
    pub message_id: String,
    pub event_type: String,
    pub payload: Value,
}

impl Event {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, StoreError> {
        let payload = serde_json::from_str(row.try_get::<String, _>("payload")?.as_str())
            .map_err(|error| StoreError::InvalidEventPayload(error.to_string()))?;
        Ok(Self {
            seq: row.try_get("seq")?,
            message_id: row.try_get("message_id")?,
            event_type: row.try_get("event_type")?,
            payload,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AppendEventResult {
    Inserted,
    Duplicate,
}

impl Store {
    pub async fn append_event(
        &self,
        message_id: &str,
        event_type: &str,
        payload: &Value,
    ) -> Result<(Event, AppendEventResult), StoreError> {
        let payload = serde_json::to_string(payload)
            .map_err(|error| StoreError::InvalidEventPayload(error.to_string()))?;
        let mut transaction = self.pool().begin().await?;
        let existing = sqlx::query(
            "SELECT seq, message_id, event_type, payload FROM events WHERE message_id = ?",
        )
        .bind(message_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let (event, status) = if let Some(row) = existing {
            (Event::from_row(&row)?, AppendEventResult::Duplicate)
        } else {
            sqlx::query("INSERT INTO events (message_id, event_type, payload) VALUES (?, ?, ?)")
                .bind(message_id)
                .bind(event_type)
                .bind(payload)
                .execute(&mut *transaction)
                .await?;
            let row = sqlx::query(
                "SELECT seq, message_id, event_type, payload FROM events WHERE message_id = ?",
            )
            .bind(message_id)
            .fetch_one(&mut *transaction)
            .await?;
            (Event::from_row(&row)?, AppendEventResult::Inserted)
        };
        transaction.commit().await?;
        Ok((event, status))
    }

    pub async fn events_since(&self, since_seq: i64) -> Result<Vec<Event>, StoreError> {
        let rows = sqlx::query(
            "SELECT seq, message_id, event_type, payload FROM events WHERE seq > ? ORDER BY seq ASC",
        )
        .bind(since_seq)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(Event::from_row).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_database() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "wan-core-events-{}-{suffix}.db",
            std::process::id()
        ))
    }

    fn cleanup(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[tokio::test]
    async fn deduplicates_message_ids_without_consuming_seq() {
        let path = temp_database();
        let store = Store::connect(&path).await.expect("database opens");
        let payload = serde_json::json!({"text": "hello"});

        let (first, first_status) = store
            .append_event("message-1", "session/turn_started", &payload)
            .await
            .expect("first event appends");
        let (duplicate, duplicate_status) = store
            .append_event(
                "message-1",
                "session/turn_started",
                &serde_json::json!({"text": "different"}),
            )
            .await
            .expect("duplicate event returns existing event");
        let (second, second_status) = store
            .append_event("message-2", "session/turn_started", &payload)
            .await
            .expect("second event appends");

        assert_eq!(first_status, AppendEventResult::Inserted);
        assert_eq!(duplicate_status, AppendEventResult::Duplicate);
        assert_eq!(second_status, AppendEventResult::Inserted);
        assert_eq!(first, duplicate);
        assert_eq!(first.seq, 1);
        assert_eq!(second.seq, 2);
        assert_eq!(
            store.events_since(0).await.expect("events query"),
            vec![first, second]
        );
        cleanup(&path);
    }

    #[tokio::test]
    async fn sequence_continues_after_reopening_database() {
        let path = temp_database();
        let payload = serde_json::json!({"ok": true});
        let store = Store::connect(&path).await.expect("database opens");
        let (first, _) = store
            .append_event("message-1", "session/turn_started", &payload)
            .await
            .expect("event appends");
        drop(store);

        let store = Store::connect(&path).await.expect("database reopens");
        let (second, _) = store
            .append_event("message-2", "session/turn_completed", &payload)
            .await
            .expect("event appends");
        assert_eq!(first.seq, 1);
        assert_eq!(second.seq, 2);
        assert_eq!(
            store.events_since(first.seq).await.expect("events query"),
            vec![second]
        );
        cleanup(&path);
    }
}
