use serde_json::Value;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use super::{
    NewStep, Redactor, ReplayPlan, StepBegin, StepKind, StepLog, StepLogError, StepOutcome,
    StepRecord,
};
use crate::store::Store;
use crate::turn::StepStatus;

const SELECT_STEP: &str = "SELECT turn_id, step_index, kind, has_side_effect, status, attempt, \
     input, output FROM turn_steps WHERE turn_id = ? AND step_index = ?";
const SELECT_TURN_STEPS: &str = "SELECT turn_id, step_index, kind, has_side_effect, status, \
     attempt, input, output FROM turn_steps WHERE turn_id = ? ORDER BY step_index";

#[derive(Debug, Clone)]
pub struct SqliteStepLog {
    store: Store,
    redactor: Redactor,
}

impl SqliteStepLog {
    pub fn new(store: Store, redactor: Redactor) -> Self {
        Self { store, redactor }
    }
}

impl StepLog for SqliteStepLog {
    async fn begin(&self, step: NewStep<'_>) -> Result<StepBegin, StepLogError> {
        let input = self.redactor.redact(step.input);
        let mut tx = self.store.pool().begin().await?;
        let existing = sqlx::query(SELECT_STEP)
            .bind(step.turn_id)
            .bind(step.index)
            .fetch_optional(&mut *tx)
            .await?;

        let begin = match existing {
            None => {
                sqlx::query(
                    "INSERT INTO turn_steps \
                     (turn_id, step_index, kind, has_side_effect, status, input) \
                     VALUES (?, ?, ?, ?, 'running', ?)",
                )
                .bind(step.turn_id)
                .bind(step.index)
                .bind(kind_name(step.kind))
                .bind(step.has_side_effect)
                .bind(serde_json::to_string(&input)?)
                .execute(&mut *tx)
                .await?;
                StepBegin::Execute { attempt: 1 }
            }
            Some(row) => {
                let record = record_from_row(&row)?;
                if record.kind != step.kind
                    || record.has_side_effect != step.has_side_effect
                    || record.input != input
                {
                    return Err(StepLogError::Mismatch {
                        turn_id: step.turn_id.to_owned(),
                        index: step.index,
                    });
                }
                match record.status {
                    StepStatus::Completed => StepBegin::Reuse {
                        output: record.output.ok_or_else(|| {
                            StepLogError::Corrupt("completed step has no output".to_owned())
                        })?,
                    },
                    StepStatus::CancelledUnknown => StepBegin::NeedsUserDecision,
                    StepStatus::Running if record.has_side_effect => {
                        // The earlier run died mid-effect; whether it happened is unknown.
                        mark_cancelled_unknown(&mut tx, step.turn_id, Some(step.index)).await?;
                        StepBegin::NeedsUserDecision
                    }
                    StepStatus::Running | StepStatus::Failed => {
                        let attempt = record.attempt + 1;
                        sqlx::query(
                            "UPDATE turn_steps SET status = 'running', attempt = ?, \
                             output = NULL, finished_at = NULL \
                             WHERE turn_id = ? AND step_index = ?",
                        )
                        .bind(attempt)
                        .bind(step.turn_id)
                        .bind(step.index)
                        .execute(&mut *tx)
                        .await?;
                        StepBegin::Execute { attempt }
                    }
                }
            }
        };

        tx.commit().await?;
        Ok(begin)
    }

    async fn finish(
        &self,
        turn_id: &str,
        index: i64,
        outcome: StepOutcome,
    ) -> Result<(), StepLogError> {
        let (status, output) = match outcome {
            StepOutcome::Completed(output) => ("completed", Some(output)),
            StepOutcome::Failed(output) => ("failed", Some(output)),
            StepOutcome::CancelledUnknown => ("cancelled_unknown", None),
        };
        let output = output
            .map(|output| serde_json::to_string(&self.redactor.redact(&output)))
            .transpose()?;

        let result = sqlx::query(
            "UPDATE turn_steps SET status = ?, output = ?, finished_at = CURRENT_TIMESTAMP \
             WHERE turn_id = ? AND step_index = ? AND status = 'running'",
        )
        .bind(status)
        .bind(output)
        .bind(turn_id)
        .bind(index)
        .execute(self.store.pool())
        .await?;
        if result.rows_affected() == 0 {
            return Err(StepLogError::NotRunning {
                turn_id: turn_id.to_owned(),
                index,
            });
        }
        Ok(())
    }

    async fn replay(&self, turn_id: &str) -> Result<ReplayPlan, StepLogError> {
        let mut tx = self.store.pool().begin().await?;
        mark_cancelled_unknown(&mut tx, turn_id, None).await?;
        let rows = sqlx::query(SELECT_TURN_STEPS)
            .bind(turn_id)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;

        let mut plan = ReplayPlan {
            completed: Vec::new(),
            questions: Vec::new(),
        };
        for row in &rows {
            let record = record_from_row(row)?;
            match record.status {
                StepStatus::Completed => plan.completed.push(record),
                StepStatus::CancelledUnknown => plan.questions.push(record),
                StepStatus::Running | StepStatus::Failed => {}
            }
        }
        Ok(plan)
    }
}

