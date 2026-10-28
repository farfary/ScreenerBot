// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Source-independent entry admission.
//!
//! Everything that decides whether *any* new entry may happen right now, regardless of
//! what produced the signal. Extracted from `evaluators::entry` so a second entry source
//! runs the identical gauntlet instead of a copy of it.

use crate::database::catalog::{StoreSpec, COPY_TRADING, PAPER_POSITIONS, POSITIONS, STRATEGIES};
use crate::database::health::{self, StoreStatus};
use crate::positions::{books, Book};
use crate::strategies::types::Strategy;
use crate::trader::paper::TradingMode;
use crate::trader::safety;

/// The stores every entry decision reads: open positions and their limits, copy tasks and
/// their budgets, and the strategies that produce signals.
pub const ENTRY_STORES: [&StoreSpec; 3] = [&POSITIONS, &COPY_TRADING, &STRATEGIES];

/// The stores a paper entry reads: the paper book with its account, and the strategies
/// that produce signals.
pub const PAPER_ENTRY_STORES: [&StoreSpec; 2] = [&PAPER_POSITIONS, &STRATEGIES];

/// Why an entry may not proceed. Every variant is a value a caller can record, count and
/// render — the checks used to be log lines only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum EntryBlock {
    ForceStopped,
    LossLimit,
    Connectivity(String),
    /// An entry store is not open, or holds restored contents not yet reconciled with
    /// the wallet history. The detail names the store and its state.
    StorageUnavailable(String),
    PositionLimit,
    AlreadyOpen,
    ReentryCooldown,
    OpenCooldown {
        wait_secs: u64,
    },
    EntryReserved,
    Blacklisted,
    /// The cash of the current paper run does not cover a paper entry.
    PaperBalanceExhausted,
    /// A check itself failed (DB/state error), not a rejection.
    CheckFailed(String),
}

/// Remaining global entry cooldown, kept separate from the strategy admission path so
/// existing strategy scheduling behavior does not change. Copy uses it to persist an
/// honest typed skip before reaching the string error in `open_position_impl`.
pub fn open_cooldown_wait_secs(
    last_open: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
    cooldown_secs: u64,
) -> Option<u64> {
    let last_open = last_open?;
    let elapsed = now.signed_duration_since(last_open).num_seconds().max(0) as u64;
    (elapsed < cooldown_secs).then_some(cooldown_secs - elapsed)
}

/// The block for the first entry store in `statuses` that is not settled. A store the
/// storage phase has not reached (`None`) does not block: its owner opens it on first use
/// and reports its own error.
fn storage_block<'a>(
    statuses: impl IntoIterator<Item = Option<&'a StoreStatus>>,
) -> Option<EntryBlock> {
    statuses
        .into_iter()
        .flatten()
        .find(|status| !status.is_settled())
        .map(|status| {
            let state = status.health.as_str();
            EntryBlock::StorageUnavailable(if status.health.is_open() {
                format!("{}: {state}, not yet reconciled", status.store)
            } else {
                format!("{}: {state}", status.store)
            })
        })
}

/// Refuse an entry while any entry store is not settled.
pub fn check_entry_storage() -> Result<(), EntryBlock> {
    check_storage(&ENTRY_STORES)
}

/// Refuse an entry while any of `stores` is not settled.
fn check_storage(stores: &[&StoreSpec]) -> Result<(), EntryBlock> {
    let statuses: Vec<Option<StoreStatus>> = stores
        .iter()
        .flat_map(|spec| spec.paths())
        .map(|path| health::status(&path))
        .collect();
    storage_block(statuses.iter().map(Option::as_ref)).map_or(Ok(()), Err)
}

