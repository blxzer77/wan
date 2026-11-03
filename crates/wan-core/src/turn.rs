use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

#[derive(Clone, Debug)]
pub struct CancellationToken {
    inner: Arc<CancellationState>,
}

#[derive(Debug)]
struct CancellationState {
    cancelled: AtomicBool,
    notify: Notify,
    parent: Option<Arc<CancellationState>>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancellationState {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
                parent: None,
            }),
        }
    }

    pub fn child_token(&self) -> Self {
        Self {
            inner: Arc::new(CancellationState {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
                parent: Some(self.inner.clone()),
            }),
        }
    }

    pub fn cancel(&self) {
        if !self.inner.cancelled.swap(true, Ordering::Release) {
            self.inner.notify.notify_waiters();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
            || self.inner.parent.as_ref().is_some_and(|parent| {
                CancellationToken {
                    inner: parent.clone(),
                }
                .is_cancelled()
            })
    }

    pub fn cancelled(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            if self.is_cancelled() {
                return;
            }
            if let Some(parent) = &self.inner.parent {
                let parent = CancellationToken {
                    inner: parent.clone(),
                };
                tokio::select! {
                    _ = self.inner.notify.notified() => {}
                    _ = parent.cancelled() => {}
                }
            } else {
                self.inner.notify.notified().await;
            }
        })
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StepStatus {
    Running,
    Completed,
    Failed,
    CancelledUnknown,
}

pub fn should_retry(status: StepStatus) -> bool {
    !matches!(status, StepStatus::CancelledUnknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeouts;
    use std::time::Duration;

    #[tokio::test]
    async fn cancellation_propagates_from_parent_to_child() {
        let parent = CancellationToken::new();
        let child = parent.child_token();
        let waiting = child.clone();
        let task = tokio::spawn(async move {
            waiting.cancelled().await;
            true
        });
        parent.cancel();
        assert!(task.await.expect("cancellation waiter completes"));
        assert!(child.is_cancelled());
    }

    #[tokio::test]
    async fn cancelling_child_does_not_cancel_parent() {
        let parent = CancellationToken::new();
        let child = parent.child_token();
        child.cancel();
        assert!(child.is_cancelled());
        assert!(!parent.is_cancelled());
    }

    #[test]
    fn cancelled_unknown_steps_are_never_retried() {
        assert!(!should_retry(StepStatus::CancelledUnknown));
        assert!(should_retry(StepStatus::Running));
        assert!(should_retry(StepStatus::Completed));
        assert!(should_retry(StepStatus::Failed));
    }

    #[test]
    fn all_timeout_values_are_defined_in_one_policy_module() {
        assert_eq!(timeouts::SIGTERM_GRACE, Duration::from_secs(5));
        assert_eq!(timeouts::MCP_CANCEL, Duration::from_secs(2));
        assert_eq!(timeouts::ACP_CANCEL, Duration::from_secs(10));
        assert_eq!(timeouts::MODEL_RESPONSE_HEADER, Duration::from_secs(600));
        assert_eq!(timeouts::MODEL_STREAM_IDLE, Duration::from_secs(300));
        assert_eq!(
            timeouts::COMMAND_FOREGROUND_DEFAULT,
            Duration::from_secs(120)
        );
        assert_eq!(timeouts::COMMAND_FOREGROUND_MAX, Duration::from_secs(600));
        assert_eq!(timeouts::COMMAND_BACKGROUND, Duration::from_secs(7_200));
        assert_eq!(timeouts::MCP_STARTUP, Duration::from_secs(30));
        assert_eq!(timeouts::MCP_CALL_DEFAULT, Duration::from_secs(120));
        assert_eq!(timeouts::MCP_CALL_MAX, Duration::from_secs(1_800));
        assert_eq!(timeouts::HELPER_IDLE, Duration::from_secs(600));
        assert_eq!(timeouts::TURN, Duration::from_secs(7_200));
        assert_eq!(timeouts::ROUTINE, Duration::from_secs(1_800));
        assert_eq!(timeouts::BACKGROUND_APPROVAL, Duration::from_secs(1_800));
    }
}
