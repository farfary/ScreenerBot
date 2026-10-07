// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position verifier — confirms transaction success and updates position state accordingly.

use super::{
    db::{get_other_open_held, OtherOpenHeld},
    queue::VerificationItem,
    round_state::{attributable_held, is_dust},
    settle,
    state::get_position_by_id,
    transitions::PositionTransition,
    types::{VerificationKind, VerificationOutcome},
};
use crate::{
    chains::adapter,
    chains::RawAmount,
    logger::{self, LogTag},
    tokens::get_decimals,
    transactions::{get_transaction, reprocess_transaction, TransactionStatus},
    utils::get_wallet_address,
};
use chrono::Utc;
use std::sync::LazyLock;
use std::time::Duration;

// Throttle repeated token accounts queries per mint to reduce RPC pressure
const TOKEN_ACCOUNTS_THROTTLE_SECS: i64 = 5; // min interval per mint between balance checks

/// Bounded cache for last token accounts check timestamps (max 5K entries, 1h TTL).
static LAST_TOKEN_ACCOUNTS_CHECK: LazyLock<moka::sync::Cache<String, chrono::DateTime<Utc>>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(5_000)
            .time_to_live(Duration::from_secs(3600))
            .build()
    });

async fn should_throttle_token_accounts(mint: &str) -> bool {
    let now = Utc::now();
    if let Some(last) = LAST_TOKEN_ACCOUNTS_CHECK.get(&mint.to_string()) {
        if (now - last).num_seconds() < TOKEN_ACCOUNTS_THROTTLE_SECS {
            return true;
        }
    }
    LAST_TOKEN_ACCOUNTS_CHECK.insert(mint.to_string(), now);
    false
}

/// Classify transient (retryable) verification errors
fn is_transient_verification_error(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("within propagation grace")
        || m.contains("still pending")
        || m.contains("within propagation")
        || m.contains("not found in system")
        || m.contains("will retry")
        || m.contains("no valid swap analysis")
        || m.contains("error getting transaction")
        || m.contains("transaction manager not available")
        || m.contains("transaction manager not initialized")
        || m.contains("transaction not found (propagation)")
        || m.contains("not yet indexed")
        || m.contains("transaction not found")
        || m.contains("failed to fetch transaction details")
        || m.contains("rpc error")
        || m.contains("transaction not available")
        || m.contains("blockchain transaction not found")
}

