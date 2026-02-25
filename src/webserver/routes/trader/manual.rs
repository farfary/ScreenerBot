//! Manual trading operations

use axum::{extract::Query, http::StatusCode, response::Response, Json};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::config::with_config;
use crate::global::{are_core_services_ready, get_pending_services};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// MANUAL TRADING HANDLERS
// =============================================================================

pub async fn manual_buy_handler(Json(req): Json<ManualBuyRequest>) -> Response {
    // Check force stop
    if crate::global::is_force_stopped() {
        return error_response(
            StatusCode::FORBIDDEN,
            "ForceStopped",
            "Manual trading disabled - Force stop is active",
            None,
        );
    }

    // Check services ready
    if !are_core_services_ready() {
        let pending = get_pending_services().join(", ");
        let error_msg = format!("Core services not ready: {pending}");
        // Create failed action for visibility
        crate::trader::actions::create_failed_buy_action(&req.mint, &error_msg).await;
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CoreServicesNotReady",
            "Core services are not ready for trading operations",
            Some(&format!("pending={pending}")),
        );
    }

    // Validate mint
    if Pubkey::from_str(&req.mint).is_err() {
        let error_msg = "Invalid token mint address";
        crate::trader::actions::create_failed_buy_action(&req.mint, error_msg).await;
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            error_msg,
            Some("Mint must be a valid base58 pubkey"),
        );
    }

    // Server-side blacklist enforcement with optional force override
    if let Some(db) = crate::tokens::database::get_global_database() {
        if let Ok(true) = tokio::task::spawn_blocking({
            let db = db.clone();
            let mint = req.mint.clone();
            move || db.is_blacklisted(&mint)
        })
        .await
        .unwrap_or(Ok(false))
        {
            if !req.force.unwrap_or_default() {
                let error_msg = "Token is blacklisted";
                crate::trader::actions::create_failed_buy_action(&req.mint, error_msg).await;
                return error_response(
                    StatusCode::FORBIDDEN,
                    "Blacklisted",
                    "Token is blacklisted; set force=true to override",
                    None,
                );
            }
        }
    }

    let size = match req.size_sol {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => with_config(|cfg| cfg.trader.trade_size_sol),
    };

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} size_sol={} force={}",
            req.mint,
            size,
            req.force.unwrap_or_default()
        ),
    );

    // Use standard manual_buy - action tracking is handled inside
    let result = crate::trader::manual::manual_buy(&req.mint, size).await;

    match result {
        Ok(tr) => {
            if !tr.success {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "ManualBuyFailed",
                    tr.error.as_deref().unwrap_or("Manual buy failed"),
                    None,
                );
            }
            let resp = ManualTradeSuccess {
                success: true,
                mint: req.mint,
                signature: tr.tx_signature,
                effective_price_sol: tr.executed_price_sol,
                size_sol: tr.executed_size_sol,
                position_id: tr.position_id,
                message: "Manual buy executed".to_owned(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            success_response(resp)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ManualBuyError",
            &e,
            None,
        ),
    }
}

pub async fn manual_add_handler(Json(req): Json<ManualAddRequest>) -> Response {
    // Check force stop
    if crate::global::is_force_stopped() {
        return error_response(
            StatusCode::FORBIDDEN,
            "ForceStopped",
            "Manual trading disabled - Force stop is active",
            None,
        );
    }

    // Enforce blacklist for add (no override)
    if let Some(db) = crate::tokens::database::get_global_database() {
        if let Ok(true) = tokio::task::spawn_blocking({
            let db = db.clone();
            let mint = req.mint.clone();
            move || db.is_blacklisted(&mint)
        })
        .await
        .unwrap_or(Ok(false))
        {
            let error_msg = "Token is blacklisted; cannot add to position";
            crate::trader::actions::create_failed_add_action(&req.mint, error_msg).await;
            return error_response(StatusCode::FORBIDDEN, "Blacklisted", error_msg, None);
        }
    }

    // Check services ready
    if !are_core_services_ready() {
        let pending = get_pending_services().join(", ");
        let error_msg = format!("Core services not ready: {pending}");
        crate::trader::actions::create_failed_add_action(&req.mint, &error_msg).await;
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CoreServicesNotReady",
            "Core services are not ready for trading operations",
            Some(&format!("pending={pending}")),
        );
    }

    // Validate mint
    if Pubkey::from_str(&req.mint).is_err() {
        let error_msg = "Invalid token mint address";
        crate::trader::actions::create_failed_add_action(&req.mint, error_msg).await;
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            error_msg,
            Some("Mint must be a valid base58 pubkey"),
        );
    }

    // Ensure there's an open position for this mint
    let has_open = positions::is_open_position(&req.mint).await;
    if !has_open {
        let error_msg = "Cannot add to position: no open position for this token";
        crate::trader::actions::create_failed_add_action(&req.mint, error_msg).await;
        return error_response(StatusCode::BAD_REQUEST, "NoOpenPosition", error_msg, None);
    }

    let size = match req.size_sol {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => with_config(|cfg| cfg.trader.trade_size_sol * 0.5), // default add = 50% size
    };

    logger::info(
        LogTag::Webserver,
        &format!("mint={} size_sol={}", req.mint, size),
    );

    // Use trader module - action tracking is handled inside
    let result = crate::trader::manual::manual_add(&req.mint, size).await;

    match result {
        Ok(tr) => {
            if !tr.success {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "ManualAddFailed",
                    tr.error.as_deref().unwrap_or("Manual add failed"),
                    None,
                );
            }
            let resp = ManualTradeSuccess {
                success: true,
                mint: req.mint,
                signature: tr.tx_signature,
                effective_price_sol: tr.executed_price_sol,
                size_sol: tr.executed_size_sol,
                position_id: tr.position_id,
                message: "Added to position".to_owned(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            success_response(resp)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ManualAddError",
            &e,
            None,
        ),
    }
}

