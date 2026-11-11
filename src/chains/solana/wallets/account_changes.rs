// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Own-change streams for addresses that thousands of other transactions reference.
//!
//! `accountSubscribe` reports the slots in which a watched address's own account
//! changed; every transaction the address signs pays its fee from that account. The
//! stream turns each changed slot into the address's signatures in that slot, and
//! the ledger here proves the references in every other slot quiet, so they are
//! handled without a `getTransaction`. A slot is provable only while the stream is
//! live on the current transport connection and once it has settled.

use std::collections::{BTreeSet, HashMap};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use async_trait::async_trait;

use crate::chains::solana::rpc::{self, ConnectionState, TopicEvent};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::transactions::fetcher::TransactionFetcher;
use crate::wallets::watch::runtime::NotificationStream;
use crate::wallets::watch::{ReferenceVerdict, StreamEvent, WatchNotification, PAGE_SIZE};

/// Seconds after its block time before a reference's slot counts as settled: the
/// change notification of that slot would have arrived by then, with margin for a
/// lagging stream and block-time skew. Only the poll's cursor waits on it; the
/// address's own signatures arrive through the change stream.
const SETTLE_SECS: i64 = 10;
/// Changed slots kept per address. Older slots stop being provable.
const RETAINED_CHANGES: usize = 4096;
/// Reads of the signature index while it has not reached a changed slot yet.
const HEAD_LOOKUP_ATTEMPTS: usize = 8;
const HEAD_LOOKUP_DELAY: Duration = Duration::from_millis(300);
/// Pages read back from the head to find a changed slot that has scrolled away.
const HEAD_LOOKUP_PAGES: usize = 3;

/// What one address's live change stream has established.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Coverage {
    /// The transport connection the stream was confirmed on.
    generation: u64,
    /// The first slot whose change would have been reported.
    from_slot: u64,
    /// Slots in which the account changed.
    changed: BTreeSet<u64>,
}

impl Coverage {
    fn record(&mut self, slot: u64) {
        self.changed.insert(slot);
        while self.changed.len() > RETAINED_CHANGES {
            if let Some(oldest) = self.changed.pop_first() {
                self.from_slot = self.from_slot.max(oldest + 1);
            }
        }
    }
}

static LEDGER: LazyLock<Mutex<HashMap<String, Coverage>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn ledger() -> std::sync::MutexGuard<'static, HashMap<String, Coverage>> {
    LEDGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn begin(address: &str, generation: u64, from_slot: u64) {
    ledger().insert(
        address.to_owned(),
        Coverage {
            generation,
            from_slot,
            changed: BTreeSet::new(),
        },
    );
}

fn record(address: &str, slot: u64) {
    if let Some(coverage) = ledger().get_mut(address) {
        coverage.record(slot);
    }
}

fn end(address: &str) {
    ledger().remove(address);
}

/// What the ledger proves about one address's references, read once for a page.
pub(super) struct Proof {
    coverage: Option<Coverage>,
    generation: u64,
    connected: bool,
    now: i64,
}

impl Proof {
    /// A snapshot of what is established for `address` right now.
    pub(super) fn for_address(address: &str) -> Self {
        Self {
            coverage: ledger().get(address).cloned(),
            generation: rpc::connection_generation(),
            connected: *rpc::connection_state().borrow() == ConnectionState::Connected,
            now: chrono::Utc::now().timestamp(),
        }
    }

    /// The verdict on a reference in `slot` with `block_time`.
    pub(super) fn verdict(&self, slot: u64, block_time: Option<i64>) -> ReferenceVerdict {
        let Some(coverage) = self
            .coverage
            .as_ref()
            .filter(|coverage| self.connected && coverage.generation == self.generation)
        else {
            return ReferenceVerdict::Unknown;
        };
        if slot < coverage.from_slot || coverage.changed.contains(&slot) {
            return ReferenceVerdict::Unknown;
        }
        match block_time {
            Some(time) if self.now - time >= SETTLE_SECS => ReferenceVerdict::Quiet,
            Some(_) => ReferenceVerdict::Unsettled,
            None => ReferenceVerdict::Unknown,
        }
    }
}

/// The live stream of one address's own changes.
pub(super) struct AccountChangeStream {
    address: String,
    pubkey: Pubkey,
    subscription: rpc::Subscription,
    /// Signatures found in changed slots and not handed out yet, newest first.
    pending: Vec<WatchNotification>,
}

impl AccountChangeStream {
    pub(super) fn new(address: &str, pubkey: Pubkey, subscription: rpc::Subscription) -> Self {
        Self {
            address: address.to_owned(),
            pubkey,
            subscription,
            pending: Vec::new(),
        }
    }

