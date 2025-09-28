use super::types::Position;
use crate::{
    arguments::is_debug_positions_enabled,
    logger::{log, LogTag},
    utils::safe_truncate,
};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
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

// Pending open-swap registry: guards against duplicate opens when the first swap lands on-chain
// but local flow fails before persisting a position. Keys are token mints; values are expiry times.
static PENDING_OPEN_SWAPS: LazyLock<RwLock<HashMap<String, DateTime<Utc>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// Global position creation semaphore to enforce MAX_OPEN_POSITIONS atomically
static GLOBAL_POSITION_SEMAPHORE: LazyLock<tokio::sync::Semaphore> = LazyLock::new(|| {
    use crate::trader::MAX_OPEN_POSITIONS;
    tokio::sync::Semaphore::new(MAX_OPEN_POSITIONS)
});

// Optional: global last open timestamp (cooldown)
pub static LAST_OPEN_TIME: LazyLock<RwLock<Option<DateTime<Utc>>>> =
    LazyLock::new(|| RwLock::new(None));

// Cooldown seconds (small to mitigate duplicate bursts; align with previous backup constant 5s)
pub const POSITION_OPEN_COOLDOWN_SECS: i64 = 5;

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
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🔓 Released position lock for mint: {}",
                    safe_truncate(&self.mint, 8)
                ),
            );
        }
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

    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🔒 Acquired position lock for mint: {}",
                safe_truncate(&mint_key, 8)
            ),
        );
    }

    PositionLockGuard {
        mint: mint_key,
        _owned_guard: Some(owned_guard),
    }
}

/// Acquire a global position creation permit to enforce MAX_OPEN_POSITIONS atomically
/// This must be called BEFORE any position creation to prevent race conditions
pub async fn acquire_global_position_permit(
) -> Result<tokio::sync::SemaphorePermit<'static>, String> {
    match GLOBAL_POSITION_SEMAPHORE.try_acquire() {
        Ok(permit) => {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "🟢 Acquired global position permit (available: {})",
                        GLOBAL_POSITION_SEMAPHORE.available_permits()
                    ),
                );
            }
            Ok(permit)
        }
        Err(_) => {
            let available = GLOBAL_POSITION_SEMAPHORE.available_permits();
            Err(format!(
                "No position slots available (permits: {})",
                available
            ))
        }
    }
}

/// Release a global position permit when a position is closed
/// This should be called after a position is successfully closed and verified
pub fn release_global_position_permit() {
    GLOBAL_POSITION_SEMAPHORE.add_permits(1);
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🔴 Released global position permit (available: {})",
                GLOBAL_POSITION_SEMAPHORE.available_permits()
            ),
        );
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
        let mut pending = PENDING_OPEN_SWAPS.write().await;
        if pending.remove(&position.mint).is_some() && is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🧹 Cleared pending-open after position add for mint: {}",
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

/// Remove position from state
pub async fn remove_position(mint: &str) -> Option<Position> {
    let mut positions = POSITIONS.write().await;

    if let Some(index) = positions.iter().position(|p| p.mint == mint) {
        let removed = positions.remove(index);

        // Update indexes
        if let Some(ref sig) = removed.entry_transaction_signature {
            SIG_TO_MINT_INDEX.write().await.remove(sig);
        }
        if let Some(ref sig) = removed.exit_transaction_signature {
            SIG_TO_MINT_INDEX.write().await.remove(sig);
        }
        MINT_TO_POSITION_INDEX.write().await.remove(&removed.mint);

        // Also clear any pending-open state for this mint (safety)
        {
            let mut pending = PENDING_OPEN_SWAPS.write().await;
            if pending.remove(&removed.mint).is_some() && is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "🧹 Cleared pending-open on removal for mint: {}",
                        &removed.mint
                    ),
                );
            }
        }

        // Rebuild position indexes for remaining positions
        rebuild_position_indexes(&positions).await;

        Some(removed)
    } else {
        None
    }
}

/// Rebuild position indexes after removal
async fn rebuild_position_indexes(positions: &[Position]) {
    let mut mint_to_index = MINT_TO_POSITION_INDEX.write().await;
    mint_to_index.clear();

    for (index, position) in positions.iter().enumerate() {
        mint_to_index.insert(position.mint.clone(), index);
    }
}

/// Get position by mint
pub async fn get_position_by_mint(mint: &str) -> Option<Position> {
    let positions = POSITIONS.read().await;
    positions.iter().find(|p| p.mint == mint).cloned()
}

