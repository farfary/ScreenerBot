use axum::{extract::Path, http::StatusCode, response::Response};
use chrono::Utc;

use super::list::map_position_to_response_async;
use super::types::*;
use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions;
use crate::sol_price;
use crate::tokens;
use crate::transactions::{get_transaction, TokenTransfer};
use crate::utils::lamports_to_sol;
use crate::webserver::utils::{error_response, success_response};

pub async fn get_position_details(Path(key): Path<String>) -> Response {
    match resolve_position_by_key(&key).await {
        Ok(Some(position)) => {
            let mint = &position.mint;

            // Fetch all data concurrently for better performance
            let (detail, transactions, state_history, (entries, exits)) = tokio::join!(
                map_position_to_detail(&position),
                build_transaction_summaries(&position),
                load_state_history_entries(&position),
                load_entry_exit_history(&position)
            );
            let executions = build_execution_rows(&position);

            // Fetch token data from database
            let token_data = tokens::database::get_full_token_async(mint)
                .await
                .ok()
                .flatten();

            // Build token info from token database
            let token_info = token_data.as_ref().map(|token| {
                let website = token.websites.first().map(|w| w.url.clone());
                let twitter = token
                    .socials
                    .iter()
                    .find(|s| s.link_type.to_lowercase().contains("twitter"))
                    .map(|s| s.url.clone());
                let telegram = token
                    .socials
                    .iter()
                    .find(|s| s.link_type.to_lowercase().contains("telegram"))
                    .map(|s| s.url.clone());

                PositionTokenInfo {
                    decimals: Some(token.decimals),
                    description: token.description.clone(),
                    image_url: token.image_url.clone(),
                    website,
                    twitter,
                    telegram,
                }
            });

            // Build market data from token database
            let market_data = token_data.as_ref().map(|token| PositionMarketData {
                market_cap: token.market_cap,
                fdv: token.fdv,
                liquidity_usd: token.liquidity_usd,
                volume_24h: token.volume_h24,
                price_change_h1: token.price_change_h1,
                price_change_h24: token.price_change_h24,
                holder_count: token.total_holders,
            });

            // Build security summary from token database
            let security = token_data.as_ref().map(|token| {
                // Rugcheck normalized score: 0-100, LOWER = SAFER, HIGHER = RISKIER
                let risk_level = match token.security_score_normalised {
                    Some(score) if score <= 20 => "low".to_string(),
                    Some(score) if score <= 50 => "medium".to_string(),
                    Some(_) => "high".to_string(),
                    None => "unknown".to_string(),
                };

                let top_risks: Vec<String> = token
                    .security_risks
                    .iter()
                    .take(3)
                    .map(|r| r.name.clone())
                    .collect();

                PositionSecuritySummary {
                    score_normalized: token.security_score_normalised,
                    risk_level,
                    has_mint_authority: token.mint_authority.is_some(),
                    has_freeze_authority: token.freeze_authority.is_some(),
                    top_risks,
                }
            });

            // Get pool info from pool service
            let pool_info = pools::get_pool_price(mint).map(|price_result| PositionPoolInfo {
                pool_address: Some(price_result.pool_address.clone()),
                dex_name: price_result.source_pool.clone(),
                liquidity_sol: Some(price_result.sol_reserves),
            });

            // Build external links
            let external_links = ExternalLinks::for_mint(mint);

            // Calculate position age in seconds
            let position_age_seconds = Some(
                Utc::now()
                    .signed_duration_since(position.entry_time)
                    .num_seconds(),
            );

            // Get SOL price in USD
            let sol_price_usd = {
                let price = sol_price::get_sol_price();
                if price > 0.0 {
                    Some(price)
                } else {
                    None
                }
            };

            success_response(PositionDetailResponse {
                position: Some(detail),
                entries,
                exits,
                executions,
                transactions,
                state_history,
                token_info,
                market_data,
                security,
                pool_info,
                external_links,
                position_age_seconds,
                sol_price_usd,
                fetched_at: Utc::now().to_rfc3339(),
            })
        }
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "POSITION_NOT_FOUND",
            "Position not found",
            Some(&format!("No position found for key {key}")),
        ),
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to resolve position for key {}: {}", key, err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "POSITION_DETAIL_ERROR",
                "Failed to load position details",
                Some(&err),
            )
        }
    }
}

async fn resolve_position_by_key(key: &str) -> Result<Option<positions::Position>, String> {
    if let Some(id_part) = key.strip_prefix("id:") {
        let id: i64 = id_part
            .parse()
            .map_err(|_| format!("Invalid position id: {id_part}"))?;
        return Ok(positions::get_position_by_id(id).await);
    }

    if let Some(mint_part) = key.strip_prefix("mint:") {
        return Ok(positions::get_position_by_mint(mint_part).await);
    }

    Ok(positions::get_position_by_mint(key).await)
}

async fn map_position_to_detail(position: &positions::Position) -> PositionDetail {
    PositionDetail {
        summary: map_position_to_response_async(position).await,
        phantom_remove: position.phantom_remove,
        phantom_first_seen: position.phantom_first_seen.map(|dt| dt.timestamp()),
    }
}

