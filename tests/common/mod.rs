//! Shared harness for the ScreenerBot integration suite (`tests/`).
//!
//! # How the suite is organised
//!
//! Integration tests are split into one file **per domain** (`apis.rs`, `rpc.rs`,
//! `ohlcv.rs`, `config.rs`, `conversions.rs`, `trading.rs`, … future: `pools.rs`,
//! `positions.rs`, `transactions.rs`). Each `tests/*.rs` is its own crate/binary, so
//! it can only see the library's `pub` API plus dev-dependencies. Internal / private
//! pure logic is tested with co-located `#[cfg(test)] mod tests` in its own source
//! file instead.
//!
//! # Test levels (expressed the standard Rust way, with `#[ignore]`)
//!
//! * **Pure** — no attribute. No network, DB, or wallet. Runs by default.
//!     `cargo nextest run`            (or `cargo test`)
//! * **Live** — `#[ignore]`. Real read-only network (APIs, RPC). No money.
//!     `cargo nextest run --run-ignored ignored-only -E 'kind(test)'`
//! * **Mainnet** — `#[ignore]` + [`require_mainnet`]. Spends real SOL. Self-skips
//!   unless `SB_TEST_MAINNET_SWAP=1` and a funded `SB_TEST_WALLET` are set, so the
//!   `--run-ignored` command above runs the live tests while mainnet ones skip.
//!
//! `kind(test)` scopes the ignored run to integration tests only, never the library's
//! own `#[ignore]`'d unit tests. `../test.sh` (wrapper root) is a thin convenience
//! around these exact commands (adds logging + a mainnet confirmation prompt); the
//! suite does not depend on it.

#![allow(dead_code)] // each test binary compiles only the part of this module it uses.

use tempfile::TempDir;

/// Point ScreenerBot at an isolated temp data dir for the rest of THIS process and
/// initialise config from defaults. Returns the `TempDir` guard — keep it bound for
/// the test's lifetime; dropping it deletes the directory.
///
/// nextest runs one test per process, so setting the env var and the config `OnceLock`
/// here is hermetic. Live tests never read or write the owner's real config/DBs.
pub fn isolated_env() -> TempDir {
    let dir = tempfile::tempdir().expect("create temp data dir");
    // SAFETY: single-threaded test setup, before any path/config access.
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    ensure_config();
    dir
}

/// Initialise the global config to defaults WITHOUT touching disk. Idempotent.
pub fn ensure_config() {
    use screenerbot::config::schemas::Config;
    use screenerbot::config::utils::CONFIG;
    let _ = CONFIG.get_or_init(|| std::sync::RwLock::new(Config::default()));
}

/// Context for a mainnet test. Carries the funded test-wallet path, the mint to trade,
/// and a hard lamports cap so a money test can never exceed it.
pub struct MainnetCtx {
    /// Filesystem path to the funded test-wallet keypair JSON.
    pub wallet_path: String,
    /// Mint address to buy then sell.
    pub mint: String,
    /// Absolute ceiling on lamports the test may spend on the buy.
    pub max_lamports: u64,
}

/// Mainnet gate: returns `Some(ctx)` only when the owner has explicitly opted into
/// spending real SOL (`SB_TEST_MAINNET_SWAP=1`) AND provided a funded test wallet.
/// Otherwise it prints a SKIP line and returns `None` so the test returns early WITHOUT
/// failing and WITHOUT spending anything — even if `#[ignore]` is lifted by hand.
pub fn require_mainnet() -> Option<MainnetCtx> {
    if !env_flag("SB_TEST_MAINNET_SWAP") {
        eprintln!("SKIP mainnet: set SB_TEST_MAINNET_SWAP=1 (./test.sh mainnet) to run");
        return None;
    }
    let Ok(wallet_path) = std::env::var("SB_TEST_WALLET") else {
        eprintln!("SKIP mainnet: SB_TEST_WALLET (funded keypair path) not set");
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
