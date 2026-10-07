// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position state management — in-memory tracking of active positions, locking, and lifecycle.

pub use super::state_pending::*;

use super::db::carry_columns_not_booked;
use super::types::Position;
use crate::logger::{self, LogTag};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, LazyLock, OnceLock},
};
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};

// Global state containers
pub static POSITIONS: LazyLock<RwLock<Vec<Position>>> = LazyLock::new(|| RwLock::new(Vec::new()));

// Constant-time indexes
pub static SIG_TO_MINT_INDEX: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static MINT_TO_POSITION_INDEX: LazyLock<RwLock<HashMap<String, usize>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// Per-position locks
static POSITION_LOCKS: LazyLock<RwLock<HashMap<String, Arc<Mutex<()>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// Per-position booking locks, keyed by database id
static BOOKING_LOCKS: LazyLock<RwLock<HashMap<i64, Arc<Mutex<()>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// Pending open-swap registry: guards against duplicate opens when the first swap lands on-chain
// but local flow fails before persisting a position. Keys are token mints; values are expiry times.
// A plain mutex, never held across an await, so a dropped [`PendingOpenGuard`] can clear its entry.
static PENDING_OPEN_SWAPS: LazyLock<std::sync::Mutex<HashMap<String, DateTime<Utc>>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// The pending-open registry. A panic while holding the lock leaves a plain map
/// behind, which is still consistent, so a poisoned lock is recovered.
fn pending_open_swaps() -> std::sync::MutexGuard<'static, HashMap<String, DateTime<Utc>>> {
    PENDING_OPEN_SWAPS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// Global position creation semaphore to enforce max_open_positions atomically
// NOTE: Uses OnceLock because initialization requires config which isn't available at static init time
static GLOBAL_POSITION_SEMAPHORE: OnceLock<tokio::sync::Semaphore> = OnceLock::new();

/// Initialize the global position semaphore with the configured max open positions
/// MUST be called during positions system initialization, after config is loaded
pub fn init_global_position_semaphore(max_positions: usize) {
    GLOBAL_POSITION_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(max_positions));
}

/// Get a reference to the global position semaphore
/// Panics if not initialized - must call init_global_position_semaphore first
fn get_global_position_semaphore() -> &'static tokio::sync::Semaphore {
    GLOBAL_POSITION_SEMAPHORE.get().expect(
        "Global position semaphore not initialized. Call init_global_position_semaphore first.",
    )
}

// Optional: global last open timestamp (cooldown)
pub static LAST_OPEN_TIME: LazyLock<RwLock<Option<DateTime<Utc>>>> =
    LazyLock::new(|| RwLock::new(None));

// Default TTL in seconds for pending open swaps. During this window we block new opens for the mint.
pub const PENDING_OPEN_TTL_SECS: i64 = 120;

// Position lock guard
#[derive(Debug)]
pub struct PositionLockGuard {
    mint: String,
    _owned_guard: Option<OwnedMutexGuard<()>>,
}

impl Drop for PositionLockGuard {
    fn drop(&mut self) {
        logger::debug(
            LogTag::Positions,
            &format!("Released position lock for mint: {}", &self.mint),
        );
    }
}

impl PositionLockGuard {
    pub fn empty(mint: String) -> Self {
        Self {
            mint,
            _owned_guard: None,
        }
    }
}

