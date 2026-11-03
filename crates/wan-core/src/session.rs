use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use sqlx::Row;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::task::JoinHandle;

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

#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum TurnPriority {
    Background,
    Group,
    User,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Delivery {
    Queue,
    GuideAtToolBoundary,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TurnRequest {
    pub id: String,
    pub priority: TurnPriority,
    pub delivery: Delivery,
}

#[derive(Debug, Clone)]
struct QueuedTurn {
    request: TurnRequest,
    enqueued_at: Duration,
    order: u64,
}

#[derive(Debug, Default)]
pub struct SessionQueue {
    turns: Vec<QueuedTurn>,
    next_order: u64,
}

impl SessionQueue {
    pub fn push(&mut self, request: TurnRequest, now: Duration) {
        let order = self.next_order;
        self.next_order += 1;
        self.turns.push(QueuedTurn {
            request,
            enqueued_at: now,
            order,
        });
    }

    pub fn pop_next(&mut self, now: Duration) -> Option<TurnRequest> {
        let index = self
            .turns
            .iter()
            .enumerate()
            .filter(|(_, turn)| now.saturating_sub(turn.enqueued_at) >= Duration::from_secs(300))
            .min_by_key(|(_, turn)| (turn.enqueued_at, turn.order))
            .map(|(index, _)| index)
            .or_else(|| {
                self.turns
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, turn)| (turn.request.priority, std::cmp::Reverse(turn.order)))
                    .map(|(index, _)| index)
            })?;
        Some(self.turns.swap_remove(index).request)
    }

    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct SessionState {
    active: Option<TurnRequest>,
    guided: VecDeque<TurnRequest>,
    queued: SessionQueue,
}

impl SessionState {
    pub fn submit(&mut self, request: TurnRequest, now: Duration, at_tool_boundary: bool) {
        if self.active.is_some()
            && request.delivery == Delivery::GuideAtToolBoundary
            && at_tool_boundary
        {
            self.guided.push_back(request);
        } else {
            self.queued.push(request, now);
        }
    }

    pub fn start_next(&mut self, now: Duration) -> Option<TurnRequest> {
        if self.active.is_some() {
            return None;
        }
        self.active = self
            .guided
            .pop_front()
            .or_else(|| self.queued.pop_next(now));
        self.active.clone()
    }

    pub fn finish(&mut self) {
        self.active = None;
    }

    pub fn active(&self) -> Option<&TurnRequest> {
        self.active.as_ref()
    }

    pub fn queued_len(&self) -> usize {
        self.guided.len() + self.queued.turns.len()
    }
}

#[derive(Debug)]
pub struct TurnLimits {
    global: Arc<Semaphore>,
    background: Arc<Semaphore>,
}

impl Default for TurnLimits {
    fn default() -> Self {
        Self::new()
    }
}

impl TurnLimits {
    pub fn new() -> Self {
        Self {
            global: Arc::new(Semaphore::new(6)),
            background: Arc::new(Semaphore::new(3)),
        }
    }

    pub async fn acquire(&self, priority: TurnPriority) -> TurnPermit {
        let global = self
            .global
            .clone()
            .acquire_owned()
            .await
            .expect("global turn limit is never closed");
        let background = if priority == TurnPriority::Background {
            Some(
                self.background
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("background turn limit is never closed"),
            )
        } else {
            None
        };
        TurnPermit { global, background }
    }

    pub fn available(&self) -> usize {
        self.global.available_permits()
    }
}

#[derive(Debug)]
pub struct TurnPermit {
    global: OwnedSemaphorePermit,
    background: Option<OwnedSemaphorePermit>,
}

impl TurnPermit {
    pub fn is_background(&self) -> bool {
        self.background.is_some()
    }

    pub fn global_permit(&self) -> &OwnedSemaphorePermit {
        &self.global
    }
}

pub type TurnFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type TurnHandler = Arc<dyn Fn(TurnRequest) -> TurnFuture + Send + Sync>;

#[derive(Debug)]
pub struct SessionSupervisor {
    sender: mpsc::Sender<TurnRequest>,
    task: JoinHandle<()>,
}

impl SessionSupervisor {
    pub fn spawn(handler: impl Fn(TurnRequest) -> TurnFuture + Send + Sync + 'static) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let handler: TurnHandler = Arc::new(handler);
        let task = tokio::spawn(supervise_actor(receiver, handler));
        Self { sender, task }
    }

    pub async fn submit(&self, request: TurnRequest) -> Result<(), TurnRequest> {
        self.sender.send(request).await.map_err(|error| error.0)
    }

    pub async fn shutdown(self) {
        drop(self.sender);
        let _ = self.task.await;
    }
}

