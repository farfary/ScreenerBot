// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy evaluation for trading decisions
//!
//! Applies configured strategies to evaluate entry and exit opportunities.
//! Renamed from StrategyManager to StrategyEvaluator for consistency.

use crate::chains::ChainId;
use crate::logger::{self, LogTag};
use crate::ohlcvs::TimeframeBundle;
use crate::pools::PriceResult;
use crate::positions::{Book, Position};
use crate::strategies;
use crate::strategies::types::{
    EvaluationContext, LiquidityPool, MarketData, PositionData, Strategy, StrategyType,
    TransactionCounts,
};
use crate::tokens::Token;
use crate::trader::constants::STRATEGY_EVALUATION_TIMEOUT_SECS;
use crate::trader::paper::TradingMode;
use crate::trader::types::{
    EntryDecisions, TradeAction, TradeDecision, TradePriority, TradeReason,
};
use chrono::{DateTime, Utc};
use std::time::Duration;

/// Pool reserves only count as a liquidity reading when they are a real measurement.
///
/// `0.0` is what an API-derived price result carries for `native_reserves` — it means
/// nobody measured the pool, not that the pool is empty. Passed through as a reading it
/// would satisfy every "liquidity below X" rule on a token whose liquidity is unknown.
fn usable_liquidity(native_reserves: f64) -> Option<f64> {
    (native_reserves.is_finite() && native_reserves > 0.0).then_some(native_reserves)
}

/// The pool a price result's reserves were read from, with what the pool's decoder
/// declares about them; `None` for a price that names no pool.
fn liquidity_pool(chain: ChainId, price: &PriceResult) -> Option<LiquidityPool> {
    (!price.pool_address.is_empty()).then(|| LiquidityPool {
        virtual_reserves: crate::pools::pool_reports_virtual_reserves(
            chain,
            &price.mint,
            &price.pool_address,
        ),
        address: price.pool_address.clone(),
    })
}

/// The size a strategy entry decision carries: none, so it buys the configured trade size.
const STRATEGY_ENTRY_SIZE: Option<f64> = None;

/// The size a strategy entry buys, as the executor sizes it.
pub fn strategy_entry_size_native() -> f64 {
    crate::trader::executors::entry_size_of(STRATEGY_ENTRY_SIZE)
}

/// The buy an entry strategy signal decides on `book`, the book of the strategy's mode:
/// the configured trade size.
fn entry_decision(mint: &str, strategy_id: String, price_native: f64, book: Book) -> TradeDecision {
    TradeDecision {
        position_id: None,
        mint: mint.to_owned(),
        action: TradeAction::Buy,
        reason: TradeReason::StrategySignal,
        strategy_id: Some(strategy_id),
        book,
        timestamp: Utc::now(),
        priority: TradePriority::Normal,
        price_native: Some(price_native),
        size_native: STRATEGY_ENTRY_SIZE,
        exit_percentage: None,
        // Auto-trader slippage always follows config.
        slippage_pct: None,
        take_profit_level: None,
        paper_leg_key: None,
    }
}

/// The sale an exit strategy signal decides: the whole position, recorded as a
/// strategy exit so closed-reason labels and the exit breakdown tell it apart
/// from the entry signal.
fn exit_decision(
    position_id: Option<i64>,
    mint: &str,
    strategy_id: String,
    price_native: f64,
    book: Book,
) -> TradeDecision {
    TradeDecision {
        position_id: position_id.map(|id| id.to_string()),
        mint: mint.to_owned(),
        action: TradeAction::Sell,
        reason: TradeReason::StrategyExit,
        strategy_id: Some(strategy_id),
        book,
        timestamp: Utc::now(),
        priority: TradePriority::Normal,
        price_native: Some(price_native),
        size_native: None,
        exit_percentage: None,
        // Auto-trader slippage always follows config.
        slippage_pct: None,
        take_profit_level: None,
        paper_leg_key: None,
    }
}