/// Whether `balance`, the wallet's holding of the mint after a full exit of the position,
/// leaves a residual of the position's own that another close must sell. A balance that is
/// dust on its own needs no attribution. While another open position of the mint has an
/// unverified entry, the balance may be its tokens, and a retried close sells the wallet's
/// holding: the residual is never taken for the position's own, so the close is booked
/// instead of retried. Fails only while the other positions' holding cannot be read.
pub async fn residual_balance_requires_retry(
    position_id: Option<i64>,
    balance: RawAmount,
) -> super::Result<bool> {
    if balance == RawAmount::ZERO {
        return Ok(false);
    }

    if let Some(pid) = position_id {
        if let Some(position) = get_position_by_id(pid).await {
            // Dust is measured against what the position acquired: what it still holds plus
            // what it already sold. Partial exits shrink the remainder, so a remainder-based
            // threshold drops exactly when a leftover is most clearly dust. Only the part of
            // the balance the other open positions of the mint do not hold is this
            // position's residual: retrying the close on theirs would sell their tokens.
            if let Some(held) = position
                .remaining_token_amount
                .filter(|remaining| *remaining > RawAmount::ZERO)
                .or(position.token_amount)
            {
                let acquired = held
                    .checked_add(position.total_exited_amount)
                    .unwrap_or(RawAmount::new(u128::MAX));
                if is_dust(balance, acquired) {
                    return Ok(false);
                }
                let others = match get_other_open_held(&position.mint, position.id).await? {
                    OtherOpenHeld::Booked(others) => others,
                    OtherOpenHeld::Unattributable { position_id } => {
                        logger::warning(
                            LogTag::Positions,
                            &format!(
                                "Residual balance {balance} for position {pid} is not retried: the entry of position {position_id} of the mint is not verified, so the balance may be its tokens"
                            ),
                        );
                        return Ok(false);
                    }
                };
                let residual = attributable_held(balance, others, acquired);
                if is_dust(residual, acquired) {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Ignoring residual balance {balance} for position {pid}: {residual} of it is its own (acquired {acquired})"
                        ),
                    );
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}

/// Verify a transaction and produce the appropriate transition. A transaction that is not
/// found, failed or is unreadable is retried: whether its swap failed on chain or never
/// landed is decided only by the settlement sweep's signature verdict
/// (`settle::disposition`).
pub async fn verify_transaction(item: &VerificationItem) -> VerificationOutcome {
    logger::debug(
        LogTag::Positions,
        &format!("Verifying transaction: {}", item.signature),
    );

    // Get transaction
    let transaction = match get_transaction(&item.signature).await {
        Ok(Some(tx)) => {
            if !tx.success {
                return VerificationOutcome::RetryTransient(format!(
                    "Transaction failed: {}",
                    tx.error_message.unwrap_or("Unknown error".to_owned())
                ));
            }

            match tx.status {
                TransactionStatus::Finalized | TransactionStatus::Confirmed => tx,
                TransactionStatus::Pending => {
                    return VerificationOutcome::RetryTransient(
                        "Transaction still pending".to_owned(),
                    );
                }
                TransactionStatus::Failed(err) => {
                    return VerificationOutcome::RetryTransient(format!(
                        "Transaction failed: {}",
                        err
                    ));
                }
            }
        }
        Ok(None) => {
            return VerificationOutcome::RetryTransient("Transaction not found".to_owned());
        }
        Err(e) => {
            let error_msg = format!("Error getting transaction: {e}");

            // Enhanced error classification for immediate verification optimization
            if error_msg.to_lowercase().contains("not found")
                || error_msg.to_lowercase().contains("not yet indexed")
                || error_msg.to_lowercase().contains("rpc error")
            {
                logger::debug(
                    LogTag::Positions,
                    &format!("RPC indexing delay for {}: {}", item.signature, error_msg),
                );
                return VerificationOutcome::RetryTransient(format!(
                    "RPC indexing delay: {}",
                    error_msg
                ));
            }

            if is_transient_verification_error(&error_msg) {
                return VerificationOutcome::RetryTransient(error_msg);
            } else {
                return VerificationOutcome::RetryTransient(error_msg); // Be conservative with RPC errors
            }
        }
    };

    // Get swap analysis. The transaction may have been stored at submit time
    // (before it landed) without swap analysis, and the fallback skips already-known
    // signatures — so swap_pnl_info can be missing for a fully-confirmed exit. In that
    // case re-process the signature from RPC (fetch + analyze + persist) so we obtain
    // real proceeds and can finalize, rather than looping forever.
    let swap_info = if let Some(pnl) = transaction.swap_pnl_info.clone() {
        pnl
    } else {
        match reprocess_transaction(&item.signature).await {
            Ok(Some(reanalyzed)) => match reanalyzed.swap_pnl_info.clone() {
                Some(pnl) => {
                    logger::info(
                        LogTag::Positions,
                        &format!(
                            "Re-analyzed {} on demand to recover swap proceeds for finalization",
                            item.signature
                        ),
                    );
                    pnl
                }
                None => {
                    return VerificationOutcome::RetryTransient(
                        "No valid swap analysis (re-analysis produced none)".to_owned(),
                    );
                }
            },
            Ok(None) | Err(_) => {
                return VerificationOutcome::RetryTransient(
                    "No valid swap analysis (re-analysis unavailable)".to_owned(),
                );
            }
        }
    };

    // Verify token mint matches
    if swap_info.token_mint != item.mint {
        return VerificationOutcome::RetryTransient("Token mint mismatch".to_owned());
    }

    let Some(position_id) = item.position_id else {
        return VerificationOutcome::RetryTransient("Verification carries no position".to_owned());
    };

    // The wallet's holding of the mint after the swap, read once. A swap that lands on a
    // closed position needs it to decide whether the position reopens, an entry caps its
    // amount by it and a full exit checks it for a residual.
    if should_throttle_token_accounts(&item.mint).await {
        logger::debug(
            LogTag::Positions,
            &format!("Throttling token accounts check for mint {}", item.mint),
        );
        return VerificationOutcome::RetryTransient("Token accounts check throttled".to_owned());
    }
    let held_after = match get_wallet_address() {
        Ok(wallet_address) => settle::holding(&wallet_address, &item.mint)
            .await
            .map(|holding| holding.amount)
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    if let Err(e) = &held_after {
        logger::warning(
            LogTag::Positions,
            &format!(
                "Holding of {} after {} unavailable: {e}",
                item.mint, item.signature
            ),
        );
    }

    match item.kind {
        VerificationKind::Entry => {
            if swap_info.swap_type != "Buy" {
                return VerificationOutcome::RetryTransient("Expected Buy transaction".to_owned());
            }

            // Convert token amount to integer units with rounding
            let decimals = match get_decimals(crate::chains::active_chain(), &item.mint).await {
                Some(dec) => dec,
                None => {
                    return VerificationOutcome::RetryTransient(
                        "Token decimals not cached".to_owned(),
                    );
                }
            };

            let scale = (10_f64).powi(decimals as i32);
            let mut token_amount_units =
                (swap_info.token_amount.abs() * scale).round().max(0.0) as u64;

            if token_amount_units == 0 {
                return VerificationOutcome::RetryTransient(
                    "Zero token amount detected".to_owned(),
                );
            }

            if item.is_dca {
                let native_spent = swap_info.effective_sol_spent.abs();
                if native_spent <= 0.0 || !native_spent.is_finite() {
                    return VerificationOutcome::RetryTransient(
                        "Invalid SOL spent reported for DCA".to_owned(),
                    );
                }

                let token_amount_float = (token_amount_units as f64) / scale;
                if token_amount_float <= 0.0 || !token_amount_float.is_finite() {
                    return VerificationOutcome::RetryTransient(
                        "Invalid token amount computed for DCA".to_owned(),
                    );
                }

                let effective_price = native_spent / token_amount_float;
                let dca_time = if let Some(block_time) = transaction.block_time {
                    chrono::DateTime::<Utc>::from_timestamp(block_time, 0)
                        .unwrap_or_else(|| Utc::now())
                } else {
                    Utc::now()
                };

                return VerificationOutcome::Transition(PositionTransition::DcaVerified {
                    position_id,
                    tokens_bought: RawAmount::from(token_amount_units),
                    native_spent,
                    effective_price,
                    fee_raw: adapter().native_to_raw(swap_info.fee_sol),
                    dca_time,
                    dca_signature: item.signature.clone(),
                    held_after: held_after.ok(),
                });
            }

            // Prefer the authoritative on-chain balance right after the entry, but only ever
            // REDUCE the transaction-derived amount to it: a larger wallet balance may include
            // later buys, and attributing them to this entry would merge duplicate buys.
            if let Ok(Ok(actual_units)) = held_after.as_ref().map(|held| u64::try_from(*held)) {
                if actual_units > 0 && actual_units < token_amount_units {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Reduced token units to on-chain balance for mint {}: tx-derived={} actual={}",
                            &item.mint, token_amount_units, actual_units
                        ),
                    );
                    token_amount_units = actual_units;
                }
            }

            // Calculate effective entry price using authoritative units when possible
            let effective_price = if token_amount_units > 0 && swap_info.effective_sol_spent > 0.0 {
                let token_amount_float = (token_amount_units as f64) / scale;
                if token_amount_float > 0.0 && token_amount_float.is_finite() {
                    swap_info.effective_sol_spent / token_amount_float
                } else {
                    swap_info.calculated_price_sol
                }
            } else {
                swap_info.calculated_price_sol
            };

            VerificationOutcome::Transition(PositionTransition::EntryVerified {
                position_id,
                effective_entry_price: effective_price,
                token_amount_units: RawAmount::from(token_amount_units),
                fee_raw: adapter().native_to_raw(swap_info.fee_sol),
                native_size: swap_info.sol_amount,
                held_after: held_after.ok(),
            })
        }
        VerificationKind::Exit => {
            if swap_info.swap_type != "Sell" {
                return VerificationOutcome::RetryTransient("Expected Sell transaction".to_owned());
            }

            let exit_time = if let Some(block_time) = transaction.block_time {
                chrono::DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now())
            } else {
                Utc::now()
            };

            // Calculate exit amount from transaction
            let exit_amount = if let Some(decimals) =
                get_decimals(crate::chains::active_chain(), &item.mint).await
            {
                let scale = (10_f64).powi(decimals as i32);
                let units = (swap_info.token_amount.abs() * scale).round();
                units.max(0.0) as u64
            } else {
                return VerificationOutcome::RetryTransient(
                    "Token decimals not cached for exit".to_owned(),
                );
            };

            // A PARTIAL exit expects a remaining balance; a FULL exit must leave none beyond
            // dust. A full exit cannot be booked without that check.
            let remaining_balance = match held_after {
                Ok(remaining_balance) => remaining_balance,
                Err(e) if !item.is_partial_exit => {
                    logger::warning(
                        LogTag::Positions,
                        &format!("Could not verify residual balance after exit: {e}"),
                    );
                    return VerificationOutcome::RetryTransient(
                        "Residual check failed after exit".to_owned(),
                    );
                }
                Err(_) => RawAmount::ZERO,
            };
            let held_after = held_after.ok();

            if item.is_partial_exit {
                // The transaction is FINAL: what it sold is what it sold. Retrying
                // verification on a mismatch can never change the answer, so the amount
                // that actually executed is recorded and the discrepancy logged.
                if let Some(expected) = item.expected_exit_amount {
                    let expected = expected.raw();
                    let actual = u128::from(exit_amount);
                    let tolerance = (expected / 1000).max(10); // 0.1% tolerance or 10 units
                    if actual < expected.saturating_sub(tolerance)
                        || actual > expected.saturating_add(tolerance)
                    {
                        logger::warning(
                            LogTag::Positions,
                            &format!(
                                "Partial exit amount mismatch for mint {}: expected={} actual={} tolerance={} - recording the ACTUAL amount",
                                item.mint, expected, exit_amount, tolerance
                            ),
                        );
                    }
                }

                if exit_amount == 0 {
                    return VerificationOutcome::RetryTransient(
                        "Partial exit sold zero tokens - will verify again".to_owned(),
                    );
                }

                logger::info(
                    LogTag::Positions,
                    &format!(
                        "Partial exit verified for mint {}: sold={} remaining={:?}",
                        item.mint, exit_amount, held_after
                    ),
                );

                return VerificationOutcome::Transition(PositionTransition::PartialExitVerified {
                    position_id,
                    exit_amount: RawAmount::from(exit_amount),
                    native_received: swap_info.effective_sol_received.abs(),
                    effective_exit_price: swap_info.calculated_price_sol,
                    fee_raw: adapter().native_to_raw(swap_info.fee_sol),
                    exit_time,
                    exit_signature: item.signature.clone(),
                    exit_percentage: match (
                        item.expected_exit_amount,
                        item.requested_exit_percentage,
                    ) {
                        (Some(expected), Some(requested)) if expected > RawAmount::ZERO => {
                            let ratio = exit_amount as f64 / expected.raw() as f64;
                            (requested * ratio).clamp(0.0, 100.0)
                        }
                        (Some(expected), _) if expected > RawAmount::ZERO => {
                            ((exit_amount as f64 / expected.raw() as f64) * 100.0)
                                .max(0.0)
                                .min(100.0)
                        }
                        (Some(_), _) => 0.0,
                        (None, _) => 100.0,
                    },
                    held_after,
                });
            }

            // FULL EXIT: Ensure complete closure (check for residual)
            let residual_requires_retry = match residual_balance_requires_retry(
                item.position_id,
                remaining_balance,
            )
            .await
            {
                Ok(requires_retry) => requires_retry,
                Err(error) => {
                    return VerificationOutcome::RetryTransient(format!(
                            "Exit residual {remaining_balance} for mint {} cannot be read against the other positions yet: {error}",
                            item.mint
                        ));
                }
            };
            if residual_requires_retry {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Exit residual {} units for mint {} → will retry another close",
                        remaining_balance, item.mint
                    ),
                );

                crate::events::record_position_event_flexible(
                    "exit_residual_detected",
                    crate::events::Severity::Warn,
                    Some(&item.mint),
                    item.position_id.map(|id| id.to_string()).as_deref(),
                    serde_json::json!({
                      "position_id": item.position_id,
                      "remaining_balance": remaining_balance,
                      "sold": exit_amount
                    }),
                )
                .await;

                // The swap SUCCEEDED: it sold `exit_amount` tokens and received SOL, it just
                // did not empty the wallet (typically tokens split across accounts, where
                // close_position_direct sells the primary ATA only). The fill is recorded as
                // a partial exit, then the exit is cleared for a retry of the residual.
                let sold_pct = {
                    let total = remaining_balance.raw() as f64 + exit_amount as f64;
                    if total > 0.0 {
                        ((exit_amount as f64 / total) * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    }
                };

                return VerificationOutcome::Transition(
                    PositionTransition::ExitResidualClearForRetry {
                        position_id,
                        exit_amount: RawAmount::from(exit_amount),
                        native_received: swap_info.effective_sol_received.abs(),
                        effective_exit_price: swap_info.calculated_price_sol,
                        fee_raw: adapter().native_to_raw(swap_info.fee_sol),
                        exit_time,
                        exit_signature: item.signature.clone(),
                        exit_percentage: sold_pct,
                        held_after,
                    },
                );
            }
            logger::info(
                LogTag::Positions,
                &format!("Exit verified with zero residual for mint {}", item.mint),
            );

            // FULL EXIT: Standard verification
            VerificationOutcome::Transition(PositionTransition::ExitVerified {
                position_id,
                effective_exit_price: swap_info.calculated_price_sol,
                native_received: swap_info.effective_sol_received.abs(),
                fee_raw: adapter().native_to_raw(swap_info.fee_sol),
                exit_time,
                exit_signature: item.signature.clone(),
                exit_amount: RawAmount::from(exit_amount),
                held_after,
            })
        }
    }
}
