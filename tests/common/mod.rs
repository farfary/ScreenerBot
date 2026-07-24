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
//!   `cargo nextest run`            (or `cargo test`)
//! * **Live** — `#[ignore]`. Real read-only network (APIs, RPC). No money.
//!   `cargo nextest run --run-ignored ignored-only -E 'kind(test)'`
//! * **Mainnet** — `#[ignore]` + [`require_mainnet`]. Spends real SOL. Self-skips
//!   unless `SB_TEST_MAINNET_SWAP=1` and a funded `SB_TEST_WALLET` are set, so the
//!   `--run-ignored` command above runs the live tests while mainnet ones skip.
//!
//! `kind(test)` scopes the ignored run to integration tests only, never the library's
//! own `#[ignore]`'d unit tests. `../test.sh` (wrapper root) is a thin convenience
//! around these exact commands (adds logging + a mainnet confirmation prompt); the
//! suite does not depend on it.

#![allow(dead_code)] // each test binary compiles only the part of this module it uses.

use chrono::{DateTime, Duration, Utc};
use screenerbot::ohlcvs::{Candle, TimeframeBundle};
use screenerbot::positions::Position;
use screenerbot::strategies::types::{Condition, EvaluationContext, Parameter};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
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

// ==================== GLOBAL-CONFIG MUTATION ====================

/// Serialises tests that WRITE the global `CONFIG`.
///
/// nextest gives every test its own process, so mutating the global config is already
/// hermetic there. Plain `cargo test` runs a whole binary's tests in ONE process with
/// threads, and two tests flipping `trader.stop_loss_enabled` in opposite directions
/// would read each other's value. Holding this guard for the whole test makes the suite
/// correct under BOTH runners.
static CONFIG_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Take the config-mutation guard and initialise config to DEFAULTS.
///
/// Returns the guard — bind it (`let _cfg = common::config_guard();`) for the test's
/// lifetime. Resetting to defaults on acquire means a test never inherits whatever the
/// previous one left behind.
pub fn config_guard() -> MutexGuard<'static, ()> {
    let guard = CONFIG_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    ensure_config();
    reset_config_to_defaults();
    guard
}

fn reset_config_to_defaults() {
    use screenerbot::config::schemas::Config;
    use screenerbot::config::utils::CONFIG;
    if let Some(lock) = CONFIG.get() {
        let mut cfg = lock.write().unwrap_or_else(|p| p.into_inner());
        *cfg = Config::default();
    }
}

/// Mutate the global config in place. Only call while holding [`config_guard`].
pub fn set_config<F: FnOnce(&mut screenerbot::config::schemas::Config)>(f: F) {
    use screenerbot::config::utils::CONFIG;
    ensure_config();
    let lock = CONFIG.get().expect("config initialised");
    let mut cfg = lock.write().unwrap_or_else(|p| p.into_inner());
    f(&mut cfg);
}

// ==================== FIXTURES ====================

/// A mint that is guaranteed not to exist on chain, so nothing can be fetched for it.
pub const TEST_MINT: &str = "TestMint1111111111111111111111111111111111";

/// Seed the decimals cache for [`TEST_MINT`] so P&L math runs fully offline.
///
/// `tokens::get_decimals` checks the in-memory cache first and only then the DB/chain;
/// pre-seeding it keeps the token-amount P&L branches reachable without any I/O. A
/// position whose decimals are unknown deliberately returns a ZERO P&L, so without this
/// the interesting branches are never exercised.
pub fn seed_decimals(mint: &str, decimals: u8) {
    screenerbot::tokens::cache_decimals(mint, decimals);
}

