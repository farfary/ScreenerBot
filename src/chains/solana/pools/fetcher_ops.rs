//! Account fetcher operations.
//!
//! Contains helper methods for the AccountFetcher including batch processing,
//! missing account handling, and account organization into pool bundles.

use super::fetcher::{
    AccountFetcher, ACCOUNT_BATCH_SIZE, ACCOUNT_STALE_THRESHOLD_SECONDS,
    OPEN_POSITION_ACCOUNT_STALE_THRESHOLD_SECONDS,
};
use super::fetcher_types::{
    AccountData, MissingAccountState, MissingPoolState, PoolAccountBundle, SOL_MINT_PUBKEY,
    SYSTEM_PROGRAM_PUBKEY,
};
use super::reserve_accounts::reserve_pubkeys;
use super::types::ProgramKind;

use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::events::{record_safe, Event, EventCategory};
use crate::logger::{self, LogTag};
use crate::pools::types::{
    account_blacklist_threshold, failure_window_secs, pool_blacklist_threshold, PoolDescriptor,
};
use crate::pools::utils::is_sol_mint;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

impl AccountFetcher {
    /// Add stale accounts from pools to pending fetch list
    pub(crate) async fn add_stale_accounts_to_pending(
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        account_last_fetch: &Arc<RwLock<HashMap<Pubkey, Instant>>>,
        pending_accounts: &mut HashSet<Pubkey>,
    ) {
        // Snapshot pools under lock (minimize lock duration)
        // Only clone pool data, not the last_fetch map
        let pools = {
            let directory = pool_directory.read().unwrap();
            directory.values().cloned().collect::<Vec<_>>()
        };

        // Collect open position mints once (async call) to avoid per-pool await cost
        let open_mints: std::collections::HashSet<String> =
            crate::positions::state::get_open_mints()
                .await
                .into_iter()
                .collect();

        // Pre-compute pool ID strings to avoid allocations in async closures
        let pool_ids: Vec<(usize, String)> = pools
            .iter()
            .enumerate()
            .map(|(idx, pool)| (idx, pool.pool_id.address().to_owned()))
            .collect();

        // Check pool blacklist status in parallel
        let pool_blacklist_futures: Vec<_> = pool_ids
            .iter()
            .map(|(idx, pool_id_str)| {
                let id_str = pool_id_str.clone();
                let pool_idx = *idx;
                async move {
                    let result = crate::pools::db::is_pool_blacklisted(
                        crate::chains::ChainId::Solana,
                        &id_str,
                    )
                    .await;
                    (pool_idx, id_str, result)
                }
            })
            .collect();

        let pool_blacklist_results = join_all(pool_blacklist_futures).await;

        // Build a set of non-blacklisted pool indices
        let mut valid_pool_indices: HashSet<usize> = HashSet::new();
        for (idx, pool_id_str, result) in pool_blacklist_results {
            match result {
                Ok(true) => {
                    // Blacklisted, skip
                }
                Ok(false) => {
                    valid_pool_indices.insert(idx);
                }
                Err(e) => {
                    logger::warning(
                        LogTag::PoolFetcher,
                        &format!(
                            "Failed to check blacklist for pool {}: {} - skipping as precaution",
                            pool_id_str, e
                        ),
                    );
                    // FAIL-CLOSED: Skip pool if blacklist check fails
                }
            }
        }

        // Collect all reserve accounts we need to check from valid pools
        let mut accounts_to_check: Vec<(Pubkey, u64)> = Vec::new();

        for (idx, pool) in pools.iter().enumerate() {
            if !valid_pool_indices.contains(&idx) {
                continue;
            }

            // Determine the tracked (non-SOL) token mint for this pool
            let target_mint = if is_sol_mint(pool.base_mint.address()) {
                pool.quote_mint.address()
            } else {
                pool.base_mint.address()
            };

            // Choose threshold – accelerate if this token has an open position
            let threshold = if open_mints.contains(target_mint) {
                OPEN_POSITION_ACCOUNT_STALE_THRESHOLD_SECONDS
            } else {
                ACCOUNT_STALE_THRESHOLD_SECONDS
            };

            match reserve_pubkeys(pool) {
                Ok(pubkeys) => {
                    for pubkey in pubkeys {
                        accounts_to_check.push((pubkey, threshold));
                    }
                }
                Err(e) => {
                    logger::warning(
                        LogTag::PoolFetcher,
                        &format!("Skipping stale-account scan for pool {}: {e}", pool.pool_id),
                    );
                }
            }
        }

        // Now check last fetch times with a single lock acquisition
        // Only read the entries we actually need
        {
            let last_fetch = account_last_fetch.read().unwrap();
            for (account, threshold) in accounts_to_check {
                let needs_fetch = match last_fetch.get(&account) {
                    Some(last_time) => last_time.elapsed().as_secs() > threshold,
                    None => true, // Never fetched
                };
                if needs_fetch {
                    pending_accounts.insert(account);
                }
            }
        }

        // Filter out blacklisted accounts in parallel
        let pending_list: Vec<Pubkey> = pending_accounts.iter().copied().collect();
        let account_blacklist_futures: Vec<_> = pending_list
            .iter()
            .map(|account| {
                let acc = *account;
                let acc_str = acc.to_string();
                async move {
                    let result = crate::pools::db::is_account_blacklisted(
                        crate::chains::ChainId::Solana,
                        &acc_str,
                    )
                    .await;
                    (acc, acc_str, result)
                }
            })
            .collect();

        let account_blacklist_results = join_all(account_blacklist_futures).await;

        for (account, account_str, result) in account_blacklist_results {
            match result {
                Ok(true) => {
                    pending_accounts.remove(&account);
                }
                Ok(false) => {
                    // Not blacklisted, keep in pending
                }
                Err(e) => {
                    logger::warning(
                        LogTag::PoolFetcher,
                        &format!(
                            "Failed to check blacklist for account {}: {} - keeping in pending for retry",
                            account_str, e
                        ),
                    );
                    // FAIL-OPEN: Keep in pending if blacklist check fails - will retry on next cycle
                    // This prevents losing track of accounts due to transient DB errors
                }
            }
        }
    }