pub async fn check_open_cooldown() -> Result<(), EntryBlock> {
    let cooldown_secs = crate::config::with_config(|config| {
        config.positions.position_open_cooldown_secs.max(0) as u64
    });
    let last_open = *crate::positions::state::LAST_OPEN_TIME.read().await;
    match open_cooldown_wait_secs(last_open, chrono::Utc::now(), cooldown_secs) {
        Some(wait_secs) => Err(EntryBlock::OpenCooldown { wait_secs }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use std::path::PathBuf;

    use super::{
        admit_each, admitted_modes, open_cooldown_wait_secs, paper_cash_covers, storage_block,
        EntryBlock,
    };
    use crate::database::health::{StoreHealth, StoreStatus};
    use crate::database::recovery::RecoveryAction;
    use crate::errors::DatabaseError;
    use crate::trader::paper::TradingMode;

    fn store(store: &'static str, health: StoreHealth, reconciled: bool) -> StoreStatus {
        StoreStatus {
            store,
            path: PathBuf::from(format!("/admission-test/{store}.db")),
            health,
            changed_at: Utc::now(),
            reconciled,
        }
    }

    fn healthy(store_id: &'static str) -> StoreStatus {
        store(store_id, StoreHealth::Healthy, true)
    }

    #[test]
    fn every_store_state_that_is_not_open_blocks_entries() {
        let disk_full = DatabaseError::DiskFull {
            operation: "open".to_owned(),
            message: "full".to_owned(),
        };
        let closed = [
            StoreHealth::Rebuilding {
                since: Utc::now(),
                phase: "restore".to_owned(),
            },
            StoreHealth::unavailable(&disk_full, RecoveryAction::Unavailable),
            StoreHealth::unavailable(&disk_full, RecoveryAction::Isolate),
        ];
        for health in closed {
            let state = health.as_str();
            let copy = store("copy_trading", health, true);
            let positions = healthy("positions");
            let strategies = healthy("strategies");
            assert_eq!(
                storage_block([Some(&positions), Some(&copy), Some(&strategies)]),
                Some(EntryBlock::StorageUnavailable(format!(
                    "copy_trading: {state}"
                )))
            );
        }
    }

    #[test]
    fn replaced_contents_block_entries_until_reconciled() {
        let quarantine = PathBuf::from("/admission-test/quarantine");
        for health in [
            StoreHealth::Restored {
                snapshot_at: None,
                quarantine: quarantine.clone(),
            },
            StoreHealth::Recovered {
                quarantine: quarantine.clone(),
            },
        ] {
            let state = health.as_str();
            let waiting = store("positions", health.clone(), false);
            let strategies = healthy("strategies");
            assert_eq!(
                storage_block([Some(&waiting), Some(&strategies)]),
                Some(EntryBlock::StorageUnavailable(format!(
                    "positions: {state}, not yet reconciled"
                )))
            );
            let reconciled = store("positions", health, true);
            assert_eq!(storage_block([Some(&reconciled), Some(&strategies)]), None);
        }
    }

    #[test]
    fn settled_or_unopened_stores_never_block_entries() {
        let positions = healthy("positions");
        let copy = healthy("copy_trading");
        let strategies = healthy("strategies");
        assert_eq!(
            storage_block([Some(&positions), Some(&copy), Some(&strategies)]),
            None
        );
        assert_eq!(storage_block([None, None, None]), None);
    }

    #[test]
    fn open_cooldown_reports_the_boundary_without_rounding_it_away() {
        let now = Utc.timestamp_opt(100, 0).unwrap();
        assert_eq!(open_cooldown_wait_secs(None, now, 5), None);
        assert_eq!(open_cooldown_wait_secs(Some(now), now, 5), Some(5));
        assert_eq!(
            open_cooldown_wait_secs(Some(now - Duration::seconds(4)), now, 5),
            Some(1)
        );
        assert_eq!(
            open_cooldown_wait_secs(Some(now - Duration::seconds(5)), now, 5),
            None
        );
    }

    #[test]
    fn each_mode_is_admitted_on_its_own_and_a_live_refusal_is_never_hidden() {
        assert_eq!(
            admitted_modes(vec![
                (TradingMode::Live, Err(EntryBlock::LossLimit)),
                (TradingMode::Paper, Ok(())),
            ]),
            Ok(vec![TradingMode::Paper])
        );
        assert_eq!(
            admitted_modes(vec![
                (TradingMode::Live, Ok(())),
                (TradingMode::Paper, Err(EntryBlock::PaperBalanceExhausted)),
            ]),
            Ok(vec![TradingMode::Live])
        );
        assert_eq!(
            admitted_modes(vec![
                (TradingMode::Paper, Err(EntryBlock::PaperBalanceExhausted)),
                (TradingMode::Live, Err(EntryBlock::PositionLimit)),
            ]),
            Err(EntryBlock::PositionLimit)
        );
        assert_eq!(
            admitted_modes(vec![(
                TradingMode::Paper,
                Err(EntryBlock::PaperBalanceExhausted)
            )]),
            Err(EntryBlock::PaperBalanceExhausted)
        );
        assert!(matches!(
            admitted_modes(Vec::new()),
            Err(EntryBlock::CheckFailed(_))
        ));
    }

    /// Only the requested modes run a gauntlet and can be admitted: a token evaluated on
    /// paper alone never takes the live gauntlet, and no requested mode admits nothing.
    #[tokio::test]
    async fn admit_modes_admits_only_the_requested_modes() {
        for requested in [
            vec![TradingMode::Paper],
            vec![TradingMode::Live],
            vec![TradingMode::Live, TradingMode::Paper],
        ] {
            let mut checked = Vec::new();
            let admitted = admit_each(&requested, |mode| {
                checked.push(mode);
                async { Ok(()) }
            })
            .await;
            assert_eq!(checked, requested);
            assert_eq!(admitted, Ok(requested.clone()));
        }
        let mut checked = Vec::new();
        let admitted = admit_each(&[], |mode| {
            checked.push(mode);
            async { Ok(()) }
        })
        .await;
        assert!(checked.is_empty());
        assert!(matches!(admitted, Err(EntryBlock::CheckFailed(_))));
    }

    fn entry(size_native: Option<f64>) -> crate::trader::types::TradeDecision {
        crate::trader::types::TradeDecision {
            position_id: None,
            mint: "AdmissionSizeMint111111111111111111111111111".to_owned(),
            action: crate::trader::types::TradeAction::Buy,
            reason: crate::trader::types::TradeReason::StrategySignal,
            strategy_id: None,
            book: crate::positions::Book::Paper,
            timestamp: Utc::now(),
            priority: crate::trader::types::TradePriority::Normal,
            price_native: None,
            size_native,
            exit_percentage: None,
            slippage_pct: None,
            take_profit_level: None,
            paper_leg_key: None,
        }
    }

    #[test]
    fn paper_cash_is_judged_against_the_size_the_entry_buys() {
        let _ = crate::config::utils::CONFIG
            .get_or_init(|| std::sync::RwLock::new(crate::config::schemas::Config::default()));
        let default_size = crate::trader::config::get_trade_size_native();
        let needed = |size| crate::trader::executors::entry_size_native(&entry(size));
        let above = Some(default_size * 10.0);
        let below = Some(default_size / 10.0);
        let cash = Ok(default_size * 2.0);

        assert_eq!(paper_cash_covers(Some(&cash), needed(None)), Ok(()));
        assert_eq!(
            paper_cash_covers(Some(&cash), needed(above)),
            Err(EntryBlock::PaperBalanceExhausted),
            "an entry sized above the default is admitted on the default size"
        );
        let short = Ok(default_size / 2.0);
        assert_eq!(
            paper_cash_covers(Some(&short), needed(None)),
            Err(EntryBlock::PaperBalanceExhausted)
        );
        assert_eq!(
            paper_cash_covers(Some(&short), needed(below)),
            Ok(()),
            "an entry sized below the default is refused on the default size"
        );
        assert!(matches!(
            paper_cash_covers(None, needed(None)),
            Err(EntryBlock::CheckFailed(_))
        ));
    }

    /// Catalog key of the label for each entry block. The match is exhaustive, so
    /// a new variant fails to compile until it is mapped here and in
    /// `ENTRY_BLOCK_LABELS` (pages/copy/format.js).
    fn block_label_key(block: &EntryBlock) -> &'static str {
        match block {
            EntryBlock::ForceStopped => "copy-entry-block-force-stopped",
            EntryBlock::LossLimit => "copy-entry-block-loss-limit",
            EntryBlock::Connectivity(_) => "copy-entry-block-connectivity",
            EntryBlock::StorageUnavailable(_) => "copy-entry-block-storage-unavailable",
            EntryBlock::PositionLimit => "copy-entry-block-position-limit",
            EntryBlock::AlreadyOpen => "copy-entry-block-already-open",
            EntryBlock::ReentryCooldown => "copy-entry-block-reentry-cooldown",
            EntryBlock::OpenCooldown { .. } => "copy-entry-block-open-cooldown",
            EntryBlock::EntryReserved => "copy-entry-block-entry-reserved",
            EntryBlock::Blacklisted => "copy-entry-block-blacklisted",
            EntryBlock::PaperBalanceExhausted => "copy-entry-block-paper-balance-exhausted",
            EntryBlock::CheckFailed(_) => "copy-entry-block-check-failed",
        }
    }

    #[test]
    fn entry_block_labels_exist_in_the_catalog() {
        let blocks = [
            EntryBlock::ForceStopped,
            EntryBlock::LossLimit,
            EntryBlock::Connectivity(String::new()),
            EntryBlock::StorageUnavailable(String::new()),
            EntryBlock::PositionLimit,
            EntryBlock::AlreadyOpen,
            EntryBlock::ReentryCooldown,
            EntryBlock::OpenCooldown { wait_secs: 0 },
            EntryBlock::EntryReserved,
            EntryBlock::Blacklisted,
            EntryBlock::PaperBalanceExhausted,
            EntryBlock::CheckFailed(String::new()),
        ];
        for block in &blocks {
            let value = serde_json::to_value(block).unwrap();
            let id = value["kind"].as_str().expect("block kind");
            let key = block_label_key(block);
            assert_eq!(
                key,
                format!("copy-entry-block-{}", id.replace('_', "-")),
                "key does not follow the serialized id {id}"
            );
            assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
        }
    }
}

/// Run the source-independent admission gauntlet for a potential entry.
///
/// Order: force stop -> loss limit -> entry storage -> connectivity -> position limits ->
/// existing position -> re-entry cooldown -> blacklist.
///
/// `required_endpoints` differs by source on purpose: strategy entries depend on filter
/// freshness and require rpc + dexscreener + rugcheck; a copy entry requires rpc only,
/// matching the live buy executor.
pub async fn check_entry_admission(
    mint: &str,
    required_endpoints: &[&str],
) -> Result<(), EntryBlock> {
    // Early exit: force stop, then the loss limit
    if let Some(block) = crate::trader::controller::entry_halt(TradingMode::Live) {
        let short_mint = &mint[..8.min(mint.len())];
        if block == EntryBlock::LossLimit {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!("[ENTRY-EVAL] {short_mint}: blocked by loss_limit"),
            );
        } else {
            crate::logger::debug(
                crate::logger::LogTag::Trader,
                &format!("[ENTRY-EVAL] {short_mint}: blocked by force_stop"),
            );
        }
        return Err(block);
    }

    // Entry stores open and reconciled: a restored or recreated ledger would size and
    // count against a past state of the positions, copy budgets or strategies.
    if let Err(block) = check_entry_storage() {
        crate::logger::debug(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by storage: {block:?}",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(block);
    }

    // 1. Connectivity check - no required endpoint may be CONFIRMED down.
    //
    // Confirmed-down, not "not known to be up": an endpoint whose monitor is disabled in
    // config is never probed and stays `Unknown` forever, so the stricter check turned
    // `connectivity.endpoints.rugcheck.enabled = false` into a permanent, silent halt on
    // all automated buying. The executors still refuse to send a swap without a healthy
    // RPC, which is where fail-closed belongs.
    if let Some(unhealthy) = crate::connectivity::check_endpoints_usable(required_endpoints).await {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by unhealthy: {}",
                &mint[..8.min(mint.len())],
                unhealthy
            ),
        );
        return Err(EntryBlock::Connectivity(unhealthy));
    }

    // 2. Position limits - check if we can open more positions
    if let Err(block) = check_slots(TradingMode::Live).await {
        if block == EntryBlock::PositionLimit {
            crate::logger::debug(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by position_limits",
                    &mint[..8.min(mint.len())]
                ),
            );
        }
        return Err(block);
    }

    // 3. Existing position check - prevent duplicate entries
    match safety::has_open_position(mint).await {
        Ok(false) => {}
        Ok(true) => {
            crate::logger::debug(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by existing_position",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Err(EntryBlock::AlreadyOpen);
        }
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
    }

    // 4. Re-entry cooldown - prevent immediate re-entry after exit
    match safety::is_in_reentry_cooldown(Book::Live, mint).await {
        Ok(false) => {}
        Ok(true) => {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "[ENTRY-EVAL] {}: blocked by reentry_cooldown",
                    &mint[..8.min(mint.len())]
                ),
            );
            return Err(EntryBlock::ReentryCooldown);
        }
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
    }

    // 5. Blacklist check - token-level only (not pool-level)
    if safety::is_blacklisted(mint).await {
        crate::logger::info(
            crate::logger::LogTag::Trader,
            &format!(
                "[ENTRY-EVAL] {}: blocked by blacklist",
                &mint[..8.min(mint.len())]
            ),
        );
        return Err(EntryBlock::Blacklisted);
    }

    Ok(())
}

