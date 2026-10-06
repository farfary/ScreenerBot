// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Manual trading operations.

use axum::{
    extract::Query,
    response::{IntoResponse as _, Response},
    Json,
};

use crate::config::with_config;
use crate::errors::ErrorClass;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::swaps::{try_get_best_quote, QuoteError};
use crate::trader::manual::guard::{self, BlacklistPolicy, ManualTradeKind};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

use super::control::trader_failure;
use super::types::*;

// =============================================================================
// MANUAL TRADING HANDLERS
// =============================================================================

fn trade_response(
    result: Result<crate::trader::TradeResult, crate::trader::Error>,
    mint: String,
) -> Response {
    match result {
        Ok(tr) if !tr.success => match tr.error {
            Some(reason) => {
                ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_TRADE_MANUAL_REFUSED)
                    .text_arg("reason", reason)
            }
            None => ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_TRADE_MANUAL_FAILED),
        }
        .into_response(),
        Ok(tr) => success_response(ManualTradeSuccess {
            success: true,
            mint,
            signature: tr.tx_signature,
            effective_price_native: tr.executed_price_native,
            size_sol: tr.executed_size_native,
            position_id: tr.position_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
        Err(error) => trader_failure(&error),
    }
}

pub async fn manual_buy_handler(Json(req): Json<ManualBuyRequest>) -> Response {
    let blacklist = if req.force.unwrap_or_default() {
        BlacklistPolicy::Override
    } else {
        BlacklistPolicy::Enforce
    };
    if let Err(error) = guard::preflight(ManualTradeKind::Buy, &req.mint, blacklist).await {
        return trader_failure(&error);
    }
    let slippage_pct = match guard::validate_slippage(req.slippage_pct) {
        Ok(v) => v,
        Err(error) => return trader_failure(&error),
    };
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
    let management = req
        .management
        .unwrap_or(crate::positions::PositionManagement::UserOnly);
    let result = crate::trader::manual::manual_buy(&req.mint, size, management, slippage_pct).await;
    trade_response(result, req.mint)
}

pub async fn manual_add_handler(Json(req): Json<ManualAddRequest>) -> Response {
    if let Err(error) =
        guard::preflight(ManualTradeKind::Add, &req.mint, BlacklistPolicy::Enforce).await
    {
        return trader_failure(&error);
    }
    let slippage_pct = match guard::validate_slippage(req.slippage_pct) {
        Ok(v) => v,
        Err(error) => return trader_failure(&error),
    };
    // Default add size = the configured DCA size (a fraction of the trade size). The
    // fraction must come from `trader.dca_size_percentage`, never a hardcoded 0.5.
    let size = match req.size_sol {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => {
            with_config(|cfg| cfg.trader.trade_size_sol * (cfg.trader.dca_size_percentage / 100.0))
        }
    };
    logger::info(
        LogTag::Webserver,
        &format!("mint={} size_sol={}", req.mint, size),
    );
    let result = crate::trader::manual::manual_add(&req.mint, size, slippage_pct).await;
    trade_response(result, req.mint)
}

pub async fn manual_sell_handler(Json(req): Json<ManualSellRequest>) -> Response {
    if let Err(error) =
        guard::preflight(ManualTradeKind::Sell, &req.mint, BlacklistPolicy::Ignore).await
    {
        return trader_failure(&error);
    }
    let pct = if req.close_all.unwrap_or_default() {
        None // Full exit (100%)
    } else {
        let requested = req
            .percentage
            .unwrap_or_else(|| with_config(|cfg| cfg.positions.partial_exit_default_pct));
        match guard::validate_percentage(requested) {
            Ok(pct) => Some(pct),
            Err(error) => return trader_failure(&error),
        }
    };
    let slippage_pct = match guard::validate_slippage(req.slippage_pct) {
        Ok(v) => v,
        Err(error) => return trader_failure(&error),
    };
    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} percentage={:?} force={}",
            req.mint,
            pct,
            req.force.unwrap_or_default()
        ),
    );
    let result = if req.force.unwrap_or_default() {
        crate::trader::manual::force_sell(&req.mint, pct, slippage_pct).await
    } else {
        crate::trader::manual::manual_sell(&req.mint, pct, slippage_pct).await
    };
    trade_response(result, req.mint)
}