    /// Start coverage after a confirmation. The confirmation carries no slot: the
    /// slot read after it is at or past the one the server started reporting from.
    async fn confirmed(&self) {
        let generation = rpc::connection_generation();
        match super::runtime::watch_slot().await {
            Some(slot) => begin(&self.address, generation, slot + 1),
            None => end(&self.address),
        }
    }

    /// The address's signatures in `slot`, newest first. Empty when the index does
    /// not show the slot in time; the next poll then decodes them, because a
    /// changed slot is never proven quiet.
    async fn signatures_in(&self, slot: u64) -> Vec<WatchNotification> {
        let fetcher = TransactionFetcher::new();
        for _ in 0..HEAD_LOOKUP_ATTEMPTS {
            let mut before = None;
            for _ in 0..HEAD_LOOKUP_PAGES {
                let Ok(page) = fetcher
                    .fetch_signature_info_page(self.pubkey, PAGE_SIZE, before.as_deref(), None)
                    .await
                else {
                    return Vec::new();
                };
                let Some(newest) = page.first() else {
                    break;
                };
                if newest.slot < slot {
                    // The index has not reached the changed slot yet.
                    break;
                }
                let found: Vec<WatchNotification> = page
                    .iter()
                    .filter(|info| info.slot == slot)
                    .map(|info| WatchNotification {
                        signature: info.signature.to_string(),
                        failed: info.err.is_some(),
                    })
                    .collect();
                let oldest_slot = page.last().map_or(0, |info| info.slot);
                if !found.is_empty() || oldest_slot < slot || page.len() < PAGE_SIZE {
                    return found;
                }
                before = page.last().map(|info| info.signature.to_string());
            }
            tokio::time::sleep(HEAD_LOOKUP_DELAY).await;
        }
        Vec::new()
    }
}

#[async_trait]
impl NotificationStream for AccountChangeStream {
    async fn recv(&mut self) -> Option<StreamEvent> {
        loop {
            if let Some(notification) = self.pending.pop() {
                return Some(StreamEvent::Notification(notification));
            }
            match self.subscription.recv().await? {
                TopicEvent::Confirmed => {
                    self.confirmed().await;
                    return Some(StreamEvent::Live);
                }
                TopicEvent::AccountChanged { slot } => {
                    record(&self.address, slot);
                    self.pending = self.signatures_in(slot).await;
                }
                TopicEvent::Mention(_) => {}
            }
        }
    }
}

impl Drop for AccountChangeStream {
    fn drop(&mut self) {
        end(&self.address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage(from_slot: u64, changed: &[u64]) -> Coverage {
        Coverage {
            generation: 4,
            from_slot,
            changed: changed.iter().copied().collect(),
        }
    }

    fn verdict_of(
        coverage: Option<&Coverage>,
        generation: u64,
        connected: bool,
        slot: u64,
        block_time: Option<i64>,
        now: i64,
    ) -> ReferenceVerdict {
        Proof {
            coverage: coverage.cloned(),
            generation,
            connected,
            now,
        }
        .verdict(slot, block_time)
    }

    #[test]
    fn a_settled_unchanged_slot_on_the_live_connection_is_quiet() {
        let live = coverage(100, &[120]);
        assert_eq!(
            verdict_of(Some(&live), 4, true, 110, Some(1_000), 1_010),
            ReferenceVerdict::Quiet
        );
        assert_eq!(
            verdict_of(Some(&live), 4, true, 120, Some(1_000), 1_010),
            ReferenceVerdict::Unknown,
            "a changed slot is always decoded"
        );
        assert_eq!(
            verdict_of(Some(&live), 4, true, 110, Some(1_009), 1_010),
            ReferenceVerdict::Unsettled,
            "a slot younger than the settle time is read again later"
        );
    }

    #[test]
    fn nothing_is_quiet_outside_live_coverage() {
        let live = coverage(100, &[]);
        for (coverage, generation, connected, slot, block_time) in [
            (None, 4, true, 110, Some(1_000)),
            (Some(&live), 5, true, 110, Some(1_000)),
            (Some(&live), 4, false, 110, Some(1_000)),
            (Some(&live), 4, true, 99, Some(1_000)),
            (Some(&live), 4, true, 110, None),
        ] {
            assert_eq!(
                verdict_of(coverage, generation, connected, slot, block_time, 1_010),
                ReferenceVerdict::Unknown,
                "{coverage:?} gen {generation} connected {connected} slot {slot}"
            );
        }
    }

    #[test]
    fn dropping_old_changes_moves_coverage_past_them() {
        let mut live = coverage(0, &[]);
        for slot in 0..(RETAINED_CHANGES as u64 + 2) {
            live.record(slot * 2);
        }
        assert_eq!(live.changed.len(), RETAINED_CHANGES);
        assert_eq!(
            live.from_slot, 3,
            "slots up to the dropped change 2 are unprovable"
        );
    }
}