/// What every entry of one cycle is admitted and evaluated against, read once per entry
/// cycle rather than once per token: the modes entries are wanted on, the enabled entry
/// strategies that signal them with when they were read and, when paper is one of the
/// modes, the cash of the current paper run. The paper executor checks the cash again when
/// it books, so a cycle-old figure never spends cash the run no longer has.
#[derive(Debug, Clone)]
pub struct EntryCycle {
    wanted: Vec<TradingMode>,
    entry_strategies: Vec<Strategy>,
    strategies_read_at: chrono::DateTime<chrono::Utc>,
    paper_cash: Option<Result<f64, EntryBlock>>,
}

impl EntryCycle {
    /// The cycle of the `wanted` modes and the enabled `entry_strategies` in priority
    /// order, as read from the store at `strategies_read_at`, reading the paper cash when
    /// paper is wanted. The time is taken before the store read: a rules change committed
    /// after it is never taken as one the strategies were read under.
    pub async fn read_at(
        strategies_read_at: chrono::DateTime<chrono::Utc>,
        wanted: Vec<TradingMode>,
        entry_strategies: Vec<Strategy>,
    ) -> Self {
        let paper_cash = if wanted.contains(&TradingMode::Paper) {
            Some(read_paper_cash().await)
        } else {
            None
        };
        Self {
            wanted,
            entry_strategies,
            strategies_read_at,
            paper_cash,
        }
    }