/// A minimal OPEN buy position: entry at `entry_price`, `size_sol` invested, no DCA,
/// no partial exits, nothing verified beyond the entry.
pub fn test_position(entry_price: f64, size_sol: f64) -> Position {
    Position {
        id: Some(1),
        mint: TEST_MINT.to_owned(),
        symbol: "TEST".to_owned(),
        name: "Test Token".to_owned(),
        entry_price,
        entry_time: Utc::now(),
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_owned(),
        entry_size_sol: size_sol,
        total_size_sol: size_sol,
        price_highest: entry_price,
        price_lowest: entry_price,
        entry_transaction_signature: Some("entry-sig".to_owned()),
        exit_transaction_signature: None,
        token_amount: None,
        effective_entry_price: Some(entry_price),
        effective_exit_price: None,
        sol_received: None,
        profit_target_min: None,
        profit_target_max: None,
        liquidity_tier: None,
        transaction_entry_verified: true,
        transaction_exit_verified: false,
        entry_fee_lamports: None,
        exit_fee_lamports: None,
        current_price: Some(entry_price),
        current_price_updated: Some(Utc::now()),
        phantom_remove: false,
        phantom_confirmations: 0,
        phantom_first_seen: None,
        synthetic_exit: false,
        closed_reason: None,
        pnl: None,
        pnl_percent: None,
        unrealized_pnl: None,
        unrealized_pnl_percent: None,
        remaining_token_amount: None,
        total_exited_amount: 0,
        average_exit_price: None,
        partial_exit_count: 0,
        dca_count: 0,
        average_entry_price: entry_price,
        last_dca_time: None,
        archived: false,
        archived_at: None,
        manual_management: false,
    }
}

/// Age a position by moving its entry time into the past.
pub fn aged(mut position: Position, age: Duration) -> Position {
    position.entry_time = Utc::now() - age;
    position
}

// ==================== CANDLES ====================

/// A candle with a real body and a positive volume (the ingest layer drops
/// volume == 0 candles, so a fixture must never rely on one).
pub fn candle(timestamp: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    Candle::new(timestamp, open, high, low, close, volume)
}

/// `count` candles ending at `last_ts`, spaced `step_secs` apart, each closing at the
/// corresponding value in `closes` (open = previous close, flat wicks, volume 1.0).
pub fn candle_series(last_ts: i64, step_secs: i64, closes: &[f64]) -> Vec<Candle> {
    let n = closes.len() as i64;
    closes
        .iter()
        .enumerate()
        .map(|(i, &close)| {
            let open = if i == 0 { close } else { closes[i - 1] };
            let ts = last_ts - (n - 1 - i as i64) * step_secs;
            candle(ts, open, open.max(close), open.min(close), close, 1.0)
        })
        .collect()
}

/// A bundle whose `timeframe` slot holds `candles` (all other timeframes stay empty).
pub fn bundle_with(timeframe: &str, candles: Vec<Candle>) -> TimeframeBundle {
    let mut bundle = TimeframeBundle::new(TEST_MINT.to_owned(), "TestPool".to_owned());
    match timeframe {
        "1m" => bundle.m1 = candles,
        "5m" => bundle.m5 = candles,
        "15m" => bundle.m15 = candles,
        "1h" => bundle.h1 = candles,
        "4h" => bundle.h4 = candles,
        "12h" => bundle.h12 = candles,
        "1d" => bundle.d1 = candles,
        other => panic!("unknown timeframe in fixture: {other}"),
    }
    bundle
}

// ==================== STRATEGY CONDITIONS ====================

/// Build a `Condition` from `(name, json value)` pairs. `default` mirrors the value —
/// the evaluators only ever read `value`.
pub fn condition(condition_type: &str, params: &[(&str, serde_json::Value)]) -> Condition {
    let mut parameters = HashMap::new();
    for (name, value) in params {
        parameters.insert(
            (*name).to_owned(),
            Parameter {
                value: value.clone(),
                default: value.clone(),
                constraints: None,
            },
        );
    }
    Condition {
        condition_type: condition_type.to_owned(),
        parameters,
    }
}

/// An evaluation context carrying a current price and one populated timeframe.
pub fn context_with_candles(
    current_price: f64,
    strategy_timeframe: &str,
    bundle: TimeframeBundle,
) -> EvaluationContext {
    EvaluationContext {
        token_mint: TEST_MINT.to_owned(),
        current_price: Some(current_price),
        position_data: None,
        market_data: None,
        timeframe_bundle: Some(bundle),
        strategy_timeframe: strategy_timeframe.to_owned(),
    }
}

/// An evaluation context with no OHLCV data at all.
pub fn context_bare(current_price: Option<f64>) -> EvaluationContext {
    EvaluationContext {
        token_mint: TEST_MINT.to_owned(),
        current_price,
        position_data: None,
        market_data: None,
        timeframe_bundle: None,
        strategy_timeframe: "5m".to_owned(),
    }
}

/// Fixed UTC timestamp used as the "now" anchor for candle fixtures, so tests never
/// depend on the wall clock.
pub fn anchor_ts() -> i64 {
    DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
        .expect("valid anchor")
        .timestamp()
}

fn env_flag(key: &str) -> bool {
    matches!(
        std::env::var(key).ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}