async fn supervise_actor(mut receiver: mpsc::Receiver<TurnRequest>, handler: TurnHandler) {
    let mut pending = None;
    loop {
        let (actor_sender, actor_receiver) = mpsc::channel(1);
        if let Some(request) = pending.take()
            && actor_sender.send(request).await.is_err()
        {
            continue;
        }
        let actor = tokio::spawn(run_actor(actor_receiver, handler.clone()));
        let mut actor = actor;
        loop {
            tokio::select! {
                result = &mut actor => {
                    if result.is_ok() {
                        return;
                    }
                    break;
                }
                request = receiver.recv() => {
                    let Some(request) = request else {
                        return;
                    };
                    if let Err(error) = actor_sender.send(request).await {
                        pending = Some(error.0);
                        break;
                    }
                }
            }
        }
    }
}

async fn run_actor(mut receiver: mpsc::Receiver<TurnRequest>, handler: TurnHandler) {
    while let Some(request) = receiver.recv().await {
        (handler)(request).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

    fn request(id: &str, priority: TurnPriority, delivery: Delivery) -> TurnRequest {
        TurnRequest {
            id: id.to_owned(),
            priority,
            delivery,
        }
    }

    #[test]
    fn queue_prefers_user_then_group_then_background() {
        let mut queue = SessionQueue::default();
        queue.push(
            request("background", TurnPriority::Background, Delivery::Queue),
            Duration::ZERO,
        );
        queue.push(
            request("group", TurnPriority::Group, Delivery::Queue),
            Duration::ZERO,
        );
        queue.push(
            request("user", TurnPriority::User, Delivery::Queue),
            Duration::ZERO,
        );

        assert_eq!(queue.pop_next(Duration::ZERO).unwrap().id, "user");
        assert_eq!(queue.pop_next(Duration::ZERO).unwrap().id, "group");
        assert_eq!(queue.pop_next(Duration::ZERO).unwrap().id, "background");
        assert!(queue.is_empty());
    }

    #[test]
    fn queue_starvation_prevention_overrides_priority_after_five_minutes() {
        let mut queue = SessionQueue::default();
        queue.push(
            request("background", TurnPriority::Background, Delivery::Queue),
            Duration::ZERO,
        );
        queue.push(
            request("user", TurnPriority::User, Delivery::Queue),
            Duration::from_secs(299),
        );

        assert_eq!(
            queue.pop_next(Duration::from_secs(300)).unwrap().id,
            "background"
        );
    }

    #[test]
    fn tool_boundary_guidance_runs_before_queued_turns() {
        let mut state = SessionState::default();
        state.submit(
            request("first", TurnPriority::User, Delivery::Queue),
            Duration::ZERO,
            false,
        );
        assert_eq!(state.start_next(Duration::ZERO).unwrap().id, "first");
        state.submit(
            request("guided", TurnPriority::User, Delivery::GuideAtToolBoundary),
            Duration::from_secs(1),
            true,
        );
        state.submit(
            request("queued", TurnPriority::User, Delivery::Queue),
            Duration::from_secs(1),
            true,
        );
        state.finish();
        assert_eq!(
            state.start_next(Duration::from_secs(1)).unwrap().id,
            "guided"
        );
        state.finish();
        assert_eq!(
            state.start_next(Duration::from_secs(1)).unwrap().id,
            "queued"
        );
    }

    #[tokio::test]
    async fn turn_limits_allow_six_turns_and_three_background_turns() {
        let limits = TurnLimits::new();
        let permits = vec![
            limits.acquire(TurnPriority::User).await,
            limits.acquire(TurnPriority::User).await,
            limits.acquire(TurnPriority::User).await,
            limits.acquire(TurnPriority::User).await,
            limits.acquire(TurnPriority::User).await,
            limits.acquire(TurnPriority::User).await,
        ];
        assert_eq!(limits.available(), 0);
        assert!(!permits[0].is_background());
        drop(permits);
        assert_eq!(limits.available(), 6);

        let background = vec![
            limits.acquire(TurnPriority::Background).await,
            limits.acquire(TurnPriority::Background).await,
            limits.acquire(TurnPriority::Background).await,
        ];
        assert!(background.iter().all(TurnPermit::is_background));
        drop(background);
    }

    #[tokio::test]
    async fn supervisor_restarts_actor_after_panic() {
        let calls = Arc::new(AtomicUsize::new(0));
        let handler_calls = calls.clone();
        let supervisor = SessionSupervisor::spawn(move |_| {
            let handler_calls = handler_calls.clone();
            Box::pin(async move {
                if handler_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    panic!("simulated actor panic");
                }
            })
        });
        supervisor
            .submit(request("first", TurnPriority::User, Delivery::Queue))
            .await
            .expect("first request queues");
        supervisor
            .submit(request("second", TurnPriority::User, Delivery::Queue))
            .await
            .expect("second request queues");

        for _ in 0..20 {
            if calls.load(Ordering::SeqCst) >= 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        supervisor.shutdown().await;
    }
}
