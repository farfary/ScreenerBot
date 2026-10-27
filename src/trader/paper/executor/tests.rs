// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper executor on an isolated paper store with offline quotes: a round trip from
//! the entry through a take-profit step to the close, every quote taken inside the paper
//! scope, the typed refusals that book nothing, a keyed decision booked once however often
//! it is replayed, and the failed-exit streak.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use chrono::Utc;

use super::io::{PaperMarket, PaperSettings, PaperToken};
use super::*;
use crate::chains::ChainId;
use crate::config::schemas::Config;
use crate::config::utils::CONFIG;
use crate::positions::db::PaperExitOwner;
use crate::positions::{PositionManagement, PositionsDatabase};
use crate::swaps::{Quote, QuoteError};
use crate::trader::paper::PaperFillError;
use crate::trader::types::TradePriority;

const MINT: &str = "PaperExecutorMint11111111111111111111111111";
const WALLET: &str = "PaperExecutorWa11et111111111111111111111111";
const DECIMALS: u8 = 6;

struct FakeIo {
    book: PaperBook,
    starting_balance_native: f64,
    wanted: AtomicBool,
    abort_at_wait: AtomicBool,
    quotes: AtomicUsize,
    quotes_outside_scope: AtomicUsize,
    waits: AtomicUsize,
    max_open: AtomicUsize,
    /// 0 answers every quote, 1 refuses every quote, 2 answers without a network fee.
    quote_mode: AtomicUsize,
}

impl FakeIo {
    async fn new(starting_balance_native: f64) -> (Self, tempfile::TempDir) {
        let _ = CONFIG.get_or_init(|| std::sync::RwLock::new(Config::default()));
        let directory = tempfile::tempdir().unwrap();
        let db = PositionsDatabase::open_at(
            Book::Paper,
            directory.path().join("paper_positions.db"),
            ChainId::Solana,
        )
        .await
        .unwrap();
        let io = Self {
            book: PaperBook::new(db, Vec::new()),
            starting_balance_native,
            wanted: AtomicBool::new(true),
            abort_at_wait: AtomicBool::new(false),
            quotes: AtomicUsize::new(0),
            quotes_outside_scope: AtomicUsize::new(0),
            waits: AtomicUsize::new(0),
            max_open: AtomicUsize::new(5),
            quote_mode: AtomicUsize::new(0),
        };
        (io, directory)
    }
}

impl PaperTradeIo for FakeIo {
    fn book(&self) -> Option<&PaperBook> {
        Some(&self.book)
    }

    fn wallet_address(&self) -> Option<String> {
        Some(WALLET.to_owned())
    }

    fn settings(&self) -> PaperSettings {
        PaperSettings {
            landing_delay_ms: 0,
            starting_balance_native: self.starting_balance_native,
            max_open: self.max_open.load(Ordering::SeqCst),
            open_cooldown_secs: 0,
        }
    }

    fn entry_size(&self, decision: &TradeDecision) -> f64 {
        decision.size_native.unwrap_or(0.1)
    }

    async fn still_wanted(&self, _: &TradeDecision, _: &PositionOrigin) -> bool {
        self.wanted.load(Ordering::SeqCst)
    }

    async fn token(&self, _: &str) -> Option<PaperToken> {
        Some(PaperToken {
            symbol: "PAPER".to_owned(),
            name: "Paper".to_owned(),
            decimals: Some(DECIMALS),
        })
    }

    async fn market_price(&self, _: &str) -> Option<PaperMarket> {
        Some(PaperMarket {
            price_native: 0.0001,
            liquidity: Some((50.0, "PaperPool".to_owned())),
        })
    }

