//! Token assembly — reconstructs full Token structs from database rows.

use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, types::FromSql, Row};
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::tokens::pools;
use crate::tokens::store;
use crate::tokens::types::{
    DataSource, DexScreenerData, GeckoTerminalData, Priority, RugcheckData, SecurityRisk,
    SocialLink, Token, TokenError, TokenHolder, TokenMetadata, TokenPoolInfo, TokenPoolSources,
    TokenPoolsSnapshot, TokenResult, WebsiteLink,
};

use super::helpers::{assemble_token, MarketDataType};
use super::TokenDatabase;

/// The Rugcheck columns the FILTERING rules read, loaded with the batch token query.
///
/// These used to be hardcoded to `None`/empty in the batch path, which meant the whole
/// security half of the filter pipeline ran against data that was never fetched: holder
/// concentration, insider counts, creator balance, transfer fee, LP providers, LP lock and
/// the risk-level check were all silently inert or — worse — rejecting every token for
/// "missing" data the query simply had not asked for. Only `score`, `rugged` and the two
/// authorities were real.
struct SecurityColumns {
    token_type: Option<String>,
    score_normalised: Option<i32>,
    top_10_holders_pct: Option<f64>,
    total_holders: Option<i64>,
    lp_provider_count: Option<i64>,
    graph_insiders_detected: Option<i64>,
    creator_balance_pct: Option<f64>,
    transfer_fee_pct: Option<f64>,
    transfer_fee_max_amount: Option<i64>,
    transfer_fee_authority: Option<String>,
    security_risks: Vec<SecurityRisk>,
    top_holders: Vec<TokenHolder>,
}

impl SecurityColumns {
    /// Read columns 66..=77 of the batch query.
    ///
    /// The two JSON columns are decoded leniently: a token whose stored risk or holder blob
    /// is malformed loses that one list rather than failing the entire batch load, which
    /// would take the whole filtering snapshot down with it.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let risks_json: Option<String> = row.get(76)?;
        let holders_json: Option<String> = row.get(77)?;

        Ok(Self {
            token_type: row.get(66)?,
            score_normalised: row.get(67)?,
            top_10_holders_pct: row.get(68)?,
            total_holders: row.get(69)?,
            lp_provider_count: row.get(70)?,
            graph_insiders_detected: row.get(71)?,
            creator_balance_pct: row.get(72)?,
            transfer_fee_pct: row.get(73)?,
            transfer_fee_max_amount: row.get(74)?,
            transfer_fee_authority: row.get(75)?,
            security_risks: parse_json_list(risks_json.as_deref()),
            top_holders: parse_json_list(holders_json.as_deref()),
        })
    }
}

fn parse_json_list<T: serde::de::DeserializeOwned>(raw: Option<&str>) -> Vec<T> {
    raw.and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default()
}

impl TokenDatabase {
    /// Assemble a complete token record with market data from the specified source
    pub fn get_full_token_for_source(
        &self,
        mint: &str,
        source: DataSource,
    ) -> TokenResult<Option<Token>> {
        let metadata = match self.get_token(mint)? {
            Some(m) => m,
            None => return Ok(None),
        };

        let (market_data, data_source) = match source {
            DataSource::DexScreener => match self.get_dexscreener_data(mint)? {
                Some(data) => (MarketDataType::DexScreener(data), DataSource::DexScreener),
                None => return Ok(None),
            },
            DataSource::GeckoTerminal => match self.get_geckoterminal_data(mint)? {
                Some(data) => (
                    MarketDataType::GeckoTerminal(data),
                    DataSource::GeckoTerminal,
                ),
                None => return Ok(None),
            },
            _ => return Ok(None),
        };

        let security = self.get_rugcheck_data(mint)?;
        let is_blacklisted = self.is_blacklisted(mint)?;
        let priority = self.get_priority(mint)?;

        // Prepare fallback images from alternate source when primary source is missing them
        let (fallback_img, fallback_header) = match (&data_source, &market_data) {
            // When using GeckoTerminal, try DexScreener images
            (DataSource::GeckoTerminal, _) => match self.get_dexscreener_data(mint)? {
                Some(ds) => (ds.image_url, ds.header_image_url),
                None => (None, None),
            },
            // When using DexScreener without image, try GeckoTerminal
            (DataSource::DexScreener, MarketDataType::DexScreener(ds))
                if ds.image_url.is_none() =>
            {
                match self.get_geckoterminal_data(mint)? {
                    Some(gt) => (gt.image_url, None),
                    None => (None, None),
                }
            }
            _ => (None, None),
        };

        let token = assemble_token(
            metadata,
            market_data,
            data_source,
            security,
            is_blacklisted,
            priority,
            fallback_img,
            fallback_header,
        );

        Ok(Some(token))
    }

