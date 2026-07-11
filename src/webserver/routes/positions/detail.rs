//! Position detail route — serves detailed data for a single position view.

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
            let (detail, state_history, (entries, exits)) = tokio::join!(
                map_position_to_detail(&position),
                load_state_history_entries(&position),
                load_entry_exit_history(&position)
            );

            // Transaction summaries are built FROM the entry/exit records, so they must
            // wait for them (local SQLite reads — cheap).
            let transactions = build_transaction_summaries(&position, &entries, &exits).await;

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
                    Some(score) if score <= 20 => "low".to_owned(),
                    Some(score) if score <= 50 => "medium".to_owned(),
                    Some(_) => "high".to_owned(),
                    None => "unknown".to_owned(),
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
                &format!("Failed to resolve position for key {key}: {err}"),
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

/// Every on-chain transaction belonging to a position: the entry, every DCA add, every
/// partial exit, and the final close.
///
/// This used to emit exactly TWO summaries, built from `entry_transaction_signature` and
/// `exit_transaction_signature`. A position's other swaps — DCA adds and partial exits —
/// have their own signatures and live only in the entry/exit RECORDS, so they never
/// appeared, even though the Transactions tab already knows how to label them ("DCA
/// Entry" / "Partial Exit" — those branches were simply unreachable).
///
/// Built from the records, which are the authoritative per-swap history, plus:
///   - the position's own signatures, for a swap that is submitted but not yet verified
///     (no record written yet) and for legacy positions that predate the records;
///   - partial exits still confirming, whose signature lives only in the pending registry
///     until verification writes the record.
async fn build_transaction_summaries(
    position: &positions::Position,
    entries: &[EntryRecordResponse],
    exits: &[ExitRecordResponse],
) -> Vec<PositionTransactionSummary> {
    // (kind, signature), newest last — de-duplicated, first kind wins.
    let mut planned: Vec<(&str, String)> = Vec::new();
    let mut push = |kind: &'static str, signature: Option<String>, planned: &mut Vec<_>| {
        let Some(signature) = signature else { return };
        if signature.is_empty() {
            return;
        }
        if planned
            .iter()
            .any(|(_, existing): &(&str, String)| *existing == signature)
        {
            return;
        }
        planned.push((kind, signature));
    };

    push(
        "entry",
        position.entry_transaction_signature.clone(),
        &mut planned,
    );
    for entry in entries {
        let kind = if entry.is_dca { "dca" } else { "entry" };
        push(
            kind,
            Some(entry.transaction_signature.clone()),
            &mut planned,
        );
    }
    for exit in exits {
        let kind = if exit.is_partial {
            "partial_exit"
        } else {
            "exit"
        };
        push(kind, Some(exit.transaction_signature.clone()), &mut planned);
    }
    for pending in positions::get_pending_partial_exits_for_mint(&position.mint).await {
        push("partial_exit", Some(pending.signature), &mut planned);
    }
    push(
        "exit",
        position.exit_transaction_signature.clone(),
        &mut planned,
    );

    let mut summaries = Vec::with_capacity(planned.len().max(2));
    for (kind, signature) in planned {
        summaries.push(fetch_transaction_summary(kind, Some(signature), position).await);
    }

    // Placeholder exit row ONLY for a position that has actually exited without a usable
    // signature (a synthetic / force close). An open position that merely took partial
    // profits has exit RECORDS but has not exited — it must not show an empty Exit row.
    let has_exited = position.exit_time.is_some() || position.synthetic_exit;
    if has_exited && !summaries.iter().any(|s| s.kind == "exit") {
        summaries.push(fetch_transaction_summary("exit", None, position).await);
    }

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
                Some("Transaction not available in cache".to_owned()),
            ),
            Err(err) => {
                logger::info(
                    LogTag::Webserver,
                    &format!("Failed to load {kind} transaction {sig}: {err}"),
                );
                PositionTransactionSummary::missing(kind, Some(sig), Some(err))
            }
        },
        None => {
            let note = if kind == "exit" && position.synthetic_exit {
                "Synthetic exit - no signature".to_owned()
            } else {
                "Signature not recorded".to_owned()
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
                &format!("Failed to load state history for position {id}: {err}"),
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
                &format!("Failed to load entry history for position {id}: {err}"),
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
                &format!("Failed to load exit history for position {id}: {err}"),
            );
            Vec::new()
        }
    };

    (entries, exits)
}
