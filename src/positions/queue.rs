// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position verification queue — tracks pending transaction verifications with retry logic.

use super::types::{ApplyFailureDisposition, GiveUpReason, VerificationKind};
use super::Error;
use crate::chains::RawAmount;
use crate::errors::ErrorClass;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::collections::{HashSet, VecDeque};
use std::sync::LazyLock;
use tokio::sync::{Notify, RwLock};

/// Maximum verification attempts before giving up. Sized to span ~24h together
/// with the long-tail backoff table below.
const MAX_VERIFICATION_ATTEMPTS: u8 = 40;

/// Maximum age for a verification item before giving up. A swap can be confirmed
/// on-chain within seconds, but on a flaky/proxied network confirmation may be
/// delayed; we keep verifying (across restarts) for a full day rather than
/// abandoning a position whose exit already landed.
const MAX_VERIFICATION_AGE_HOURS: i64 = 24;

/// Attempts after which `requeue` stops re-inserting an item. The first twelve backoffs
/// sum to about four hours.
pub const MAX_REQUEUE_ATTEMPTS: u8 = 12;

/// Backoff intervals in seconds for verification retries (dynamic, widening).
/// Front-loaded for sub-minute confirmation in the common case, then ramping up
/// to hourly so a position is still re-checked many times across 24h without
/// hammering RPC: 5s..1m fast, then minutes, then hours up to 12h.
const BACKOFF_INTERVALS_SECS: [i64; 16] = [
    5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 14400, 28800, 43200, 86400,
];

/// Maximum backoff interval in seconds (used when attempts exceed table size): 12h.
const BACKOFF_MAX_SECS: i64 = 43200;

/// Jitter fraction for backoff randomization (±10%)
const BACKOFF_JITTER_FRACTION: f64 = 0.1;

#[derive(Debug, Clone)]
pub struct VerificationItem {
    pub signature: String,
    pub mint: String,
    pub position_id: Option<i64>,
    pub kind: VerificationKind,
    pub created_at: DateTime<Utc>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>, // backoff scheduling
    pub attempts: u8,
    pub expiry_height: Option<u64>,
    // Partial exit support
    pub is_partial_exit: bool,
    pub expected_exit_amount: Option<RawAmount>,
    pub requested_exit_percentage: Option<f64>,
    // DCA support
    pub is_dca: bool,
    /// The swap is confirmed on-chain: its verdict read it as landed, or only applying it to
    /// the position failed. A confirmed swap is never an orphan, so such an item is never
    /// settled as not landed.
    pub swap_confirmed: bool,
    /// Consecutive settlement reads that found the swap did not land. One read is never
    /// final, and each deferral widens the wait before the next read.
    pub not_landed_reads: u8,
}

impl VerificationItem {
    pub fn new(
        signature: String,
        mint: String,
        position_id: Option<i64>,
        kind: VerificationKind,
        expiry_height: Option<u64>,
    ) -> Self {
        Self {
            signature,
            mint,
            position_id,
            kind,
            created_at: Utc::now(),
            last_attempt_at: None,
            next_retry_at: None,
            attempts: 0,
            expiry_height,
            is_partial_exit: false,
            expected_exit_amount: None,
            requested_exit_percentage: None,
            is_dca: false,
            swap_confirmed: false,
            not_landed_reads: 0,
        }
    }

    /// Create verification item for partial exit
    pub fn new_partial_exit(
        signature: String,
        mint: String,
        position_id: Option<i64>,
        expected_exit_amount: RawAmount,
        exit_percentage: f64,
        expiry_height: Option<u64>,
    ) -> Self {
        Self {
            signature,
            mint,
            position_id,
            kind: VerificationKind::Exit,
            created_at: Utc::now(),
            last_attempt_at: None,
            next_retry_at: None,
            attempts: 0,
            expiry_height,
            is_partial_exit: true,
            expected_exit_amount: Some(expected_exit_amount),
            requested_exit_percentage: Some(exit_percentage),
            is_dca: false,
            swap_confirmed: false,
            not_landed_reads: 0,
        }
    }

    /// Create verification item for DCA entries
    pub fn new_dca(
        signature: String,
        mint: String,
        position_id: Option<i64>,
        expiry_height: Option<u64>,
    ) -> Self {
        let mut item = Self::new(
            signature,
            mint,
            position_id,
            VerificationKind::Entry,
            expiry_height,
        );
        item.is_dca = true;
        item
    }