/// Marks running side-effect steps of a turn as unknown, optionally limited to one step.
async fn mark_cancelled_unknown(
    tx: &mut sqlx::SqliteConnection,
    turn_id: &str,
    index: Option<i64>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE turn_steps SET status = 'cancelled_unknown', finished_at = CURRENT_TIMESTAMP \
         WHERE turn_id = ? AND status = 'running' AND has_side_effect = 1 \
         AND (? IS NULL OR step_index = ?)",
    )
    .bind(turn_id)
    .bind(index)
    .bind(index)
    .execute(tx)
    .await?;
    Ok(())
}

fn kind_name(kind: StepKind) -> &'static str {
    match kind {
        StepKind::ModelCall => "model_call",
        StepKind::ToolCall => "tool_call",
        StepKind::Approval => "approval",
    }
}

fn record_from_row(row: &SqliteRow) -> Result<StepRecord, StepLogError> {
    let kind = match row.try_get::<String, _>("kind")?.as_str() {
        "model_call" => StepKind::ModelCall,
        "tool_call" => StepKind::ToolCall,
        "approval" => StepKind::Approval,
        other => return Err(StepLogError::Corrupt(format!("unknown step kind {other}"))),
    };
    let status = match row.try_get::<String, _>("status")?.as_str() {
        "running" => StepStatus::Running,
        "completed" => StepStatus::Completed,
        "failed" => StepStatus::Failed,
        "cancelled_unknown" => StepStatus::CancelledUnknown,
        other => {
            return Err(StepLogError::Corrupt(format!(
                "unknown step status {other}"
            )));
        }
    };
    let input: String = row.try_get("input")?;
    let output: Option<String> = row.try_get("output")?;
    Ok(StepRecord {
        turn_id: row.try_get("turn_id")?,
        index: row.try_get("step_index")?,
        kind,
        has_side_effect: row.try_get("has_side_effect")?,
        status,
        attempt: row.try_get("attempt")?,
        input: serde_json::from_str::<Value>(&input)?,
        output: output
            .as_deref()
            .map(serde_json::from_str::<Value>)
            .transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use serde_json::json;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_database() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "wan-core-steplog-{}-{suffix}.db",
            std::process::id()
        ))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    async fn open(path: &Path) -> SqliteStepLog {
        let store = Store::connect(path).await.expect("database opens");
        let redactor = Redactor::new([SecretString::from("sk-live-abc")]);
        SqliteStepLog::new(store, redactor)
    }

    fn step<'a>(index: i64, kind: StepKind, side_effect: bool, input: &'a Value) -> NewStep<'a> {
        NewStep {
            turn_id: "turn-1",
            index,
            kind,
            has_side_effect: side_effect,
            input,
        }
    }

    #[tokio::test]
    async fn completed_steps_are_reused_after_restart() {
        let path = temp_database();
        let input = json!({"prompt": "hello"});
        let log = open(&path).await;
        let started = log
            .begin(step(0, StepKind::ModelCall, false, &input))
            .await
            .expect("step begins");
        assert_eq!(started, StepBegin::Execute { attempt: 1 });
        log.finish("turn-1", 0, StepOutcome::Completed(json!({"text": "hi"})))
            .await
            .expect("step finishes");
        drop(log);

        let log = open(&path).await;
        let plan = log.replay("turn-1").await.expect("replay succeeds");
        assert_eq!(plan.completed.len(), 1);
        assert!(plan.questions.is_empty());
        let again = log
            .begin(step(0, StepKind::ModelCall, false, &input))
            .await
            .expect("step resolves");
        assert_eq!(
            again,
            StepBegin::Reuse {
                output: json!({"text": "hi"})
            }
        );
        drop(log);
        cleanup(&path);
    }

    #[tokio::test]
    async fn crashed_side_effect_step_becomes_a_question_and_is_never_retried() {
        let path = temp_database();
        let input = json!({"tool": "send_email"});
        let log = open(&path).await;
        log.begin(step(0, StepKind::ToolCall, true, &input))
            .await
            .expect("step begins");
        drop(log);

        let log = open(&path).await;
        let plan = log.replay("turn-1").await.expect("replay succeeds");
        assert!(plan.completed.is_empty());
        assert_eq!(plan.questions.len(), 1);
        assert_eq!(plan.questions[0].status, StepStatus::CancelledUnknown);
        for _ in 0..2 {
            let again = log
                .begin(step(0, StepKind::ToolCall, true, &input))
                .await
                .expect("step resolves");
            assert_eq!(again, StepBegin::NeedsUserDecision);
        }
        drop(log);
        cleanup(&path);
    }

    #[tokio::test]
    async fn begin_without_replay_still_refuses_to_repeat_a_running_side_effect() {
        let path = temp_database();
        let input = json!({"tool": "send_email"});
        let log = open(&path).await;
        log.begin(step(0, StepKind::ToolCall, true, &input))
            .await
            .expect("step begins");

        let again = log
            .begin(step(0, StepKind::ToolCall, true, &input))
            .await
            .expect("step resolves");
        assert_eq!(again, StepBegin::NeedsUserDecision);
        drop(log);
        cleanup(&path);
    }

    #[tokio::test]
    async fn crashed_read_only_step_runs_again_with_a_new_attempt() {
        let path = temp_database();
        let input = json!({"tool": "read_file"});
        let log = open(&path).await;
        log.begin(step(0, StepKind::ToolCall, false, &input))
            .await
            .expect("step begins");
        drop(log);

        let log = open(&path).await;
        let plan = log.replay("turn-1").await.expect("replay succeeds");
        assert!(plan.completed.is_empty() && plan.questions.is_empty());
        let again = log
            .begin(step(0, StepKind::ToolCall, false, &input))
            .await
            .expect("step resolves");
        assert_eq!(again, StepBegin::Execute { attempt: 2 });
        drop(log);
        cleanup(&path);
    }

    #[tokio::test]
    async fn failed_steps_are_retried() {
        let path = temp_database();
        let input = json!({"tool": "send_email"});
        let log = open(&path).await;
        log.begin(step(0, StepKind::ToolCall, true, &input))
            .await
            .expect("step begins");
        log.finish(
            "turn-1",
            0,
            StepOutcome::Failed(json!({"error": "refused"})),
        )
        .await
        .expect("step finishes");

        let again = log
            .begin(step(0, StepKind::ToolCall, true, &input))
            .await
            .expect("step resolves");
        assert_eq!(again, StepBegin::Execute { attempt: 2 });
        drop(log);
        cleanup(&path);
    }

    #[tokio::test]
    async fn secrets_are_redacted_before_they_are_written() {
        let path = temp_database();
        let log = open(&path).await;
        let input = json!({"authorization": "Bearer sk-live-abc"});
        log.begin(step(0, StepKind::ModelCall, false, &input))
            .await
            .expect("step begins");
        log.finish(
            "turn-1",
            0,
            StepOutcome::Completed(json!({"echo": "sk-live-abc"})),
        )
        .await
        .expect("step finishes");

        let row = sqlx::query("SELECT input, output FROM turn_steps")
            .fetch_one(log.store.pool())
            .await
            .expect("row reads");
        let stored = format!(
            "{} {}",
            row.get::<String, _>("input"),
            row.get::<String, _>("output")
        );
        assert!(!stored.contains("sk-live-abc"));
        assert!(stored.contains("[REDACTED]"));

        let again = log
            .begin(step(0, StepKind::ModelCall, false, &input))
            .await
            .expect("redacted input still matches the recorded step");
        assert_eq!(
            again,
            StepBegin::Reuse {
                output: json!({"echo": "[REDACTED]"})
            }
        );
        drop(log);
        cleanup(&path);
    }

    #[tokio::test]
    async fn changed_step_shape_is_rejected() {
        let path = temp_database();
        let log = open(&path).await;
        let input = json!({"prompt": "hello"});
        log.begin(step(0, StepKind::ModelCall, false, &input))
            .await
            .expect("step begins");

        let other = json!({"prompt": "different"});
        let error = log
            .begin(step(0, StepKind::ModelCall, false, &other))
            .await
            .expect_err("different input rejects");
        assert!(matches!(error, StepLogError::Mismatch { index: 0, .. }));
        let error = log
            .begin(step(0, StepKind::ToolCall, false, &input))
            .await
            .expect_err("different kind rejects");
        assert!(matches!(error, StepLogError::Mismatch { index: 0, .. }));
        drop(log);
        cleanup(&path);
    }

    #[tokio::test]
    async fn finish_requires_a_running_step() {
        let path = temp_database();
        let log = open(&path).await;
        let input = json!({"prompt": "hello"});

        let error = log
            .finish("turn-1", 0, StepOutcome::Completed(json!(null)))
            .await
            .expect_err("unknown step rejects");
        assert!(matches!(error, StepLogError::NotRunning { index: 0, .. }));

        log.begin(step(0, StepKind::ModelCall, false, &input))
            .await
            .expect("step begins");
        log.finish("turn-1", 0, StepOutcome::Completed(json!(null)))
            .await
            .expect("step finishes");
        let error = log
            .finish("turn-1", 0, StepOutcome::Completed(json!(null)))
            .await
            .expect_err("finished step rejects");
        assert!(matches!(error, StepLogError::NotRunning { index: 0, .. }));
        drop(log);
        cleanup(&path);
    }
}
