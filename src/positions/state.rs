use super::{db, types::Position};
use crate::logger::{self, LogTag};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
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

// Pending partial exits registry (mint -> count of pending partial exits)
// We serialize to a single pending at a time, but using a count keeps API flexible
static PENDING_PARTIAL_EXITS: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPartialExit {
    pub signature: String,
    pub mint: String,
    pub position_id: i64,
    pub expected_exit_amount: u64,
    pub requested_exit_percentage: f64,
    pub expiry_height: Option<u64>,
    pub created_at: DateTime<Utc>,
}

static PENDING_PARTIAL_EXIT_DETAILS: LazyLock<RwLock<HashMap<String, PendingPartialExit>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
const PENDING_PARTIAL_EXIT_METADATA_KEY: &str = "pending_partial_exits";

// Pending open-swap registry: guards against duplicate opens when the first swap lands on-chain
// but local flow fails before persisting a position. Keys are token mints; values are expiry times.
static PENDING_OPEN_SWAPS: LazyLock<RwLock<HashMap<String, DateTime<Utc>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// Pending DCA swaps registry: ensures DCA verifications survive restarts and duplicate submissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDcaSwap {
    pub signature: String,
    pub mint: String,
    pub position_id: i64,
    pub expiry_height: Option<u64>,
    pub created_at: DateTime<Utc>,
}

static PENDING_DCA_SWAPS: LazyLock<RwLock<HashMap<String, PendingDcaSwap>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

const PENDING_DCA_METADATA_KEY: &str = "pending_dca_swaps";

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
            &format!("🔓 Released position lock for mint: {}", &self.mint),
        );
    }
}

/// Mark that a partial exit is pending for a mint (increments count)
pub async fn mark_partial_exit_pending(mint: &str) {
    let mut map = PENDING_PARTIAL_EXITS.write().await;
    let counter = map.entry(mint.to_string()).or_insert(0);
    *counter = counter.saturating_add(1);
}

/// Clear pending mark for a partial exit for a mint (decrements count and removes if zero)
pub async fn clear_partial_exit_pending(mint: &str) {
    let mut map = PENDING_PARTIAL_EXITS.write().await;
    if let Some(counter) = map.get_mut(mint) {
        if *counter > 1 {
            *counter -= 1;
        } else {
            map.remove(mint);
        }
    }
}

/// Check if a mint currently has an in-flight partial exit pending
pub async fn has_partial_exit_pending(mint: &str) -> bool {
    let map = PENDING_PARTIAL_EXITS.read().await;
    map.get(mint).copied().unwrap_or(0) > 0
}

/// Persist current pending DCA map to the database metadata store
async fn persist_pending_dca_swaps() -> Result<(), String> {
    let pending: Vec<PendingDcaSwap> = {
        let map = PENDING_DCA_SWAPS.read().await;
        map.values().cloned().collect()
    };

    let serialized = serde_json::to_string(&pending)
        .map_err(|e| format!("Failed to serialize pending DCA swaps: {}", e))?;

    db::set_metadata(PENDING_DCA_METADATA_KEY, &serialized).await
}

/// Register a pending DCA swap for durability
pub async fn register_pending_dca_swap(entry: PendingDcaSwap) -> Result<(), String> {
    let signature = entry.signature.clone();
    {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.insert(signature.clone(), entry);
    }

    if let Err(err) = persist_pending_dca_swaps().await {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.remove(&signature);
        return Err(err);
    }

    Ok(())
}

/// Clear a pending DCA swap once processed
pub async fn clear_pending_dca_swap(signature: &str) -> Result<Option<PendingDcaSwap>, String> {
    let removed = {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.remove(signature)
    };

    if let Some(entry) = removed.clone() {
        if let Err(err) = persist_pending_dca_swaps().await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to persist pending DCA metadata after clearing {}: {}",
                    signature, err
                ),
            );
            // Reinsert to keep in-memory state consistent if persistence fails
            {
                let mut map = PENDING_DCA_SWAPS.write().await;
                map.insert(entry.signature.clone(), entry);
            }
            return Err(err);
        }
    }

    Ok(removed)
}

/// Load pending DCA swaps from metadata into memory (used at startup)
pub async fn rehydrate_pending_dca_swaps() -> Result<Vec<PendingDcaSwap>, String> {
    let raw = db::get_metadata(PENDING_DCA_METADATA_KEY).await?;

    let entries: Vec<PendingDcaSwap> = match raw {
        Some(payload) if !payload.is_empty() => serde_json::from_str(&payload)
            .map_err(|e| format!("Failed to deserialize pending DCA metadata payload: {}", e))?,
        _ => Vec::new(),
    };

    {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.clear();
        for entry in &entries {
            map.insert(entry.signature.clone(), entry.clone());
        }
    }

    Ok(entries)
}

async fn persist_pending_partial_exits() -> Result<(), String> {
    let pending: Vec<PendingPartialExit> = {
        let map = PENDING_PARTIAL_EXIT_DETAILS.read().await;
        map.values().cloned().collect()
    };

    let serialized = serde_json::to_string(&pending)
        .map_err(|e| format!("Failed to serialize pending partial exits: {}", e))?;

    db::set_metadata(PENDING_PARTIAL_EXIT_METADATA_KEY, &serialized).await
}

/// Register a pending partial exit for durability
pub async fn register_pending_partial_exit(entry: PendingPartialExit) -> Result<(), String> {
    let signature = entry.signature.clone();
    {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.insert(signature.clone(), entry);
    }

    if let Err(err) = persist_pending_partial_exits().await {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.remove(&signature);
        return Err(err);
    }

    Ok(())
}

