// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy evaluation recorder: evaluations are queued and written in batches on the blocking pool, so an evaluation never waits on SQLite.

use std::sync::{Arc, OnceLock};

use chrono::Utc;
use tokio::sync::mpsc;

use crate::logger::{self, LogTag};
use crate::strategies::database::{self, EvaluationRecord};
use crate::strategies::types::EvaluationResult;

/// Evaluations queued before a caller waits for the writer to drain them.
const QUEUE_CAPACITY: usize = 4_096;

/// The most evaluations one transaction writes.
const BATCH_SIZE: usize = 512;

static QUEUE: OnceLock<mpsc::Sender<EvaluationRecord>> = OnceLock::new();

/// Start the writer of the strategies store. Later calls keep the running writer.
pub(crate) fn start() {
    QUEUE.get_or_init(|| spawn_writer(database::record_evaluations));
}

/// Queue one evaluation of `token_mint` for the store, stamped now.
pub(crate) async fn record(result: EvaluationResult, token_mint: &str) -> crate::Result<()> {
    let queue = QUEUE
        .get()
        .ok_or_else(|| crate::Error::internal_error("Strategy evaluation recorder not started"))?;
    queue
        .send(EvaluationRecord {
            result,
            token_mint: token_mint.to_owned(),
            evaluated_at: Utc::now(),
        })
        .await
        .map_err(|_| crate::Error::internal_error("Strategy evaluation recorder stopped"))
}

/// Spawn the task that drains the queue into `write`, returning the queue's sender.
fn spawn_writer<W>(write: W) -> mpsc::Sender<EvaluationRecord>
where
    W: Fn(&[EvaluationRecord]) -> crate::Result<()> + Send + Sync + 'static,
{
    let (sender, receiver) = mpsc::channel(QUEUE_CAPACITY);
    tokio::spawn(write_batches(receiver, Arc::new(write)));
    sender
}

/// Write every queued evaluation, one transaction per batch, each on the blocking pool.
/// Evaluations queued while a batch is written go out together in the next one.
async fn write_batches<W>(mut receiver: mpsc::Receiver<EvaluationRecord>, write: Arc<W>)
where
    W: Fn(&[EvaluationRecord]) -> crate::Result<()> + Send + Sync + 'static,
{
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    while receiver.recv_many(&mut batch, BATCH_SIZE).await > 0 {
        let records = std::mem::replace(&mut batch, Vec::with_capacity(BATCH_SIZE));
        let count = records.len();
        let write = Arc::clone(&write);
        let outcome = tokio::task::spawn_blocking(move || write(&records))
            .await
            .map_err(crate::Error::from)
            .and_then(|written| written);
        if let Err(e) = outcome {
            logger::warning(
                LogTag::System,
                &format!("Failed to record {count} strategy evaluations: {e}"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    fn evaluation(strategy_id: &str) -> EvaluationRecord {
        EvaluationRecord {
            result: EvaluationResult {
                strategy_id: strategy_id.to_owned(),
                result: false,
                confidence: 0.0,
                execution_time_ms: 1,
                details: HashMap::new(),
            },
            token_mint: "mint".to_owned(),
            evaluated_at: Utc::now(),
        }
    }

    /// A store write that blocks must not stall the runtime that queued it: on a
    /// single-threaded runtime a write run on the runtime thread would hold every other
    /// task for the whole write.
    #[tokio::test(flavor = "current_thread")]
    async fn a_blocked_store_write_never_stalls_the_runtime() {
        let blocked_write = Duration::from_secs(2);
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let gate = Mutex::new(gate);
        let queue = spawn_writer(move |_| {
            let _ = gate.lock().unwrap().recv_timeout(blocked_write);
            Ok(())
        });

        let started = Instant::now();
        queue.send(evaluation("entry")).await.unwrap();
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        let elapsed = started.elapsed();
        release.send(()).unwrap();

        assert!(
            elapsed < blocked_write / 2,
            "the runtime waited {elapsed:?} on a blocked store write"
        );
    }

    /// Evaluations queued while a batch is being written go out in one transaction, not
    /// one each.
    #[tokio::test(flavor = "current_thread")]
    async fn evaluations_queued_during_a_write_share_the_next_transaction() {
        let batches = Arc::new(Mutex::new(Vec::new()));
        let (release, gate) = std::sync::mpsc::channel::<()>();
        let gate = Mutex::new(gate);
        let recorded = Arc::clone(&batches);
        let queue = spawn_writer(move |records| {
            recorded.lock().unwrap().push(records.len());
            let _ = gate.lock().unwrap().recv_timeout(Duration::from_secs(5));
            Ok(())
        });

        queue.send(evaluation("entry")).await.unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while batches.lock().unwrap().is_empty() {
            assert!(Instant::now() < deadline, "the first write never started");
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        for _ in 0..99 {
            queue.send(evaluation("entry")).await.unwrap();
        }
        release.send(()).unwrap();
        release.send(()).unwrap();

        while batches.lock().unwrap().iter().sum::<usize>() < 100 {
            assert!(
                Instant::now() < deadline,
                "the queued evaluations were not written"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(*batches.lock().unwrap(), vec![1, 99]);
    }
}