/// The market inputs of a strategy: the live pool's liquidity and the pool it was read
/// from, the trade size an entry would spend, and from the cached token snapshot its transaction counts, when that
/// snapshot was fetched and when the token was created on-chain.
///
/// Each value stays absent when its own source is: an uncached token has no counts and no
/// snapshot time, and a token without a valid on-chain creation time has none, whatever
/// other time the token carries.
pub fn strategy_market_data(
    token: Option<&Token>,
    liquidity_sol: Option<f64>,
    liquidity_pool: Option<LiquidityPool>,
    reference_size_native: Option<f64>,
) -> MarketData {
    MarketData {
        liquidity_sol,
        liquidity_pool,
        transactions: token.map(TransactionCounts::from_token),
        snapshot_at: token.map(|token| token.market_data_last_fetched_at),
        token_created_at: token.and_then(Token::on_chain_created_at),
        reference_size_native,
    }
}

/// The strategy view of an open position at `current_price`.
///
/// The peak is the basis peak (the highest pool price since the average entry last
/// changed), never the round maximum. The value still held is priced at the live pool
/// price only, and is `None` when that price, the held amount or the token's decimals
/// are unknown.
pub fn strategy_position_data(
    position: &Position,
    current_price: f64,
    pool_price: Option<f64>,
    decimals: Option<u8>,
    now: DateTime<Utc>,
) -> PositionData {
    let unrealized_profit_pct = (position.average_entry_price > 0.0).then(|| {
        ((current_price - position.average_entry_price) / position.average_entry_price) * 100.0
    });
    let pool_price = pool_price.filter(|price| price.is_finite() && *price > 0.0);
    let remaining_value_native = position
        .held_amount()
        .zip(decimals)
        .zip(pool_price)
        .map(|((held, decimals), price)| held.to_whole_units(decimals) * price)
        .filter(|value| value.is_finite());

    PositionData {
        entry_price: position.average_entry_price,
        entry_time: position.entry_time,
        current_size_sol: position.total_size_native,
        unrealized_profit_pct,
        position_age_hours: (now - position.entry_time).num_seconds() as f64 / 3600.0,
        peak_price: Some(position.basis_peak_price).filter(|peak| peak.is_finite() && *peak > 0.0),
        dca_count: position.dca_count,
        partial_exit_count: position.partial_exit_count,
        remaining_value_native,
        entry_liquidity_native: position.entry_liquidity_native,
        entry_liquidity_pool: position.entry_liquidity_pool.clone(),
    }
}

/// The candles a strategy reads for `mint`: the cached bundle, else one built now and
/// cached. `None` when neither is available; every candle condition then reads as
/// unknown, so the outage is logged here, where it is unambiguous.
pub async fn strategy_candles(chain: ChainId, mint: &str) -> Option<TimeframeBundle> {
    match crate::ohlcvs::get_timeframe_bundle(chain, mint).await {
        Ok(Some(bundle)) => {
            logger::debug(
                LogTag::Trader,
                &format!(
                    "OHLCV bundle available for {} (age: {}s)",
                    mint, bundle.cache_age_seconds
                ),
            );
            Some(bundle)
        }
        Ok(None) => {
            logger::debug(
                LogTag::Trader,
                &format!("OHLCV bundle cache miss for {mint} - building on demand"),
            );
            match crate::ohlcvs::build_timeframe_bundle(chain, mint).await {
                Ok(bundle) => {
                    let _ =
                        crate::ohlcvs::store_bundle(chain, mint.to_string(), bundle.clone()).await;
                    Some(bundle)
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Trader,
                        &format!(
                            "Failed to build OHLCV bundle for {mint}: {e} - evaluating without OHLCV"
                        ),
                    );
                    None
                }
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Trader,
                &format!("Failed to get OHLCV bundle for {mint}: {e} - evaluating without OHLCV"),
            );
            None
        }
    }
}

