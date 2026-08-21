//! Solana pool runtime supervisor — owns the concrete discovery/analyzer/fetcher/
//! calculator components and their lifecycle. Chain-neutral orchestration (the
//! running flag, shutdown protocol, event recording, db/cache init) stays in
//! `crate::pools::service`, which selects this module's `initialize_components`
//! and `clear_components` as its Solana implementation — see that module's doc
//! comment for the composition boundary.

use crate::chains::solana::pools::analyzer::PoolAnalyzer;
use crate::chains::solana::pools::calculator::PriceCalculator;
use crate::chains::solana::pools::discovery::PoolDiscovery;
use crate::chains::solana::pools::fetcher::AccountFetcher;
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
pub fn get_pool_discovery() -> Option<Arc<PoolDiscovery>> {
    POOL_DISCOVERY.read().ok()?.clone()
}

/// Get the shared account fetcher component, if initialized.
pub fn get_account_fetcher() -> Option<Arc<AccountFetcher>> {
    ACCOUNT_FETCHER.read().ok()?.clone()
}

/// Get the shared price calculator component, if initialized.
pub fn get_price_calculator() -> Option<Arc<PriceCalculator>> {
    PRICE_CALCULATOR.read().ok()?.clone()
}

/// Get the shared pool analyzer component, if initialized.
pub fn get_pool_analyzer() -> Option<Arc<PoolAnalyzer>> {
    POOL_ANALYZER.read().ok()?.clone()
}

/// Get pools associated with a token from the analyzer's in-memory directory.
/// Returns a single canonical pool first (if present) followed by other pools.
///
/// Requires the pool runtime to be running (checked by the caller via
/// `crate::pools::service::is_pool_service_running`); returns an empty list
/// when the analyzer is not yet initialized.
pub fn get_token_pools(mint: &str) -> Vec<PoolDescriptor> {
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

/// Initialize the concrete Solana pool runtime components (discovery, analyzer,
/// fetcher, calculator) and store them in global state. Returns the RPC provider
/// count for logging/event purposes.
pub async fn initialize_components() -> Result<u32, String> {
    logger::debug(
        LogTag::PoolService,
        "Initializing Solana pool runtime components...",
    );

    let rpc_client = get_rpc_client();
    let rpc_urls_count = rpc_client.provider_count().await;

    // Pool directory shared between analyzer/fetcher/calculator
    let pool_directory = Arc::new(RwLock::new(HashMap::new()));

    let pool_discovery = Arc::new(PoolDiscovery::new());
    let pool_analyzer = Arc::new(PoolAnalyzer::new(pool_directory.clone()));
    let account_fetcher = Arc::new(AccountFetcher::new(pool_directory.clone()));
    let price_calculator = Arc::new(PriceCalculator::new(pool_directory.clone()));

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

    logger::debug(
        LogTag::PoolService,
        "Solana pool runtime components initialized",
    );

    Ok(rpc_urls_count as u32)
}

/// Clear the concrete Solana pool runtime components from global state.
/// Called by the composition root when the pool service stops.
pub fn clear_components() {
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
