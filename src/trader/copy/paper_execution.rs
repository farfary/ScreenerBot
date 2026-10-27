// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Service-owned bounded FIFO dispatcher for the paper work of copy tasks: entries,
//! mirror sells, closes and resets of one task run one at a time, in arrival order.

use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, Mutex};

use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;

use super::{CopyDatabase, CopyOutcome, CopySkip};
use crate::trader::executors::ExecutionContext;
use crate::trader::types::{TradeDecision, TradeResult};
mod error;
pub(super) use error::EnqueueError;

/// A queued job; the outcome it returns, if any, is settled into the copy store.
type Job = Pin<Box<dyn Future<Output = Option<CopyOutcome>> + Send + 'static>>;
type SettleFuture = Pin<Box<dyn Future<Output = crate::trader::Result<bool>> + Send + 'static>>;
type Settlement = Arc<dyn Fn(CopyDatabase, CopyOutcome) -> SettleFuture + Send + Sync>;

/// Runs a paper trade decision of a copy task on the shared paper executor, where no
/// stop of the FIFO can cancel it half booked.
pub type PaperDispatch = Arc<
    dyn Fn(
            TradeDecision,
            ExecutionContext,
        ) -> Pin<Box<dyn Future<Output = crate::trader::Result<TradeResult>> + Send>>
        + Send
        + Sync,
>;

#[derive(Default)]
struct AdmissionState {
    queued: HashMap<i64, usize>,
    keys: HashSet<(i64, String)>,
}

struct AdmissionGuard {
    task_id: i64,
    key: String,
    state: Arc<Mutex<AdmissionState>>,
    pending: bool,
}

impl AdmissionGuard {
    fn started(&mut self) {
        if self.pending {
            if let Ok(mut state) = self.state.lock() {
                if let Some(count) = state.queued.get_mut(&self.task_id) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        state.queued.remove(&self.task_id);
                    }
                }
            }
            self.pending = false;
        }
    }
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.keys.remove(&(self.task_id, self.key.clone()));
            if self.pending {
                if let Some(count) = state.queued.get_mut(&self.task_id) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        state.queued.remove(&self.task_id);
                    }
                }
            }
        }
    }
}

struct QueuedJob {
    admission: AdmissionGuard,
    database: CopyDatabase,
    mint: Option<String>,
    job: Job,
    completion: Option<oneshot::Sender<Result<(CopyOutcome, bool), crate::trader::Error>>>,
}

struct ActiveJobGuard {
    task_id: i64,
    finished: mpsc::UnboundedSender<i64>,
}

impl Drop for ActiveJobGuard {
    fn drop(&mut self) {
        let _ = self.finished.send(self.task_id);
    }
}

struct QueueHandle {
    sender: mpsc::UnboundedSender<(i64, QueuedJob)>,
    admission: Arc<Mutex<AdmissionState>>,
}

/// Cloneable scheduling capability. The service owns dispatch and settlement.
#[derive(Clone)]
pub struct PaperExecutionHandle(Arc<QueueHandle>);

impl PaperExecutionHandle {
    fn new(sender: mpsc::UnboundedSender<(i64, QueuedJob)>) -> Self {
        Self(Arc::new(QueueHandle {
            sender,
            admission: Arc::new(Mutex::new(AdmissionState::default())),
        }))
    }

    /// Enqueue without waiting. Capacity is read from current configuration on every admission.
    pub fn enqueue<F>(
        &self,
        task_id: i64,
        key: String,
        database: CopyDatabase,
        mint: Option<String>,
        future: F,
    ) -> Result<(), EnqueueError>
    where
        F: Future<Output = CopyOutcome> + Send + 'static,
    {
        self.enqueue_inner(
            task_id,
            key,
            database,
            mint,
            Box::pin(async move { Some(future.await) }),
            None,
        )
    }

