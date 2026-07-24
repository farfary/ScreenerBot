//! Shared test harness for the ScreenerBot integration suite.
//!
//! Provides the three things every non-pure test needs: an isolated data
//! directory (so live tests never touch the owner's real config/DBs), in-memory
//! config initialisation, and the tier gates that keep read-only and money-spending
//! tests from running unless explicitly requested.
//!
//! This file lives at `tests/common/mod.rs`, so it is compiled into each test
//! binary as a shared module (`mod common;`) rather than as its own test binary.

#![allow(dead_code)] // each test binary uses only the part of this module it needs.

use tempfile::TempDir;

/// Point ScreenerBot at an isolated temp data dir for the rest of THIS process and
/// initialise config from defaults. Returns the `TempDir` guard — keep it bound for
/// the test's lifetime; dropping it deletes the directory.
///
/// nextest runs one test per process, so setting the env var and the config OnceLock
/// here is hermetic. Under plain `cargo test` (shared process) the env is set once
/// and config is `get_or_init`'d, which stays correct for the read-only tests.
pub fn isolated_env() -> TempDir {
    let dir = tempfile::tempdir().expect("create temp data dir");
    // SAFETY: single-threaded test setup, before any path/config access.
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    ensure_config();
    dir
}

/// Initialise the global config to defaults WITHOUT touching disk. Idempotent, so it
/// is safe to call from many tests sharing one process.
pub fn ensure_config() {
    use screenerbot::config::schemas::Config;
    use screenerbot::config::utils::CONFIG;
    let _ = CONFIG.get_or_init(|| std::sync::RwLock::new(Config::default()));
}

/// L1 gate: real read-only network. L1 tests are `#[ignore]`'d, so they only run when
/// the harness lifts ignores (`test.sh live`). This flag is the harness's signal, and
/// is also readable inside a test that wants to no-op when run in isolation.
pub fn live_enabled() -> bool {
    env_flag("SB_TEST_LIVE")
}

/// Context for an L2 (real mainnet swap) test. Carries the funded test wallet path,
/// the mint to trade, and a hard lamports cap so a money test can never exceed it.
pub struct MainnetCtx {
    /// Filesystem path to the funded test-wallet keypair JSON.
    pub wallet_path: String,
    /// Mint address to buy then sell.
    pub mint: String,
    /// Absolute ceiling on lamports the test may spend on the buy.
    pub max_lamports: u64,
}

/// L2 gate: returns `Some(ctx)` only when the owner has explicitly opted into spending
/// real SOL (`SB_TEST_MAINNET_SWAP=1`) AND provided a funded test wallet. Otherwise it
/// prints a SKIP line and returns `None` so the test returns early WITHOUT failing and
/// WITHOUT spending anything — even if someone lifts `#[ignore]` by hand.
pub fn require_mainnet() -> Option<MainnetCtx> {
    if !env_flag("SB_TEST_MAINNET_SWAP") {
        eprintln!("SKIP mainnet swap: set SB_TEST_MAINNET_SWAP=1 (test.sh mainnet) to run");
        return None;
    }
    let Ok(wallet_path) = std::env::var("SB_TEST_WALLET") else {
        eprintln!("SKIP mainnet swap: SB_TEST_WALLET (funded keypair path) not set");
        return None;
    };
    let mint = std::env::var("SB_TEST_MINT")
        .unwrap_or_else(|_| "So11111111111111111111111111111111111111112".to_string());
    let max_lamports = std::env::var("SB_TEST_MAX_LAMPORTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000_000); // 0.002 SOL default ceiling.
    Some(MainnetCtx {
        wallet_path,
        mint,
        max_lamports,
    })
}

fn env_flag(key: &str) -> bool {
    matches!(
        std::env::var(key).ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}