/// Acquire a position-level lock
pub async fn acquire_position_lock(mint: &str) -> PositionLockGuard {
    let mint_key = mint.to_string();

    let lock: Arc<tokio::sync::Mutex<()>> = {
        let mut locks = POSITION_LOCKS.write().await;
        locks
            .entry(mint_key.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };

    let owned_guard = lock.clone().lock_owned().await;

    logger::debug(
        LogTag::Positions,
        &format!("Acquired position lock for mint: {}", &mint_key),
    );

    PositionLockGuard {
        mint: mint_key,
        _owned_guard: Some(owned_guard),
    }
}

/// Runs `work` under the booking lock of one position. A booking holds it from its read
/// until memory has adopted the committed row, so memory adopts the rows of one position
/// in commit order; the exit-submission write and its memory mirror hold it too, so no
/// booking publishes a row read before that write. Lock order: the mint lock, then this
/// lock, then `POSITIONS`. The lock is dropped from the map once no other caller holds or
/// awaits it.
pub(crate) async fn with_booking_lock<F: Future>(position_id: i64, work: F) -> F::Output {
    let lock = BOOKING_LOCKS
        .write()
        .await
        .entry(position_id)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    let guard = lock.lock_owned().await;
    let output = work.await;
    drop(guard);
    prune_booking_lock(position_id).await;
    output
}

/// Drops the booking lock of a position unless a caller holds or awaits it.
async fn prune_booking_lock(position_id: i64) {
    let mut locks = BOOKING_LOCKS.write().await;
    if locks
        .get(&position_id)
        .is_some_and(|lock| Arc::strong_count(lock) == 1)
    {
        locks.remove(&position_id);
    }
}

/// Makes `live` the committed row: every column a booking writes comes from `committed`,
/// while the columns other writers own and the fields that are never persisted keep their
/// in-memory values.
fn adopt_committed(live: &mut Position, committed: &Position) {
    let mut adopted = committed.clone();
    carry_columns_not_booked(&mut adopted, live);
    adopted.phantom_remove = live.phantom_remove;
    *live = adopted;
}

/// Publishes a committed row to the in-memory position of the same id. Returns false
/// when the position is not in memory.
pub(crate) async fn publish_committed(committed: &Position) -> bool {
    let Some(position_id) = committed.id else {
        return false;
    };
    update_position_state_by_id(position_id, |live| adopt_committed(live, committed)).await
}

/// Acquire a global position creation permit to enforce MAX_OPEN_POSITIONS atomically
/// This must be called BEFORE any position creation to prevent race conditions
pub async fn acquire_global_position_permit(
) -> crate::positions::Result<tokio::sync::SemaphorePermit<'static>> {
    let semaphore = get_global_position_semaphore();
    match semaphore.try_acquire() {
        Ok(permit) => {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "Acquired global position permit (available: {})",
                    semaphore.available_permits()
                ),
            );
            Ok(permit)
        }
        Err(_) => {
            let available = semaphore.available_permits();
            Err(crate::positions::Error::SlotUnavailable {
                remaining: available as usize,
            })
        }
    }
}

/// Release a global position permit (low-level).
///
/// Prefer [`release_position_slot`], which is idempotent per position. This primitive adds
/// a permit unconditionally and is only for reconciliation, which recomputes the whole
/// semaphore from the open-position count.
pub fn release_global_position_permit() {
    let semaphore = get_global_position_semaphore();
    semaphore.add_permits(1);
    logger::debug(
        LogTag::Positions,
        &format!(
            "Released global position permit (available: {})",
            semaphore.available_permits()
        ),
    );
}

/// Position IDs that currently hold a global position slot.
///
/// A slot must be released EXACTLY ONCE per position, and several terminal paths can run
/// for the SAME one: archiving or force-closing an OPEN position frees its slot straight
/// away, and the exit verification that was already queued frees it AGAIN when it lands.
/// Each double release hands the trader a slot it does not own, so it opens MORE positions
/// than `max_open_positions` — the risk limit silently exceeded, and only repaired by the
/// reconcile at the next startup. Tracking the holders makes releasing idempotent.
static SLOT_HOLDERS: LazyLock<RwLock<std::collections::HashSet<i64>>> =
    LazyLock::new(|| RwLock::new(std::collections::HashSet::new()));

/// Record that a position now holds a global slot (it consumed a permit).
pub async fn register_position_slot(position_id: i64) {
    SLOT_HOLDERS.write().await.insert(position_id);
}

/// Free the slot held by a position — at most once, however many terminal paths run.
pub async fn release_position_slot(position_id: i64) {
    let held = SLOT_HOLDERS.write().await.remove(&position_id);

    if !held {
        logger::debug(
            LogTag::Positions,
            &format!("Position {position_id} holds no slot - nothing to release"),
        );
        return;
    }

    release_global_position_permit();
}

/// Rebuild the slot registry from the positions that are actually open (startup).
pub async fn rebuild_position_slot_holders() {
    let open_ids: std::collections::HashSet<i64> = get_capacity_consuming_positions()
        .await
        .iter()
        .filter_map(|position| position.id)
        .collect();

    *SLOT_HOLDERS.write().await = open_ids;
}

