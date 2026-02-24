use axum::{extract::Path, Json};
use chrono::Utc;
use serde::Serialize;

use crate::positions;
use crate::tokens;

// =============================================================================
// DEBUG INFO ENDPOINT FOR POSITIONS
// =============================================================================

#[derive(Debug, Serialize)]
pub struct PositionDebugResponse {
    pub mint: String,
    pub timestamp: String,
    pub position_data: Option<PositionData>,
    pub token_info: Option<TokenInfo>,
    pub price_data: Option<PriceData>,
    pub market_data: Option<MarketData>,
    pub pools: Vec<PoolInfo>,
    pub security: Option<SecurityInfo>,
    pub social: Option<SocialInfo>,
    pub position_debug: Option<PositionDebugDetails>,
}

#[derive(Debug, Serialize)]
pub struct PositionData {
    pub open_position: Option<PositionSummary>,
    pub closed_positions_count: usize,
    pub total_pnl: f64,
    pub win_rate: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct PositionSummary {
    pub id: Option<i64>,
    pub entry_price: f64,
    pub entry_time: i64,
    pub entry_size_sol: f64,
    pub current_price: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub unrealized_pnl_percent: Option<f64>,
    pub phantom_confirmations: u32,
}

#[derive(Debug, Serialize)]
pub struct TokenInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: Option<u8>,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub tags: Vec<String>,
    pub is_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct PriceData {
    pub pool_price_sol: f64,
    pub pool_price_usd: Option<f64>,
    pub confidence: f32,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct MarketData {
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PoolInfo {
    pub pool_address: String,
    pub program_kind: String,
    pub dex_name: String,
    pub sol_reserves: f64,
    pub token_reserves: f64,
    pub price_sol: f64,
    pub confidence: f32,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct SecurityInfo {
    pub score: i32,
    pub score_normalised: i32,
    pub rugged: bool,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub creator: Option<String>,
    pub total_holders: i32,
    pub top_10_concentration: Option<f64>,
    pub risks: Vec<RiskInfo>,
    pub analyzed_at: String,
}

#[derive(Debug, Serialize)]
pub struct RiskInfo {
    pub name: String,
    pub level: String,
    pub description: String,
    pub score: i32,
}

#[derive(Debug, Serialize)]
pub struct SocialInfo {
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PositionDebugDetails {
    pub transaction_details: TransactionDetails,
    pub fee_details: FeeDetails,
    pub profit_targets: ProfitTargets,
    pub price_tracking: PriceTracking,
    pub phantom_details: Option<PhantomDetails>,
    pub proceeds_metrics: ProceedsMetrics,
}

#[derive(Debug, Serialize)]
pub struct TransactionDetails {
    pub entry_signature: Option<String>,
    pub entry_verified: bool,
    pub exit_signature: Option<String>,
    pub exit_verified: bool,
    pub synthetic_exit: bool,
    pub closed_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeeDetails {
    pub entry_fee_lamports: Option<u64>,
    pub entry_fee_sol: Option<f64>,
    pub exit_fee_lamports: Option<u64>,
    pub exit_fee_sol: Option<f64>,
    pub total_fees_sol: f64,
}

#[derive(Debug, Serialize)]
pub struct ProfitTargets {
    pub min_target_percent: Option<f64>,
    pub max_target_percent: Option<f64>,
    pub liquidity_tier: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PriceTracking {
    pub price_highest: f64,
    pub price_lowest: f64,
    pub current_price: Option<f64>,
    pub current_price_updated: Option<String>,
    pub drawdown_from_high: Option<f64>,
    pub gain_from_low: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PhantomDetails {
    pub phantom_remove: bool,
    pub phantom_confirmations: u32,
    pub phantom_first_seen: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProceedsMetrics {
    pub accepted_quotes: u64,
    pub rejected_quotes: u64,
    pub accepted_profit_quotes: u64,
    pub accepted_loss_quotes: u64,
    pub average_shortfall_bps: f64,
    pub worst_shortfall_bps: u64,
}

/// Get comprehensive debug information for a position
pub async fn get_position_debug_info(Path(mint): Path<String>) -> Json<PositionDebugResponse> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Load decimals from cache
    let decimals = crate::tokens::get_decimals(&mint).await;

    // 1. Get position data
    let open_position_record = positions::get_open_positions()
        .await
        .into_iter()
        .find(|p| p.mint == mint);

    let open_position_summary = open_position_record.as_ref().map(|p| {
        let unrealized_pnl = p.current_price.map(|current| {
            let current_value = current * p.entry_size_sol;
            current_value - p.entry_size_sol
        });

        let unrealized_pnl_percent = unrealized_pnl.map(|pnl| {
            if p.entry_size_sol > 0.0 {
                (pnl / p.entry_size_sol) * 100.0
            } else {
                0.0
            }
        });

        PositionSummary {
            id: p.id,
            entry_price: p.entry_price,
            entry_time: p.entry_time.timestamp(),
            entry_size_sol: p.entry_size_sol,
            current_price: p.current_price,
            unrealized_pnl,
            unrealized_pnl_percent,
            phantom_confirmations: p.phantom_confirmations,
        }
    });

    let matching_closed: Vec<_> = positions::get_closed_positions()
        .await
        .into_iter()
        .filter(|p| p.mint == mint)
        .collect();

    let (closed_count, total_pnl, win_rate) = if matching_closed.is_empty() {
        (0, 0.0, 0.0)
    } else {
        let count = matching_closed.len();
        let total_pnl: f64 = matching_closed
            .iter()
            .filter_map(|p| p.sol_received.map(|received| received - p.entry_size_sol))
            .sum();
        let wins = matching_closed
            .iter()
            .filter(|p| {
                p.sol_received
                    .map(|r| r > p.entry_size_sol)
                    .unwrap_or_default()
            })
            .count();
        let win_rate = if count > 0 {
            (wins as f64 / count as f64) * 100.0
        } else {
            0.0
        };
        (count, total_pnl, win_rate)
    };

    let position_data = Some(PositionData {
        open_position: open_position_summary.clone(),
        closed_positions_count: closed_count,
        total_pnl,
        win_rate,
    });

    // 2. Get token info from database (with market data)
    let snapshot = crate::tokens::get_full_token_async(&mint)
        .await
        .ok()
        .flatten();
    let api_token = snapshot.as_ref();

    let token_info = api_token.map(|token| TokenInfo {
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        decimals,
        logo_url: token.image_url.clone(),
        website: token.websites.first().map(|w| w.url.clone()),
        tags: Vec::new(), // Tags not available in unified Token
        // Normalized score is 0-100 where HIGHER = MORE RISKY
        // Token is "verified" (safe) if score <= 30 (low risk)
        is_verified: token
            .security_score_normalised
            .map(|s| s <= 30)
            .unwrap_or_default(),
    });

    // 3. Get current price from pool service
    let price_data = crate::pools::get_pool_price(&mint).map(|price_result| {
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let price_unix_time = now_unix - (age_seconds as i64);

        PriceData {
            pool_price_sol: price_result.price_sol,
            pool_price_usd: None,
            confidence: price_result.confidence,
            last_updated: price_unix_time,
        }
    });

    // 4. Get market data from token database
    let market_data = api_token.as_ref().map(|token| MarketData {
        market_cap: token.market_cap,
        fdv: token.fdv,
        liquidity_usd: token.liquidity_usd,
        volume_24h: token.volume_h24,
    });

    // 5. Get pool info
    let mut pools_vec = Vec::new();
    if let Some(price_result) = crate::pools::get_pool_price(&mint) {
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let price_unix_time = now_unix - (age_seconds as i64);

        pools_vec.push(PoolInfo {
            pool_address: price_result.pool_address.clone(),
            program_kind: format!(
                "{:?}",
                price_result
                    .source_pool
                    .as_ref()
                    .unwrap_or(&"Unknown".to_string())
            ),
            dex_name: price_result
                .source_pool
                .as_ref()
                .unwrap_or(&"Unknown".to_string())
                .clone(),
            sol_reserves: price_result.sol_reserves,
            token_reserves: price_result.token_reserves,
            price_sol: price_result.price_sol,
            confidence: price_result.confidence,
            last_updated: price_unix_time,
        });
    }

    // 6. Get security info (temporarily unavailable until SecurityProvider integration)
    let security = None::<SecurityInfo>;

    // 7. Get social info from token database
    let social = api_token.as_ref().map(|token| SocialInfo {
        website: token.websites.first().map(|w| w.url.clone()),
        twitter: token
            .socials
            .iter()
            .find(|s| s.link_type.to_lowercase().contains("twitter"))
            .map(|s| s.url.clone()),
        telegram: token
            .socials
            .iter()
            .find(|s| s.link_type.to_lowercase().contains("telegram"))
            .map(|s| s.url.clone()),
    });

    // 8. Get position debug data
    let position_debug = if let Some(pos) = open_position_record.clone() {
        // Transaction details
        let transaction_details = TransactionDetails {
            entry_signature: pos.entry_transaction_signature.clone(),
            entry_verified: pos.transaction_entry_verified,
            exit_signature: pos.exit_transaction_signature.clone(),
            exit_verified: pos.transaction_exit_verified,
            synthetic_exit: pos.synthetic_exit,
            closed_reason: pos.closed_reason.clone(),
        };

        // Fee details
        let entry_fee_sol = pos.entry_fee_lamports.map(|l| (l as f64) / 1_000_000_000.0);
        let exit_fee_sol = pos.exit_fee_lamports.map(|l| (l as f64) / 1_000_000_000.0);
        let total_fees_sol = entry_fee_sol.unwrap_or_default() + exit_fee_sol.unwrap_or_default();

        let fee_details = FeeDetails {
            entry_fee_lamports: pos.entry_fee_lamports,
            entry_fee_sol,
            exit_fee_lamports: pos.exit_fee_lamports,
            exit_fee_sol,
            total_fees_sol,
        };

        // Profit targets
        let profit_targets = ProfitTargets {
            min_target_percent: pos.profit_target_min,
            max_target_percent: pos.profit_target_max,
            liquidity_tier: pos.liquidity_tier.clone(),
        };

        // Price tracking
        let current = pos.current_price.unwrap_or(pos.entry_price);
        let drawdown_from_high = if pos.price_highest > 0.0 {
            Some(((current - pos.price_highest) / pos.price_highest) * 100.0)
        } else {
            None
        };
        let gain_from_low = if pos.price_lowest > 0.0 {
            Some(((current - pos.price_lowest) / pos.price_lowest) * 100.0)
        } else {
            None
        };

        let price_tracking = PriceTracking {
            price_highest: pos.price_highest,
            price_lowest: pos.price_lowest,
            current_price: pos.current_price,
            current_price_updated: pos.current_price_updated.map(|dt| dt.to_rfc3339()),
            drawdown_from_high,
            gain_from_low,
        };

        // Phantom details
        let phantom_details = if pos.phantom_remove || pos.phantom_confirmations > 0 {
            Some(PhantomDetails {
                phantom_remove: pos.phantom_remove,
                phantom_confirmations: pos.phantom_confirmations,
                phantom_first_seen: pos.phantom_first_seen.map(|dt| dt.to_rfc3339()),
            })
        } else {
            None
        };

        // Proceeds metrics
        let proceeds_metrics = crate::positions::metrics::get_proceeds_metrics_snapshot().await;
        let proceeds = ProceedsMetrics {
            accepted_quotes: proceeds_metrics.accepted_quotes,
            rejected_quotes: proceeds_metrics.rejected_quotes,
            accepted_profit_quotes: proceeds_metrics.accepted_profit_quotes,
            accepted_loss_quotes: proceeds_metrics.accepted_loss_quotes,
            average_shortfall_bps: proceeds_metrics.average_shortfall_bps,
            worst_shortfall_bps: proceeds_metrics.worst_shortfall_bps,
        };

        Some(PositionDebugDetails {
            transaction_details,
            fee_details,
            profit_targets,
            price_tracking,
            phantom_details,
            proceeds_metrics: proceeds,
        })
    } else {
        None
    };

    Json(PositionDebugResponse {
        mint,
        timestamp,
        position_data,
        token_info,
        price_data,
        market_data,
        pools: pools_vec,
        security,
        social,
        position_debug,
    })
}
