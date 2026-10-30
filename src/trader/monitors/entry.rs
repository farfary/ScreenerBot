// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Entry opportunity monitoring - orchestration only
//!
//! This module handles:
//! - Monitoring loop and timing
//! - Filtered-token intersection with available pool tokens (every filtered token on an
//!   install without live pool prices, priced per token by `price_resolution::entry_price`)
//! - No entry on a chain whose access service is not ready
//! - Concurrency control via semaphore
//! - Calling evaluators for entry logic
//! - Delegating to `trader::entry` for reservation and submission: live decisions in the
//!   cycle, paper decisions afterwards in the monitor's detached paper batch
//! - Reserving a book only for a priced token in a cycle with an enabled entry strategy of
//!   that book's mode
//!
//! Token reservation and the submission pipeline (execute trade, event recording, action
//! tracking) live in `trader::entry`, shared with any other entry source.

use crate::chains::{ChainId, ChainScope};
use crate::i18n::{ids, UiText};
use crate::logger::{self, LogTag};
use crate::positions::{price_resolution, Book};
use crate::trader::paper::TradingMode;
use crate::trader::types::{EntryDecisions, TradeDecision};
use crate::trader::{config, constants, entry, evaluators};

use super::paper_batch::PaperBatch;
use std::collections::{HashMap, HashSet};
use tokio::time::{sleep, Duration, Instant};