fn build_execution_rows(position: &positions::Position) -> Vec<PositionExecutionRow> {
    let mut rows = Vec::with_capacity(2);

    rows.push(PositionExecutionRow {
        kind: "entry".to_string(),
        timestamp: Some(position.entry_time.timestamp()),
        price_sol: Some(position.entry_price),
        effective_price_sol: position.effective_entry_price,
        size_sol: Some(position.entry_size_sol),
        total_size_sol: Some(position.total_size_sol),
        sol_delta: Some(-position.entry_size_sol.abs()),
        token_amount: position.token_amount,
        signature: position.entry_transaction_signature.clone(),
        verified: position.transaction_entry_verified,
        fee_lamports: position.entry_fee_lamports,
        fee_sol: lamports_option_to_sol(position.entry_fee_lamports),
        notes: Some(format!("Position type: {}", position.position_type)),
    });

    let mut exit_notes: Vec<String> = Vec::new();
    if position.synthetic_exit {
        exit_notes.push("Synthetic exit".to_string());
    }
    if let Some(reason) = &position.closed_reason {
        if !reason.is_empty() {
            exit_notes.push(reason.clone());
        }
    }
    if !position.transaction_exit_verified && position.exit_time.is_none() {
        exit_notes.push("Exit pending".to_string());
    }

    rows.push(PositionExecutionRow {
        kind: "exit".to_string(),
        timestamp: position.exit_time.map(|dt| dt.timestamp()),
        price_sol: position.exit_price,
        effective_price_sol: position.effective_exit_price,
        size_sol: Some(position.entry_size_sol),
        total_size_sol: Some(position.total_size_sol),
        sol_delta: position.sol_received,
        token_amount: position.token_amount,
        signature: position.exit_transaction_signature.clone(),
        verified: position.transaction_exit_verified,
        fee_lamports: position.exit_fee_lamports,
        fee_sol: lamports_option_to_sol(position.exit_fee_lamports),
        notes: if exit_notes.is_empty() {
            None
        } else {
            Some(exit_notes.join(" · "))
        },
    });

    rows
}

async fn build_transaction_summaries(
    position: &positions::Position,
) -> Vec<PositionTransactionSummary> {
    let entry_sig = position.entry_transaction_signature.clone();
    let exit_sig = position.exit_transaction_signature.clone();

    let mut summaries = Vec::with_capacity(2);
    summaries.push(fetch_transaction_summary("entry", entry_sig, position).await);
    summaries.push(fetch_transaction_summary("exit", exit_sig, position).await);
    summaries
}

async fn fetch_transaction_summary(
    kind: &str,
    signature: Option<String>,
    position: &positions::Position,
) -> PositionTransactionSummary {
    match signature {
        Some(sig) => match get_transaction(&sig).await {
            Ok(Some(tx)) => PositionTransactionSummary::from_transaction(kind, sig, &tx, position),
            Ok(None) => PositionTransactionSummary::missing(
                kind,
                Some(sig),
                Some("Transaction not available in cache".to_string()),
            ),
            Err(err) => {
                logger::info(
                    LogTag::Webserver,
                    &format!("Failed to load {} transaction {}: {}", kind, sig, err),
                );
                PositionTransactionSummary::missing(kind, Some(sig), Some(err))
            }
        },
        None => {
            let note = if kind == "exit" && position.synthetic_exit {
                "Synthetic exit - no signature".to_string()
            } else {
                "Signature not recorded".to_string()
            };
            PositionTransactionSummary::missing(kind, None, Some(note))
        }
    }
}

async fn load_state_history_entries(
    position: &positions::Position,
) -> Vec<PositionStateTimelineEntry> {
    let Some(id) = position.id else {
        return Vec::new();
    };

    let db_arc = match positions::get_positions_database().await {
        Ok(db) => db,
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!(
                    "Failed to access positions database for position {}: {}",
                    id, err
                ),
            );
            return Vec::new();
        }
    };

    let db_clone = {
        let db_guard = db_arc.lock().await;
        db_guard.clone()
    };

    let Some(db) = db_clone else {
        logger::info(
            LogTag::Webserver,
            &format!(
                "Positions database not initialized when loading history for position {}",
                id
            ),
        );
        return Vec::new();
    };

    match db.get_position_state_history(id).await {
        Ok(history) => history
            .into_iter()
            .map(|entry| PositionStateTimelineEntry {
                state: entry.state.to_string(),
                changed_at: entry.changed_at.timestamp(),
                reason: entry.reason,
            })
            .collect(),
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to load state history for position {}: {}", id, err),
            );
            Vec::new()
        }
    }
}

/// Load entry and exit history for a position
async fn load_entry_exit_history(
    position: &positions::Position,
) -> (Vec<EntryRecordResponse>, Vec<ExitRecordResponse>) {
    let Some(id) = position.id else {
        return (Vec::new(), Vec::new());
    };

    // Load entries
    let entries = match positions::get_entry_history(id).await {
        Ok(records) => records
            .into_iter()
            .map(|r| EntryRecordResponse {
                id: r.id,
                timestamp: r.timestamp.timestamp(),
                amount: r.amount,
                price: r.price,
                sol_spent: r.sol_spent,
                transaction_signature: r.transaction_signature,
                is_dca: r.is_dca,
                fees_sol: r.fees_lamports.map(lamports_to_sol),
            })
            .collect(),
        Err(err) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to load entry history for position {}: {}", id, err),
            );
            Vec::new()
        }
    };

    // Load exits
    let exits = match positions::get_exit_history(id).await {
        Ok(records) => records
            .into_iter()
            .map(|r| ExitRecordResponse {
                id: r.id,
                timestamp: r.timestamp.timestamp(),
                amount: r.amount,
                price: r.price,
                sol_received: r.sol_received,
                transaction_signature: r.transaction_signature,
                is_partial: r.is_partial,
                percentage: r.percentage,
                fees_sol: r.fees_lamports.map(lamports_to_sol),
            })
            .collect(),
        Err(err) => {
            logger::debug(
                LogTag::Webserver,
                &format!("Failed to load exit history for position {}: {}", id, err),
            );
            Vec::new()
        }
    };

    (entries, exits)
}

fn lamports_option_to_sol(value: Option<u64>) -> Option<f64> {
    value.map(lamports_to_sol)
}
