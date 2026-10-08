// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV candlestick data — fetching, caching, and gap filling for price charts.
mod aggregator;
mod cache;
mod database;
mod feeds;
mod fetcher;
mod gaps;
mod manager;
mod monitor;
pub mod native_usd_chart;
mod priorities;
mod service;
mod service_api;
mod types;

pub use types::{
    Candle, MonitorStats, MonitorTelemetrySnapshot, OhlcvError, OhlcvMetrics, OhlcvResult,
    OhlcvStatus, OhlcvTimeframeStatus, PoolConfig, PoolMetadata, Priority, Timeframe,
    TimeframeBundle, TokenOhlcvConfig, BUNDLE_CANDLE_COUNT,
};

pub use database::{ClearAllResult, DatabaseStats, DeleteResult, OhlcvTokenStatus};
pub use feeds::{CandleFeed, CandleFeedFn};
pub use priorities::ActivityType;
pub use service::OhlcvService;
pub use service_api::{
    add_token_monitoring, build_timeframe_bundle, clear_all_ohlcv_data, delete_inactive_tokens,
    delete_token_data, get_all_tokens_with_status, get_available_pools, get_data_gaps,
    get_database_stats, get_metrics, get_mints_with_data, get_monitor_stats, get_ohlcv_data,
    get_status, get_timeframe_bundle, has_data, record_activity, remove_token_monitoring,
    request_refresh, store_bundle, update_token_priority,
};