/// The context the trader would evaluate `strategy` in for `mint` now: the live pool price
/// and its liquidity, the token snapshot, the candles and, for an exit strategy, the
/// mint's open round on `book`. An entry strategy reads the configured trade size. A value
/// whose source is absent stays absent, and the rules that read it answer unknown.
pub async fn live_strategy_context(
    chain: ChainId,
    mint: &str,
    strategy: &Strategy,
    book: Book,
) -> EvaluationContext {
    let live_pool = crate::positions::price_resolution::live_pool_price(mint);
    let pool_price = live_pool.as_ref().map(|price| price.price_native);
    let now = Utc::now();
    let (position_data, reference_size_native) = match strategy.strategy_type {
        StrategyType::Entry => (None, Some(crate::trader::config::get_trade_size_native())),
        StrategyType::Exit => {
            let position = crate::positions::books::open_round(book, mint).await;
            let position_data = position.as_ref().and_then(|position| {
                pool_price.or(position.current_price).map(|current_price| {
                    strategy_position_data(
                        position,
                        current_price,
                        pool_price,
                        crate::tokens::get_cached_decimals(chain, mint),
                        now,
                    )
                })
            });
            (position_data, None)
        }
    };
    let market_data = strategy_market_data(
        crate::tokens::get_cached_token(chain, mint).as_ref(),
        live_pool
            .as_ref()
            .and_then(|price| usable_liquidity(price.native_reserves)),
        live_pool
            .as_ref()
            .and_then(|price| liquidity_pool(chain, price)),
        reference_size_native,
    );
    EvaluationContext {
        token_mint: mint.to_owned(),
        current_price: pool_price,
        position_data,
        market_data: Some(market_data),
        timeframe_bundle: strategy_candles(chain, mint).await,
        strategy_timeframe: strategy.timeframe.clone(),
        evaluated_at: now,
    }
}

/// Whether a strategy evaluation failed for lack of data (absent, too short, stale or
/// unusable) rather than because a strategy's configuration or rules are wrong.
fn is_data_unavailable(error: &crate::Error) -> bool {
    matches!(error, crate::Error::Strategies(error) if error.is_data_unavailable())
}

/// Log a strategy evaluation that failed as a whole: missing data at debug, since the next
/// cycle may have it, and any other error at warn, since it persists until the strategy or
/// its configuration changes.
fn log_strategy_error(subject: &str, error: &crate::Error) {
    let message = format!("Strategy evaluation error for {subject}: {error}");
    if is_data_unavailable(error) {
        logger::debug(LogTag::Trader, &message);
    } else {
        logger::warning(LogTag::Trader, &message);
    }
}

/// The outcome of each book's entry `pass`, live first whatever the order of `books`, all
/// under the one `deadline`. A pass still running at the deadline is abandoned, and a pass
/// whose turn comes after the deadline never starts; both read `None`. A pass that
/// finished keeps its outcome whatever a later pass does. The passes run one after the
/// other: they are CPU-bound on one task, so running them together gains nothing and loses
/// the fixed order.
async fn decide_books<T, F, Fut>(
    books: &[TradingMode],
    deadline: tokio::time::Instant,
    mut pass: F,
) -> Vec<(TradingMode, Option<T>)>
where
    F: FnMut(TradingMode) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let mut outcomes = Vec::with_capacity(books.len());
    for mode in [TradingMode::Live, TradingMode::Paper] {
        if !books.contains(&mode) {
            continue;
        }
        let outcome = if tokio::time::Instant::now() >= deadline {
            None
        } else {
            tokio::time::timeout_at(deadline, pass(mode)).await.ok()
        };
        outcomes.push((mode, outcome));
    }
    outcomes
}

/// Evaluator for applying strategies to trading decisions
pub struct StrategyEvaluator;