    /// Process pending accounts by fetching them in batches
    pub(crate) async fn process_pending_accounts(
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        account_bundles: &Arc<RwLock<HashMap<Pubkey, PoolAccountBundle>>>,
        account_last_fetch: &Arc<RwLock<HashMap<Pubkey, Instant>>>,
        pending_accounts: &mut HashSet<Pubkey>,
        account_failure_tracker: &mut HashMap<Pubkey, MissingAccountState>,
        pool_failure_tracker: &mut HashMap<Pubkey, MissingPoolState>,
        operations: &Arc<std::sync::atomic::AtomicU64>,
        errors: &Arc<std::sync::atomic::AtomicU64>,
        accounts_fetched: &Arc<std::sync::atomic::AtomicU64>,
        rpc_batches: &Arc<std::sync::atomic::AtomicU64>,
    ) {
        if pending_accounts.is_empty() {
            return;
        }

        // Convert to vector and batch, filtering out native SOL mint which is not a
        // real on-chain account (RPC returns null for it, wasting batch slots)
        let drained_accounts: Vec<Pubkey> = pending_accounts
            .drain()
            .filter(|key| *key != *SOL_MINT_PUBKEY && *key != *SYSTEM_PROGRAM_PUBKEY)
            .collect();

        if drained_accounts.is_empty() {
            return;
        }

        // Pre-compute string representations to avoid allocations in the async loop
        let account_strings: Vec<(Pubkey, String)> = drained_accounts
            .into_iter()
            .map(|acc| {
                let s = acc.to_string();
                (acc, s)
            })
            .collect();

        // Check blacklist status in parallel using join_all
        let blacklist_futures: Vec<_> = account_strings
            .iter()
            .map(|(account, account_key)| {
                let key = account_key.clone();
                let acc = *account;
                async move {
                    let is_blacklisted = crate::pools::db::is_account_blacklisted(
                        crate::chains::ChainId::Solana,
                        &key,
                    )
                    .await;
                    (acc, key, is_blacklisted)
                }
            })
            .collect();

        let blacklist_results = futures::future::join_all(blacklist_futures).await;

        let mut accounts_to_fetch = Vec::with_capacity(blacklist_results.len());
        for (account, account_key, result) in blacklist_results {
            match result {
                Ok(true) => {
                    // Blacklisted, skip
                }
                Ok(false) => {
                    accounts_to_fetch.push(account);
                }
                Err(e) => {
                    logger::warning(
                        LogTag::PoolFetcher,
                        &format!(
                            "Failed to check blacklist for account {}: {} - skipping as precaution",
                            account_key, e
                        ),
                    );
                }
            }
        }

        if accounts_to_fetch.is_empty() {
            return;
        }

        logger::debug(
            LogTag::PoolFetcher,
            &format!("Processing {} pending accounts", accounts_to_fetch.len()),
        );

        // Process in batches
        for batch in accounts_to_fetch.chunks(ACCOUNT_BATCH_SIZE) {
            let batch_start = Instant::now();

            record_safe(Event::info(
                EventCategory::Pool,
                Some("rpc_batch_started".to_owned()),
                None,
                None,
                serde_json::json!({
                    "batch_size": batch.len(),
                    "max_batch_size": ACCOUNT_BATCH_SIZE,
                    "accounts": batch.iter().map(|p| p.to_string()).collect::<Vec<_>>()
                }),
            ))
            .await;

            match Self::fetch_account_batch(batch).await {
                Ok((account_data_list, missing_accounts)) => {
                    let batch_duration = batch_start.elapsed();

                    // Track metrics
                    operations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    accounts_fetched.fetch_add(
                        account_data_list.len() as u64,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    rpc_batches.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    record_safe(Event::info(
                        EventCategory::Pool,
                        Some("rpc_batch_completed".to_owned()),
                        None,
                        None,
                        serde_json::json!({
                            "batch_size": batch.len(),
                            "accounts_fetched": account_data_list.len(),
                            "duration_ms": batch_duration.as_millis(),
                            "success": true
                        }),
                    ))
                    .await;

                    // Update last fetch times only for successful accounts
                    {
                        let mut last_fetch = account_last_fetch.write().unwrap();
                        let now = Instant::now();
                        for acc_data in &account_data_list {
                            last_fetch.insert(acc_data.pubkey, now);
                        }
                        for missing in &missing_accounts {
                            last_fetch.insert(*missing, now);
                        }
                    }

                    // Ensure missing accounts are not kept pending within this tick
                    for missing in &missing_accounts {
                        pending_accounts.remove(missing);
                    }

                    Self::handle_missing_accounts(
                        &missing_accounts,
                        pool_directory,
                        account_failure_tracker,
                        pool_failure_tracker,
                    )
                    .await;
                    Self::cleanup_missing_failure_trackers(
                        account_failure_tracker,
                        pool_failure_tracker,
                    );

                    // Organize accounts into pool bundles
                    Self::organize_accounts_into_bundles(
                        &account_data_list,
                        pool_directory,
                        account_bundles,
                    )
                    .await;

                    logger::debug(
                        LogTag::PoolFetcher,
                        &format!("Successfully fetched {} accounts", account_data_list.len()),
                    );
                }
                Err(e) => {
                    let batch_duration = batch_start.elapsed();

                    // Track error
                    errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    rpc_batches.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    logger::error(
                        LogTag::PoolFetcher,
                        &format!("Failed to fetch account batch: {e}"),
                    );

                    record_safe(Event::error(
                        EventCategory::Pool,
                        Some("rpc_batch_failed".to_owned()),
                        None,
                        None,
                        serde_json::json!({
                            "batch_size": batch.len(),
                            "error": e,
                            "duration_ms": batch_duration.as_millis(),
                            "accounts": batch.iter().map(|p| p.to_string()).collect::<Vec<_>>()
                        }),
                    ))
                    .await;
                }
            }

            // Small delay between batches to respect rate limits
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Track missing accounts and blacklist after threshold failures
    pub(crate) async fn handle_missing_accounts(
        missing_accounts: &[Pubkey],
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        account_failure_tracker: &mut HashMap<Pubkey, MissingAccountState>,
        pool_failure_tracker: &mut HashMap<Pubkey, MissingPoolState>,
    ) {
        if missing_accounts.is_empty() {
            return;
        }

        for account in missing_accounts {
            let account_str = account.to_string();
            let directory_snapshot: Vec<(Pubkey, PoolDescriptor)> = {
                let directory_guard = pool_directory.read().unwrap();
                directory_guard
                    .iter()
                    .filter(|(_, descriptor)| {
                        descriptor
                            .reserve_accounts
                            .iter()
                            .any(|id| id.address() == account_str)
                    })
                    .map(|(pool_id, descriptor)| (*pool_id, descriptor.clone()))
                    .collect()
            };

            let account_state =
                account_failure_tracker
                    .entry(*account)
                    .or_insert(MissingAccountState {
                        failures: 0,
                        last_failure: Instant::now(),
                        blacklisted: false,
                    });
            account_state.failures = account_state.failures.saturating_add(1);
            account_state.last_failure = Instant::now();

            if account_state.failures >= account_blacklist_threshold() && !account_state.blacklisted
            {
                let (pool_id_str, token_mint_str) = directory_snapshot
                    .first()
                    .map(|(pool_id, descriptor)| {
                        let token_mint = if is_sol_mint(descriptor.base_mint.address()) {
                            descriptor.quote_mint.address().to_owned()
                        } else {
                            descriptor.base_mint.address().to_owned()
                        };
                        (Some(pool_id.to_string()), Some(token_mint))
                    })
                    .unwrap_or((None, None));

                match crate::pools::db::add_account_to_blacklist(
                    crate::chains::ChainId::Solana,
                    &account.to_string(),
                    "account_not_found_threshold",
                    Some("rpc_fetch"),
                    pool_id_str.as_deref(),
                    token_mint_str.as_deref(),
                )
                .await
                {
                    Ok(()) => {
                        account_state.blacklisted = true;
                        logger::warning(
                            LogTag::PoolFetcher,
                            &format!(
                                "Blacklisted account {} after {} consecutive misses",
                                account, account_state.failures
                            ),
                        );
                        record_safe(Event::warn(
                            EventCategory::Pool,
                            Some("account_blacklisted_after_threshold".to_owned()),
                            token_mint_str.clone(),
                            pool_id_str.clone(),
                            serde_json::json!({
                                "account": account.to_string(),
                                "failures": account_state.failures,
                                "threshold": account_blacklist_threshold(),
                                "pool_id": pool_id_str,
                                "token_mint": token_mint_str,
                            }),
                        ))
                        .await;
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolFetcher,
                            &format!("Failed to persist account blacklist for {account}: {e}"),
                        );
                    }
                }
            }

            for (pool_id, descriptor) in directory_snapshot.iter() {
                let pool_state = pool_failure_tracker
                    .entry(*pool_id)
                    .or_insert(MissingPoolState {
                        failures: 0,
                        last_failure: Instant::now(),
                        blacklisted: false,
                    });
                pool_state.failures = pool_state.failures.saturating_add(1);
                pool_state.last_failure = Instant::now();

                if pool_state.failures >= pool_blacklist_threshold() && !pool_state.blacklisted {
                    let token_mint = if is_sol_mint(descriptor.base_mint.address()) {
                        descriptor.quote_mint.address().to_owned()
                    } else {
                        descriptor.base_mint.address().to_owned()
                    };
                    let program_kind = ProgramKind::from_protocol_id(&descriptor.program_kind);
                    let program_id = program_kind.program_id();

                    match crate::pools::db::add_pool_to_blacklist(
                        crate::chains::ChainId::Solana,
                        &pool_id.to_string(),
                        "missing_accounts",
                        Some(&token_mint),
                        if program_id.is_empty() {
                            None
                        } else {
                            Some(program_id)
                        },
                    )
                    .await
                    {
                        Ok(()) => {
                            pool_state.blacklisted = true;
                            logger::warning(
                                LogTag::PoolFetcher,
                                &format!(
                                    "Blacklisted pool {} (token {}) after {} missing-account hits",
                                    pool_id, token_mint, pool_state.failures
                                ),
                            );
                            record_safe(Event::warn(
                                EventCategory::Pool,
                                Some("pool_blacklisted_missing_accounts".to_owned()),
                                Some(token_mint.clone()),
                                Some(pool_id.to_string()),
                                serde_json::json!({
                                    "pool_id": pool_id.to_string(),
                                    "program_kind": descriptor.program_kind.as_str(),
                                    "program_id": program_id,
                                    "failures": pool_state.failures,
                                    "threshold": pool_blacklist_threshold(),
                                    "missing_account": account.to_string(),
                                }),
                            ))
                            .await;
                        }
                        Err(e) => {
                            logger::warning(
                                LogTag::PoolFetcher,
                                &format!("Failed to persist pool blacklist for {pool_id}: {e}"),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Remove expired entries from account and pool failure trackers
    pub(crate) fn cleanup_missing_failure_trackers(
        account_failure_tracker: &mut HashMap<Pubkey, MissingAccountState>,
        pool_failure_tracker: &mut HashMap<Pubkey, MissingPoolState>,
    ) {
        let expiry = Duration::from_secs(failure_window_secs());
        let now = Instant::now();

        account_failure_tracker.retain(|_, state| {
            state.blacklisted || now.duration_since(state.last_failure) <= expiry
        });

        pool_failure_tracker.retain(|_, state| {
            state.blacklisted || now.duration_since(state.last_failure) <= expiry
        });
    }

    /// Fetch a batch of accounts
    pub(crate) async fn fetch_account_batch(
        accounts: &[Pubkey],
    ) -> Result<(Vec<AccountData>, Vec<Pubkey>), String> {
        // Check connectivity before RPC batch fetch - graceful degradation
        if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
            logger::debug(
                LogTag::PoolFetcher,
                &format!(
                    "Skipping account batch fetch ({} accounts) - Unhealthy endpoints: {}",
                    accounts.len(),
                    unhealthy
                ),
            );
            // Return empty list - caller will use cached data
            return Ok((Vec::new(), Vec::new()));
        }

        if accounts.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        logger::debug(
            LogTag::PoolFetcher,
            &format!("Fetching batch of {} accounts", accounts.len()),
        );

        // Fetch accounts using new RPC client
        let rpc_client = get_rpc_client();
        let rpc_start = Instant::now();
        let account_results = match rpc_client.get_multiple_accounts(accounts).await {
            Ok(results) => {
                let rpc_duration = rpc_start.elapsed();

                record_safe(Event::info(
                    EventCategory::Rpc,
                    Some("get_multiple_accounts_success".to_owned()),
                    None,
                    None,
                    serde_json::json!({
                        "account_count": accounts.len(),
                        "duration_ms": rpc_duration.as_millis(),
                        "success": true
                    }),
                ))
                .await;

                results
            }
            Err(e) => {
                let rpc_duration = rpc_start.elapsed();

                record_safe(Event::error(
                    EventCategory::Rpc,
                    Some("get_multiple_accounts_failed".to_owned()),
                    None,
                    None,
                    serde_json::json!({
                        "account_count": accounts.len(),
                        "error": e.to_string(),
                        "duration_ms": rpc_duration.as_millis(),
                        "accounts": accounts.iter().map(|p| p.to_string()).collect::<Vec<_>>()
                    }),
                ))
                .await;

                return Err(e.to_string());
            }
        };

        let mut account_data_list: Vec<AccountData> = Vec::new();
        let mut missing_accounts: Vec<Pubkey> = Vec::new();

        for (i, account_opt) in account_results.iter().enumerate() {
            if let Some(account) = account_opt {
                let account_data = AccountData::from_account(accounts[i], account.clone(), 0);
                account_data_list.push(account_data);
            } else {
                let missing_key = accounts[i];
                missing_accounts.push(missing_key);
                logger::warning(
                    LogTag::PoolFetcher,
                    &format!("Account not found: {missing_key}"),
                );
            }
        }

        if !missing_accounts.is_empty() {
            record_safe(Event::warn(
                EventCategory::Pool,
                Some("accounts_not_found".to_owned()),
                None,
                None,
                serde_json::json!({
                    "missing_count": missing_accounts.len(),
                    "total_requested": accounts.len(),
                    "missing_accounts": missing_accounts.iter().map(|p| p.to_string()).collect::<Vec<_>>(),
                    "action": "failure_recorded"
                }),
            ))
            .await;
        }

        Ok((account_data_list, missing_accounts))
    }

    /// Organize fetched accounts into pool bundles
    ///
    /// Creates isolated account data instances for each pool to prevent race conditions
    /// when multiple pools share the same vault accounts (common in Raydium Legacy AMM)
    ///
    /// Uses a two-phase approach to minimize lock contention:
    /// 1. Build updates in local HashMap (no locks held)
    /// 2. Apply updates to shared state (brief write lock)
    /// 3. Trigger calculations (after releasing lock)
    pub(crate) async fn organize_accounts_into_bundles(
        account_data_list: &[AccountData],
        pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
        account_bundles: &Arc<RwLock<HashMap<Pubkey, PoolAccountBundle>>>,
    ) {
        // Phase 1: Snapshot pools (brief read lock)
        let pools = {
            let directory = pool_directory.read().unwrap();
            directory.clone()
        };

        // Convert each pool's chain-neutral reserve accounts to `Pubkey` once per
        // cycle (not once per account lookup below) — this is the hot ~500ms
        // fetch loop, so address parsing is amortized over pools, not accounts.
        // All-or-error per pool: a pool with a malformed reserve address is
        // omitted entirely rather than bundled with a shrunken reserve set,
        // which would let it wrongly report itself complete and price a
        // partial pool.
        let mut pool_reserve_pubkeys: HashMap<Pubkey, Vec<Pubkey>> =
            HashMap::with_capacity(pools.len());
        for (pool_id, descriptor) in pools.iter() {
            match reserve_pubkeys(descriptor) {
                Ok(reserves) => {
                    pool_reserve_pubkeys.insert(*pool_id, reserves);
                }
                Err(e) => {
                    logger::warning(
                        LogTag::PoolFetcher,
                        &format!("Skipping bundle organization for pool {pool_id}: {e}"),
                    );
                }
            }
        }

        // Phase 2: Build local updates without holding any locks
        // Maps pool_id -> (bundle, pool_descriptor, needs_calculation)
        let mut local_updates: HashMap<Pubkey, (PoolAccountBundle, PoolDescriptor, bool)> =
            HashMap::new();

        // Get existing bundles to merge with (brief read lock)
        let existing_bundles: HashMap<Pubkey, PoolAccountBundle> = {
            let bundles = account_bundles.read().unwrap();
            bundles.clone()
        };

        // Build updates locally
        for account_data in account_data_list {
            for (pool_id, pool_descriptor) in &pools {
                let reserves = pool_reserve_pubkeys
                    .get(pool_id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if reserves.contains(&account_data.pubkey) {
                    let entry = local_updates.entry(*pool_id).or_insert_with(|| {
                        let bundle = existing_bundles
                            .get(pool_id)
                            .cloned()
                            .unwrap_or_else(|| PoolAccountBundle::new(*pool_id));
                        (bundle, pool_descriptor.clone(), false)
                    });

                    // Create isolated account data for each pool to prevent race conditions
                    let isolated_account_data = AccountData {
                        pubkey: account_data.pubkey,
                        data: account_data.data.clone(),
                        slot: account_data.slot,
                        fetched_at: Instant::now(),
                        lamports: account_data.lamports,
                        owner: account_data.owner,
                    };
                    entry.0.add_account(isolated_account_data);

                    logger::debug(
                        LogTag::PoolFetcher,
                        &format!(
                            "Added account {} to bundle for token {} in pool {}",
                            account_data.pubkey,
                            if is_sol_mint(pool_descriptor.base_mint.address()) {
                                pool_descriptor.quote_mint.address()
                            } else {
                                pool_descriptor.base_mint.address()
                            },
                            pool_id
                        ),
                    );

                    // Check if bundle is now complete and needs (re)calculation.
                    // First check handles initial calculation (never calculated before).
                    // Second check handles price refresh: when a bundle was already calculated
                    // but its price has expired from the cache (TTL-based), reset the flag
                    // so the price gets recalculated from the re-fetched accounts.
                    if entry.0.is_complete_and_needs_calculation(reserves) {
                        entry.0.mark_calculation_requested();
                        entry.2 = true; // Mark needs calculation
                    } else if entry.0.calculation_requested && entry.0.is_complete(reserves) {
                        // Price was calculated before but may have expired from cache.
                        // Check if the target token still has a valid cached price.
                        let target_mint = if is_sol_mint(pool_descriptor.base_mint.address()) {
                            pool_descriptor.quote_mint.address()
                        } else {
                            pool_descriptor.base_mint.address()
                        };
                        if !crate::pools::cache::is_price_fresh(target_mint) {
                            // Price expired — reset flag and re-trigger calculation
                            entry.0.calculation_requested = false;
                            entry.0.mark_calculation_requested();
                            entry.2 = true;
                        }
                    }
                }
            }
        }

        // Phase 3: Apply updates (brief write lock)
        {
            let mut bundles = account_bundles.write().unwrap();
            for (pool_id, (bundle, _, _)) in &local_updates {
                bundles.insert(*pool_id, bundle.clone());
            }
        }

        // Phase 4: Trigger calculations (no locks held)
        for (pool_id, (bundle, pool_descriptor, needs_calculation)) in local_updates {
            if needs_calculation {
                if let Some(calculator) =
                    crate::chains::solana::pools::service::get_price_calculator()
                {
                    let target_token = if is_sol_mint(pool_descriptor.base_mint.address()) {
                        pool_descriptor.quote_mint.address().to_owned()
                    } else {
                        pool_descriptor.base_mint.address().to_owned()
                    };

                    if let Err(e) = calculator.request_calculation(pool_id, pool_descriptor, bundle)
                    {
                        logger::warning(
                            LogTag::PoolFetcher,
                            &format!(
                                "Failed to request calculation for token {} in pool {}: {}",
                                target_token, pool_id, e
                            ),
                        );
                    } else {
                        logger::debug(
                            LogTag::PoolFetcher,
                            &format!(
                                "Requested calculation for complete bundle - token {} in pool {}",
                                target_token, pool_id
                            ),
                        );
                    }
                }
            }
        }
    }
}