    /// A buy returns ten raw token units per lamport (0.0001 SOL a token); a sale returns
    /// 0.12 lamports per raw unit (0.00012 SOL a token). The minimum is 90 percent.
    async fn quote(&self, request: QuoteRequest) -> std::result::Result<Quote, QuoteError> {
        self.quotes.fetch_add(1, Ordering::SeqCst);
        if !crate::global::in_paper_scope() {
            self.quotes_outside_scope.fetch_add(1, Ordering::SeqCst);
        }
        let mode = self.quote_mode.load(Ordering::SeqCst);
        if mode == 1 {
            return Err(QuoteError::NoRoute {
                router: "fixture".to_owned(),
                detail: "offline fixture refuses".to_owned(),
            });
        }
        let buying = crate::chains::adapter_for(request.chain).is_native_asset(&request.input_mint);
        let input = request.input_amount.raw();
        let output = if buying { input * 10 } else { input * 12 / 100 };
        Ok(Quote {
            chain: request.chain,
            router_id: "fixture".to_owned(),
            router_name: "offline fixture".to_owned(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: request.input_amount,
            output_amount: RawAmount::new(output),
            minimum_output_amount: RawAmount::new(output * 9 / 10),
            price_impact_pct: 0.0,
            platform_fee_lamports: None,
            estimated_network_fee_lamports: (mode != 2).then_some(5_000),
            slippage_bps: 100,
            route_plan: "offline fixture".to_owned(),
            swap_mode: request.swap_mode,
            wallet_address: request.wallet_address.clone(),
            exclude_dexes: request.exclude_dexes.clone(),
            execution_data: Vec::new(),
        })
    }

    async fn wait(&self, duration: Duration) {
        assert_eq!(duration, Duration::ZERO);
        self.waits.fetch_add(1, Ordering::SeqCst);
        if self.abort_at_wait.load(Ordering::SeqCst) {
            self.wanted.store(false, Ordering::SeqCst);
        }
    }
}

fn decision(action: TradeAction, reason: TradeReason) -> TradeDecision {
    TradeDecision {
        position_id: None,
        mint: MINT.to_owned(),
        action,
        reason,
        strategy_id: Some("paper-entry".to_owned()),
        book: Book::Paper,
        timestamp: Utc::now(),
        priority: TradePriority::Normal,
        price_native: Some(0.0001),
        size_native: Some(0.1),
        exit_percentage: None,
        slippage_pct: None,
        take_profit_level: None,
        paper_leg_key: None,
    }
}

fn context(decision: &TradeDecision) -> ExecutionContext {
    ExecutionContext::for_decision(decision)
}

#[tokio::test]
async fn a_paper_round_trip_books_entry_take_profit_and_close_without_a_signature() {
    let (io, _directory) = FakeIo::new(1.0).await;

    let entry = decision(TradeAction::Buy, TradeReason::StrategySignal);
    let bought = execute_with(&entry, &context(&entry), &io).await.unwrap();
    assert!(bought.success, "{:?}", bought.error);
    assert_eq!(bought.tx_signature, None);
    let position_id: i64 = bought.position_id.as_deref().unwrap().parse().unwrap();
    let opened = io.book.position(position_id).await.unwrap();
    assert_eq!(opened.book, Book::Paper);
    assert_eq!(opened.entry_transaction_signature, None);
    assert_eq!(opened.held_amount(), Some(RawAmount::new(1_000_000_000)));
    assert_eq!(
        opened.origin,
        PositionOrigin::Auto {
            strategy_id: Some("paper-entry".to_owned())
        }
    );
    assert_eq!(opened.management, PositionManagement::AutoTrader);

    let mut take_profit = decision(TradeAction::Sell, TradeReason::TakeProfit);
    take_profit.exit_percentage = Some(50.0);
    take_profit.take_profit_level = Some(1);
    let partial = execute_with(&take_profit, &context(&take_profit), &io)
        .await
        .unwrap();
    assert!(partial.success, "{:?}", partial.error);
    let after_partial = io.book.position(position_id).await.unwrap();
    assert_eq!(after_partial.take_profit_level, 1);
    assert_eq!(
        after_partial.held_amount(),
        Some(RawAmount::new(500_000_000))
    );

    let repeated = execute_with(&take_profit, &context(&take_profit), &io)
        .await
        .unwrap();
    assert!(!repeated.success, "a take-profit step sold twice");
    assert_eq!(
        io.book.position(position_id).await.unwrap().held_amount(),
        Some(RawAmount::new(500_000_000))
    );

    let stop = decision(TradeAction::Sell, TradeReason::StopLoss);
    let closed = execute_with(&stop, &context(&stop), &io).await.unwrap();
    assert!(closed.success, "{:?}", closed.error);
    assert!(io.book.open_positions().await.is_empty());
    let stored = io.book.stored(position_id).await.unwrap().unwrap();
    assert!(stored.exit_time.is_some());
    assert_eq!(stored.closed_reason.as_deref(), Some("StopLoss"));

    let account = io.book.account(WALLET, 1.0).await.unwrap();
    assert!(
        account.cash_native > 1.0,
        "a round sold above its entry leaves more cash than it started with: {}",
        account.cash_native
    );
    assert_eq!(io.quotes_outside_scope.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_live_decision_is_never_filled_on_paper() {
    let (io, _directory) = FakeIo::new(1.0).await;
    let mut live = decision(TradeAction::Buy, TradeReason::StrategySignal);
    live.book = Book::Live;
    match execute_with(&live, &context(&live), &io).await {
        Err(Error::PaperDecisionOnLivePath { mint }) => assert_eq!(mint, MINT),
        other => panic!("a live decision reached the paper book: {other:?}"),
    }
    assert_eq!(io.quotes.load(Ordering::SeqCst), 0);
    assert!(io.book.open_positions().await.is_empty());
}

#[tokio::test]
async fn a_buy_the_paper_cash_cannot_cover_takes_no_quote_and_books_nothing() {
    let (io, _directory) = FakeIo::new(0.05).await;
    let entry = decision(TradeAction::Buy, TradeReason::StrategySignal);
    let result = execute_with(&entry, &context(&entry), &io).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.failed_step, Some(TradeStep::Validation));
    assert_eq!(refusal(&result), Some(PaperRefusal::BalanceExhausted));
    assert_eq!(io.quotes.load(Ordering::SeqCst), 0);
    assert!(io.book.open_positions().await.is_empty());
}

#[tokio::test]
async fn a_buy_no_longer_wanted_after_the_landing_wait_books_nothing() {
    let (io, _directory) = FakeIo::new(1.0).await;
    io.abort_at_wait.store(true, Ordering::SeqCst);
    let entry = decision(TradeAction::Buy, TradeReason::StrategySignal);
    let result = execute_with(&entry, &context(&entry), &io).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.failed_step, Some(TradeStep::Validation));
    assert_eq!(refusal(&result), Some(PaperRefusal::NotWanted));
    assert!(io.book.open_positions().await.is_empty());
}

#[tokio::test]
async fn an_add_without_a_size_is_refused_before_anything_runs() {
    let (io, _directory) = FakeIo::new(1.0).await;
    let mut add = decision(TradeAction::DCA, TradeReason::DCAScheduled);
    add.size_native = None;
    assert!(matches!(
        execute_with(&add, &context(&add), &io).await,
        Err(Error::AddSizeRequired { .. })
    ));
    assert_eq!(io.quotes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_failed_exit_streak_is_kept_in_the_paper_store_per_strategy_and_a_fill_ends_it() {
    let (io, _directory) = FakeIo::new(1.0).await;
    let entry = decision(TradeAction::Buy, TradeReason::StrategySignal);
    let bought = execute_with(&entry, &context(&entry), &io).await.unwrap();
    assert!(bought.success, "{:?}", bought.error);
    let position_id: i64 = bought.position_id.as_deref().unwrap().parse().unwrap();
    let position = io.book.position(position_id).await.unwrap();
    let entry_strategy = "paper-entry";
    let exit_strategy = "streak-exit-strategy";
    let mut sale = decision(TradeAction::Sell, TradeReason::StrategyExit);
    sale.strategy_id = Some(exit_strategy.to_owned());

    let record =
        |filled| record_exit_outcome(&io.book, WALLET, &position, position_id, &sale, filled);
    assert_eq!(record(false).await.unwrap(), 1);
    assert_eq!(record(false).await.unwrap(), 2);
    for strategy_id in [entry_strategy, exit_strategy] {
        assert_eq!(
            io.book
                .failed_exit_streak(WALLET, &strategy(strategy_id))
                .await
                .unwrap(),
            2,
            "{strategy_id}"
        );
    }
    assert_eq!(
        io.book
            .failed_exit_streak(WALLET, &strategy("another-strategy"))
            .await
            .unwrap(),
        0
    );

    assert_eq!(record(true).await.unwrap(), 0);
    for strategy_id in [entry_strategy, exit_strategy] {
        assert_eq!(
            io.book
                .failed_exit_streak(WALLET, &strategy(strategy_id))
                .await
                .unwrap(),
            0,
            "{strategy_id}"
        );
    }
    assert_eq!(record(false).await.unwrap(), 1, "a fill ends the run");
}

fn strategy(strategy_id: &str) -> PaperExitOwner {
    PaperExitOwner::Strategy(strategy_id.to_owned())
}

fn refusal(result: &TradeResult) -> Option<PaperRefusal> {
    result.paper.as_ref().and_then(PaperExecution::refusal)
}

fn evidence(result: &TradeResult) -> PaperLegEvidence {
    result
        .paper
        .as_ref()
        .and_then(PaperExecution::filled)
        .cloned()
        .unwrap_or_else(|| panic!("no paper leg booked: {:?}", result.paper))
}

#[tokio::test]
async fn a_success_carries_the_leg_its_record_is_booked_under() {
    let (io, _directory) = FakeIo::new(1.0).await;
    let entry = decision(TradeAction::Buy, TradeReason::StrategySignal);
    let bought = execute_with(&entry, &context(&entry), &io).await.unwrap();
    let booked = evidence(&bought);
    assert!(!booked.replayed);
    assert!(matches!(booked.fill, PaperLegFill::Buy(_)));
    let mut stop = decision(TradeAction::Sell, TradeReason::StopLoss);
    stop.position_id = Some(booked.position_id.to_string());
    let sold = execute_with(&stop, &context(&stop), &io).await.unwrap();
    let sale = evidence(&sold);
    assert!(matches!(sale.fill, PaperLegFill::Sell(_)));

    let history = io
        .book
        .round_history(WALLET, booked.position_id)
        .await
        .unwrap();
    assert_eq!(history.entries[0].transaction_signature, booked.leg_ref);
    assert_eq!(history.exits[0].transaction_signature, sale.leg_ref);
}

#[tokio::test]
async fn a_replayed_keyed_decision_returns_the_stored_leg_without_a_quote_or_a_wait() {
    let (io, _directory) = FakeIo::new(1.0).await;
    let mut entry = decision(TradeAction::Buy, TradeReason::CopyBuy);
    entry.paper_leg_key = Some("copy-1-buy-signature".to_owned());
    let first = execute_with(&entry, &context(&entry), &io).await.unwrap();
    let booked = evidence(&first);
    let quotes = io.quotes.load(Ordering::SeqCst);
    let waits = io.waits.load(Ordering::SeqCst);

    let replay = execute_with(&entry, &context(&entry), &io).await.unwrap();
    assert!(replay.success);
    let replayed = evidence(&replay);
    assert!(replayed.replayed);
    assert_eq!(replayed.leg_ref, booked.leg_ref);
    assert_eq!(replayed.position_id, booked.position_id);
    assert_eq!(replayed.fill, booked.fill);
    assert_eq!(io.quotes.load(Ordering::SeqCst), quotes);
    assert_eq!(io.waits.load(Ordering::SeqCst), waits);
    assert_eq!(io.book.open_positions().await.len(), 1);

    let mut sale = decision(TradeAction::Sell, TradeReason::CopySell);
    sale.exit_percentage = Some(50.0);
    sale.paper_leg_key = Some("copy-1-sell-signature".to_owned());
    let sold = execute_with(&sale, &context(&sale), &io).await.unwrap();
    let sold_leg = evidence(&sold);
    let quotes = io.quotes.load(Ordering::SeqCst);
    let again = execute_with(&sale, &context(&sale), &io).await.unwrap();
    let again_leg = evidence(&again);
    assert!(again_leg.replayed);
    assert_eq!(again_leg.leg_ref, sold_leg.leg_ref);
    assert_eq!(io.quotes.load(Ordering::SeqCst), quotes);
    assert_eq!(
        io.book
            .position(booked.position_id)
            .await
            .unwrap()
            .partial_exit_count,
        1
    );
}

#[tokio::test]
async fn a_buy_refused_by_the_book_carries_its_typed_refusal() {
    let (io, _directory) = FakeIo::new(5.0).await;
    let entry = decision(TradeAction::Buy, TradeReason::StrategySignal);
    let first = execute_with(&entry, &context(&entry), &io).await.unwrap();
    assert!(first.success, "{:?}", first.error);
    let held = execute_with(&entry, &context(&entry), &io).await.unwrap();
    assert_eq!(refusal(&held), Some(PaperRefusal::AlreadyOpen));

    io.max_open.store(1, Ordering::SeqCst);
    let mut other = decision(TradeAction::Buy, TradeReason::StrategySignal);
    other.mint = "PaperExecutorOtherMint111111111111111111111".to_owned();
    let full = execute_with(&other, &context(&other), &io).await.unwrap();
    assert_eq!(refusal(&full), Some(PaperRefusal::PositionLimit));
    assert_eq!(full.capacity_guard_remaining, Some(0));
    assert_eq!(io.book.open_positions().await.len(), 1);
}

#[tokio::test]
async fn a_fill_that_does_not_settle_carries_its_fill_error() {
    for (mode, expected) in [
        (1, PaperFillError::QuoteUnavailable),
        (2, PaperFillError::QuoteUnfillable),
    ] {
        let (io, _directory) = FakeIo::new(1.0).await;
        io.quote_mode.store(mode, Ordering::SeqCst);
        let entry = decision(TradeAction::Buy, TradeReason::StrategySignal);
        let result = execute_with(&entry, &context(&entry), &io).await.unwrap();
        assert_eq!(
            refusal(&result),
            Some(PaperRefusal::Fill(expected)),
            "{mode}"
        );
        assert!(io.book.open_positions().await.is_empty());
    }
}

#[tokio::test]
async fn a_buy_of_no_size_and_a_sale_of_no_round_are_refused_by_type() {
    let (io, _directory) = FakeIo::new(1.0).await;
    let mut empty = decision(TradeAction::Buy, TradeReason::StrategySignal);
    empty.size_native = Some(0.0);
    let result = execute_with(&empty, &context(&empty), &io).await.unwrap();
    assert_eq!(refusal(&result), Some(PaperRefusal::InvalidSize));

    let sale = decision(TradeAction::Sell, TradeReason::StopLoss);
    let result = execute_with(&sale, &context(&sale), &io).await.unwrap();
    assert_eq!(refusal(&result), Some(PaperRefusal::NoOpenRound));

    io.wanted.store(false, Ordering::SeqCst);
    let entry = decision(TradeAction::Buy, TradeReason::StrategySignal);
    let result = execute_with(&entry, &context(&entry), &io).await.unwrap();
    assert_eq!(refusal(&result), Some(PaperRefusal::NotWanted));
    assert_eq!(io.quotes.load(Ordering::SeqCst), 0);
}
