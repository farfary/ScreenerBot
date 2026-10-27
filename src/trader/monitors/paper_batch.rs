// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The detached paper batch of a trader monitor: the paper decisions of one cycle run in
//! sequence in one task of their own, started after the cycle's live decisions, so the
//! landing wait of a paper fill never delays a live trade. A monitor keeps at most one
//! batch in flight; the paper scope is entered inside the paper executor, so the task it
//! runs in needs none.

use std::future::Future;

use tokio::task::JoinHandle;

use crate::logger::{self, LogTag};
use crate::positions::Book;

/// The paper batch slot of one monitor.
pub(super) struct PaperBatch {
    monitor: &'static str,
    in_flight: Option<JoinHandle<()>>,
}

impl PaperBatch {
    /// The empty slot of the `monitor` named in logs.
    pub(super) fn new(monitor: &'static str) -> Self {
        Self {
            monitor,
            in_flight: None,
        }
    }

    /// Starts `batch`, which runs `decisions` paper decisions, in its own task unless the
    /// previous batch of the monitor is still running. A cycle whose batch does not start
    /// drops its paper decisions; they are decided again next cycle. Returns whether the
    /// batch started.
    pub(super) fn launch<F>(&mut self, decisions: usize, batch: F) -> bool
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if self
            .in_flight
            .as_ref()
            .is_some_and(|task| !task.is_finished())
        {
            logger::info(
                LogTag::Trader,
                &format!(
                    "Paper batch of the {} monitor is still running; {decisions} paper decisions wait for the next cycle",
                    self.monitor
                ),
            );
            return false;
        }
        self.in_flight = Some(tokio::spawn(batch));
        true
    }
}

impl Drop for PaperBatch {
    /// A monitor that stops takes its paper batch with it.
    fn drop(&mut self) {
        if let Some(task) = self.in_flight.take() {
            task.abort();
        }
    }
}

/// Splits `items` into the live ones and the paper ones by `book_of`, each in its
/// original order.
pub(super) fn split_by_book<T>(items: Vec<T>, book_of: impl Fn(&T) -> Book) -> (Vec<T>, Vec<T>) {
    items
        .into_iter()
        .partition(|item| book_of(item) == Book::Live)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use tokio::sync::Notify;

    use super::*;

    /// One cycle of a monitor as the exit and entry monitors run it: the live decisions
    /// awaited in order, then the paper ones handed to the batch. A paper decision whose
    /// landing wait never ends stands for the slowest paper fill.
    async fn run_cycle(
        batch: &mut PaperBatch,
        decisions: Vec<(Book, &'static str)>,
        executed: Arc<Mutex<Vec<&'static str>>>,
        landing: Arc<Notify>,
    ) -> bool {
        let (live, paper) = split_by_book(decisions, |decision| decision.0);
        for (_, name) in live {
            executed.lock().unwrap().push(name);
        }
        let count = paper.len();
        batch.launch(count, async move {
            for (_, name) in paper {
                landing.notified().await;
                executed.lock().unwrap().push(name);
            }
        })
    }

    #[tokio::test]
    async fn a_blocked_paper_batch_never_delays_the_live_decisions_of_its_cycle() {
        let mut batch = PaperBatch::new("test");
        let executed = Arc::new(Mutex::new(Vec::new()));
        let landing = Arc::new(Notify::new());
        let decisions = vec![
            (Book::Paper, "paper-stop-loss"),
            (Book::Live, "live-stop-loss"),
            (Book::Paper, "paper-take-profit"),
            (Book::Live, "live-take-profit"),
        ];
        let started = tokio::time::timeout(
            Duration::from_secs(1),
            run_cycle(&mut batch, decisions, executed.clone(), landing.clone()),
        )
        .await
        .expect("the cycle waited on its paper batch");
        assert!(started);
        assert_eq!(
            *executed.lock().unwrap(),
            ["live-stop-loss", "live-take-profit"],
            "a live decision ran behind a paper fill"
        );

        landing.notify_one();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(
            *executed.lock().unwrap(),
            ["live-stop-loss", "live-take-profit", "paper-stop-loss"],
            "paper decisions run after the live ones, in order"
        );
    }

    #[tokio::test]
    async fn a_cycle_whose_paper_batch_is_still_running_drops_its_paper_decisions() {
        let mut batch = PaperBatch::new("test");
        let executed = Arc::new(Mutex::new(Vec::new()));
        let landing = Arc::new(Notify::new());
        assert!(
            run_cycle(
                &mut batch,
                vec![(Book::Paper, "first-paper")],
                executed.clone(),
                landing.clone(),
            )
            .await
        );
        let second = run_cycle(
            &mut batch,
            vec![(Book::Live, "second-live"), (Book::Paper, "second-paper")],
            executed.clone(),
            landing.clone(),
        )
        .await;
        assert!(!second, "a second paper batch started beside the first");
        assert_eq!(*executed.lock().unwrap(), ["second-live"]);

        landing.notify_one();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(
            *executed.lock().unwrap(),
            ["second-live", "first-paper"],
            "a dropped paper decision ran"
        );

        assert!(
            run_cycle(
                &mut batch,
                vec![(Book::Paper, "third-paper")],
                executed.clone(),
                landing.clone(),
            )
            .await,
            "a finished batch blocks the next one"
        );
    }
}