/// Clear a pending partial exit once processed
pub async fn clear_pending_partial_exit(
    signature: &str,
) -> Result<Option<PendingPartialExit>, String> {
    let removed = {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.remove(signature)
    };

    if let Some(entry) = removed.clone() {
        if let Err(err) = persist_pending_partial_exits().await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to persist pending partial exit metadata after clearing {}: {}",
                    signature, err
                ),
            );

            let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
            map.insert(entry.signature.clone(), entry);
            return Err(err);
        }
    }

    Ok(removed)
}

/// Fetch a pending partial exit by signature
pub async fn get_pending_partial_exit(signature: &str) -> Option<PendingPartialExit> {
    let map = PENDING_PARTIAL_EXIT_DETAILS.read().await;
    map.get(signature).cloned()
}

/// Load pending partial exits from metadata into memory (used at startup)
pub async fn rehydrate_pending_partial_exits() -> Result<Vec<PendingPartialExit>, String> {
    let raw = db::get_metadata(PENDING_PARTIAL_EXIT_METADATA_KEY).await?;

    let entries: Vec<PendingPartialExit> = match raw {
        Some(payload) if !payload.is_empty() => serde_json::from_str(&payload)
            .map_err(|e| format!("Failed to deserialize pending partial exit payload: {}", e))?,
        _ => Vec::new(),
    };

    {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.clear();
        for entry in &entries {
            map.insert(entry.signature.clone(), entry.clone());
        }
    }

    {
        let mut counters = PENDING_PARTIAL_EXITS.write().await;
        counters.clear();
        for entry in &entries {
            let counter = counters.entry(entry.mint.clone()).or_insert(0);
            *counter = counter.saturating_add(1);
        }
    }

    Ok(entries)
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
        &format!("🔒 Acquired position lock for mint: {}", &mint_key),
    );

    PositionLockGuard {
        mint: mint_key,
        _owned_guard: Some(owned_guard),
    }
}

/// Acquire a global position creation permit to enforce MAX_OPEN_POSITIONS atomically
/// This must be called BEFORE any position creation to prevent race conditions
pub async fn acquire_global_position_permit(
) -> Result<tokio::sync::SemaphorePermit<'static>, String> {
    let semaphore = get_global_position_semaphore();
    match semaphore.try_acquire() {
        Ok(permit) => {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "🟢 Acquired global position permit (available: {})",
                    semaphore.available_permits()
                ),
            );
            Ok(permit)
        }
        Err(_) => {
            let available = semaphore.available_permits();
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
    let semaphore = get_global_position_semaphore();
    semaphore.add_permits(1);
    logger::debug(
        LogTag::Positions,
        &format!(
            "🔴 Released global position permit (available: {})",
            semaphore.available_permits()
        ),
    );
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
        if pending.remove(&position.mint).is_some() {
            logger::debug(
                LogTag::Positions,
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

/// Retrieve a position by database ID from in-memory state
pub async fn get_position_by_id(position_id: i64) -> Option<Position> {
    let positions = POSITIONS.read().await;
    positions
        .iter()
        .find(|p| p.id == Some(position_id))
        .cloned()
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
            if pending.remove(&removed.mint).is_some() {
                logger::debug(
                    LogTag::Positions,
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
                logger::debug(
                    LogTag::Positions,
                    &format!("⏳ Pending-open expired for mint: {}", m),
                );
            }
        }

        if is_pending {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "🚫 is_open_position pending-open lock active for mint: {}",
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

/// Mark a mint as having a pending open swap for ttl_secs seconds
pub async fn set_pending_open(mint: &str, ttl_secs: i64) {
    let expires_at = Utc::now() + chrono::Duration::seconds(ttl_secs);
    let mut pending = PENDING_OPEN_SWAPS.write().await;
    pending.insert(mint.to_string(), expires_at);
    logger::debug(
        LogTag::Positions,
        &format!(
            "⏳ Set pending-open for mint: {} (ttl {}s, until {})",
            mint, ttl_secs, expires_at
        ),
    );
}

/// Clear a mint's pending open swap state, if present
pub async fn clear_pending_open(mint: &str) {
    let mut pending = PENDING_OPEN_SWAPS.write().await;
    if pending.remove(mint).is_some() {
        logger::debug(
            LogTag::Positions,
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
    use crate::logger::{self, LogTag};

    let semaphore = get_global_position_semaphore();
    let open_positions = get_open_positions().await; // clones but infrequent (startup)
    let open_count = open_positions.len();
    let available_before = semaphore.available_permits();
    let consumed_before = max_open - available_before;

    // Check for leaked permits (consumed > open positions)
    if consumed_before > open_count {
        let leaked = consumed_before - open_count;
        logger::warning(
            LogTag::Positions,
            &format!(
                "⚠️ Semaphore audit: {} leaked permits detected ({} consumed, {} open positions). Releasing leaked permits...",
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
                "✅ Released {} leaked permits. Available: {} -> {}",
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
                "⚠️ Semaphore reconcile: {} open positions exceed capacity (consumed {} of {}, available after {})",
                open_count,
                consumed,
                max_open,
                available_after
            )
        );
    } else {
        logger::info(
            LogTag::Positions,
            &format!(
                "✅ Semaphore reconcile: consumed {} permits for {} open positions (avail {} -> {})",
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
            && p.exit_time.map_or(false, |exit_time| exit_time > cutoff)
    })
}
