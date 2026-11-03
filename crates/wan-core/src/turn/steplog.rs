//! Durable step log: every model call, tool call, and approval is recorded as a step so a
//! restarted turn can reuse completed work and never blindly repeat an unknown side effect.

mod redact;
mod sqlite;

use std::future::Future;

use serde_json::Value;
use thiserror::Error;

use super::StepStatus;

pub use redact::Redactor;
pub use sqlite::SqliteStepLog;

#[derive(Debug, Error)]
pub enum StepLogError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("step payload is not valid JSON: {0}")]
    Payload(#[from] serde_json::Error),
    #[error("stored step is corrupt: {0}")]
    Corrupt(String),
    #[error("step {index} of turn {turn_id} does not match the recorded step")]
    Mismatch { turn_id: String, index: i64 },
    #[error("step {index} of turn {turn_id} is not running")]
    NotRunning { turn_id: String, index: i64 },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StepKind {
    ModelCall,
    ToolCall,
    Approval,
}

#[derive(Debug, Clone, Copy)]
pub struct NewStep<'a> {
    pub turn_id: &'a str,
    pub index: i64,
    pub kind: StepKind,
    pub has_side_effect: bool,
    pub input: &'a Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepRecord {
    pub turn_id: String,
    pub index: i64,
    pub kind: StepKind,
    pub has_side_effect: bool,
    pub status: StepStatus,
    pub attempt: i64,
    pub input: Value,
    pub output: Option<Value>,
}

/// What the executor must do with a step it is about to run.
#[derive(Debug, Clone, PartialEq)]
pub enum StepBegin {
    Execute { attempt: i64 },
    Reuse { output: Value },
    NeedsUserDecision,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepOutcome {
    Completed(Value),
    Failed(Value),
    CancelledUnknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayPlan {
    pub completed: Vec<StepRecord>,
    pub questions: Vec<StepRecord>,
}

pub trait StepLog: Send + Sync {
    /// Records the start of a step, or resolves it against what an earlier run recorded.
    fn begin(
        &self,
        step: NewStep<'_>,
    ) -> impl Future<Output = Result<StepBegin, StepLogError>> + Send;

    fn finish(
        &self,
        turn_id: &str,
        index: i64,
        outcome: StepOutcome,
    ) -> impl Future<Output = Result<(), StepLogError>> + Send;

    /// Turns steps left running by a crashed process into questions. Call it only while no
    /// executor owns the turn, such as at startup.
    fn replay(
        &self,
        turn_id: &str,
    ) -> impl Future<Output = Result<ReplayPlan, StepLogError>> + Send;
}