pub async fn manual_sell_handler(Json(req): Json<ManualSellRequest>) -> Response {
    // Check force stop
    if crate::global::is_force_stopped() {
        return error_response(
            StatusCode::FORBIDDEN,
            "ForceStopped",
            "Manual trading disabled - Force stop is active",
            None,
        );
    }

    // Check services ready
    if !are_core_services_ready() {
        let pending = get_pending_services().join(", ");
        let error_msg = format!("Core services not ready: {pending}");
        crate::trader::actions::create_failed_sell_action(&req.mint, &error_msg).await;
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CoreServicesNotReady",
            "Core services are not ready for trading operations",
            Some(&format!("pending={pending}")),
        );
    }

    // Validate mint
    if Pubkey::from_str(&req.mint).is_err() {
        let error_msg = "Invalid token mint address";
        crate::trader::actions::create_failed_sell_action(&req.mint, error_msg).await;
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            error_msg,
            Some("Mint must be a valid base58 pubkey"),
        );
    }

    let is_open = positions::is_open_position(&req.mint).await;
    if !is_open {
        let error_msg = "Cannot sell: no open position for this token";
        crate::trader::actions::create_failed_sell_action(&req.mint, error_msg).await;
        return error_response(StatusCode::BAD_REQUEST, "NoOpenPosition", error_msg, None);
    }

    // Determine percentage
    let close_all = req.close_all.unwrap_or_default();
    let pct = if close_all {
        None // Full exit (100%)
    } else {
        Some(
            req.percentage
                .unwrap_or_else(|| with_config(|cfg| cfg.positions.partial_exit_default_pct)),
        )
    };

    // Validate percentage if provided
    if let Some(percentage) = pct {
        if !percentage.is_finite() || percentage <= 0.0 || percentage > 100.0 {
            let error_msg = format!("Invalid percentage: {percentage}. Must be in (0, 100]");
            crate::trader::actions::create_failed_sell_action(&req.mint, &error_msg).await;
            return error_response(
                StatusCode::BAD_REQUEST,
                "InvalidPercentage",
                "percentage must be in (0, 100]",
                None,
            );
        }
    }

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} percentage={:?} force={}",
            req.mint,
            pct,
            req.force.unwrap_or_default()
        ),
    );

    // Route to trader module - action tracking is handled inside
    let result = if req.force.unwrap_or_default() {
        crate::trader::manual::force_sell(&req.mint, pct).await
    } else {
        crate::trader::manual::manual_sell(&req.mint, pct).await
    };

    match result {
        Ok(tr) => {
            if !tr.success {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "ManualSellFailed",
                    tr.error.as_deref().unwrap_or("Manual sell failed"),
                    None,
                );
            }
            let resp = ManualTradeSuccess {
                success: true,
                mint: req.mint,
                signature: tr.tx_signature,
                effective_price_sol: tr.executed_price_sol,
                size_sol: tr.executed_size_sol,
                position_id: tr.position_id,
                message: if pct.unwrap_or(100.0) == 100.0 {
                    "Full position closed".to_owned()
                } else {
                    format!("Partial position closed ({}%)", pct.unwrap_or(100.0))
                },
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            success_response(resp)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ManualSellError",
            &e,
            None,
        ),
    }
}

// =============================================================================
// QUOTE PREVIEW HANDLER
// =============================================================================