/// Monitor for new entry opportunities
pub async fn monitor_entries(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> crate::trader::Result<()> {
    logger::info(LogTag::Trader, "Starting entry opportunity monitor");

    // Record monitor start event
    crate::events::record_trader_event(
        "entry_monitor_started",
        crate::events::Severity::Info,
        None,
        None,
        crate::events::with_text(
            serde_json::json!({
                "monitor": "entry",
            }),
            &UiText::new(ids::EVENTS_ENTRY_MONITOR_STARTED),
        ),
    )
    .await;

    // Create semaphore for concurrent entry checks
    let entry_check_concurrency = config::get_entry_check_concurrency();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(entry_check_concurrency));

    let mut was_paused = false; // Track paused state
    let mut access_paused: HashSet<ChainId> = HashSet::new();
    let mut paper_batch = PaperBatch::new("entry");

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            logger::info(LogTag::Trader, "Entry monitor shutting down");
            break;
        }

        // Check force stop first (emergency halt)
        if crate::global::is_force_stopped() {
            if !was_paused {
                logger::warning(LogTag::Trader, "Entry monitor paused - FORCE STOPPED");
                was_paused = true;
            }
            // Check more frequently during force stop
            super::pause_or_shutdown(&mut shutdown, Duration::from_secs(1)).await;
            continue;
        }

        // Check if entry monitor specifically is enabled (uses combined check)
        let entry_enabled = config::is_entry_monitor_enabled();
        if !entry_enabled {
            // Only log when transitioning to paused state
            if !was_paused {
                logger::info(LogTag::Trader, "Entry monitor paused - disabled via config");
                was_paused = true;
            }
            super::pause_or_shutdown(&mut shutdown, Duration::from_secs(5)).await;
            continue;
        }

        // Reset pause tracking if we're running
        if was_paused {
            logger::info(LogTag::Trader, "Entry monitor resumed");
            was_paused = false;
        }

        // Start cycle timing
        let cycle_start = Instant::now();

        // Get tokens that passed filtering — only these should be evaluated for entry,
        // grouped by the chain whose snapshot they passed
        let mut passed_by_chain: HashMap<ChainId, HashSet<String>> = HashMap::new();
        match crate::filtering::get_passed_tokens(ChainScope::All).await {
            Ok(tokens) => {
                for token in tokens {
                    passed_by_chain
                        .entry(token.chain)
                        .or_default()
                        .insert(token.mint);
                }
            }
            Err(e) => {
                logger::warning(
                    LogTag::Trader,
                    &format!("Failed to get passed tokens for entry: {e}"),
                );
            }
        }
        let passed_count: usize = passed_by_chain.values().map(HashSet::len).sum();

        // Only consider pool-tracked tokens that also passed filtering on the same chain
        let mut pool_count = 0;
        let mut available_tokens: Vec<(ChainId, String)> = Vec::new();
        for chain in ChainScope::All.chains() {
            if !access_allows_entries(chain, &mut access_paused) {
                continue;
            }
            let passed = passed_by_chain.get(&chain);
            match price_resolution::live_priced_mints(chain) {
                Some(pool_tokens) => {
                    pool_count += pool_tokens.len();
                    if let Some(passed) = passed {
                        available_tokens.extend(
                            pool_tokens
                                .into_iter()
                                .filter(|mint| passed.contains(mint))
                                .map(|mint| (chain, mint)),
                        );
                    }
                }
                None => available_tokens.extend(
                    passed
                        .into_iter()
                        .flatten()
                        .map(|mint| (chain, mint.clone())),
                ),
            }
        }

        // Log periodically (every ~30s = every 10 cycles at 3s interval)
        static CYCLE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let cycle = CYCLE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if cycle % 10 == 0 || !available_tokens.is_empty() {
            logger::info(
                LogTag::Trader,
                &format!(
                    "[ENTRY] cycle={} pool={} passed={} intersection={} tokens=[{}]",
                    cycle,
                    pool_count,
                    passed_count,
                    available_tokens.len(),
                    available_tokens
                        .iter()
                        .take(5)
                        .map(|(_, m)| &m[..8.min(m.len())])
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            );
        }

        // The token-independent admission inputs, read once for the whole cycle
        let cycle_inputs = if available_tokens.is_empty() {
            None
        } else {
            Some(std::sync::Arc::new(evaluators::entry_cycle().await))
        };

        // Process tokens with concurrency control. A cycle without an enabled entry
        // strategy wants no book, so it reserves and evaluates nothing.
        let mut futures = Vec::new();
        let cycle_inputs = cycle_inputs.filter(|cycle| !cycle.wanted().is_empty());
        if cycle_inputs.is_none() && !available_tokens.is_empty() {
            logger::debug(
                LogTag::Trader,
                "No enabled entry strategy: no token is evaluated this cycle",
            );
        }

        if let Some(cycle_inputs) = &cycle_inputs {
            for (chain, token) in &available_tokens {
                // A token without an entry price cannot be evaluated, so it takes no
                // reservation at all.
                let Some(price_info) = price_resolution::entry_price(*chain, token).await else {
                    logger::debug(
                        LogTag::Trader,
                        &format!("Token {token} has no entry price, skipping entry evaluation"),
                    );
                    continue;
                };

                // Reserve the books this token is evaluated on. The live reservation prevents
                // duplicate concurrent live entries and is held until the collect loop below
                // submits or releases it; the paper one is taken only when a paper decision
                // exists.
                let (live_reserved, books) = reserve_books(cycle_inputs.wanted(), token).await;
                if books.is_empty() {
                    // Reached only when the live reservation was not taken, so nothing is
                    // held for this token.
                    logger::debug(
                        LogTag::Trader,
                        &format!(
                            "Token {} already reserved by another thread, skipping",
                            token
                        ),
                    );
                    continue;
                }
                if cycle_inputs.wanted().contains(&TradingMode::Live) && !live_reserved {
                    logger::debug(
                        LogTag::Trader,
                        &format!(
                            "Token {token} live entry reserved by another path, skipping its live pass"
                        ),
                    );
                }

                let sem_clone = semaphore.clone();
                let token_clone = token.clone();
                let chain = *chain;

                let cycle_inputs = cycle_inputs.clone();
                let future = tokio::spawn(async move {
                    let _permit = match tokio::time::timeout(
                        Duration::from_secs(constants::ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS),
                        sem_clone.acquire(),
                    )
                    .await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            logger::error(
                                LogTag::Trader,
                                &format!("Failed to acquire semaphore for entry check: {e}"),
                            );
                            return EntryDecisions::default();
                        }
                        Err(_) => {
                            logger::warning(
                                LogTag::Trader,
                                &format!(
                                    "Timeout waiting for entry check semaphore for {}",
                                    token_clone
                                ),
                            );
                            return EntryDecisions::default();
                        }
                    };

                    // Evaluate entry opportunity (all safety checks + strategy evaluation)
                    match evaluators::evaluate_entry_for_token(
                        chain,
                        &token_clone,
                        &price_info,
                        &cycle_inputs,
                        &books,
                    )
                    .await
                    {
                        Ok(decisions) => decisions,
                        Err(e) => {
                            logger::error(
                                LogTag::Trader,
                                &format!("Entry evaluation failed for {token_clone}: {e}"),
                            );
                            EntryDecisions::default()
                        }
                    }
                });

                futures.push((token.clone(), live_reserved, future));
            }
        }

        // Collect results and process trade decisions: live ones now, paper ones after
        let mut paper_decisions = Vec::new();
        for (token, live_reserved, future) in futures {
            let decisions = match future.await {
                Ok(decisions) => decisions,
                Err(e) => {
                    logger::error(
                        LogTag::Trader,
                        &format!("Entry evaluation task failed for {token}: {e}"),
                    );
                    EntryDecisions::default()
                }
            };
            let route = route(live_reserved, decisions);
            if let Some(live) = route.live {
                // The submission releases the live reservation on every path
                entry::submit_entry(live).await;
            } else if route.release_live {
                entry::clear_entry_reservation(Book::Live, &token).await;
            }
            if let Some(paper) = route.paper {
                if entry::try_reserve_entry(Book::Paper, &token).await {
                    paper_decisions.push(paper);
                }
            }
        }

        launch_paper_entries(&mut paper_batch, paper_decisions).await;

        // Calculate wait time for next cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time =
            if cycle_duration >= Duration::from_secs(constants::ENTRY_MONITOR_INTERVAL_SECS) {
                Duration::from_millis(constants::ENTRY_CYCLE_MIN_WAIT_MS)
            } else {
                Duration::from_secs(constants::ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
            };

        // Wait for next cycle or shutdown
        tokio::select! {
            _ = sleep(wait_time) => {},
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    logger::info(LogTag::Trader, "Entry monitor shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}

/// The books a token is evaluated on: the `wanted` modes, with live dropped unless the
/// token's live reservation was taken.
/// Whether `chain` takes new entries: only while its chain access service is
/// ready (always under own-RPC access). Logs each change once, tracked in
/// `paused`.
fn access_allows_entries(chain: ChainId, paused: &mut HashSet<ChainId>) -> bool {
    let state = crate::chains::access::service_state(chain);
    if state.ready() {
        if paused.remove(&chain) {
            logger::info(
                LogTag::Trader,
                &format!("Entries on {chain} resume: chain access is ready"),
            );
        }
        return true;
    }
    if paused.insert(chain) {
        logger::warning(
            LogTag::Trader,
            &format!("Entries on {chain} paused: chain access is {}", state.key()),
        );
    }
    false
}

fn evaluation_books(wanted: &[TradingMode], live_reserved: bool) -> Vec<TradingMode> {
    wanted
        .iter()
        .copied()
        .filter(|mode| *mode != TradingMode::Live || live_reserved)
        .collect()
}

/// Takes the live reservation of `mint` when the cycle wants live, and returns whether it
/// was taken with the books the token is evaluated on. A paper-only cycle never reserves
/// live, and a live reservation held elsewhere, such as a live copy entry in flight, costs
/// the token only its live pass.
async fn reserve_books(wanted: &[TradingMode], mint: &str) -> (bool, Vec<TradingMode>) {
    let live_reserved =
        wanted.contains(&TradingMode::Live) && entry::try_reserve_entry(Book::Live, mint).await;
    (live_reserved, evaluation_books(wanted, live_reserved))
}

/// Where the entry decisions of one token go once its evaluation ended.
#[derive(Debug)]
struct EntryRoute {
    /// The live decision to submit inline; its submission releases the live reservation.
    live: Option<TradeDecision>,
    /// Whether the live reservation is released here because no live decision is
    /// submitted.
    release_live: bool,
    /// The paper decision for the paper batch, under its own paper reservation.
    paper: Option<TradeDecision>,
}

/// Routes a token's `decisions`: a live decision is submitted only under the live
/// reservation, and a live reservation without a live decision is released, so each one
/// taken is consumed or released exactly once. A paper decision only ever goes to the
/// paper batch. A failed evaluation arrives as no decisions.
fn route(live_reserved: bool, decisions: EntryDecisions) -> EntryRoute {
    let (live, paper) = decisions.into_books();
    let live = live.filter(|_| live_reserved);
    EntryRoute {
        release_live: live_reserved && live.is_none(),
        live,
        paper,
    }
}

/// Submits the paper entry `decisions` of a cycle in sequence in the monitor's paper
/// batch, each releasing its paper reservation when its submission ends. When the
/// previous batch is still running the decisions are dropped and their reservations
/// released, to be decided again next cycle.
async fn launch_paper_entries(batch: &mut PaperBatch, decisions: Vec<TradeDecision>) {
    if decisions.is_empty() {
        return;
    }
    let mints: Vec<String> = decisions
        .iter()
        .map(|decision| decision.mint.clone())
        .collect();
    let started = batch.launch(decisions.len(), async move {
        for decision in decisions {
            entry::submit_entry(decision).await;
        }
    });
    if !started {
        for mint in mints {
            entry::clear_entry_reservation(Book::Paper, &mint).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trader::types::{TradeAction, TradePriority};
    use crate::trader::TradeReason;
    use TradingMode::{Live, Paper};

    fn decision(book: Book) -> TradeDecision {
        TradeDecision {
            position_id: None,
            mint: "RouteMint".to_owned(),
            action: TradeAction::Buy,
            reason: TradeReason::StrategySignal,
            strategy_id: Some(format!("{}-strategy", book.as_str())),
            book,
            timestamp: chrono::Utc::now(),
            priority: TradePriority::Normal,
            price_native: Some(1.0),
            size_native: None,
            exit_percentage: None,
            slippage_pct: None,
            take_profit_level: None,
            paper_leg_key: None,
        }
    }

    fn decisions(live: bool, paper: bool) -> EntryDecisions {
        let mut decisions = EntryDecisions::default();
        for (wanted, book) in [(live, Book::Live), (paper, Book::Paper)] {
            if wanted {
                decisions.insert(decision(book));
            }
        }
        decisions
    }

    /// How many times a route consumes or releases the live reservation: a submitted live
    /// decision releases it on every path of its submission, and `release_live` does
    /// otherwise.
    fn live_releases(route: &EntryRoute) -> usize {
        usize::from(route.live.is_some()) + usize::from(route.release_live)
    }

    /// Across every outcome of an evaluation (live, paper, both, none, and a failed one,
    /// which arrives as none), a live reservation taken is consumed or released exactly
    /// once, and none is released that was not taken.
    #[test]
    fn every_live_reservation_is_consumed_or_released_exactly_once() {
        for live_reserved in [true, false] {
            for (live, paper) in [(true, false), (false, true), (true, true), (false, false)] {
                if live && !live_reserved {
                    // Not producible: the live pass runs only under the live reservation.
                    continue;
                }
                let route = route(live_reserved, decisions(live, paper));
                assert_eq!(
                    live_releases(&route),
                    usize::from(live_reserved),
                    "reserved={live_reserved} live={live} paper={paper}"
                );
                assert_eq!(route.paper.is_some(), paper);
            }
            let failed = route(live_reserved, EntryDecisions::default());
            assert_eq!(live_releases(&failed), usize::from(live_reserved));
            assert!(failed.live.is_none() && failed.paper.is_none());
        }
    }

    /// A live decision without the live reservation is never submitted, and nothing is
    /// released for a reservation the token does not hold.
    #[test]
    fn a_live_decision_without_a_live_reservation_is_never_submitted() {
        let route = route(false, decisions(true, true));
        assert!(route.live.is_none());
        assert!(!route.release_live);
        assert!(route.paper.is_some());
    }

    /// A paper decision only ever goes to the paper batch: the live submission carries a
    /// live decision or nothing.
    #[test]
    fn a_paper_decision_never_goes_to_the_live_submit() {
        for live_reserved in [true, false] {
            let route = route(live_reserved, decisions(false, true));
            assert!(route.live.is_none());
            assert_eq!(route.paper.map(|decision| decision.book), Some(Book::Paper));
        }
        let both = route(true, decisions(true, true));
        assert_eq!(both.live.map(|decision| decision.book), Some(Book::Live));
        assert_eq!(both.paper.map(|decision| decision.book), Some(Book::Paper));
    }

    /// Live is evaluated only under its reservation; paper is evaluated whatever live holds.
    #[test]
    fn the_books_evaluated_follow_the_live_reservation() {
        assert_eq!(evaluation_books(&[Live, Paper], true), [Live, Paper]);
        assert_eq!(evaluation_books(&[Live, Paper], false), [Paper]);
        assert_eq!(evaluation_books(&[Live], false), Vec::<TradingMode>::new());
        assert_eq!(evaluation_books(&[Paper], false), [Paper]);
        assert_eq!(evaluation_books(&[], false), Vec::<TradingMode>::new());
    }

    /// A paper-only cycle never takes the live reservation, so a live entry from another
    /// source (a live copy entry) can still reserve the mint.
    #[tokio::test]
    async fn a_paper_only_cycle_never_reserves_live() {
        let mint = "PaperOnlyCycleMint11111111111111111111111111";
        assert_eq!(reserve_books(&[Paper], mint).await, (false, vec![Paper]));
        assert!(
            entry::try_reserve_entry(Book::Live, mint).await,
            "a paper-only cycle held the live reservation"
        );
        entry::clear_entry_reservation(Book::Live, mint).await;
    }

    /// A live reservation held elsewhere costs the token only its live pass: its paper pass
    /// still runs, and the reservation held elsewhere is left in place.
    #[tokio::test]
    async fn a_live_reservation_held_elsewhere_leaves_the_paper_pass() {
        let mint = "HeldElsewhereMint1111111111111111111111111111";
        assert!(entry::try_reserve_entry(Book::Live, mint).await);
        assert_eq!(
            reserve_books(&[Live, Paper], mint).await,
            (false, vec![Paper])
        );
        assert_eq!(
            reserve_books(&[Live], mint).await,
            (false, Vec::<TradingMode>::new())
        );
        assert!(
            !entry::try_reserve_entry(Book::Live, mint).await,
            "the reservation held elsewhere was released"
        );
        entry::clear_entry_reservation(Book::Live, mint).await;

        assert_eq!(
            reserve_books(&[Live, Paper], mint).await,
            (true, vec![Live, Paper])
        );
        entry::clear_entry_reservation(Book::Live, mint).await;
    }
}