    /// The modes entries are wanted on this cycle.
    pub fn wanted(&self) -> &[TradingMode] {
        &self.wanted
    }

    /// The enabled entry strategies of this cycle, in priority order; each one's mode
    /// decides the book its entry is made on.
    pub fn entry_strategies(&self) -> &[Strategy] {
        &self.entry_strategies
    }

    /// When the entry strategies of this cycle were read: every entry decision of the
    /// cycle is decided on rules no newer than this.
    pub fn strategies_read_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.strategies_read_at
    }
}

fn paper_book_not_loaded() -> EntryBlock {
    EntryBlock::StorageUnavailable(format!("{}: not loaded", PAPER_POSITIONS.id))
}

/// The cash of the current paper run of the active wallet.
async fn read_paper_cash() -> Result<f64, EntryBlock> {
    let paper = crate::positions::paper::book().ok_or_else(paper_book_not_loaded)?;
    let wallet =
        crate::utils::get_wallet_address().map_err(|e| EntryBlock::CheckFailed(e.to_string()))?;
    let starting_balance =
        crate::config::with_config(|config| config.trader.paper.starting_balance_sol);
    paper
        .account(&wallet, starting_balance)
        .await
        .map(|account| account.cash_native)
        .map_err(|e| EntryBlock::CheckFailed(e.to_string()))
}