impl StrategyEvaluator {
    /// The entry decision of each book in `books` for a token, from the entry cycle's
    /// `entry_strategies`: each book runs its own pass over the strategies of its mode and
    /// stops at its own first signal, so a strategy of one book never takes a token from
    /// the other. The live pass runs first and both share one deadline of
    /// `STRATEGY_EVALUATION_TIMEOUT_SECS`, so a paper stall never drops a live signal.
    pub async fn check_entry_strategies(
        chain: ChainId,
        token_mint: &str,
        price_info: &PriceResult,
        books: &[TradingMode],
        entry_strategies: &[Strategy],
    ) -> crate::trader::Result<EntryDecisions> {
        // Early check: skip if no entry strategy of a requested book is configured
        if !entry_strategies
            .iter()
            .any(|strategy| books.contains(&strategy.mode))
        {
            return Ok(EntryDecisions::default());
        }

        // NOTE: Connectivity is already verified by the caller (entry.rs)
        // to avoid duplicate checks in the evaluation pipeline

        logger::info(
            LogTag::Trader,
            &format!(
                "Checking entry strategies for token {} (price={:.9} SOL, liquidity={:.2} SOL)",
                token_mint, price_info.price_native, price_info.native_reserves
            ),
        );

        // Build market data from price info, the token snapshot and the configured size
        let market_data = strategy_market_data(
            crate::tokens::get_cached_token(chain, token_mint).as_ref(),
            usable_liquidity(price_info.native_reserves),
            liquidity_pool(chain, price_info),
            Some(crate::trader::config::get_trade_size_native()),
        );

        let timeframe_bundle = strategy_candles(chain, token_mint).await;

        // One deadline shared by every book's pass, so the worst case stays one timeout
        let strategy_timeout = Duration::from_secs(STRATEGY_EVALUATION_TIMEOUT_SECS);
        let deadline = tokio::time::Instant::now() + strategy_timeout;
        let outcomes = decide_books(books, deadline, |mode| {
            strategies::evaluate_entry_strategies(
                entry_strategies,
                mode,
                token_mint,
                price_info.price_native,
                Some(&market_data),
                timeframe_bundle.as_ref(),
            )
        })
        .await;

        let mut decisions = EntryDecisions::default();
        let mut undecided = Vec::new();
        for (mode, outcome) in outcomes {
            match outcome {
                Some(Ok(Some(strategy_id))) => {
                    logger::info(
                        LogTag::Trader,
                        &format!(
                            "Entry strategy signal: token={}, strategy={}, mode={}, price={:.9} SOL",
                            token_mint,
                            strategy_id,
                            mode.as_str(),
                            price_info.price_native
                        ),
                    );
                    decisions.insert(entry_decision(
                        token_mint,
                        strategy_id,
                        price_info.price_native,
                        mode.book(),
                    ));
                }
                Some(Ok(None)) => {}
                Some(Err(e)) => {
                    // Don't fail trading on strategy errors
                    log_strategy_error(token_mint, &e);
                }
                None => undecided.push(mode.as_str()),
            }
        }

        // One stall is one record: a pass abandoned at the deadline and a pass that never
        // started because of it are named together.
        if !undecided.is_empty() {
            logger::warning(
                LogTag::Trader,
                &format!(
                    "Entry strategy evaluation for {} ran out of its {}s deadline; no entry signal was decided for the {} book(s)",
                    token_mint,
                    strategy_timeout.as_secs(),
                    undecided.join(", ")
                ),
            );

            // Record event for metrics tracking
            crate::events::record_trader_event(
                "strategy_evaluation_timeout",
                crate::events::Severity::Warn,
                Some(token_mint),
                None,
                serde_json::json!({
                  "timeout_seconds": strategy_timeout.as_secs(),
                  "evaluation_type": "entry",
                  "books": undecided,
                }),
            )
            .await;
        }
        Ok(decisions)
    }

