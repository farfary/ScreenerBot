//! Token assembly helpers — free functions for constructing Token structs from components.

use crate::errors::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::{types::FromSql, Row};

use crate::chains::ChainId;
use crate::tokens::types::{
    DataSource, DexScreenerData, GeckoTerminalData, Priority, RugcheckData, Token, TokenMetadata,
    TokenResult,
};
use crate::tokens::Error;

pub(super) enum MarketDataType {
    DexScreener(DexScreenerData),
    GeckoTerminal(GeckoTerminalData),
}

pub(super) fn assemble_token(
    chain: ChainId,
    metadata: TokenMetadata,
    market_data: MarketDataType,
    data_source: DataSource,
    security: Option<RugcheckData>,
    is_blacklisted: bool,
    priority: Priority,
    fallback_image_url: Option<String>,
    fallback_header_url: Option<String>,
) -> Token {
    // Extract timestamps from metadata
    let first_discovered_dt =
        DateTime::from_timestamp(metadata.first_discovered_at, 0).unwrap_or_else(|| Utc::now());
    let metadata_last_fetched_dt = DateTime::from_timestamp(metadata.metadata_last_fetched_at, 0)
        .unwrap_or_else(|| Utc::now());

    // Capture primary source images (DexScreener provides them) without moving market_data
    let (primary_image_url, primary_header_url) = match &market_data {
        MarketDataType::DexScreener(data) => {
            (data.image_url.clone(), data.header_image_url.clone())
        }
        MarketDataType::GeckoTerminal(data) => (data.image_url.clone(), None),
    };

    // Extract market data fields based on source
    let (
        price_usd,
        price_sol,
        price_native,
        price_changes,
        market_metrics,
        volumes,
        txns,
        market_data_last_fetched_at,
        pair_blockchain_created_at,
        pool_metrics,
        _market_data_first_fetched_at,
    ) = match market_data {
        MarketDataType::DexScreener(data) => {
            let txns = (
                data.txns_5m.map(|(b, s)| (b as i64, s as i64)),
                data.txns_1h.map(|(b, s)| (b as i64, s as i64)),
                data.txns_6h.map(|(b, s)| (b as i64, s as i64)),
                data.txns_24h.map(|(b, s)| (b as i64, s as i64)),
            );

            (
                data.price_usd,
                data.price_sol,
                data.price_native,
                (
                    data.price_change_5m,
                    data.price_change_1h,
                    data.price_change_6h,
                    data.price_change_24h,
                ),
                (data.market_cap, data.fdv, data.liquidity_usd),
                (
                    data.volume_5m,
                    data.volume_1h,
                    data.volume_6h,
                    data.volume_24h,
                ),
                txns,
                data.market_data_last_fetched_at,
                data.pair_blockchain_created_at,
                (None, None),
                data.market_data_first_fetched_at,
            )
        }
        MarketDataType::GeckoTerminal(data) => {
            let txns = (None, None, None, None); // GeckoTerminal doesn't provide txn data

            (
                data.price_usd,
                data.price_sol,
                data.price_native,
                (
                    data.price_change_5m,
                    data.price_change_1h,
                    data.price_change_6h,
                    data.price_change_24h,
                ),
                (data.market_cap, data.fdv, data.liquidity_usd),
                (
                    data.volume_5m,
                    data.volume_1h,
                    data.volume_6h,
                    data.volume_24h,
                ),
                txns,
                data.market_data_last_fetched_at,
                None,
                (data.pool_count, data.reserve_in_usd),
                data.market_data_first_fetched_at,
            )
        }
    };

    // Extract security data
    let security_ref = security.as_ref();

    let token_type = security_ref.and_then(|sec| sec.token_type.clone());

    // Authority data: primary from Rugcheck, fallback from SPL Mint authority cache
    let (mint_authority, freeze_authority) = {
        let rc_mint = security_ref.and_then(|sec| sec.mint_authority.clone());
        let rc_freeze = security_ref.and_then(|sec| sec.freeze_authority.clone());
        if rc_mint.is_some() || rc_freeze.is_some() {
            (rc_mint, rc_freeze)
        } else if let Some(cached) =
            crate::tokens::authority_cache::get_cached(chain, &metadata.mint)
        {
            (cached.mint_authority, cached.freeze_authority)
        } else {
            (None, None)
        }
    };
    let update_authority = security_ref.and_then(|sec| sec.update_authority.clone());
    let is_mutable = security_ref.and_then(|sec| sec.is_mutable);
    let security_score = security_ref.and_then(|sec| sec.score);
    let security_score_normalised = security_ref.and_then(|sec| sec.score_normalised);
    let is_rugged = security_ref.is_some_and(|sec| sec.rugged);
    let security_risks = security_ref
        .map(|sec| sec.risks.clone())
        .unwrap_or_default();
    let top_holders = security_ref
        .map(|sec| sec.top_holders.clone())
        .unwrap_or_default();
    let total_holders = security_ref.and_then(|sec| sec.total_holders);
    let top_10_holders_pct = security_ref.and_then(|sec| sec.top_10_holders_pct);
    let creator_balance_pct = security_ref.and_then(|sec| sec.creator_balance_pct);
    let transfer_fee_pct = security_ref.and_then(|sec| sec.transfer_fee_pct);
    let transfer_fee_max_amount = security_ref.and_then(|sec| sec.transfer_fee_max_amount);
    let transfer_fee_authority = security_ref.and_then(|sec| sec.transfer_fee_authority.clone());
    let graph_insiders_detected = security_ref.and_then(|sec| sec.graph_insiders_detected);
    let lp_provider_count = security_ref.and_then(|sec| sec.total_lp_providers);

    // Stays `None` when neither source knows: `Token::decimals` means "the decimals we
    // have", and inventing 9 here made every consumer — including the filtering hot path —
    // unable to tell a real 9-decimal token from one we have never resolved.
    let resolved_decimals = metadata
        .decimals
        .or_else(|| security_ref.and_then(|data| data.token_decimals));

    // For now, only use primary source-provided images. Fallbacks can be added upstream where DB is available.
    let resolved_image_url = primary_image_url.or(fallback_image_url);
    let resolved_header_url = primary_header_url.or(fallback_header_url);

    // Security data timestamp (if available)
    let security_data_last_fetched_dt = security_ref.map(|sec| sec.security_data_last_fetched_at);

    Token {
        // Core identity
        mint: metadata.mint.clone(),
        symbol: metadata.symbol.unwrap_or_else(|| "UNKNOWN".to_owned()),
        name: metadata.name.unwrap_or_else(|| "Unknown Token".to_owned()),
        decimals: resolved_decimals,
        description: None,
        image_url: resolved_image_url,
        header_image_url: resolved_header_url,
        supply: None,

        // Data source
        data_source,

        // Discovery & Creation timestamps
        first_discovered_at: first_discovered_dt,
        blockchain_created_at: pair_blockchain_created_at,

        // Metadata timestamps
        metadata_last_fetched_at: metadata_last_fetched_dt,
        decimals_last_fetched_at: metadata_last_fetched_dt, // Same as metadata for now

        // Market data timestamps
        market_data_last_fetched_at: market_data_last_fetched_at,

        // Security data timestamp
        security_data_last_fetched_at: security_data_last_fetched_dt,

        // Pool price timestamps (defaults - will be updated by pool service)
        pool_price_last_calculated_at: market_data_last_fetched_at, // Fallback to market fetch
        pool_price_last_used_pool: None,

        // Price information
        price_usd,
        price_sol,
        price_native,
        price_change_m5: price_changes.0,
        price_change_h1: price_changes.1,
        price_change_h6: price_changes.2,
        price_change_h24: price_changes.3,

        // Market metrics
        market_cap: market_metrics.0,
        fdv: market_metrics.1,
        liquidity_usd: market_metrics.2,

        // Volume data
        volume_m5: volumes.0,
        volume_h1: volumes.1,
        volume_h6: volumes.2,
        volume_h24: volumes.3,

        // Pool metrics
        pool_count: pool_metrics.0,
        reserve_in_usd: pool_metrics.1,

        // Transaction activity
        txns_m5_buys: txns.0.map(|(b, _)| b),
        txns_m5_sells: txns.0.map(|(_, s)| s),
        txns_h1_buys: txns.1.map(|(b, _)| b),
        txns_h1_sells: txns.1.map(|(_, s)| s),
        txns_h6_buys: txns.2.map(|(b, _)| b),
        txns_h6_sells: txns.2.map(|(_, s)| s),
        txns_h24_buys: txns.3.map(|(b, _)| b),
        txns_h24_sells: txns.3.map(|(_, s)| s),

        // Social & links (not available from current APIs)
        websites: vec![],
        socials: vec![],

        // Security information
        mint_authority,
        freeze_authority,
        update_authority,
        is_mutable,
        security_score,
        security_score_normalised,
        is_rugged,
        token_type,
        graph_insiders_detected,
        lp_provider_count,
        security_risks,
        total_holders,
        top_10_holders_pct,
        top_holders,
        creator_balance_pct,
        transfer_fee_pct,
        transfer_fee_max_amount,
        transfer_fee_authority,

        // Bot-specific state
        is_blacklisted,
        priority,

        // Filtering State
        last_rejection_reason: None,
        last_rejection_source: None,
        last_rejection_at: None,
    }
}

