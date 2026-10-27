// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Entry opportunity monitoring - orchestration only
//!
//! This module handles:
//! - Monitoring loop and timing
//! - Filtered-token intersection with available pool tokens
//! - Concurrency control via semaphore
//! - Calling evaluators for entry logic
//! - Delegating to `trader::entry` for reservation and submission: live decisions in the
//!   cycle, paper decisions afterwards in the monitor's detached paper batch
//!
//! Token reservation and the submission pipeline (execute trade, event recording, action
//! tracking) live in `trader::entry`, shared with any other entry source.

use crate::chains::{ChainId, ChainScope};
use crate::i18n::{ids, UiText};
use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions::Book;
use crate::trader::types::TradeDecision;
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
            sleep(Duration::from_secs(1)).await; // Check more frequently during force stop
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
            sleep(Duration::from_secs(5)).await;
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
            let pool_tokens = pools::get_available_tokens(chain);
            pool_count += pool_tokens.len();
            if let Some(passed) = passed_by_chain.get(&chain) {
                available_tokens.extend(
                    pool_tokens
                        .into_iter()
                        .filter(|mint| passed.contains(mint))
                        .map(|mint| (chain, mint)),
                );
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

        // Process tokens with concurrency control
        let mut futures = Vec::new();

        for (chain, token) in &available_tokens {
            // Try to reserve token for this cycle - prevents duplicate concurrent entries.
            // Evaluation may decide a live entry, so it holds the live reservation.
            if !entry::try_reserve_entry(Book::Live, token).await {
                logger::debug(
                    LogTag::Trader,
                    &format!(
                        "Token {} already reserved by another thread, skipping",
                        token
                    ),
                );
                continue;
            }

            // Get latest price info
            // Note: If no price info, let reservation expire naturally via timeout
            // instead of clearing immediately to avoid race conditions
            if let Some(price_info) = pools::get_pool_price(*chain, token) {
                // Acquire semaphore permit with timeout
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
                            return None;
                        }
                        Err(_) => {
                            logger::warning(
                                LogTag::Trader,
                                &format!(
                                    "Timeout waiting for entry check semaphore for {}",
                                    token_clone
                                ),
                            );
                            return None;
                        }
                    };

                    let cycle_inputs = cycle_inputs?;

                    // Evaluate entry opportunity (all safety checks + strategy evaluation)
                    match evaluators::evaluate_entry_for_token(
                        chain,
                        &token_clone,
                        &price_info,
                        &cycle_inputs,
                    )
                    .await
                    {
                        Ok(Some(decision)) => Some(decision),
                        Ok(None) => None,
                        Err(e) => {
                            logger::error(
                                LogTag::Trader,
                                &format!("Entry evaluation failed for {token_clone}: {e}"),
                            );
                            None
                        }
                    }
                });

                futures.push((token.clone(), future));
            }
            // Note: If no price info available, reservation will expire via timeout
            // This prevents race conditions from immediate clearing
        }

        // Collect results and process trade decisions: live ones now, paper ones after
        let mut paper_decisions = Vec::new();
        for (token, future) in futures {
            match future.await {
                Ok(Some(decision)) if decision.book == Book::Paper => {
                    // No live entry follows: the paper entry holds its own reservation
                    entry::clear_entry_reservation(Book::Live, &token).await;
                    if entry::try_reserve_entry(Book::Paper, &token).await {
                        paper_decisions.push(decision);
                    }
                }
                Ok(Some(decision)) => {
                    entry::submit_entry(decision).await;
                }
                Ok(None) => {
                    // No entry signal, clear reservation
                    entry::clear_entry_reservation(Book::Live, &token).await;
                }
                Err(e) => {
                    // Task error, clear reservation
                    entry::clear_entry_reservation(Book::Live, &token).await;
                    logger::error(
                        LogTag::Trader,
                        &format!("Entry evaluation task failed for {token}: {e}"),
                    );
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
