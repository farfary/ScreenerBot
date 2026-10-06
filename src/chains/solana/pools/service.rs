// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana pool runtime state — owns the concrete discovery/analyzer/fetcher/
//! calculator components. Chain-neutral orchestration (the running flag, event
//! recording, db/cache init) stays in `crate::pools::service`, which reaches
//! these components only through the Solana pricing driver (`super::driver`).
//! The components reference each other through the getters below, which no
//! code outside the Solana module can call.

use crate::chains::solana::pools::analyzer::PoolAnalyzer;
use crate::chains::solana::pools::calculator::PriceCalculator;
use crate::chains::solana::pools::discovery::PoolDiscovery;
use crate::chains::solana::pools::fetcher::AccountFetcher;
use crate::chains::solana::pools::selection::new_selected_pools;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::rpc::get_rpc_client;
use crate::logger::{self, LogTag};
use crate::pools::types::PoolDescriptor;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

static POOL_DISCOVERY: LazyLock<RwLock<Option<Arc<PoolDiscovery>>>> =
    LazyLock::new(|| RwLock::new(None));
static POOL_ANALYZER: LazyLock<RwLock<Option<Arc<PoolAnalyzer>>>> =
    LazyLock::new(|| RwLock::new(None));
static ACCOUNT_FETCHER: LazyLock<RwLock<Option<Arc<AccountFetcher>>>> =
    LazyLock::new(|| RwLock::new(None));
static PRICE_CALCULATOR: LazyLock<RwLock<Option<Arc<PriceCalculator>>>> =
    LazyLock::new(|| RwLock::new(None));

/// Get the shared pool discovery component, if initialized.
pub(in crate::chains::solana) fn get_pool_discovery() -> Option<Arc<PoolDiscovery>> {
    POOL_DISCOVERY.read().ok()?.clone()
}

/// Get the shared account fetcher component, if initialized.
pub(in crate::chains::solana) fn get_account_fetcher() -> Option<Arc<AccountFetcher>> {
    ACCOUNT_FETCHER.read().ok()?.clone()
}

/// Get the shared price calculator component, if initialized.
pub(in crate::chains::solana) fn get_price_calculator() -> Option<Arc<PriceCalculator>> {
    PRICE_CALCULATOR.read().ok()?.clone()
}

/// Get the shared pool analyzer component, if initialized.
pub(in crate::chains::solana) fn get_pool_analyzer() -> Option<Arc<PoolAnalyzer>> {
    POOL_ANALYZER.read().ok()?.clone()
}

/// Get pools associated with a token from the analyzer's in-memory directory.
/// The token's selected pricing pool comes first (if present).
///
/// Requires the pool runtime to be running (checked by the caller via
/// `crate::pools::service::is_pool_service_running`); returns an empty list
/// when the analyzer is not yet initialized.
pub(in crate::chains::solana) fn get_token_pools(mint: &str) -> Vec<PoolDescriptor> {
    if !crate::pools::service::is_pool_service_running() {
        return Vec::new();
    }

    let analyzer = match get_pool_analyzer() {
        Some(analyzer) => analyzer,
        None => return Vec::new(),
    };

    let mut pools = analyzer.get_pools_for_token(mint);

    if let Some(canonical) = analyzer.get_canonical_pool(mint) {
        if let Some(position) = pools
            .iter()
            .position(|pool| pool.pool_id == canonical.pool_id)
        {
            if position != 0 {
                let canonical_pool = pools.remove(position);
                pools.insert(0, canonical_pool);
            }
        }
    }

    pools
}

/// Program of a registered pool of `mint`, as the stable `ProgramKind::protocol_slug()`
/// that presentation layers label (the dashboard's `POOL_PROGRAM_LABELS`), never the
/// display name. A legacy display-name identity maps to its slug; a pool the analyzer
/// does not know returns `None`.
pub(in crate::chains::solana) fn get_pool_program(
    mint: &str,
    pool_address: &str,
) -> Option<&'static str> {
    get_token_pools(mint)
        .into_iter()
        .find(|pool| pool.pool_id.address() == pool_address)
        .map(|pool| ProgramKind::from_protocol_id(&pool.program_kind).protocol_slug())
}

/// Initialize the concrete Solana pool runtime components (discovery, analyzer,
/// fetcher, calculator) and store them in global state, replacing any previous
/// set. Returns the RPC provider count for logging/event purposes.
pub(super) async fn initialize_components() -> crate::chains::solana::Result<usize> {
    logger::debug(
        LogTag::PoolService,
        "Initializing Solana pool runtime components...",
    );

    let rpc_client = get_rpc_client();
    let rpc_urls_count = rpc_client.provider_count().await;

    install_components();

    logger::debug(
        LogTag::PoolService,
        "Solana pool runtime components initialized",
    );

    Ok(rpc_urls_count)
}

/// Build a fresh set of components around one shared pool directory and
/// install it. Each fresh component holds an untaken request receiver, so its
/// stage loop can be started once.
pub(super) fn install_components() {
    // Pool directory shared between analyzer/fetcher/calculator
    let pool_directory = Arc::new(RwLock::new(HashMap::new()));
    // Pool each token is priced from: written by the analyzer, read by the calculator
    let selected_pools = new_selected_pools();

    let pool_discovery = Arc::new(PoolDiscovery::new());
    let pool_analyzer = Arc::new(PoolAnalyzer::new(
        pool_directory.clone(),
        selected_pools.clone(),
    ));
    let account_fetcher = Arc::new(AccountFetcher::new(pool_directory.clone()));
    let price_calculator = Arc::new(PriceCalculator::new(pool_directory.clone(), selected_pools));

    if let Ok(mut discovery) = POOL_DISCOVERY.write() {
        *discovery = Some(pool_discovery);
    }
    if let Ok(mut analyzer) = POOL_ANALYZER.write() {
        *analyzer = Some(pool_analyzer);
    }
    if let Ok(mut fetcher) = ACCOUNT_FETCHER.write() {
        *fetcher = Some(account_fetcher);
    }
    if let Ok(mut calculator) = PRICE_CALCULATOR.write() {
        *calculator = Some(price_calculator);
    }
}

/// Clear the concrete Solana pool runtime components from global state.
/// Called through the pricing driver when the pool service stops.
pub(super) fn clear_components() {
    if let Ok(mut discovery) = POOL_DISCOVERY.write() {
        *discovery = None;
    }
    if let Ok(mut analyzer) = POOL_ANALYZER.write() {
        *analyzer = None;
    }
    if let Ok(mut fetcher) = ACCOUNT_FETCHER.write() {
        *fetcher = None;
    }
    if let Ok(mut calculator) = PRICE_CALCULATOR.write() {
        *calculator = None;
    }
}