pub(super) fn read_row_value<T: FromSql>(
    row: &Row<'_>,
    index: usize,
    field: &str,
) -> TokenResult<T> {
    row.get(index).map_err(|e| {
        Error::Database(DatabaseError::Query {
            operation: "Failed to read {field}".to_owned(),
            message: e.to_string(),
        })
    })
}

pub(super) fn assemble_token_without_market_data(
    chain: ChainId,
    metadata: TokenMetadata,
    security: Option<RugcheckData>,
    is_blacklisted: bool,
    priority: Priority,
    metadata_updated_at_override: Option<DateTime<Utc>>,
    last_market_update: Option<DateTime<Utc>>,
    blockchain_created_at_override: Option<DateTime<Utc>>,
    last_rejection_reason: Option<String>,
    last_rejection_source: Option<String>,
    last_rejection_at: Option<DateTime<Utc>>,
) -> Token {
    // Extract security data
    let security_ref = security.as_ref();

    let token_type = security_ref.and_then(|sec| sec.token_type.clone());

    // Authority data: primary from Rugcheck, fallback from SPL Mint authority cache
    let (mint_authority, freeze_authority) = {
        let rc_mint = security_ref.and_then(|sec| sec.mint_authority.clone());
        let rc_freeze = security_ref.and_then(|sec| sec.freeze_authority.clone());
        if rc_mint.is_some() || rc_freeze.is_some() {
            (rc_mint, rc_freeze)
        } else if let Some(cached) =
            crate::tokens::authority_cache::get_cached(chain, &metadata.mint)
        {
            (cached.mint_authority, cached.freeze_authority)
        } else {
            (None, None)
        }
    };
    let update_authority = security_ref.and_then(|sec| sec.update_authority.clone());
    let is_mutable = security_ref.and_then(|sec| sec.is_mutable);
    let security_score = security_ref.and_then(|sec| sec.score);
    let security_score_normalised = security_ref.and_then(|sec| sec.score_normalised);
    let is_rugged = security_ref.is_some_and(|sec| sec.rugged);
    let security_risks = security_ref
        .map(|sec| sec.risks.clone())
        .unwrap_or_default();
    let top_holders = security_ref
        .map(|sec| sec.top_holders.clone())
        .unwrap_or_default();
    let total_holders = security_ref.and_then(|sec| sec.total_holders);
    let top_10_holders_pct = security_ref.and_then(|sec| sec.top_10_holders_pct);
    let creator_balance_pct = security_ref.and_then(|sec| sec.creator_balance_pct);
    let transfer_fee_pct = security_ref.and_then(|sec| sec.transfer_fee_pct);
    let transfer_fee_max_amount = security_ref.and_then(|sec| sec.transfer_fee_max_amount);
    let transfer_fee_authority = security_ref.and_then(|sec| sec.transfer_fee_authority.clone());
    let graph_insiders_detected = security_ref.and_then(|sec| sec.graph_insiders_detected);
    let lp_provider_count = security_ref.and_then(|sec| sec.total_lp_providers);

    // Parse timestamps from metadata
    let first_discovered_dt =
        DateTime::from_timestamp(metadata.first_discovered_at, 0).unwrap_or_else(|| Utc::now());
    let metadata_last_fetched_dt = DateTime::from_timestamp(metadata.metadata_last_fetched_at, 0)
        .unwrap_or_else(|| Utc::now());

    // Override if provided, otherwise use metadata timestamp
    let final_metadata_updated = metadata_updated_at_override.unwrap_or(metadata_last_fetched_dt);

    // Market data last fetched fallback
    let market_data_last_fetched_dt = last_market_update.unwrap_or(final_metadata_updated);

    // Security timestamp (if available)
    let security_data_last_fetched_dt = security_ref.map(|sec| sec.security_data_last_fetched_at);

    // See the note on the other assembly path: unknown decimals stay `None`.
    let resolved_decimals = metadata
        .decimals
        .or_else(|| security_ref.and_then(|data| data.token_decimals));

    Token {
        // Core Identity & Metadata
        mint: metadata.mint.clone(),
        symbol: metadata.symbol.unwrap_or_else(|| "UNKNOWN".to_owned()),
        name: metadata.name.unwrap_or_else(|| "Unknown Token".to_owned()),
        decimals: resolved_decimals,
        description: None,
        image_url: None,
        header_image_url: None,
        supply: None,

        // Data source
        data_source: DataSource::Unknown,

        // Discovery & Creation timestamps
        first_discovered_at: first_discovered_dt,
        blockchain_created_at: blockchain_created_at_override,

        // Metadata timestamps
        metadata_last_fetched_at: final_metadata_updated,
        decimals_last_fetched_at: final_metadata_updated, // Same as metadata

        // Market data timestamps (defaults since no market data)
        market_data_last_fetched_at: market_data_last_fetched_dt,

        // Security data timestamp
        security_data_last_fetched_at: security_data_last_fetched_dt,

        // Pool price timestamps (defaults)
        pool_price_last_calculated_at: market_data_last_fetched_dt, // Fallback
        pool_price_last_used_pool: None,

        // Price Information (defaults for missing market data)
        price_usd: 0.0,
        price_sol: 0.0,
        price_native: "0".to_owned(),
        price_change_m5: None,
        price_change_h1: None,
        price_change_h6: None,
        price_change_h24: None,

        // Market Metrics
        market_cap: None,
        fdv: None,
        liquidity_usd: None,

        // Volume Data
        volume_m5: None,
        volume_h1: None,
        volume_h6: None,
        volume_h24: None,

        // Pool metrics
        pool_count: None,
        reserve_in_usd: None,

        // Transaction Activity
        txns_m5_buys: None,
        txns_m5_sells: None,
        txns_h1_buys: None,
        txns_h1_sells: None,
        txns_h6_buys: None,
        txns_h6_sells: None,
        txns_h24_buys: None,
        txns_h24_sells: None,

        // Social & Links
        websites: vec![],
        socials: vec![],

        // Security Information
        mint_authority,
        freeze_authority,
        update_authority,
        is_mutable,
        security_score,
        security_score_normalised,
        is_rugged,
        token_type,
        graph_insiders_detected,
        lp_provider_count,
        security_risks,
        total_holders,
        top_10_holders_pct,
        top_holders,
        creator_balance_pct,
        transfer_fee_pct,
        transfer_fee_max_amount,
        transfer_fee_authority,

        // Bot-Specific State
        is_blacklisted,
        priority,

        // Filtering State
        last_rejection_reason,
        last_rejection_source,
        last_rejection_at,
    }
}