/// Try to consume one global position permit at runtime (e.g. when an already-open
/// position is unarchived and re-enters active management). Returns true if a slot
/// was available. The permit is `forget()`-ten so it stays consumed for the
/// position's lifetime, matching how open positions hold their slot.
pub fn try_consume_global_position_permit() -> bool {
    let semaphore = get_global_position_semaphore();
    match semaphore.try_acquire() {
        Ok(permit) => {
            permit.forget();
            true
        }
        Err(_) => false,
    }
}

/// Add position to state
pub async fn add_position(position: Position) -> usize {
    let mut positions = POSITIONS.write().await;
    positions.push(position.clone());
    let index = positions.len() - 1;

    // Update indexes
    if let Some(ref sig) = position.entry_transaction_signature {
        SIG_TO_MINT_INDEX
            .write()
            .await
            .insert(sig.clone(), position.mint.clone());
    }
    if let Some(ref sig) = position.exit_transaction_signature {
        SIG_TO_MINT_INDEX
            .write()
            .await
            .insert(sig.clone(), position.mint.clone());
    }
    MINT_TO_POSITION_INDEX
        .write()
        .await
        .insert(position.mint.clone(), index);

    // Clear any pending-open flag for this mint now that the position exists
    {
        let mut pending = pending_open_swaps();
        if pending.remove(&position.mint).is_some() {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "Cleared pending-open after position add for mint: {}",
                    &position.mint
                ),
            );
        }
    }

    index
}

/// Update position in state
pub async fn update_position_state(mint: &str, updater: impl FnOnce(&mut Position)) -> bool {
    let mut positions = POSITIONS.write().await;
    if let Some(position) = positions.iter_mut().find(|p| p.mint == mint) {
        updater(position);
        true
    } else {
        false
    }
}

/// Update position in state by position ID (database ID)
/// Use this when multiple positions may exist for the same token to ensure
/// updates target the correct specific position.
pub async fn update_position_state_by_id(
    position_id: i64,
    updater: impl FnOnce(&mut Position),
) -> bool {
    let mut positions = POSITIONS.write().await;
    if let Some(position) = positions.iter_mut().find(|p| p.id == Some(position_id)) {
        updater(position);
        true
    } else {
        false
    }
}

/// Retrieve a position by database ID from in-memory state
pub async fn get_position_by_id(position_id: i64) -> Option<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .find(|p| p.id == Some(position_id))
        .cloned()
}

/// Drop every signature the index maps to this mint.
///
/// A position owns MORE signatures than the two on its struct: every partial exit and
/// every DCA add registers its own signature in the index. Removing only
/// entry/exit_transaction_signature (as this used to) leaked one index entry per
/// partial/DCA of every position ever removed.
async fn remove_all_signatures_for_mint(mint: &str) {
    let mut index = SIG_TO_MINT_INDEX.write().await;
    index.retain(|_, indexed_mint| indexed_mint != mint);
}

/// Set the archived flag (and archived_at) on an in-memory position by ID.
/// Returns true if a matching position was found and updated.
pub async fn set_position_archived_in_memory(position_id: i64, archived: bool) -> bool {
    update_position_state_by_id(position_id, |p| {
        p.archived = archived;
        p.archived_at = if archived { Some(Utc::now()) } else { None };
    })
    .await
}

/// Mirror a recorded full-exit submission into memory: the exit signature, the market exit
/// price and the pending closed reason. A position whose exit is already verified keeps
/// its booked values. Returns true if the position was found.
pub async fn set_exit_submission_in_memory(
    position_id: i64,
    exit_signature: &str,
    exit_price: f64,
    closed_reason: &str,
) -> bool {
    update_position_state_by_id(position_id, |p| {
        if p.transaction_exit_verified {
            return;
        }
        p.exit_transaction_signature = Some(exit_signature.to_owned());
        p.exit_price = Some(exit_price);
        p.closed_reason = Some(closed_reason.to_owned());
    })
    .await
}

/// Change ownership in memory after persistence so evaluators see it immediately.
pub async fn set_position_management_in_memory(
    position_id: i64,
    management: crate::positions::types::PositionManagement,
) -> bool {
    update_position_state_by_id(position_id, |p| {
        p.management = management;
    })
    .await
}