/// The cash of the current paper run of the active wallet without starting a run: before
/// the first run it is the configured starting balance that run starts with. The paper
/// store is read on the blocking pool.
pub(in crate::trader) async fn standing_paper_cash() -> Result<f64, EntryBlock> {
    let paper = crate::positions::paper::book().ok_or_else(paper_book_not_loaded)?;
    let wallet =
        crate::utils::get_wallet_address().map_err(|e| EntryBlock::CheckFailed(e.to_string()))?;
    let starting_balance =
        crate::config::with_config(|config| config.trader.paper.starting_balance_sol);
    tokio::task::spawn_blocking(move || paper.current_account(&wallet))
        .await
        .map_err(|e| EntryBlock::CheckFailed(e.to_string()))?
        .map(|account| account.map_or(starting_balance, |account| account.cash_native))
        .map_err(|e| EntryBlock::CheckFailed(e.to_string()))
}

/// Refuse an entry on `mode` while that mode's book holds as many open positions as the
/// configured limit allows: the live position limit, or the paper book's own slots.
pub(in crate::trader) async fn check_slots(mode: TradingMode) -> Result<(), EntryBlock> {
    let has_room = match mode {
        TradingMode::Live => safety::check_position_limits()
            .await
            .map_err(|e| EntryBlock::CheckFailed(e.to_string()))?,
        TradingMode::Paper => {
            let paper = crate::positions::paper::book().ok_or_else(paper_book_not_loaded)?;
            paper.open_positions().await.len() < crate::trader::config::get_max_open_positions()
        }
    };
    if has_room {
        Ok(())
    } else {
        Err(EntryBlock::PositionLimit)
    }
}