    /// Check if a position should be exited based on the exit cycle's `exit_strategies`
    /// that judge the position's book.
    pub async fn check_exit_strategies(
        chain: ChainId,
        position: &Position,
        current_price: f64,
        exit_strategies: &[Strategy],
    ) -> crate::trader::Result<Option<TradeDecision>> {
        // Early check: skip if no exit strategy judges the position's book
        let modes = strategies::exit_strategy_modes(position.book);
        if !exit_strategies
            .iter()
            .any(|strategy| modes.contains(&strategy.mode))
        {
            return Ok(None);
        }

        // NOTE: Exit strategy evaluation is called after other exit checks (blacklist, risk, trailing, take profit, time)
        // Connectivity check here is defensive since exit coordinator doesn't pre-check
        if let Some(unhealthy) = crate::connectivity::check_endpoints_usable(&["rpc"]).await {
            logger::debug(
                LogTag::Trader,
                &format!(
                    "Skipping exit strategy evaluation for position {:?} - Unhealthy RPC: {}",
                    position.id, unhealthy
                ),
            );
            return Ok(None);
        }

        logger::info(
            LogTag::Trader,
            &format!(
                "Checking exit strategies for position {:?} token {} (current_price={:.9} SOL)",
                position.id, position.mint, current_price
            ),
        );

        // The live pool price result: the one source of the liquidity and of the price the
        // held tokens are valued at.
        let live_pool = crate::positions::price_resolution::live_pool_price(&position.mint);

        let position_data = strategy_position_data(
            position,
            current_price,
            live_pool.as_ref().map(|price| price.price_native),
            crate::tokens::get_cached_decimals(chain, &position.mint),
            Utc::now(),
        );

        // Build market data. Liquidity comes from the same single source the entry path
        // reads — the live pool price result. It used to be left `None` here, which made
        // the liquidity-drain exit (the documented exit use of LiquidityLevel) impossible:
        // the condition errored on every tick, the error aborted the whole strategy, and
        // the rule never fired once in the life of any position.
        let market_data = strategy_market_data(
            crate::tokens::get_cached_token(chain, &position.mint).as_ref(),
            live_pool
                .as_ref()
                .and_then(|price| usable_liquidity(price.native_reserves)),
            live_pool
                .as_ref()
                .and_then(|price| liquidity_pool(chain, price)),
            None,
        );

        let timeframe_bundle = strategy_candles(chain, &position.mint).await;

        // Call strategies module for evaluation with timeout
        let strategy_timeout = Duration::from_secs(STRATEGY_EVALUATION_TIMEOUT_SECS);
        let evaluation_result = tokio::time::timeout(
            strategy_timeout,
            strategies::evaluate_exit_strategies(
                exit_strategies,
                position.book,
                &position.mint,
                current_price,
                &position_data,
                Some(&market_data),
                timeframe_bundle.as_ref(),
                position.origin.entry_strategy_id(),
            ),
        )
        .await;

        match evaluation_result {
            Ok(Ok(Some(strategy_id))) => {
                logger::info(
                    LogTag::Trader,
                    &format!(
                        "Exit strategy signal: position={:?}, strategy={}, price={:.9} SOL",
                        position.id, strategy_id, current_price
                    ),
                );

                Ok(Some(exit_decision(
                    position.id,
                    &position.mint,
                    strategy_id,
                    current_price,
                    position.book,
                )))
            }
            Ok(Ok(None)) => Ok(None),
            Ok(Err(e)) => {
                log_strategy_error(&format!("position {:?}", position.id), &e);
                Ok(None) // Don't fail trading on strategy errors
            }
            Err(_timeout) => {
                logger::warning(
                    LogTag::Trader,
                    &format!(
                        "Exit strategy evaluation for position {:?} (mint={}) timed out after {}s; no exit signal was decided",
                        position.id,
                        position.mint,
                        strategy_timeout.as_secs()
                    ),
                );

                // Record event for metrics tracking
                crate::events::record_position_event_flexible(
                    "strategy_evaluation_timeout",
                    crate::events::Severity::Warn,
                    Some(&position.mint),
                    None,
                    serde_json::json!({
                      "timeout_seconds": strategy_timeout.as_secs(),
                      "evaluation_type": "exit",
                      "position_id": position.id,
                      "book": position.book.as_str(),
                    }),
                )
                .await;

                Ok(None) // Skip this position on timeout
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entry_signal_buys_as_a_strategy_signal() {
        let decision = entry_decision("mint", "entry-strategy".to_owned(), 0.5, Book::Paper);
        assert_eq!(decision.action, TradeAction::Buy);
        assert_eq!(decision.book, Book::Paper);
        assert_eq!(decision.reason, TradeReason::StrategySignal);
        assert_eq!(decision.strategy_id.as_deref(), Some("entry-strategy"));
    }

    #[test]
    fn an_exit_signal_sells_the_whole_position_as_a_strategy_exit() {
        // Recorded as the entry reason, a strategy exit was labelled and counted in the
        // exit breakdown as "Strategy Signal".
        let decision = exit_decision(Some(7), "mint", "exit-strategy".to_owned(), 0.5, Book::Live);
        assert_eq!(decision.book, Book::Live);
        assert_eq!(decision.action, TradeAction::Sell);
        assert_eq!(decision.reason, TradeReason::StrategyExit);
        assert_eq!(decision.position_id.as_deref(), Some("7"));
        assert_eq!(decision.exit_percentage, None);
        assert_eq!(decision.strategy_id.as_deref(), Some("exit-strategy"));
    }

    /// A paper pass that stalls is abandoned at the shared deadline and the live signal
    /// found before it is kept; the live pass runs first even when paper is listed first,
    /// and the whole evaluation never outlasts one timeout.
    #[tokio::test(start_paused = true)]
    async fn a_stalled_paper_pass_keeps_the_live_signal() {
        let start = tokio::time::Instant::now();
        let deadline = start + Duration::from_secs(STRATEGY_EVALUATION_TIMEOUT_SECS);
        let mut started = Vec::new();
        let outcomes = decide_books(&[TradingMode::Paper, TradingMode::Live], deadline, |mode| {
            started.push(mode);
            async move {
                if mode == TradingMode::Paper {
                    std::future::pending::<()>().await;
                }
                "live-signal"
            }
        })
        .await;
        assert_eq!(started, [TradingMode::Live, TradingMode::Paper]);
        assert_eq!(
            outcomes,
            [
                (TradingMode::Live, Some("live-signal")),
                (TradingMode::Paper, None)
            ]
        );
        assert_eq!(
            tokio::time::Instant::now() - start,
            Duration::from_secs(STRATEGY_EVALUATION_TIMEOUT_SECS)
        );
    }

    /// A live pass that runs out the shared deadline leaves the paper pass no time: the
    /// paper pass never starts and the evaluation still ends at the one deadline.
    #[tokio::test(start_paused = true)]
    async fn a_stalled_live_pass_leaves_the_paper_pass_no_time() {
        let start = tokio::time::Instant::now();
        let deadline = start + Duration::from_secs(STRATEGY_EVALUATION_TIMEOUT_SECS);
        let mut started = Vec::new();
        let outcomes = decide_books(&[TradingMode::Live, TradingMode::Paper], deadline, |mode| {
            started.push(mode);
            async move {
                if mode == TradingMode::Live {
                    std::future::pending::<()>().await;
                }
                "paper-signal"
            }
        })
        .await;
        assert_eq!(started, [TradingMode::Live], "the paper pass started late");
        assert_eq!(
            outcomes,
            [(TradingMode::Live, None), (TradingMode::Paper, None)]
        );
        assert_eq!(
            tokio::time::Instant::now() - start,
            Duration::from_secs(STRATEGY_EVALUATION_TIMEOUT_SECS)
        );
    }

    /// A book that was not requested runs no pass.
    #[tokio::test(start_paused = true)]
    async fn only_the_requested_books_run_a_pass() {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        let outcomes =
            decide_books(&[TradingMode::Paper], deadline, |mode| async move { mode }).await;
        assert_eq!(outcomes, [(TradingMode::Paper, Some(TradingMode::Paper))]);
        let outcomes = decide_books(&[], deadline, |mode| async move { mode }).await;
        assert!(outcomes.is_empty());
    }

    /// A strategy error that says data is missing is told apart from one that says the
    /// strategy or its configuration is wrong, and from an error outside the strategies.
    #[test]
    fn a_strategy_error_is_data_unavailable_only_when_the_strategy_lacked_data() {
        let missing_data = crate::Error::from(strategies::Error::InsufficientCandles {
            indicator: "rsi",
            available: 3,
            required: 14,
        });
        let stale_data = crate::Error::from(strategies::Error::StaleCandleData {
            timeframe: "5m".to_owned(),
            age_seconds: 900,
            max_age_seconds: 600,
        });
        let broken_rule = crate::Error::from(strategies::Error::InvalidRuleTree {
            reason: "empty group",
        });
        let missing_parameter =
            crate::Error::from(strategies::Error::MissingConditionParameter { field: "period" });
        let engine_down = crate::Error::internal_error("Strategy engine not available");

        assert!(is_data_unavailable(&missing_data));
        assert!(is_data_unavailable(&stale_data));
        assert!(!is_data_unavailable(&broken_rule));
        assert!(!is_data_unavailable(&missing_parameter));
        assert!(!is_data_unavailable(&engine_down));
    }
}
