//! Manual trading operations

use axum::{extract::Query, http::StatusCode, response::Response, Json};

use crate::config::with_config;
use crate::errors::ErrorClass;
use crate::logger::{self, LogTag};
use crate::swaps::{try_get_best_quote, QuoteError};
use crate::trader::manual::guard::{self, BlacklistPolicy, ManualTradeKind};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// MANUAL TRADING HANDLERS
// =============================================================================

/// Map a refused or failed manual trade to its response; the codes are the ones
/// the dashboard trade dialog has always received.
fn manual_error(fallback_code: &str, error: &crate::trader::Error) -> Response {
    use crate::trader::Error;
    let code = match error {
        Error::ForceStopped => "ForceStopped",
        Error::CoreServicesNotReady { .. } => "CoreServicesNotReady",
        Error::InvalidMint { .. } => "InvalidMint",
        Error::Blacklisted { .. } => "Blacklisted",
        Error::NoOpenPosition { .. } => "NoOpenPosition",
        Error::InvalidSlippage { .. } => "InvalidSlippage",
        Error::InvalidPercentage { .. } => "InvalidPercentage",
        _ => fallback_code,
    };
    let status =
        StatusCode::from_u16(error.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    error_response(status, code, &error.to_string(), None)
}

fn trade_response(
    result: Result<crate::trader::TradeResult, crate::trader::Error>,
    mint: String,
    failed_code: &str,
    error_code: &str,
    message: String,
) -> Response {
    match result {
        Ok(tr) if !tr.success => error_response(
            StatusCode::BAD_REQUEST,
            failed_code,
            tr.error.as_deref().unwrap_or("Manual trade failed"),
            None,
        ),
        Ok(tr) => success_response(ManualTradeSuccess {
            success: true,
            mint,
            signature: tr.tx_signature,
            effective_price_sol: tr.executed_price_sol,
            size_sol: tr.executed_size_sol,
            position_id: tr.position_id,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
        Err(error) => manual_error(error_code, &error),
    }
}

pub async fn manual_buy_handler(Json(req): Json<ManualBuyRequest>) -> Response {
    let blacklist = if req.force.unwrap_or_default() {
        BlacklistPolicy::Override
    } else {
        BlacklistPolicy::Enforce
    };
    if let Err(error) = guard::preflight(ManualTradeKind::Buy, &req.mint, blacklist).await {
        return manual_error("ManualBuyError", &error);
    }
    let slippage_pct = match guard::validate_slippage(req.slippage_pct) {
        Ok(v) => v,
        Err(error) => return manual_error("ManualBuyError", &error),
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
    trade_response(
        result,
        req.mint,
        "ManualBuyFailed",
        "ManualBuyError",
        "Manual buy executed".to_owned(),
    )
}

pub async fn manual_add_handler(Json(req): Json<ManualAddRequest>) -> Response {
    if let Err(error) =
        guard::preflight(ManualTradeKind::Add, &req.mint, BlacklistPolicy::Enforce).await
    {
        return manual_error("ManualAddError", &error);
    }
    let slippage_pct = match guard::validate_slippage(req.slippage_pct) {
        Ok(v) => v,
        Err(error) => return manual_error("ManualAddError", &error),
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
    trade_response(
        result,
        req.mint,
        "ManualAddFailed",
        "ManualAddError",
        "Added to position".to_owned(),
    )
}

pub async fn manual_sell_handler(Json(req): Json<ManualSellRequest>) -> Response {
    if let Err(error) =
        guard::preflight(ManualTradeKind::Sell, &req.mint, BlacklistPolicy::Ignore).await
    {
        return manual_error("ManualSellError", &error);
    }
    let pct = if req.close_all.unwrap_or_default() {
        None // Full exit (100%)
    } else {
        let requested = req
            .percentage
            .unwrap_or_else(|| with_config(|cfg| cfg.positions.partial_exit_default_pct));
        match guard::validate_percentage(requested) {
            Ok(pct) => Some(pct),
            Err(error) => return manual_error("ManualSellError", &error),
        }
    };
    let slippage_pct = match guard::validate_slippage(req.slippage_pct) {
        Ok(v) => v,
        Err(error) => return manual_error("ManualSellError", &error),
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
    let message = match pct {
        Some(pct) if pct < 100.0 => format!("Partial position closed ({pct}%)"),
        _ => "Full position closed".to_owned(),
    };
    trade_response(
        result,
        req.mint,
        "ManualSellFailed",
        "ManualSellError",
        message,
    )
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
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "NoTokensInWallet",
                "No tokens found in wallet for this position",
                Some("Token balance is 0; the position cannot be closed via swap"),
            );
        }

        // Percentage of the real balance (default = full close). Legacy callers may
        // still send amount_tokens (whole tokens); honour it only as a fallback.
        let amount_raw = if let Some(pct) = req.percentage {
            if !pct.is_finite() || pct <= 0.0 || pct > 100.0 {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "InvalidAmount",
                    "percentage must be in (0, 100]",
                    None,
                );
            }
            ((actual_balance as f64) * pct / 100.0).floor() as u64
        } else if let Some(tokens) = req.amount_tokens {
            if !tokens.is_finite() || tokens <= 0.0 {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "InvalidAmount",
                    "amount_tokens must be positive",
                    None,
                );
            }
            // Clamp to the real balance so we never quote more than exists.
            ((tokens * 10f64.powi(token_decimals as i32)) as u64).min(actual_balance)
        } else {
            actual_balance // No amount given → quote a full close.
        };

        if amount_raw == 0 {
            return error_response(
                StatusCode::BAD_REQUEST,
                "InvalidAmount",
                "Computed sell amount is zero",
                None,
            );
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
        input_amount,
        wallet_address,
        // Price the preview at the slippage the trade will actually use, so the quote
        // the user confirms is the quote they get.
        slippage_pct: match guard::validate_slippage(req.slippage_pct) {
            Ok(Some(pct)) => pct,
            Ok(None) => with_config(|cfg| cfg.swaps.slippage.quote_default_pct),
            Err(error) => return manual_error("InvalidSlippage", &error),
        },
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };

    // Fetch quote
    match try_get_best_quote(quote_request).await {
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
                let output_sol = crate::chains::adapter().raw_to_native(quote.output_amount);
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
            let platform_fee_sol = quote
                .platform_fee_lamports
                .map(|lamports| crate::chains::adapter().raw_to_native(lamports));
            let network_fee_sol = quote
                .estimated_network_fee_lamports
                .map(|lamports| crate::chains::adapter().raw_to_native(lamports));

            // The floor the wallet is guaranteed, as the router enforces it --
            // reconstructing it from the expected output and slippage ignores
            // the fee leg and overstates a sell.
            let minimum_output_amount = if direction == "buy" {
                quote.minimum_output_amount as f64 / 10f64.powi(token_decimals as i32)
            } else {
                crate::chains::adapter().raw_to_native(quote.minimum_output_amount)
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
        Err(e) => {
            // The trade dialog explains WHY a quote couldn't be fetched. Every
            // part of that answer — status, code, headline, hint — comes from
            // the QuoteError variant the router produced, so a provider
            // rewording its response cannot change what the user is told.
            // `Unavailable` is the only case that shows the raw detail, and
            // even then only as supporting text.
            let status =
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let details = match &e {
                QuoteError::Unavailable { .. } => e.to_string(),
                _ => e.hint().to_owned(),
            };
            error_response(status, e.code(), e.title(), Some(&details))
        }
    }
}