// =============================================================================
// QUOTE PREVIEW HANDLER
// =============================================================================

/// GET /api/trader/quote - Get quote preview without execution
/// For BUY: requires amount_sol (SOL to spend), returns tokens received
/// For SELL: requires amount_tokens (tokens to sell), returns SOL received
pub async fn quote_preview_handler(Query(req): Query<QuotePreviewRequest>) -> Response {
    use crate::swaps::types::{QuoteRequest, SwapMode};
    use crate::tokens::database::get_token_async;
    use crate::utils::get_wallet_address;

    // Validate mint
    if crate::chains::adapter()
        .validate_address(&req.mint)
        .is_err()
    {
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_TOOLS_MINT_INVALID)
            .into_response();
    }

    let direction = if req.direction.eq_ignore_ascii_case("sell") {
        "sell"
    } else {
        "buy"
    };

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(_) => {
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_TRADE_WALLET_NOT_CONFIGURED,
            )
            .into_response();
        }
    };

    // Get token info for decimals (needed for sell)
    let token_decimals = match get_token_async(crate::chains::active_chain(), &req.mint).await {
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
                return ApiError::new(
                    ApiErrorCode::InvalidInput,
                    ids::ERRORS_TRADE_AMOUNT_SOL_INVALID,
                )
                .into_response();
            }
        };
        let amount_lamports = crate::chains::adapter().native_to_raw(amount_sol);
        (
            crate::chains::adapter().native_asset_address().to_string(),
            req.mint.clone(),
            amount_lamports,
            amount_sol,
        )
    } else {
        // SELL: Token → SOL. Quote against the REAL on-chain balance (raw units),
        // not the frontend's holdings. The DB-stored token_amount is in raw units
        // and can be stale, so trusting it (and re-scaling by decimals) produced a
        // wildly inflated, misleading quote. Execution is already percentage-based
        // against the real balance — the preview now matches it exactly.
        let actual_balance =
            crate::chains::solana::assets::ata::get_total_token_balance(&wallet_address, &req.mint)
                .await
                .unwrap_or(0);
        if actual_balance == 0 {
            return ApiError::new(
                ApiErrorCode::IntegrityFailed,
                ids::ERRORS_TRADE_NO_TOKENS_IN_WALLET,
            )
            .into_response();
        }

        // Percentage of the real balance (default = full close). Legacy callers may
        // still send amount_tokens (whole tokens); honour it only as a fallback.
        let amount_raw = if let Some(pct) = req.percentage {
            if !pct.is_finite() || pct <= 0.0 || pct > 100.0 {
                return ApiError::new(
                    ApiErrorCode::InvalidInput,
                    ids::ERRORS_TRADE_PERCENTAGE_RANGE,
                )
                .into_response();
            }
            ((actual_balance as f64) * pct / 100.0).floor() as u64
        } else if let Some(tokens) = req.amount_tokens {
            if !tokens.is_finite() || tokens <= 0.0 {
                return ApiError::new(
                    ApiErrorCode::InvalidInput,
                    ids::ERRORS_TRADE_AMOUNT_TOKENS_INVALID,
                )
                .into_response();
            }
            // Clamp to the real balance so we never quote more than exists.
            ((tokens * 10f64.powi(token_decimals as i32)) as u64).min(actual_balance)
        } else {
            actual_balance // No amount given → quote a full close.
        };

        if amount_raw == 0 {
            return ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_TRADE_SELL_AMOUNT_ZERO,
            )
            .into_response();
        }

        let amount_tokens_display = amount_raw as f64 / 10f64.powi(token_decimals as i32);
        (
            req.mint.clone(),
            crate::chains::adapter().native_asset_address().to_string(),
            amount_raw,
            amount_tokens_display,
        )
    };

    let quote_request = QuoteRequest {
        chain: crate::chains::active_chain(),
        input_mint,
        output_mint,
        input_amount: input_amount.into(),
        wallet_address,
        // Price the preview at the slippage the trade will actually use, so the quote
        // the user confirms is the quote they get.
        slippage_pct: match guard::validate_slippage(req.slippage_pct) {
            Ok(Some(pct)) => pct,
            Ok(None) => with_config(|cfg| cfg.trader.slippage.quote_default_pct),
            Err(error) => return trader_failure(&error),
        },
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };

    // Fetch quote
    match try_get_best_quote(quote_request).await {
        Ok(quote) => {
            // Format input/output based on direction
            let (input_formatted, output_display, output_formatted, price_per_token) =
                if direction == "buy" {
                    // BUY: input is SOL, output is tokens
                    let input_fmt = format!("{:.4} SOL", input_amount_display);
                    let output_tokens =
                        quote.output_amount.raw() as f64 / 10f64.powi(token_decimals as i32);
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
                    let output_sol = quote.output_amount.raw() as f64
                        / crate::chains::adapter().raw_units_per_native() as f64;
                    let output_fmt = format!("{:.6} SOL", output_sol);
                    let price = if input_amount_display > 0.0 {
                        output_sol / input_amount_display
                    } else {
                        0.0
                    };
                    (input_fmt, output_sol, output_fmt, price)
                };

            // The rate is a platform constant; the AMOUNT is only shown when the
            // router that produced this quote could state it in SOL honestly.
            let platform_fee_pct =
                f64::from(crate::chains::solana::swaps::revenue::PLATFORM_FEE_BPS) / 100.0;
            let platform_fee_native = quote
                .platform_fee_lamports
                .map(|lamports| crate::chains::adapter().raw_to_native(lamports));
            let network_fee_sol = quote
                .estimated_network_fee_lamports
                .map(|lamports| crate::chains::adapter().raw_to_native(lamports));

            // The floor the wallet is guaranteed, as the router enforces it --
            // reconstructing it from the expected output and slippage ignores
            // the fee leg and overstates a sell.
            let minimum_output_amount = if direction == "buy" {
                quote.minimum_output_amount.raw() as f64 / 10f64.powi(token_decimals as i32)
            } else {
                quote.minimum_output_amount.raw() as f64
                    / crate::chains::adapter().raw_units_per_native() as f64
            };

            let response = QuotePreviewResponse {
                success: true,
                router: quote.router_name,
                direction: direction.to_string(),
                input_amount: input_amount_display,
                input_formatted,
                output_amount: output_display,
                minimum_output_amount,
                output_formatted,
                price_per_token_native: price_per_token,
                price_impact_pct: quote.price_impact_pct,
                platform_fee_pct,
                platform_fee_native,
                network_fee_sol,
                route: quote.route_plan,
                slippage_bps: quote.slippage_bps,
                expires_in_secs: 30, // Quotes typically valid for ~30s
            };

            success_response(response)
        }
        Err(e) => {
            // The trade dialog explains WHY a quote couldn't be fetched. Status
            // and message come from the QuoteError variant the router produced,
            // so a provider rewording its response cannot change what the user
            // is told; the message's `hint` attribute is rendered by the
            // dashboard. `details` carries only the technical values: the raw
            // failure for `Unavailable`, and which router was refused and why
            // for a refused quote.
            let text = e.ui_text();
            let details = match &e {
                QuoteError::Unavailable { .. } => Some(e.to_string()),
                QuoteError::RouterRejected { router, detail } => {
                    Some(format!("{router}: {detail}"))
                }
                _ => None,
            };
            let error = ApiError::with_text(ApiErrorCode::for_status(e.http_status()), text);
            match details {
                Some(details) => error.details(details),
                None => error,
            }
            .into_response()
        }
    }
}