/// Get all open positions
pub async fn get_open_positions() -> Vec<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .filter(|p| {
            p.position_type == "buy"
                && p.exit_time.is_none()
                && (!p.exit_transaction_signature.is_some() || !p.transaction_exit_verified)
        })
        .cloned()
        .collect()
}

/// Get all closed positions
pub async fn get_closed_positions() -> Vec<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .filter(|p| p.transaction_exit_verified)
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
        if positions.iter().any(|p| {
            p.mint == mint
                && p.position_type == "buy"
                && p.exit_time.is_none()
                && (!p.exit_transaction_signature.is_some() || !p.transaction_exit_verified)
        }) {
            return true;
        }
    }

    // Then check pending-open window (lazily expire any stale entries)
    {
        let now = Utc::now();
        let mut to_remove: Vec<String> = Vec::new();
        let pending_read = PENDING_OPEN_SWAPS.read().await;
        let is_pending = pending_read.get(mint).map_or(false, |exp| *exp > now);
        drop(pending_read);

        // Cleanup any expired entries opportunistically
        {
            let mut pending_write = PENDING_OPEN_SWAPS.write().await;
            for (m, exp) in pending_write.iter() {
                if *exp <= now {
                    to_remove.push(m.clone());
                }
            }
            for m in to_remove.drain(..) {
                pending_write.remove(&m);
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("⏳ Pending-open expired for mint: {}", m),
                    );
                }
            }
        }

        if is_pending {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "🚫 is_open_position pending-open lock active for mint: {}",
                        mint
                    ),
                );
            }
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

/// Mark a mint as having a pending open swap for ttl_secs seconds
pub async fn set_pending_open(mint: &str, ttl_secs: i64) {
    let expires_at = Utc::now() + chrono::Duration::seconds(ttl_secs);
    let mut pending = PENDING_OPEN_SWAPS.write().await;
    pending.insert(mint.to_string(), expires_at);
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "⏳ Set pending-open for mint: {} (ttl {}s, until {})",
                mint, ttl_secs, expires_at
            ),
        );
    }
}

/// Clear a mint's pending open swap state, if present
pub async fn clear_pending_open(mint: &str) {
    let mut pending = PENDING_OPEN_SWAPS.write().await;
    if pending.remove(mint).is_some() && is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!("🧹 Cleared pending-open for mint: {}", mint),
        );
    }
}

/// Reconcile global semaphore capacity with currently open positions at startup.
/// This is CRITICAL to prevent exceeding MAX_OPEN_POSITIONS after a process restart.
/// Existing open positions did not re-acquire permits; we retroactively consume one
/// permit per open position (up to capacity). If there are more open positions than
/// MAX_OPEN_POSITIONS we log a warning and consume all available permits.
pub async fn reconcile_global_position_semaphore(max_open: usize) {
    use crate::arguments::is_debug_positions_enabled;
    use crate::logger::{log, LogTag};

    let open_positions = get_open_positions().await; // clones but infrequent (startup)
    let open_count = open_positions.len();
    let available_before = GLOBAL_POSITION_SEMAPHORE.available_permits();

    if open_count == 0 {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                "Semaphore reconcile: no open positions",
            );
        }
        return;
    }

    let mut consumed = 0usize;
    for _ in 0..open_count {
        match GLOBAL_POSITION_SEMAPHORE.try_acquire() {
            Ok(permit) => {
                permit.forget(); // keep slot consumed for lifetime of position
                consumed += 1;
            }
            Err(_) => {
                break;
            }
        }
    }

    let available_after = GLOBAL_POSITION_SEMAPHORE.available_permits();
    if consumed < open_count {
        log(
            LogTag::Positions,
            "WARNING",
            &format!(
                "Semaphore reconcile: {} open positions exceed capacity (consumed {} of {}, available after {})",
                open_count,
                consumed,
                max_open,
                available_after
            )
        );
    } else if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
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
    use crate::trader::POSITION_CLOSE_COOLDOWN_MINUTES;
    use chrono::{Duration as ChronoDuration, Utc};

    let now = Utc::now();
    let cutoff = now - ChronoDuration::minutes(POSITION_CLOSE_COOLDOWN_MINUTES);

    let positions = POSITIONS.read().await;
    positions.iter().any(|p| {
        p.mint == mint
            && p.transaction_exit_verified
            && p.exit_time.map_or(false, |exit_time| exit_time > cutoff)
    })
}