    /// Check if verification should be abandoned due to excessive retries or age
    /// Returns Some(reason) if should give up, None otherwise
    pub fn should_give_up(&self) -> Option<GiveUpReason> {
        // Check attempt limit first
        if self.attempts >= MAX_VERIFICATION_ATTEMPTS {
            return Some(GiveUpReason::MaxAttemptsReached {
                attempts: self.attempts,
                max: MAX_VERIFICATION_ATTEMPTS,
            });
        }

        // Check age limit
        let age_hours = (Utc::now() - self.created_at).num_hours();
        if age_hours >= MAX_VERIFICATION_AGE_HOURS {
            return Some(GiveUpReason::MaxAgeReached {
                age_hours,
                max: MAX_VERIFICATION_AGE_HOURS,
            });
        }

        None
    }

    /// Decides whether an item whose transition failed to apply is retried or dropped.
    /// A non-retryable error drops first, then the verification give-up limits, then
    /// the requeue cap that `requeue` enforces.
    pub fn apply_failure_disposition(&self, error: &Error) -> ApplyFailureDisposition {
        if !error.is_retryable() {
            return ApplyFailureDisposition::Drop(GiveUpReason::ApplyRejected {
                error: error.to_string(),
            });
        }
        if let Some(reason) = self.should_give_up() {
            return ApplyFailureDisposition::Drop(reason);
        }
        if self.attempts >= MAX_REQUEUE_ATTEMPTS {
            return ApplyFailureDisposition::Drop(GiveUpReason::MaxAttemptsReached {
                attempts: self.attempts,
                max: MAX_REQUEUE_ATTEMPTS,
            });
        }
        ApplyFailureDisposition::Requeue
    }

    pub fn with_retry(&self) -> Self {
        let next_attempts = self.attempts.saturating_add(1);
        Self {
            last_attempt_at: Some(Utc::now()),
            next_retry_at: Some(retry_at(&self.signature, next_attempts)),
            attempts: next_attempts,
            ..self.clone()
        }
    }

    /// The same item after a settlement read found its swap did not land but could not act
    /// on it yet: the next read waits a backoff that widens with each consecutive such read.
    /// Verification attempts are unchanged, so a deferral never moves the item toward its
    /// attempt cap.
    pub fn deferred(&self) -> Self {
        let reads = self.not_landed_reads.saturating_add(1);
        Self {
            next_retry_at: Some(retry_at(&self.signature, reads)),
            not_landed_reads: reads,
            ..self.clone()
        }
    }

    /// True when the previous settlement read already found the swap did not land.
    pub fn not_landed_seen(&self) -> bool {
        self.not_landed_reads > 0
    }

    /// True while the chain has not confirmed the swap landed and the item carries the bound
    /// after which the chain can prove it never will: only such an item is settled by a
    /// signature verdict.
    pub fn awaits_settlement(&self) -> bool {
        !self.swap_confirmed && self.expiry_height.is_some()
    }

    /// The same verification started afresh: a new age and no attempts, with its kind,
    /// flags, expiry bound and confirmation kept.
    pub fn renewed(&self) -> Self {
        Self {
            created_at: Utc::now(),
            last_attempt_at: None,
            next_retry_at: None,
            attempts: 0,
            ..self.clone()
        }
    }

    pub fn age_seconds(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds()
    }

    pub fn is_due(&self) -> bool {
        match self.next_retry_at {
            None => true,
            Some(when) => Utc::now() >= when,
        }
    }
}

/// When retry `step` (1-based) of `signature` is due: the widening backoff table with a
/// deterministic jitter of up to ±10%, so retries of many items do not align.
fn retry_at(signature: &str, step: u8) -> DateTime<Utc> {
    let backoff_secs = BACKOFF_INTERVALS_SECS
        .get(usize::from(step).saturating_sub(1))
        .copied()
        .unwrap_or(BACKOFF_MAX_SECS);
    let jitter = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        signature.hash(&mut hasher);
        step.hash(&mut hasher);
        let h = hasher.finish();
        let sign = if (h & 1) == 0 { 1.0 } else { -1.0 };
        let frac = (((h >> 1) as f64) / ((u64::MAX >> 1) as f64)) * BACKOFF_JITTER_FRACTION;
        ((backoff_secs as f64) * frac * sign) as i64
    };
    Utc::now() + ChronoDuration::seconds(1_i64.max(backoff_secs + jitter))
}