/// Remove a position from state by database ID (mint is not unique — a token can
/// have multiple positions, so deletes must target the exact row).
///
/// Only the row's own entry and exit signatures leave the index while another row of the
/// mint remains; the partial-exit and DCA signatures the index also maps to the mint cannot
/// be told apart by row, so they leave only with the mint's last row. The mint's lock is
/// dropped from the map only when no caller holds or awaits it.
pub async fn remove_position_by_id(position_id: i64) -> Option<Position> {
    let mut positions = POSITIONS.write().await;

    let index = positions.iter().position(|p| p.id == Some(position_id))?;
    let removed = positions.remove(index);
    let mint_still_held = positions.iter().any(|p| p.mint == removed.mint);

    if mint_still_held {
        let mut signatures = SIG_TO_MINT_INDEX.write().await;
        for signature in [
            &removed.entry_transaction_signature,
            &removed.exit_transaction_signature,
        ]
        .into_iter()
        .flatten()
        {
            signatures.remove(signature);
        }
    } else {
        remove_all_signatures_for_mint(&removed.mint).await;
    }

    {
        let mut locks = POSITION_LOCKS.write().await;
        if locks
            .get(&removed.mint)
            .is_some_and(|lock| Arc::strong_count(lock) == 1)
        {
            locks.remove(&removed.mint);
        }
    }

    pending_open_swaps().remove(&removed.mint);

    rebuild_position_indexes(&positions).await;

    Some(removed)
}

/// Rebuild position indexes after removal
async fn rebuild_position_indexes(positions: &[Position]) {
    let mut mint_to_index = MINT_TO_POSITION_INDEX.write().await;
    mint_to_index.clear();

    for (index, position) in positions.iter().enumerate() {
        mint_to_index.insert(position.mint.clone(), index);
    }
}

/// Is this position OPEN — the single definition, used by every reader.
///
/// A close that is still confirming counts as open (`exit_transaction_signature` set but
/// not yet verified): the tokens are still held, and the close/verification paths must be
/// able to find the position they are working on.
///
/// Having two definitions of "has a position" is what made a buy impossible after a close:
/// the read side (`get_position_by_mint`) used to fall back to ANY position with that mint,
/// closed ones included, so the dashboard saw a dead position as "held" and switched the
/// user's Buy to an Add — which the write side then rejected with "no open position",
/// because `is_open_position` (below) correctly excluded it.
pub(crate) fn is_position_open(position: &Position) -> bool {
    !position.archived
        && position.position_type == "buy"
        && position.exit_time.is_none()
        && (position.exit_transaction_signature.is_none() || !position.transaction_exit_verified)
}

/// The OPEN position for a mint, or None. A closed position is never returned —
/// history is reached explicitly by position id (`get_position_by_id`).
pub async fn get_position_by_mint(mint: &str) -> Option<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .find(|p| p.mint == mint && is_position_open(p))
        .cloned()
}

/// Get all open positions (archived positions are excluded — they live in the Archived tab)
pub async fn get_open_positions() -> Vec<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .filter(|p| is_position_open(p))
        .cloned()
        .collect()
}

/// Get all closed positions (archived positions are excluded — they live in the Archived tab)
pub async fn get_closed_positions() -> Vec<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .filter(|p| !p.archived && p.transaction_exit_verified)
        .cloned()
        .collect()
}

/// Get all archived positions (user removed them from the open/closed lists)
pub async fn get_archived_positions() -> Vec<Position> {
    let positions = POSITIONS.read().await;
    positions.iter().filter(|p| p.archived).cloned().collect()
}

/// Open positions that consume the bot's trading capacity.
///
/// Wallet-derived rounds are excluded: they were never opened by us, never consumed a
/// global position permit, and counting them would let a user who already holds twenty
/// tokens exhaust `max_open_positions` before the trader ever places a trade.
pub async fn get_capacity_consuming_positions() -> Vec<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .filter(|p| is_position_open(p) && !p.is_wallet_derived())
        .cloned()
        .collect()
}

/// Get count of open positions
pub async fn get_open_positions_count() -> usize {
    get_open_positions().await.len()
}