/// GET /api/trader/quote - Get quote preview without execution
/// For BUY: requires amount_sol (SOL to spend), returns tokens received
/// For SELL: requires amount_tokens (tokens to sell), returns SOL received
pub async fn quote_preview_handler(Query(req): Query<QuotePreviewRequest>) -> Response {
    use crate::constants::SOL_MINT;
    use crate::swaps::operations::get_best_quote;
    use crate::swaps::router::{QuoteRequest, SwapMode};
    use crate::tokens::database::get_token_async;
    use crate::utils::get_wallet_address;

    // Validate mint
    if Pubkey::from_str(&req.mint).is_err() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            "Invalid token mint address",
            None,
        );
    }

    let direction = if req.direction.eq_ignore_ascii_case("sell") {
        "sell"
    } else {
        "buy"
    };

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(_) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "WalletNotAvailable",
                "Wallet not configured",
                None,
            );
        }
    };

    // Get token info for decimals (needed for sell)
    let token_decimals = match get_token_async(&req.mint).await {
        Ok(Some(token)) => token.decimals.unwrap_or(9) as u32,
        Ok(None) => 9, // Default to 9 decimals if token not found
        Err(_) => 9,
    };

    // Build quote request based on direction
    let (input_mint, output_mint, input_amount, input_amount_display) = if direction == "buy" {
        // BUY: SOL → Token
        let amount_sol = match req.amount_sol {
            Some(amt) if amt > 0.0 && amt.is_finite() => amt,
            _ => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "InvalidAmount",
                    "amount_sol is required for buy and must be positive",
                    None,
                );
            }
        };
        let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;
        (
            SOL_MINT.to_string(),
            req.mint.clone(),
            amount_lamports,
            amount_sol,
        )
    } else {
        // SELL: Token → SOL
        let amount_tokens = match req.amount_tokens {
            Some(amt) if amt > 0.0 && amt.is_finite() => amt,
            _ => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "InvalidAmount",
                    "amount_tokens is required for sell and must be positive",
                    None,
                );
            }
        };
        // Convert token amount to smallest units based on decimals
        let amount_raw = (amount_tokens * 10f64.powi(token_decimals as i32)) as u64;
        (
            req.mint.clone(),
            SOL_MINT.to_string(),
            amount_raw,
            amount_tokens,
        )
    };

    let quote_request = QuoteRequest {
        input_mint,
        output_mint,
        input_amount,
        wallet_address,
        slippage_pct: with_config(|cfg| cfg.swaps.slippage.quote_default_pct),
        swap_mode: SwapMode::ExactIn,
    };

    // Fetch quote
    match get_best_quote(quote_request).await {
        Ok(quote) => {
            // Format input/output based on direction
            let (input_formatted, output_display, output_formatted, price_per_token) = if direction
                == "buy"
            {
                // BUY: input is SOL, output is tokens
                let input_fmt = format!("{:.4} SOL", input_amount_display);
                let output_tokens = quote.output_amount as f64 / 10f64.powi(token_decimals as i32);
                let output_fmt = if output_tokens >= 1_000_000_000.0 {
                    format!("{:.2}B tokens", output_tokens / 1_000_000_000.0)
                } else if output_tokens >= 1_000_000.0 {
                    format!("{:.2}M tokens", output_tokens / 1_000_000.0)
                } else if output_tokens >= 1_000.0 {
                    format!("{:.2}K tokens", output_tokens / 1_000.0)
                } else {
                    format!("{:.4} tokens", output_tokens)
                };
                let price = if output_tokens > 0.0 {
                    input_amount_display / output_tokens
                } else {
                    0.0
                };
                (input_fmt, output_tokens, output_fmt, price)
            } else {
                // SELL: input is tokens, output is SOL
                let input_fmt = if input_amount_display >= 1_000_000_000.0 {
                    format!("{:.2}B tokens", input_amount_display / 1_000_000_000.0)
                } else if input_amount_display >= 1_000_000.0 {
                    format!("{:.2}M tokens", input_amount_display / 1_000_000.0)
                } else if input_amount_display >= 1_000.0 {
                    format!("{:.2}K tokens", input_amount_display / 1_000.0)
                } else {
                    format!("{:.4} tokens", input_amount_display)
                };
                let output_sol = quote.output_amount as f64 / 1_000_000_000.0;
                let output_fmt = format!("{:.6} SOL", output_sol);
                let price = if input_amount_display > 0.0 {
                    output_sol / input_amount_display
                } else {
                    0.0
                };
                (input_fmt, output_sol, output_fmt, price)
            };

            // Platform fee (0.5%) - calculated on SOL side
            let platform_fee_pct = 0.5;
            let platform_fee_sol = if direction == "buy" {
                input_amount_display * (platform_fee_pct / 100.0)
            } else {
                output_display * (platform_fee_pct / 100.0)
            };

            // Network fee estimate (approx 0.000005 SOL)
            let network_fee_sol = 0.000005;

            let response = QuotePreviewResponse {
                success: true,
                router: quote.router_name,
                direction: direction.to_string(),
                input_amount: input_amount_display,
                input_formatted,
                output_amount: output_display,
                output_formatted,
                price_per_token_sol: price_per_token,
                price_impact_pct: quote.price_impact_pct,
                platform_fee_pct,
                platform_fee_sol,
                network_fee_sol,
                route: quote.route_plan,
                slippage_bps: quote.slippage_bps,
                expires_in_secs: 30, // Quotes typically valid for ~30s
            };

            success_response(response)
        }
        Err(e) => error_response(
            StatusCode::BAD_GATEWAY,
            "QuoteFailed",
            &format!("Failed to fetch quote: {e}"),
            None,
        ),
    }
}