    /// Enqueue a step of the task's paper work that records no outcome, and receive
    /// its result. The receiver closes without a value when the dispatcher stops
    /// before the step runs.
    pub fn enqueue_step<F, T>(
        &self,
        task_id: i64,
        key: String,
        database: CopyDatabase,
        future: F,
    ) -> Result<oneshot::Receiver<crate::trader::Result<T>>, EnqueueError>
    where
        F: Future<Output = crate::trader::Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        self.enqueue_inner(
            task_id,
            key,
            database,
            None,
            Box::pin(async move {
                let _ = sender.send(future.await);
                None
            }),
            None,
        )?;
        Ok(receiver)
    }

    pub fn enqueue_manual<F>(
        &self,
        task_id: i64,
        key: String,
        mint: String,
        database: CopyDatabase,
        future: F,
    ) -> Result<oneshot::Receiver<Result<(CopyOutcome, bool), crate::trader::Error>>, EnqueueError>
    where
        F: Future<Output = CopyOutcome> + Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        self.enqueue_inner(
            task_id,
            key,
            database,
            Some(mint),
            Box::pin(async move { Some(future.await) }),
            Some(sender),
        )?;
        Ok(receiver)
    }

    fn enqueue_inner(
        &self,
        task_id: i64,
        key: String,
        database: CopyDatabase,
        mint: Option<String>,
        job: Job,
        completion: Option<oneshot::Sender<Result<(CopyOutcome, bool), crate::trader::Error>>>,
    ) -> Result<(), EnqueueError> {
        let capacity =
            crate::config::with_config(|config| config.copy_trading.paper_queue_capacity as usize);
        let admission = {
            let mut state = self
                .0
                .admission
                .lock()
                .map_err(|_| EnqueueError::Unavailable)?;
            if !state.keys.insert((task_id, key.clone())) {
                return Err(EnqueueError::Duplicate);
            }
            if state.queued.get(&task_id).copied().unwrap_or_default() >= capacity {
                state.keys.remove(&(task_id, key));
                return Err(EnqueueError::Full);
            }
            *state.queued.entry(task_id).or_default() += 1;
            AdmissionGuard {
                task_id,
                key,
                state: self.0.admission.clone(),
                pending: true,
            }
        };
        self.0
            .sender
            .send((
                task_id,
                QueuedJob {
                    admission,
                    database,
                    mint,
                    job,
                    completion,
                },
            ))
            .map_err(|_| EnqueueError::Unavailable)
    }

    /// Whether a close of paper round `position_id` of task `task_id` is queued or
    /// running.
    pub fn manual_close_pending(&self, task_id: i64, position_id: i64) -> bool {
        self.0.admission.lock().is_ok_and(|state| {
            state
                .keys
                .contains(&(task_id, manual_close_key(position_id)))
        })
    }
}

/// The skip a refused admission to a task's paper queue is reported as.
pub(super) fn queue_refusal(error: EnqueueError) -> CopySkip {
    match error {
        EnqueueError::Full => CopySkip::PaperQueueFull,
        EnqueueError::Unavailable | EnqueueError::Duplicate => CopySkip::PaperExecutionUnavailable,
    }
}

/// The admission key of a copy activity's paper work, one per target transaction.
pub fn activity_key(signature: &str) -> String {
    format!("activity:{signature}")
}

/// The admission key of a close of paper round `position_id`.
pub fn manual_close_key(position_id: i64) -> String {
    format!("manual:{position_id}")
}

/// The admission key of a reset of a task's paper run; one reset of a task at a time.
pub fn reset_key() -> String {
    "reset".to_owned()
}

/// The admission key of a task's removal; one removal of a task at a time.
pub fn delete_key() -> String {
    "delete".to_owned()
}

static ACTIVE: LazyLock<Mutex<Option<PaperExecutionHandle>>> = LazyLock::new(|| Mutex::new(None));

/// Current service-owned scheduler, if the copy service is running.
pub fn active() -> Option<PaperExecutionHandle> {
    ACTIVE.lock().ok().and_then(|handle| handle.clone())
}

struct ActiveGuard(PaperExecutionHandle);

impl Drop for ActiveGuard {
    fn drop(&mut self) {
        self.clear();
    }
}

impl ActiveGuard {
    fn clear(&self) {
        if let Ok(mut active) = ACTIVE.lock() {
            if active
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(&current.0, &self.0 .0))
            {
                *active = None;
            }
        }
    }
}

/// Install the scheduling capability before the service accepts observations.
pub fn install() -> (
    PaperExecutionHandle,
    watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let handle = PaperExecutionHandle::new(sender);
    let (stop, stopped) = watch::channel(false);
    if let Ok(mut active) = ACTIVE.lock() {
        *active = Some(handle.clone());
    }
    let settlement: Settlement = Arc::new(|database, outcome| {
        Box::pin(async move { super::notify::record(&database, outcome).await })
    });
    let join = tokio::spawn(run_dispatcher(
        handle.clone(),
        receiver,
        stopped,
        settlement,
    ));
    (handle, stop, join)
}