    /// Assemble complete Token struct from all data sources
    ///
    /// This is the bridge function that external code should use when they need
    /// full token data (market + security). It assembles data from:
    /// - TokenMetadata (basic info)
    /// - DexScreenerData or GeckoTerminalData (market data, based on config)
    /// - RugcheckData (security data)
    /// - Blacklist status
    ///
    /// Returns None if token doesn't exist or has no market data from preferred source.
    pub fn get_full_token(&self, mint: &str) -> TokenResult<Option<Token>> {
        // Determine preferred source from config
        let preferred_source =
            crate::config::with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());
        let primary_source = if preferred_source.eq_ignore_ascii_case("geckoterminal") {
            DataSource::GeckoTerminal
        } else {
            DataSource::DexScreener
        };

        if let Some(token) = self.get_full_token_for_source(mint, primary_source)? {
            return Ok(Some(token));
        }

        let fallback_source = match primary_source {
            DataSource::DexScreener => DataSource::GeckoTerminal,
            DataSource::GeckoTerminal => DataSource::DexScreener,
            _ => return Ok(None),
        };

        self.get_full_token_for_source(mint, fallback_source)
    }

    /// Get all tokens from database with optional market data.
    /// Unlike get_full_token(), this returns tokens EVEN WITHOUT market data,
    /// using default/null values for missing fields.
    ///
    /// Use this for "All Tokens" view to show complete database contents.
    ///
    /// If limit=0, returns ALL tokens. Otherwise returns limit tokens with offset.
    ///
    /// If require_market_data=true, only returns tokens that have DexScreener OR GeckoTerminal data.
    /// This significantly reduces memory usage for filtering (144k -> ~56k tokens).
    ///
    /// PERFORMANCE: Uses LEFT JOINs to fetch all data in a single query, avoiding N+1 problem.

    pub fn get_all_tokens_optional_market(
        &self,
        limit: usize,
        offset: usize,
        sort_by: Option<&str>,
        sort_direction: Option<&str>,
        require_market_data: bool,
    ) -> TokenResult<Vec<Token>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        // Map sort_by to SQL column with table prefix
        let order_column = match sort_by {
            Some("symbol") => "t.symbol",
            Some("market_data_last_fetched_at") =>
                "COALESCE(ut.market_data_last_updated_at, d.market_data_last_fetched_at, g.market_data_last_fetched_at, t.metadata_last_fetched_at)",
            Some("first_discovered_at") => "t.first_discovered_at",
            Some("metadata_last_fetched_at") => "COALESCE(ut.metadata_last_updated_at, t.metadata_last_fetched_at)",
            Some("blockchain_created_at") =>
                "COALESCE(d.pair_blockchain_created_at, t.blockchain_created_at, t.first_discovered_at)",
            Some("pool_price_last_calculated_at") =>
                "COALESCE(ut.pool_price_last_calculated_at, t.first_discovered_at)",
            Some("mint") => "t.mint",
            Some("risk_score") => "sr.score",
            Some("price_sol") => "COALESCE(d.price_sol, g.price_sol)",
            Some("liquidity_usd") => "COALESCE(d.liquidity_usd, g.liquidity_usd)",
            Some("volume_24h") => "COALESCE(d.volume_24h, g.volume_24h)",
            Some("fdv") => "COALESCE(d.fdv, g.fdv)",
            Some("market_cap") => "COALESCE(d.market_cap, g.market_cap)",
            Some("price_change_h1") => "COALESCE(d.price_change_1h, g.price_change_1h)",
            Some("price_change_h24") => "COALESCE(d.price_change_24h, g.price_change_24h)",
            Some("txns_5m") => "COALESCE(d.txns_5m_buys, 0) + COALESCE(d.txns_5m_sells, 0)",
            Some("txns_1h") => "COALESCE(d.txns_1h_buys, 0) + COALESCE(d.txns_1h_sells, 0)",
            Some("txns_6h") => "COALESCE(d.txns_6h_buys, 0) + COALESCE(d.txns_6h_sells, 0)",
            Some("txns_24h") => "COALESCE(d.txns_24h_buys, 0) + COALESCE(d.txns_24h_sells, 0)",
            _ =>
                "COALESCE(ut.market_data_last_updated_at, d.market_data_last_fetched_at, g.market_data_last_fetched_at, t.metadata_last_fetched_at)",
        };

        let direction = match sort_direction {
            Some("asc") => "ASC",
            Some("desc") => "DESC",
            _ => "DESC", // default
        };

        // Build query (always join market tables so we can populate Token fields consistently)
        // PERF: This single query with JOINs avoids N+1 problem for filtering
        let select_base = r#"
            SELECT
                t.mint, t.symbol, t.name, t.decimals,
                t.first_discovered_at, t.blockchain_created_at,
                t.metadata_last_fetched_at, t.decimals_last_fetched_at,
                sr.score, sr.rugged, sr.security_data_last_fetched_at,
                sr.mint_authority, sr.freeze_authority,
                bl.reason as blacklist_reason,
                ut.priority, ut.pool_price_last_calculated_at, ut.pool_price_last_used_pool_address,
                d.price_usd, d.price_sol, d.price_native,
                d.price_change_5m, d.price_change_1h, d.price_change_6h, d.price_change_24h,
                d.market_cap, d.fdv, d.liquidity_usd,
                d.volume_5m, d.volume_1h, d.volume_6h, d.volume_24h,
                d.txns_5m_buys, d.txns_5m_sells, d.txns_1h_buys, d.txns_1h_sells,
                d.txns_6h_buys, d.txns_6h_sells, d.txns_24h_buys, d.txns_24h_sells,
                d.market_data_last_fetched_at as d_market_data_last_fetched_at,
                d.image_url as d_image_url, d.header_image_url as d_header_image_url,
                d.pair_blockchain_created_at,
                g.price_usd, g.price_sol, g.price_native,
                g.price_change_5m, g.price_change_1h, g.price_change_6h, g.price_change_24h,
                g.market_cap, g.fdv, g.liquidity_usd,
                g.volume_5m, g.volume_1h, g.volume_6h, g.volume_24h,
                g.pool_count, g.reserve_in_usd,
                g.market_data_last_fetched_at as g_market_data_last_fetched_at,
                g.image_url as g_image_url,
                ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at,
                sr.update_authority, sr.is_mutable,
                sr.token_type, sr.score_normalised, sr.top_10_holders_pct,
                sr.total_holders, sr.total_lp_providers, sr.graph_insiders_detected,
                sr.creator_balance_pct, sr.transfer_fee_pct, sr.transfer_fee_max_amount,
                sr.transfer_fee_authority, sr.risks, sr.top_holders
            FROM tokens t
            LEFT JOIN security_rugcheck sr ON t.mint = sr.mint
            LEFT JOIN blacklist bl ON t.mint = bl.mint
            LEFT JOIN update_tracking ut ON t.mint = ut.mint
            LEFT JOIN market_dexscreener d ON t.mint = d.mint
            LEFT JOIN market_geckoterminal g ON t.mint = g.mint
        "#;

        // PERF: When require_market_data=true, only load tokens with market data
        // AND exclude tokens with stale market data (configurable, default 7 days).
        // This reduces initial load from ~172k to ~15-30k tokens.
        // Stale tokens are dead tokens that will never pass filtering anyway —
        // excluding them saves ~122 MB of memory per filter snapshot.
        // Cutoff is pre-computed to avoid per-row strftime() calls in SQLite.
        // Configure via maintenance.stale_token_days (0 = include all).
        let where_clause = if require_market_data {
            let stale_days = crate::config::with_config(|cfg| cfg.maintenance.stale_token_days);
            let mut clause = " WHERE (d.mint IS NOT NULL OR g.mint IS NOT NULL)".to_owned();
            if stale_days > 0 {
                let cutoff_secs =
                    chrono::Utc::now().timestamp() - (stale_days as i64 * 24 * 60 * 60);
                clause.push_str(&format!(
                    " AND COALESCE(d.market_data_last_fetched_at, g.market_data_last_fetched_at) > {}",
                    cutoff_secs
                ));
            }
            clause
        } else {
            String::new()
        };

        // Stable tiebreaker: many tokens share the same (or NULL) value for the
        // chosen sort column — especially timestamp sorts like "updated". Without a
        // unique secondary key SQLite's order among ties is unspecified and can
        // differ between consecutive OFFSET fetches, so infinite-scroll pages
        // duplicate and skip rows (and the "updated" order looks wrong/jittery).
        // `t.mint` is the primary key, so appending it makes the total order fully
        // deterministic across every page request.
        let query = if limit == 0 {
            format!(
                "{}{} ORDER BY {} {}, t.mint ASC",
                select_base, where_clause, order_column, direction
            )
        } else {
            format!(
                "{}{} ORDER BY {} {}, t.mint ASC LIMIT {} OFFSET {}",
                select_base, where_clause, order_column, direction, limit, offset
            )
        };

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        // Parse row data
        let tokens_iter = stmt
            .query_map(params![], |row| {
                let mint: String = row.get(0)?;
                let symbol: Option<String> = row.get(1)?;
                let name: Option<String> = row.get(2)?;
                let decimals: Option<i64> = row.get(3)?;
                let first_discovered_at: i64 = row.get(4)?;
                let blockchain_created_at: Option<i64> = row.get(5)?;
                let metadata_last_fetched_at: i64 = row.get(6)?;
                let decimals_last_fetched_at: i64 = row.get(7)?;

                // Security data (optional) - includes authority fields for filtering
                let security_score: Option<i32> = row.get(8)?;
                let is_rugged: bool = row
                    .get::<_, Option<i64>>(9)?
                    .map(|v| v != 0)
                    .unwrap_or_default();
                let security_data_last_fetched_at: Option<i64> = row.get(10)?;
                let mint_authority: Option<String> = row.get(11)?;
                let freeze_authority: Option<String> = row.get(12)?;

                // Blacklist status
                let is_blacklisted = row.get::<_, Option<String>>(13)?.is_some();

                // Priority and pool price tracking
                let priority_value: Option<i32> = row.get(14)?;
                let pool_price_last_calculated_at: Option<i64> = row.get(15)?;
                let pool_price_last_used_pool: Option<String> = row.get(16)?;

                // DexScreener fields 17..=37
                let d_price_usd: Option<f64> = row.get(17)?;
                let d_price_sol: Option<f64> = row.get(18)?;
                let d_price_native: Option<String> = row.get(19)?;
                let d_change_5m: Option<f64> = row.get(20)?;
                let d_change_1h: Option<f64> = row.get(21)?;
                let d_change_6h: Option<f64> = row.get(22)?;
                let d_change_24h: Option<f64> = row.get(23)?;
                let d_market_cap: Option<f64> = row.get(24)?;
                let d_fdv: Option<f64> = row.get(25)?;
                let d_liquidity_usd: Option<f64> = row.get(26)?;
                let d_vol_5m: Option<f64> = row.get(27)?;
                let d_vol_1h: Option<f64> = row.get(28)?;
                let d_vol_6h: Option<f64> = row.get(29)?;
                let d_vol_24h: Option<f64> = row.get(30)?;
                let d_txn_5m_buys: Option<i64> = row.get(31)?;
                let d_txn_5m_sells: Option<i64> = row.get(32)?;
                let d_txn_1h_buys: Option<i64> = row.get(33)?;
                let d_txn_1h_sells: Option<i64> = row.get(34)?;
                let d_txn_6h_buys: Option<i64> = row.get(35)?;
                let d_txn_6h_sells: Option<i64> = row.get(36)?;
                let d_txn_24h_buys: Option<i64> = row.get(37)?;
                let d_txn_24h_sells: Option<i64> = row.get(38)?;
                let d_market_data_last_fetched_at: Option<i64> = row.get(39)?;
                let d_image_url: Option<String> = row.get(40)?;
                let d_header_image_url: Option<String> = row.get(41)?;
                let d_pair_blockchain_created_at: Option<i64> = row.get(42)?;

                // GeckoTerminal fields 43..=60
                let g_price_usd: Option<f64> = row.get(43)?;
                let g_price_sol: Option<f64> = row.get(44)?;
                let g_price_native: Option<String> = row.get(45)?;
                let g_change_5m: Option<f64> = row.get(46)?;
                let g_change_1h: Option<f64> = row.get(47)?;
                let g_change_6h: Option<f64> = row.get(48)?;
                let g_change_24h: Option<f64> = row.get(49)?;
                let g_market_cap: Option<f64> = row.get(50)?;
                let g_fdv: Option<f64> = row.get(51)?;
                let g_liquidity_usd: Option<f64> = row.get(52)?;
                let g_vol_5m: Option<f64> = row.get(53)?;
                let g_vol_1h: Option<f64> = row.get(54)?;
                let g_vol_6h: Option<f64> = row.get(55)?;
                let g_vol_24h: Option<f64> = row.get(56)?;
                let g_pool_count: Option<i64> = row.get(57)?;
                let g_reserve_in_usd: Option<f64> = row.get(58)?;
                let g_market_data_last_fetched_at: Option<i64> = row.get(59)?;
                let g_image_url: Option<String> = row.get(60)?;

                // Rejection tracking fields 61..=63
                let last_rejection_reason: Option<String> = row.get(61)?;
                let last_rejection_source: Option<String> = row.get(62)?;
                let last_rejection_at: Option<i64> = row.get(63)?;

                // New security fields 64..=65
                let update_authority: Option<String> = row.get(64)?;
                let is_mutable: Option<bool> = row.get::<_, Option<i64>>(65)?.map(|v| v != 0);

                // Remaining Rugcheck columns 66..=77, carried as one value so the row tuple
                // does not grow another dozen slots.
                let security = SecurityColumns::from_row(row)?;

                Ok((
                    mint,
                    symbol,
                    name,
                    decimals.map(|d| d as u8),
                    first_discovered_at,
                    blockchain_created_at,
                    metadata_last_fetched_at,
                    decimals_last_fetched_at,
                    security_score,
                    is_rugged,
                    security_data_last_fetched_at,
                    mint_authority,
                    freeze_authority,
                    is_blacklisted,
                    priority_value,
                    pool_price_last_calculated_at,
                    pool_price_last_used_pool,
                    // Dex (match SELECT order)
                    d_price_usd,
                    d_price_sol,
                    d_price_native,
                    d_change_5m,
                    d_change_1h,
                    d_change_6h,
                    d_change_24h,
                    d_market_cap,
                    d_fdv,
                    d_liquidity_usd,
                    d_vol_5m,
                    d_vol_1h,
                    d_vol_6h,
                    d_vol_24h,
                    d_txn_5m_buys,
                    d_txn_5m_sells,
                    d_txn_1h_buys,
                    d_txn_1h_sells,
                    d_txn_6h_buys,
                    d_txn_6h_sells,
                    d_txn_24h_buys,
                    d_txn_24h_sells,
                    d_market_data_last_fetched_at,
                    d_image_url,
                    d_header_image_url,
                    d_pair_blockchain_created_at,
                    // Gecko (match SELECT order)
                    g_price_usd,
                    g_price_sol,
                    g_price_native,
                    g_change_5m,
                    g_change_1h,
                    g_change_6h,
                    g_change_24h,
                    g_market_cap,
                    g_fdv,
                    g_liquidity_usd,
                    g_vol_5m,
                    g_vol_1h,
                    g_vol_6h,
                    g_vol_24h,
                    g_pool_count,
                    g_reserve_in_usd,
                    g_market_data_last_fetched_at,
                    g_image_url,
                    // Rejection tracking
                    last_rejection_reason,
                    last_rejection_source,
                    last_rejection_at,
                    update_authority,
                    is_mutable,
                    security,
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        let mut tokens = Vec::new();
        for row_result in tokens_iter {
            let (
                mint,
                symbol,
                name,
                decimals,
                first_discovered_at,
                blockchain_created_at,
                metadata_last_fetched_at,
                decimals_last_fetched_at,
                security_score,
                is_rugged,
                security_data_last_fetched_at,
                mint_authority,
                freeze_authority,
                is_blacklisted,
                priority_value,
                pool_price_last_calculated_at,
                pool_price_last_used_pool,
                // Dex fields
                d_price_usd,
                d_price_sol,
                d_price_native,
                d_change_5m,
                d_change_1h,
                d_change_6h,
                d_change_24h,
                d_market_cap,
                d_fdv,
                d_liquidity_usd,
                d_vol_5m,
                d_vol_1h,
                d_vol_6h,
                d_vol_24h,
                d_txn_5m_buys,
                d_txn_5m_sells,
                d_txn_1h_buys,
                d_txn_1h_sells,
                d_txn_6h_buys,
                d_txn_6h_sells,
                d_txn_24h_buys,
                d_txn_24h_sells,
                d_market_data_last_fetched_at,
                d_image_url,
                d_header_image_url,
                d_pair_blockchain_created_at,
                // Gecko fields
                g_price_usd,
                g_price_sol,
                g_price_native,
                g_change_5m,
                g_change_1h,
                g_change_6h,
                g_change_24h,
                g_market_cap,
                g_fdv,
                g_liquidity_usd,
                g_vol_5m,
                g_vol_1h,
                g_vol_6h,
                g_vol_24h,
                g_pool_count,
                g_reserve_in_usd,
                g_market_data_last_fetched_at,
                g_image_url,
                // Rejection tracking
                last_rejection_reason,
                last_rejection_source,
                last_rejection_at,
                update_authority,
                is_mutable,
                security,
            ) = row_result.map_err(|e| TokenError::Database(format!("Row parse failed: {e}")))?;

            // Parse all timestamps
            let first_discovered_dt =
                DateTime::from_timestamp(first_discovered_at, 0).unwrap_or_else(|| Utc::now());
            let blockchain_created_dt =
                blockchain_created_at.and_then(|ts| DateTime::from_timestamp(ts, 0));
            let metadata_last_fetched_dt =
                DateTime::from_timestamp(metadata_last_fetched_at, 0).unwrap_or_else(|| Utc::now());
            let decimals_last_fetched_dt =
                DateTime::from_timestamp(decimals_last_fetched_at, 0).unwrap_or_else(|| Utc::now());
            let security_data_last_fetched_dt =
                security_data_last_fetched_at.and_then(|ts| DateTime::from_timestamp(ts, 0));
            let pool_price_last_calculated_dt = pool_price_last_calculated_at
                .and_then(|ts| DateTime::from_timestamp(ts, 0))
                .unwrap_or(metadata_last_fetched_dt); // Fallback

            // Parse rejection timestamp
            let last_rejection_at_dt =
                last_rejection_at.and_then(|ts| DateTime::from_timestamp(ts, 0));

            let priority = priority_value
                .map(Priority::from_value)
                .unwrap_or(Priority::Standard);
            // Determine chosen market source based on config preference then fallback
            let preferred_source =
                crate::config::with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());
            let dex_available = d_price_sol.is_some() || d_price_usd.is_some();
            let gecko_available = g_price_sol.is_some() || g_price_usd.is_some();

            let (
                data_source,
                market_data_last_fetched_dt,
                price_usd,
                price_sol,
                price_native,
                change_5m,
                change_1h,
                change_6h,
                change_24h,
                market_cap,
                fdv,
                liquidity_usd,
                vol_5m,
                vol_1h,
                vol_6h,
                vol_24h,
                tx5b,
                tx5s,
                tx1b,
                tx1s,
                tx6b,
                tx6s,
                tx24b,
                tx24s,
            ) = if preferred_source == "geckoterminal" {
                if gecko_available {
                    (
                        DataSource::GeckoTerminal,
                        g_market_data_last_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(metadata_last_fetched_dt),
                        g_price_usd.unwrap_or_default(),
                        g_price_sol.unwrap_or_default(),
                        g_price_native.unwrap_or_else(|| "0".to_owned()),
                        g_change_5m,
                        g_change_1h,
                        g_change_6h,
                        g_change_24h,
                        g_market_cap,
                        g_fdv,
                        g_liquidity_usd,
                        g_vol_5m,
                        g_vol_1h,
                        g_vol_6h,
                        g_vol_24h,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                } else if dex_available {
                    (
                        DataSource::DexScreener,
                        d_market_data_last_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(metadata_last_fetched_dt),
                        d_price_usd.unwrap_or_default(),
                        d_price_sol.unwrap_or_default(),
                        d_price_native.unwrap_or_else(|| "0".to_owned()),
                        d_change_5m,
                        d_change_1h,
                        d_change_6h,
                        d_change_24h,
                        d_market_cap,
                        d_fdv,
                        d_liquidity_usd,
                        d_vol_5m,
                        d_vol_1h,
                        d_vol_6h,
                        d_vol_24h,
                        d_txn_5m_buys,
                        d_txn_5m_sells,
                        d_txn_1h_buys,
                        d_txn_1h_sells,
                        d_txn_6h_buys,
                        d_txn_6h_sells,
                        d_txn_24h_buys,
                        d_txn_24h_sells,
                    )
                } else {
                    (
                        DataSource::Unknown,
                        metadata_last_fetched_dt,
                        0.0,
                        0.0,
                        "0".to_owned(),
                        // price changes
                        None,
                        None,
                        None,
                        None,
                        // market metrics
                        None,
                        None,
                        None,
                        // volumes
                        None,
                        None,
                        None,
                        None,
                        // txns (all None for Unknown)
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
            } else {
                if dex_available {
                    (
                        DataSource::DexScreener,
                        d_market_data_last_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(metadata_last_fetched_dt),
                        d_price_usd.unwrap_or_default(),
                        d_price_sol.unwrap_or_default(),
                        d_price_native.unwrap_or_else(|| "0".to_owned()),
                        d_change_5m,
                        d_change_1h,
                        d_change_6h,
                        d_change_24h,
                        d_market_cap,
                        d_fdv,
                        d_liquidity_usd,
                        d_vol_5m,
                        d_vol_1h,
                        d_vol_6h,
                        d_vol_24h,
                        d_txn_5m_buys,
                        d_txn_5m_sells,
                        d_txn_1h_buys,
                        d_txn_1h_sells,
                        d_txn_6h_buys,
                        d_txn_6h_sells,
                        d_txn_24h_buys,
                        d_txn_24h_sells,
                    )
                } else if gecko_available {
                    (
                        DataSource::GeckoTerminal,
                        g_market_data_last_fetched_at
                            .and_then(|ts| DateTime::from_timestamp(ts, 0))
                            .unwrap_or(metadata_last_fetched_dt),
                        g_price_usd.unwrap_or_default(),
                        g_price_sol.unwrap_or_default(),
                        g_price_native.unwrap_or_else(|| "0".to_owned()),
                        g_change_5m,
                        g_change_1h,
                        g_change_6h,
                        g_change_24h,
                        g_market_cap,
                        g_fdv,
                        g_liquidity_usd,
                        g_vol_5m,
                        g_vol_1h,
                        g_vol_6h,
                        g_vol_24h,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                } else {
                    (
                        DataSource::Unknown,
                        metadata_last_fetched_dt,
                        0.0,
                        0.0,
                        "0".to_owned(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
            };

            let pool_count = if data_source == DataSource::GeckoTerminal {
                g_pool_count.map(|value| value as u32)
            } else {
                None
            };

            let reserve_in_usd = if data_source == DataSource::GeckoTerminal {
                g_reserve_in_usd
            } else {
                None
            };

            // Determine image_url and header_image_url with cross-source fallback
            // If primary source doesn't have image, try the other source
            let (resolved_image_url, resolved_header_image_url) = match data_source {
                DataSource::DexScreener => {
                    // Use DexScreener image, fallback to GeckoTerminal if missing
                    let img = d_image_url.clone().or(g_image_url.clone());
                    (img, d_header_image_url.clone())
                }
                DataSource::GeckoTerminal => {
                    // Use GeckoTerminal image, fallback to DexScreener if missing
                    let img = g_image_url.clone().or(d_image_url.clone());
                    (img, d_header_image_url.clone())
                }
                DataSource::Unknown => {
                    // Try any available image
                    let img = d_image_url.clone().or(g_image_url.clone());
                    (img, d_header_image_url.clone())
                }
                _ => (None, None),
            };

            let token = Token {
                // Core Identity & Metadata
                mint: mint.clone(),
                symbol: symbol.unwrap_or_else(|| "UNKNOWN".to_owned()),
                name: name.unwrap_or_else(|| "Unknown Token".to_owned()),
                decimals,
                description: None,
                image_url: resolved_image_url,
                header_image_url: resolved_header_image_url,
                supply: None,

                // Data source
                data_source,

                // Discovery & Creation timestamps
                first_discovered_at: first_discovered_dt,
                blockchain_created_at: d_pair_blockchain_created_at
                    .and_then(|ts| DateTime::from_timestamp(ts, 0)),

                // Metadata timestamps
                metadata_last_fetched_at: metadata_last_fetched_dt,
                decimals_last_fetched_at: decimals_last_fetched_dt,

                // Market data timestamps
                market_data_last_fetched_at: market_data_last_fetched_dt,

                // Security data timestamp
                security_data_last_fetched_at: security_data_last_fetched_dt,

                // Pool price timestamps
                pool_price_last_calculated_at: pool_price_last_calculated_dt,
                pool_price_last_used_pool: pool_price_last_used_pool,

                // Price Information
                price_usd,
                price_sol,
                price_native,
                price_change_m5: change_5m,
                price_change_h1: change_1h,
                price_change_h6: change_6h,
                price_change_h24: change_24h,

                // Market Metrics
                market_cap,
                fdv,
                liquidity_usd,

                // Volume Data
                volume_m5: vol_5m,
                volume_h1: vol_1h,
                volume_h6: vol_6h,
                volume_h24: vol_24h,

                // Pool metrics
                pool_count,
                reserve_in_usd,

                // Transaction Activity (only available for DexScreener)
                txns_m5_buys: tx5b,
                txns_m5_sells: tx5s,
                txns_h1_buys: tx1b,
                txns_h1_sells: tx1s,
                txns_h6_buys: tx6b,
                txns_h6_sells: tx6s,
                txns_h24_buys: tx24b,
                txns_h24_sells: tx24s,

                // Social & Links
                websites: vec![],
                socials: vec![],

                // Security Information - authority fields loaded for filtering
                mint_authority,
                freeze_authority,
                update_authority,
                is_mutable,
                security_score,
                security_score_normalised: security.score_normalised,
                is_rugged,
                token_type: security.token_type,
                graph_insiders_detected: security.graph_insiders_detected,
                lp_provider_count: security.lp_provider_count,
                security_risks: security.security_risks,
                total_holders: security.total_holders,
                top_10_holders_pct: security.top_10_holders_pct,
                top_holders: security.top_holders,
                creator_balance_pct: security.creator_balance_pct,
                transfer_fee_pct: security.transfer_fee_pct,
                transfer_fee_max_amount: security.transfer_fee_max_amount,
                transfer_fee_authority: security.transfer_fee_authority,

                // Bot-Specific State
                is_blacklisted,
                priority,

                // Filtering State
                last_rejection_reason,
                last_rejection_source,
                last_rejection_at: last_rejection_at_dt,
            };

            tokens.push(token);
        }

        tokens.shrink_to_fit(); // reclaim over-allocated Vec capacity
        Ok(tokens)
    }

    fn get_optional_market_data(
        &self,
        mint: &str,
    ) -> TokenResult<(Option<MarketDataType>, DataSource)> {
        let preferred_source =
            crate::config::with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());

        if preferred_source == "geckoterminal" {
            if let Some(data) = self.get_geckoterminal_data(mint)? {
                return Ok((
                    Some(MarketDataType::GeckoTerminal(data)),
                    DataSource::GeckoTerminal,
                ));
            }
            if let Some(data) = self.get_dexscreener_data(mint)? {
                return Ok((
                    Some(MarketDataType::DexScreener(data)),
                    DataSource::DexScreener,
                ));
            }
        } else {
            if let Some(data) = self.get_dexscreener_data(mint)? {
                return Ok((
                    Some(MarketDataType::DexScreener(data)),
                    DataSource::DexScreener,
                ));
            }
            if let Some(data) = self.get_geckoterminal_data(mint)? {
                return Ok((
                    Some(MarketDataType::GeckoTerminal(data)),
                    DataSource::GeckoTerminal,
                ));
            }
        }

        Ok((None, DataSource::Unknown))
    }
}