/// Check if position is open for given mint
pub async fn is_open_position(mint: &str) -> bool {
    // Check existing open position first
    {
        let positions = POSITIONS.read().await;
        if positions
            .iter()
            .any(|p| p.mint == mint && is_position_open(p))
        {
            return true;
        }
    }

    // Then check pending-open window (lazily expire any stale entries)
    {
        let now = Utc::now();
        let is_pending = {
            let mut pending = pending_open_swaps();
            let is_pending = pending.get(mint).is_some_and(|exp| *exp > now);
            pending.retain(|m, exp| {
                let live = *exp > now;
                if !live {
                    logger::debug(
                        LogTag::Positions,
                        &format!("Pending-open expired for mint: {m}"),
                    );
                }
                live
            });
            is_pending
        };

        if is_pending {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "is_open_position pending-open lock active for mint: {}",
                    mint
                ),
            );
            return true;
        }
    }

    false
}

/// Get list of open position mints
pub async fn get_open_mints() -> Vec<String> {
    get_open_positions()
        .await
        .iter()
        .map(|p| p.mint.clone())
        .collect()
}

/// Get position index by mint
pub async fn get_position_index_by_mint(mint: &str) -> Option<usize> {
    let mint_to_index = MINT_TO_POSITION_INDEX.read().await;
    mint_to_index.get(mint).copied()
}

/// Find mint by signature
pub async fn get_mint_by_signature(signature: &str) -> Option<String> {
    let sig_to_mint = SIG_TO_MINT_INDEX.read().await;
    sig_to_mint.get(signature).cloned()
}

/// Add signature to index
pub async fn add_signature_to_index(signature: &str, mint: &str) {
    SIG_TO_MINT_INDEX
        .write()
        .await
        .insert(signature.to_string(), mint.to_string());
}

/// Remove signature from index (used when clearing failed exit for retry)
pub async fn remove_signature_from_index(signature: &str) {
    SIG_TO_MINT_INDEX.write().await.remove(signature);
}

/// Mark `mint` as having an open swap pending for `ttl_secs` seconds, so no
/// second open can start for it, and return the guard that owns the mark.
///
/// The guard clears the mark when it is dropped, which covers every early
/// return of the open path: a quote that failed, a swap that provably never
/// reached the chain. Once the swap may be in flight the caller calls
/// [`PendingOpenGuard::keep`], and the mark then lives until the position is
/// added (which clears it), [`PendingOpenGuard::release`] is called on a
/// proven never-sent failure, or the TTL expires.
pub fn hold_pending_open(mint: &str, ttl_secs: i64) -> PendingOpenGuard {
    let expires_at = Utc::now() + chrono::Duration::seconds(ttl_secs);
    pending_open_swaps().insert(mint.to_string(), expires_at);
    logger::debug(
        LogTag::Positions,
        &format!(
            "Set pending-open for mint: {} (ttl {}s, until {})",
            mint, ttl_secs, expires_at
        ),
    );
    PendingOpenGuard {
        mint: mint.to_string(),
        expires_at,
        clear_on_drop: true,
    }
}

/// Owner of one pending-open mark; see [`hold_pending_open`].
#[derive(Debug)]
pub struct PendingOpenGuard {
    mint: String,
    expires_at: DateTime<Utc>,
    clear_on_drop: bool,
}

impl PendingOpenGuard {
    /// Keep the mark past this guard: the swap may be in flight, and a buy
    /// that may land must not be opened twice.
    pub fn keep(&mut self) {
        self.clear_on_drop = false;
    }

    /// Clear the mark now: the swap provably never reached the chain.
    pub fn release(mut self) {
        self.clear_on_drop = true;
    }

    /// Remove this guard's own mark. A mark set again since — by a later open
    /// of the same mint — carries a different expiry and is left alone.
    fn clear(&self) {
        let mut pending = pending_open_swaps();
        if pending.get(&self.mint) == Some(&self.expires_at) {
            pending.remove(&self.mint);
            logger::debug(
                LogTag::Positions,
                &format!("Cleared pending-open for mint: {}", self.mint),
            );
        }
    }
}

impl Drop for PendingOpenGuard {
    fn drop(&mut self) {
        if self.clear_on_drop {
            self.clear();
        }
    }
}