/// Refuse a paper entry costing `needed_native` that the paper `cash` read for the cycle
/// does not cover, on the terms the paper executor checks at booking.
pub(in crate::trader) fn paper_cash_covers(
    cash: Option<&Result<f64, EntryBlock>>,
    needed_native: f64,
) -> Result<(), EntryBlock> {
    match cash {
        Some(Ok(cash)) if *cash + f64::EPSILON < needed_native => {
            Err(EntryBlock::PaperBalanceExhausted)
        }
        Some(Ok(_)) => Ok(()),
        Some(Err(block)) => Err(block.clone()),
        None => Err(EntryBlock::CheckFailed(
            "the paper cash of this entry cycle was not read".to_owned(),
        )),
    }
}

/// Run the admission gauntlet of a paper entry of `entry_size_native`, the size the entry
/// will buy, against the inputs of its `cycle`. Paper never takes live capacity and a live
/// loss-limit pause, which protects real funds, does not stop it.
///
/// Order: force stop -> paper storage -> connectivity -> paper slots -> paper open
/// cooldown -> existing paper round -> paper re-entry cooldown -> paper cash -> blacklist.
pub async fn check_paper_entry_admission(
    mint: &str,
    required_endpoints: &[&str],
    cycle: &EntryCycle,
    entry_size_native: f64,
) -> Result<(), EntryBlock> {
    if let Some(block) = crate::trader::controller::entry_halt(TradingMode::Paper) {
        return Err(block);
    }
    check_storage(&PAPER_ENTRY_STORES)?;
    if let Some(unhealthy) = crate::connectivity::check_endpoints_usable(required_endpoints).await {
        return Err(EntryBlock::Connectivity(unhealthy));
    }
    check_slots(TradingMode::Paper).await?;
    let paper = crate::positions::paper::book().ok_or_else(paper_book_not_loaded)?;
    let cooldown_secs = crate::config::with_config(|config| {
        config.positions.position_open_cooldown_secs.max(0) as u64
    });
    if let Some(wait_secs) =
        open_cooldown_wait_secs(paper.last_open(), chrono::Utc::now(), cooldown_secs)
    {
        return Err(EntryBlock::OpenCooldown { wait_secs });
    }
    if books::holds_open_round(Book::Paper, mint).await {
        return Err(EntryBlock::AlreadyOpen);
    }
    match safety::is_in_reentry_cooldown(Book::Paper, mint).await {
        Ok(false) => {}
        Ok(true) => return Err(EntryBlock::ReentryCooldown),
        Err(e) => return Err(EntryBlock::CheckFailed(e.to_string())),
    }
    paper_cash_covers(cycle.paper_cash.as_ref(), entry_size_native)?;
    if safety::is_blacklisted(mint).await {
        return Err(EntryBlock::Blacklisted);
    }
    Ok(())
}