/// Verification queue
pub struct VerificationQueue {
    items: VecDeque<VerificationItem>,
    /// Signatures dropped after a failed apply. Held in memory only, so a restart
    /// gives each of them one more attempt.
    abandoned: HashSet<String>,
}

impl VerificationQueue {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
            abandoned: HashSet::new(),
        }
    }

    /// Inserts the item unless its signature is already queued or abandoned.
    /// Returns whether it was inserted.
    pub fn enqueue(&mut self, item: VerificationItem) -> bool {
        if self.abandoned.contains(&item.signature)
            || self.items.iter().any(|i| i.signature == item.signature)
        {
            return false;
        }
        self.items.push_back(item);
        true
    }

    /// Refuses every later `enqueue` of the signature for the rest of the session.
    pub fn abandon(&mut self, signature: String) {
        self.abandoned.insert(signature);
    }

    pub fn poll_batch(&mut self, limit: usize) -> Vec<VerificationItem> {
        let mut batch = Vec::new();

        // Sort by priority: due items first, then recent (within 60s), then by age
        self.items.make_contiguous().sort_by(|a, b| {
            let a_due = a.is_due();
            let b_due = b.is_due();
            if a_due && !b_due {
                return std::cmp::Ordering::Less;
            }
            if !a_due && b_due {
                return std::cmp::Ordering::Greater;
            }

            let a_recent = a.age_seconds() <= 60;
            let b_recent = b.age_seconds() <= 60;

            match (a_recent, b_recent) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.age_seconds().cmp(&b.age_seconds()),
            }
        });

        // Drain up to limit DUE items and keep the rest
        let mut remaining: VecDeque<VerificationItem> = VecDeque::with_capacity(self.items.len());
        while let Some(item) = self.items.pop_front() {
            if batch.len() < limit && item.is_due() {
                batch.push(item);
            } else {
                remaining.push_back(item);
            }
        }
        self.items = remaining;

        batch
    }

    pub fn requeue(&mut self, item: VerificationItem) {
        // Allow more retries but with backoff; hard cap attempts to avoid infinite loops
        if item.attempts < MAX_REQUEUE_ATTEMPTS {
            self.items.push_back(item.with_retry());
        }
    }

    pub fn remove(&mut self, signature: &str) -> Option<VerificationItem> {
        if let Some(pos) = self.items.iter().position(|i| i.signature == signature) {
            self.items.remove(pos)
        } else {
            None
        }
    }

    /// Copies of the queued items that await settlement by a signature verdict and are due.
    pub fn settlement_candidates(&self) -> Vec<VerificationItem> {
        self.items
            .iter()
            .filter(|item| item.awaits_settlement() && item.is_due())
            .cloned()
            .collect()
    }

    /// The signatures of the unconfirmed items that lack the bound after which their swap can
    /// no longer land.
    pub fn unbounded_signatures(&self) -> Vec<String> {
        self.items
            .iter()
            .filter(|item| !item.swap_confirmed && item.expiry_height.is_none())
            .map(|item| item.signature.clone())
            .collect()
    }

    /// Gives `bound` to the items of `signatures` that are still unconfirmed and unbounded.
    /// `signatures` must be taken before `bound` was read, so the bound is read after each of
    /// them was submitted. Returns how many items got it.
    pub fn assign_expiry_bound(&mut self, bound: u64, signatures: &[String]) -> usize {
        let mut assigned = 0;
        for item in self.items.iter_mut().filter(|item| {
            !item.swap_confirmed
                && item.expiry_height.is_none()
                && signatures.contains(&item.signature)
        }) {
            item.expiry_height = Some(bound);
            assigned += 1;
        }
        assigned
    }

    /// Defers the next settlement read of the queued item of `signature` (see
    /// [`VerificationItem::deferred`]). Returns whether the signature is queued.
    pub fn defer_settlement(&mut self, signature: &str) -> bool {
        match self
            .items
            .iter_mut()
            .find(|item| item.signature == signature)
        {
            Some(item) => {
                *item = item.deferred();
                true
            }
            None => false,
        }
    }

    /// Records that a settlement read of `signature` found its swap still pending, so a later
    /// not-landed read starts a new count. Returns whether the signature is queued.
    pub fn clear_not_landed(&mut self, signature: &str) -> bool {
        match self
            .items
            .iter_mut()
            .find(|item| item.signature == signature)
        {
            Some(item) => {
                item.not_landed_reads = 0;
                true
            }
            None => false,
        }
    }

    /// Marks the queued item of `signature` as a swap the chain confirmed. Returns whether
    /// the signature is queued.
    pub fn mark_swap_confirmed(&mut self, signature: &str) -> bool {
        match self
            .items
            .iter_mut()
            .find(|item| item.signature == signature)
        {
            Some(item) => {
                item.swap_confirmed = true;
                true
            }
            None => false,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Global verification queue
static VERIFICATION_QUEUE: LazyLock<RwLock<VerificationQueue>> =
    LazyLock::new(|| RwLock::new(VerificationQueue::new()));

/// Wakes the verification worker the moment NEW work arrives.
///
/// The worker runs on an adaptive nap (5s while the queue is empty), and it computes that nap
/// BEFORE it looks at the queue — so a swap enqueued a moment after it dozed off sat there for
/// the rest of the nap before anyone even tried to verify it. That is dead time bolted onto the
/// front of every trade: the executors enqueue right after submission, the swap often
/// confirms within seconds, and a fresh item is immediately due (`next_retry_at: None`), so
/// there is nothing to wait for. It is why a DCA
/// whose notification already said "done" took another 10-15s to show up on the position: the
/// tokens and SOL only land on the position when the verification applies `DcaVerified`.
///
/// Only `enqueue_verification` signals — a `requeue_verification` after a failed attempt carries
/// a backoff and must NOT drag the worker back out of bed to look at an item that is not due.
/// `notify_one` stores a permit, so a signal fired while the worker is mid-cycle is not lost.
static QUEUE_SIGNAL: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Enqueue verification item. Returns whether it was inserted; the worker is
/// signalled only then.
pub async fn enqueue_verification(item: VerificationItem) -> bool {
    let inserted = {
        let mut queue = VERIFICATION_QUEUE.write().await;
        queue.enqueue(item)
    };
    if inserted {
        QUEUE_SIGNAL.notify_one();
    }
    inserted
}

/// Stop verifying a signature for the rest of the session.
pub async fn abandon_verification(signature: String) {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.abandon(signature);
}

/// Resolves as soon as new work is enqueued (see [`QUEUE_SIGNAL`]).
pub async fn wait_for_new_work() {
    QUEUE_SIGNAL.notified().await;
}

/// Poll batch of verification items
pub async fn poll_verification_batch(limit: usize) -> Vec<VerificationItem> {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.poll_batch(limit)
}

/// Requeue verification item
pub async fn requeue_verification(item: VerificationItem) {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.requeue(item);
}

/// Remove verification item
pub async fn remove_verification(signature: &str) -> Option<VerificationItem> {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.remove(signature)
}

/// Copies of the queued items that await settlement by a signature verdict and are due.
pub async fn settlement_candidates() -> Vec<VerificationItem> {
    let queue = VERIFICATION_QUEUE.read().await;
    queue.settlement_candidates()
}

/// The signatures of the unconfirmed queued items without an expiry bound.
pub async fn unbounded_signatures() -> Vec<String> {
    let queue = VERIFICATION_QUEUE.read().await;
    queue.unbounded_signatures()
}

/// Gives `bound` to the queued items of `signatures` that are still unconfirmed and unbounded.
pub async fn assign_expiry_bound(bound: u64, signatures: &[String]) -> usize {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.assign_expiry_bound(bound, signatures)
}

/// Defers the next settlement read of the queued item of `signature`.
pub async fn defer_settlement(signature: &str) -> bool {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.defer_settlement(signature)
}

/// Restarts the not-landed count of the queued item of `signature`.
pub async fn clear_not_landed(signature: &str) -> bool {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.clear_not_landed(signature)
}

/// Marks the queued item of `signature` as a swap the chain confirmed.
pub async fn mark_swap_confirmed(signature: &str) -> bool {
    let mut queue = VERIFICATION_QUEUE.write().await;
    queue.mark_swap_confirmed(signature)
}

/// Get queue status
pub async fn get_queue_status() -> (usize, Vec<String>) {
    let queue = VERIFICATION_QUEUE.read().await;
    let size = queue.len();
    let signatures: Vec<String> = queue.items.iter().map(|i| i.signature.clone()).collect();
    (size, signatures)
}

/// Get queue status synchronously (for metrics access)
/// Returns None if queue is currently locked
pub fn get_queue_status_sync() -> Option<(usize, Vec<String>)> {
    let queue = VERIFICATION_QUEUE.try_read().ok()?;
    let size = queue.len();
    let signatures: Vec<String> = queue.items.iter().map(|i| i.signature.clone()).collect();
    Some((size, signatures))
}