async fn run_dispatcher(
    handle: PaperExecutionHandle,
    mut receiver: mpsc::UnboundedReceiver<(i64, QueuedJob)>,
    mut stop: watch::Receiver<bool>,
    settlement: Settlement,
) {
    let active_guard = ActiveGuard(handle);
    let mut pending: HashMap<i64, VecDeque<QueuedJob>> = HashMap::new();
    let mut active: HashSet<i64> = HashSet::new();
    let mut joins = JoinSet::new();
    let (finished_sender, mut finished_receiver) = mpsc::unbounded_channel();
    let mut stopping = false;
    loop {
        tokio::select! {
            changed = stop.changed(), if !stopping => {
                if changed.is_err() || *stop.borrow() {
                    stopping = true;
                    active_guard.clear();
                    pending.clear();
                    receiver.close();
                    while let Ok((_, queued)) = receiver.try_recv() { drop(queued); }
                    if active.is_empty() { break; }
                }
            }
            incoming = receiver.recv(), if !stopping => {
                if let Some((task_id, queued)) = incoming {
                    if active.contains(&task_id) {
                        pending.entry(task_id).or_default().push_back(queued);
                    } else {
                        launch(task_id, queued, &mut active, &mut joins, stop.clone(), settlement.clone(), finished_sender.clone());
                    }
                } else {
                    stopping = true;
                    active_guard.clear();
                    pending.clear();
                    if active.is_empty() { break; }
                }
            }
            Some(task_id) = finished_receiver.recv() => {
                active.remove(&task_id);
                if !stopping {
                    if let Some(next) = pending.get_mut(&task_id).and_then(VecDeque::pop_front) {
                        if pending.get(&task_id).is_some_and(VecDeque::is_empty) { pending.remove(&task_id); }
                        launch(task_id, next, &mut active, &mut joins, stop.clone(), settlement.clone(), finished_sender.clone());
                    } else {
                        pending.remove(&task_id);
                    }
                }
            }
            Some(result) = joins.join_next(), if !joins.is_empty() => {
                log_join_result(result);
            }
        }
        if stopping && active.is_empty() {
            break;
        }
    }
    while let Some(result) = joins.join_next().await {
        log_join_result(result);
    }
}

fn log_join_result(result: Result<i64, tokio::task::JoinError>) {
    if let Err(error) = result {
        crate::logger::error(
            crate::logger::LogTag::Trader,
            &format!("Paper execution job failed: {error}"),
        );
    }
}

fn launch(
    task_id: i64,
    mut queued: QueuedJob,
    active: &mut HashSet<i64>,
    joins: &mut JoinSet<i64>,
    mut stop: watch::Receiver<bool>,
    settlement: Settlement,
    finished: mpsc::UnboundedSender<i64>,
) {
    active.insert(task_id);
    queued.admission.started();
    let admission = queued.admission;
    joins.spawn(async move {
        let _finished = ActiveJobGuard { task_id, finished };
        let _admission = admission;
        if *stop.borrow() {
            reject_completion(&mut queued.completion);
            return task_id;
        }
        let outcome = tokio::select! {
            biased;
            changed = stop.changed() => {
                let _ = changed;
                None
            },
            outcome = &mut queued.job => outcome,
        };
        if let Some(mut outcome) = outcome {
            if *stop.borrow() {
                reject_completion(&mut queued.completion);
                return task_id;
            }
            let decided_at = chrono::Utc::now();
            match &mut outcome {
                CopyOutcome::PaperFilled(decision) => decision.telemetry.decided_at = decided_at,
                CopyOutcome::PaperSellObserved(decision) => decision.telemetry.decided_at = decided_at,
                CopyOutcome::Skipped { decided_at: recorded, telemetry, .. } => {
                    *recorded = decided_at;
                    if let Some(telemetry) = telemetry { telemetry.decided_at = decided_at; }
                }
                _ => {}
            }
            if *stop.borrow() {
                reject_completion(&mut queued.completion);
                return task_id;
            }
            let settled = match settlement(queued.database.clone(), outcome.clone()).await {
                Ok(inserted) => Ok((outcome, inserted)),
                Err(error) => {
                    crate::logger::error(
                        crate::logger::LogTag::Trader,
                        &format!("Paper outcome settlement failed for task {task_id}, mint {:?}: {error}", queued.mint),
                    );
                    Err(error)
                }
            };
            if let Some(completion) = queued.completion.take() {
                let _ = completion.send(settled);
            }
        } else {
            reject_completion(&mut queued.completion);
        }
        task_id
    });
}

fn reject_completion(
    completion: &mut Option<oneshot::Sender<Result<(CopyOutcome, bool), crate::trader::Error>>>,
) {
    if let Some(completion) = completion.take() {
        let _ = completion.send(Err(crate::trader::Error::CopyTaskRejected {
            reason: CopySkip::PaperExecutionUnavailable,
        }));
    }
}

mod executor;
mod wanted;
pub(super) use executor::{
    as_paper, execute_paper_entry, observation_telemetry, skipped, skipped_activity, skipped_paper,
    skipped_paper_intent,
};
pub use wanted::paper_trade_wanted;

#[cfg(test)]
mod tests;