/// The modes among `modes`, the books `mint` is evaluated on, that an entry of `mint` of
/// `entry_size_native` is admitted on, each through its own book's gauntlet against the
/// inputs of its `cycle`. When none is admitted, the block reported is the live one when
/// live was requested, so a live refusal is never hidden behind a paper one.
pub async fn admit_modes(
    mint: &str,
    required_endpoints: &[&str],
    cycle: &EntryCycle,
    modes: &[TradingMode],
    entry_size_native: f64,
) -> Result<Vec<TradingMode>, EntryBlock> {
    admit_each(modes, |mode| async move {
        match mode {
            TradingMode::Live => check_entry_admission(mint, required_endpoints).await,
            TradingMode::Paper => {
                check_paper_entry_admission(mint, required_endpoints, cycle, entry_size_native)
                    .await
            }
        }
    })
    .await
}

/// Runs `gauntlet` for each of `modes` and nothing else, in order, and reads the outcomes
/// through [`admitted_modes`].
async fn admit_each<F, Fut>(
    modes: &[TradingMode],
    mut gauntlet: F,
) -> Result<Vec<TradingMode>, EntryBlock>
where
    F: FnMut(TradingMode) -> Fut,
    Fut: std::future::Future<Output = Result<(), EntryBlock>>,
{
    let mut outcomes = Vec::with_capacity(modes.len());
    for mode in modes {
        outcomes.push((*mode, gauntlet(*mode).await));
    }
    admitted_modes(outcomes)
}

/// The admitted modes of per-mode admission `outcomes`, or the block to report when none
/// is admitted: the live block when live was wanted, else the first.
fn admitted_modes(
    outcomes: Vec<(TradingMode, Result<(), EntryBlock>)>,
) -> Result<Vec<TradingMode>, EntryBlock> {
    let mut admitted = Vec::with_capacity(outcomes.len());
    let mut first_block = None;
    let mut live_block = None;
    for (mode, outcome) in outcomes {
        match outcome {
            Ok(()) => admitted.push(mode),
            Err(block) => {
                if mode == TradingMode::Live {
                    live_block = Some(block.clone());
                }
                first_block.get_or_insert(block);
            }
        }
    }
    if admitted.is_empty() {
        return Err(live_block
            .or(first_block)
            .unwrap_or(EntryBlock::CheckFailed(
                "no trading mode was requested".to_owned(),
            )));
    }
    Ok(admitted)
}
