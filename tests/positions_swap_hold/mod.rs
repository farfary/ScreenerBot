// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Swap holds against a real positions store and a scripted settlement reader: a held swap
//! blocks its mint and its reconciliation, and leaves only on a settled proof or the hold
//! limit.
//!
//! * `partial_exit` — a held partial exit refuses the next partial exit and returns its
//!   take-profit step only on proof that nothing was sold.

// See `tests/trader_exits.rs` — the config guard is a `std::sync::Mutex` on purpose, and
// holding it across an await cannot deadlock in a per-test runtime that nothing else shares.
#![allow(clippy::await_holding_lock)]

mod partial_exit;

use crate::common;

use chrono::{DateTime, Utc};
use screenerbot::chains::{
    ChainId, Holding, RawAmount, SettlementReader, SignatureCheck, SignatureVerdict,
};
use screenerbot::positions::swap_hold::{self, HoldOutcome, ReleasedHold, SwapHold};
use screenerbot::positions::{db, state, Position};

const HELD: u128 = 1_000_000;
const STEP: u32 = 2;
const BOUND: u64 = 1_000;

/// A settlement reader whose bound and settled holding are fixed by the test.
struct ScriptedReader {
    bound: u64,
    settled: Option<RawAmount>,
}

#[async_trait::async_trait]
impl SettlementReader for ScriptedReader {
    async fn signature_verdicts(
        &self,
        checks: &[SignatureCheck],
    ) -> screenerbot::chains::Result<Vec<SignatureVerdict>> {
        Ok(vec![SignatureVerdict::Pending; checks.len()])
    }

    async fn holding(&self, _owner: &str, _asset: &str) -> screenerbot::chains::Result<Holding> {
        Err(screenerbot::chains::Error::SettlementRead {
            chain: ChainId::Solana,
            detail: "a hold is settled from the settled holding only".to_owned(),
        })
    }

    async fn expiry_bound(&self) -> screenerbot::chains::Result<u64> {
        Ok(self.bound)
    }

    async fn settled_holding(
        &self,
        _owner: &str,
        _asset: &str,
        _expiry_bound: u64,
    ) -> screenerbot::chains::Result<Option<Holding>> {
        Ok(self.settled.map(|amount| Holding {
            amount,
            frozen: false,
        }))
    }
}

fn reader(settled: Option<u128>) -> ScriptedReader {
    ScriptedReader {
        bound: BOUND,
        settled: settled.map(RawAmount::new),
    }
}

/// Opens an isolated store and an OPEN position holding [`HELD`] that took take-profit step
/// [`STEP`], persisted and in memory.
async fn open_position() -> i64 {
    screenerbot::paths::ensure_all_directories().expect("create data directories");
    common::configure_own_wallet();
    common::seed_decimals(common::TEST_MINT, 6);
    screenerbot::positions::initialize_positions_database()
        .await
        .expect("initialise positions database");
    store_position(|_| {}).await
}

/// Persists one more position of the test mint holding [`HELD`] at step [`STEP`] and loads
/// it into memory.
async fn store_position(configure: impl FnOnce(&mut Position)) -> i64 {
    let mut position = common::test_position(1.0, 1.0);
    position.id = None;
    position.token_amount = Some(RawAmount::new(HELD));
    position.remaining_token_amount = Some(RawAmount::new(HELD));
    position.take_profit_level = STEP;
    configure(&mut position);
    let id = db::save_position(&position)
        .await
        .expect("persist test position");
    position.id = Some(id);
    state::add_position(position).await;
    id
}

/// The holds stored for `position_id`, as storage lists them.
async fn stored_holds_of(position_id: i64) -> Vec<SwapHold> {
    let raw = db::get_metadata("swap_holds")
        .await
        .expect("read stored holds")
        .unwrap_or_default();
    if raw.is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<SwapHold>>(&raw)
        .expect("stored holds decode")
        .into_iter()
        .filter(|hold| hold.position_id == position_id)
        .collect()
}

/// The readings that treat the mint or the position as busy whatever kind of swap is held:
/// the reconciliation's mint set, a late fill's other-swap check and the position's own
/// pending-swap check.
async fn mint_busy(position_id: i64) -> [bool; 3] {
    let mint = common::TEST_MINT;
    [
        state::mints_with_pending_swaps().await.contains(mint),
        state::other_swap_in_flight(mint, position_id, None).await,
        state::position_has_pending_swap(mint, position_id).await,
    ]
}

fn outcomes(released: &[ReleasedHold]) -> Vec<HoldOutcome> {
    released.iter().map(|released| released.outcome).collect()
}

async fn settle(reader: &ScriptedReader, now: DateTime<Utc>) -> Vec<ReleasedHold> {
    swap_hold::settle_swap_holds_on(reader, "wallet", now).await
}