/// Clear a mint's pending open swap state, if present
pub fn clear_pending_open(mint: &str) {
    if pending_open_swaps().remove(mint).is_some() {
        logger::debug(
            LogTag::Positions,
            &format!("Cleared pending-open for mint: {mint}"),
        );
    }
}

/// Reconcile global semaphore capacity with currently open positions at startup.
/// This is CRITICAL to prevent exceeding MAX_OPEN_POSITIONS after a process restart.
/// Existing open positions did not re-acquire permits; we retroactively consume one
/// permit per open position (up to capacity). If there are more open positions than
/// MAX_OPEN_POSITIONS we log a warning and consume all available permits.
pub async fn reconcile_global_position_semaphore(max_open: usize) {
    use crate::logger::{self, LogTag};

    let semaphore = get_global_position_semaphore();
    // Only positions that actually consumed a permit are reconciled against the
    // semaphore; wallet-derived rounds never took one.
    let open_positions = get_capacity_consuming_positions().await; // clones but infrequent (startup)
    let open_count = open_positions.len();
    let available_before = semaphore.available_permits();
    let consumed_before = max_open - available_before;

    // Check for leaked permits (consumed > open positions)
    if consumed_before > open_count {
        let leaked = consumed_before - open_count;
        logger::warning(
      LogTag::Positions,
      &format!(
 "Semaphore audit: {} leaked permits detected ({} consumed, {} open positions). Releasing leaked permits...",
        leaked, consumed_before, open_count
      )
    );

        // Release leaked permits
        for _ in 0..leaked {
            release_global_position_permit();
        }

        logger::info(
            LogTag::Positions,
            &format!(
                "Released {} leaked permits. Available: {} -> {}",
                leaked,
                available_before,
                semaphore.available_permits()
            ),
        );

        return;
    }

    // No open positions - nothing to reconcile
    if open_count == 0 {
        logger::debug(
            LogTag::Positions,
            "Semaphore reconcile: no open positions, all permits available",
        );
        return;
    }

    // Consume permits for existing open positions
    let mut consumed = 0usize;
    for _ in 0..open_count {
        match semaphore.try_acquire() {
            Ok(permit) => {
                permit.forget(); // keep slot consumed for lifetime of position
                consumed += 1;
            }
            Err(_) => {
                break;
            }
        }
    }

    let available_after = semaphore.available_permits();
    if consumed < open_count {
        logger::warning(
            LogTag::Positions,
            &format!(
 "Semaphore reconcile: {} open positions exceed capacity (consumed {} of {}, available after {})",
        open_count,
        consumed,
        max_open,
        available_after
      ),
        );
    } else {
        logger::info(
            LogTag::Positions,
            &format!(
                "Semaphore reconcile: consumed {} permits for {} open positions (avail {} -> {})",
                consumed, open_count, available_before, available_after
            ),
        );
    }
}

/// Get active frozen cooldowns - stub implementation
pub async fn get_active_frozen_cooldowns() -> Vec<(String, i64)> {
    // Placeholder - no cooldown functionality in new module yet
    Vec::new()
}

/// Check if a token was recently closed and is in cooldown period
/// Returns true if the token should be blocked from re-entry
pub async fn is_token_in_cooldown(mint: &str) -> bool {
    use chrono::{Duration as ChronoDuration, Utc};

    let now = Utc::now();
    let cooldown_minutes =
        crate::config::with_config(|cfg| cfg.trader.position_close_cooldown_minutes);
    let cutoff = now - ChronoDuration::minutes(cooldown_minutes);

    let positions = POSITIONS.read().await;
    positions.iter().any(|p| {
        p.mint == mint
            && p.transaction_exit_verified
            && p.exit_time.is_some_and(|exit_time| exit_time > cutoff)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_released_booking_lock_leaves_no_entry_behind() {
        const POSITION_ID: i64 = -7;
        let first = tokio::spawn(with_booking_lock(POSITION_ID, async {
            tokio::task::yield_now().await;
        }));
        let second = tokio::spawn(with_booking_lock(POSITION_ID, async {}));
        first.await.expect("first booking");
        second.await.expect("second booking");

        assert!(!BOOKING_LOCKS.read().await.contains_key(&POSITION_ID));
    }
}
