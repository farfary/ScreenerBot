use super::{
    queue::{VerificationItem, VerificationKind},
    state::{get_mint_by_signature, get_position_by_id},
    transitions::PositionTransition,
};
use crate::{
    arguments::is_debug_positions_enabled,
    logger::{self, LogTag},
    tokens::get_decimals,
    transactions::{
        get_global_transaction_manager, get_transaction, Transaction, TransactionStatus,
    },
    utils::{get_token_balance, get_total_token_balance, get_wallet_address, sol_to_lamports},
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;

// Throttle repeated token accounts queries per mint to reduce RPC pressure
const TOKEN_ACCOUNTS_THROTTLE_SECS: i64 = 5; // min interval per mint between balance checks

static LAST_TOKEN_ACCOUNTS_CHECK: LazyLock<RwLock<HashMap<String, chrono::DateTime<Utc>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

async fn should_throttle_token_accounts(mint: &str) -> bool {
    let now = Utc::now();
    {
        let map = LAST_TOKEN_ACCOUNTS_CHECK.read().await;
        if let Some(last) = map.get(mint) {
            if (now - *last).num_seconds() < TOKEN_ACCOUNTS_THROTTLE_SECS {
                return true;
            }
        }
    }
    // Upgrade to write: record this check time
    {
        let mut map = LAST_TOKEN_ACCOUNTS_CHECK.write().await;
        map.insert(mint.to_string(), now);
    }
    false
}
use serde_json::Value;

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

#[derive(Debug)]
pub enum VerificationOutcome {
    Transition(PositionTransition),
    RetryTransient(String),
    PermanentFailure(PositionTransition),
}

async fn residual_balance_requires_retry(position_id: Option<i64>, balance: u64) -> bool {
    if balance == 0 {
        return false;
    }

    if let Some(pid) = position_id {
        if let Some(position) = get_position_by_id(pid).await {
            if let Some(token_amount) = position.token_amount {
                let dust_threshold = std::cmp::max(token_amount / 1_000, 10);
                if balance <= dust_threshold {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Ignoring residual dust balance {} (threshold {} tokens) for position {}",
                            balance,
                            dust_threshold,
                            pid
                        ),
                    );
                    return false;
                }
            }
        }
    }

    true
}

/// Verify a transaction and produce the appropriate transition
pub async fn verify_transaction(item: &VerificationItem) -> VerificationOutcome {
        logger::debug(
        LogTag::Positions,
        &format!("🔍 Verifying transaction: {}", item.signature),
        );

    // Get transaction
    let transaction = match get_transaction(&item.signature).await {
        Ok(Some(tx)) => {
            if !tx.success {
                let error_msg = tx.error_message.unwrap_or("Unknown error".to_string());
                if error_msg.contains("[PERMANENT]") {
                    return match item.kind {
                        VerificationKind::Entry => VerificationOutcome::PermanentFailure(
                            PositionTransition::RemoveOrphanEntry {
                                position_id: item.position_id.unwrap_or(0),
                            },
                        ),
                        VerificationKind::Exit => {
                            // For exit permanent failures, check wallet balance
                            if let (Ok(wallet_address), Some(position_id)) =
                                (get_wallet_address(), item.position_id)
                            {
                                match get_total_token_balance(&wallet_address, &item.mint).await {
                                    Ok(balance) => {
                                        if residual_balance_requires_retry(
                                            Some(position_id),
                                            balance,
                                        )
                                        .await
                                        {
                                            VerificationOutcome::Transition(
                                                PositionTransition::ExitFailedClearForRetry {
                                                    position_id,
                                                },
                                            )
                                        } else {
                                            VerificationOutcome::PermanentFailure(
                                                PositionTransition::ExitPermanentFailureSynthetic {
                                                    position_id,
                                                    exit_time: Utc::now(),
                                                },
                                            )
                                        }
                                    }
                                    Err(_) => VerificationOutcome::PermanentFailure(
                                        PositionTransition::ExitPermanentFailureSynthetic {
                                            position_id,
                                            exit_time: Utc::now(),
                                        },
                                    ),
                                }
                            } else {
                                VerificationOutcome::PermanentFailure(
                                    PositionTransition::ExitPermanentFailureSynthetic {
                                        position_id: item.position_id.unwrap_or(0),
                                        exit_time: Utc::now(),
                                    },
                                )
                            }
                        }
                    };
                } else {
                    let retry_msg = format!("Transaction failed: {}", error_msg);
                    if is_transient_verification_error(&retry_msg) {
                        return VerificationOutcome::RetryTransient(retry_msg);
                    } else {
                        return VerificationOutcome::RetryTransient(retry_msg); // Be conservative
                    }
                }
            }

            match tx.status {
                TransactionStatus::Finalized | TransactionStatus::Confirmed => tx,
                TransactionStatus::Pending => {
                    return VerificationOutcome::RetryTransient(
                        "Transaction still pending".to_string(),
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
            // Event-aware shortcut: if our events DB shows a confirmed/failed outcome, act accordingly
            if let Ok(events) = crate::events::search_events(
                Some("transaction"),
                None,
                Some(&item.signature),
                Some(24),
                5,
            )
            .await
            {
                // Prefer the latest decisive outcome among returned events
                let mut decided: Option<(String, bool, Option<Value>)> = None; // (status, success, payload)
                for ev in events {
                    if let Some(cs) = ev
                        .payload
                        .get("confirmation_status")
                        .and_then(|v| v.as_str())
                    {
                        match cs {
                            "confirmed" => {
                                decided = Some((cs.to_string(), true, Some(ev.payload)));
                                break;
                            }
                            "failed" => {
                                decided = Some((cs.to_string(), false, Some(ev.payload)));
                                break;
                            }
                            _ => {}
                        }
                    }
                }

                if let Some((status, success, payload)) = decided {
                    if success && status == "confirmed" {
                        // Confirmed by events but transaction object not yet available → retry shortly
                        return VerificationOutcome::RetryTransient(
                            "Transaction confirmed by events; awaiting RPC indexing".to_string(),
                        );
                    } else if !success && status == "failed" {
                        // Failed by events → map to existing failure handling per kind
                        match item.kind {
                            VerificationKind::Entry => {
                                return VerificationOutcome::PermanentFailure(
                                    PositionTransition::RemoveOrphanEntry {
                                        position_id: item.position_id.unwrap_or(0),
                                    },
                                );
                            }
                            VerificationKind::Exit => {
                                // For exit failures, prefer wallet residual check
                                if let (Ok(wallet_address), Some(position_id)) =
                                    (get_wallet_address(), item.position_id)
                                {
                                    match get_total_token_balance(&wallet_address, &item.mint).await
                                    {
                                        Ok(balance) => {
                                            if residual_balance_requires_retry(
                                                Some(position_id),
                                                balance,
                                            )
                                            .await
                                            {
                                                return VerificationOutcome::Transition(
                                                    PositionTransition::ExitFailedClearForRetry {
                                                        position_id,
                                                    },
                                                );
                                            } else {
                                                return VerificationOutcome::PermanentFailure(
                                                    PositionTransition::ExitPermanentFailureSynthetic {
                                                        position_id,
                                                        exit_time: Utc::now(),
                                                    }
                                                );
                                            }
                                        }
                                        Err(_) => {
                                            return VerificationOutcome::PermanentFailure(
                                                PositionTransition::ExitPermanentFailureSynthetic {
                                                    position_id,
                                                    exit_time: Utc::now(),
                                                },
                                            );
                                        }
                                    }
                                } else {
                                    return VerificationOutcome::PermanentFailure(
                                        PositionTransition::ExitPermanentFailureSynthetic {
                                            position_id: item.position_id.unwrap_or(0),
                                            exit_time: Utc::now(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Progressive timeout logic - different timeouts for entry vs exit
            let timeout_threshold = match item.kind {
                VerificationKind::Exit => 60,  // 1 minute for exit transactions
                VerificationKind::Entry => 90, // 1.5 minutes for entry transactions
            };

            if item.age_seconds() > timeout_threshold {
                // Handle timeout based on transaction type
                match item.kind {
                    VerificationKind::Exit => {
                        // For exit timeouts, check wallet balance before giving up
                        if let (Ok(wallet_address), Some(position_id)) =
                            (get_wallet_address(), item.position_id)
                        {
                            match get_total_token_balance(&wallet_address, &item.mint).await {
                                Ok(balance) => {
                                    if residual_balance_requires_retry(Some(position_id), balance)
                                        .await
                                    {
                                        logger::debug(
                                            LogTag::Positions,
                                            &format!(
                                                "🔄 Exit timeout but significant balance remains ({} tokens), clearing for retry: {}",
                                                balance,
                                                item.signature
                                            ),
                                        );
                                        return VerificationOutcome::Transition(
                                            PositionTransition::ExitFailedClearForRetry {
                                                position_id,
                                            },
                                        );
                                    } else {
                                        return VerificationOutcome::PermanentFailure(
                                            PositionTransition::ExitPermanentFailureSynthetic {
                                                position_id,
                                                exit_time: Utc::now(),
                                            },
                                        );
                                    }
                                }
                                Err(_) => {
                                    // Balance check failed - be conservative and retry
                                    return VerificationOutcome::RetryTransient(
                                        "Exit timeout but balance check failed - will retry"
                                            .to_string(),
                                    );
                                }
                            }
                        } else {
                            return VerificationOutcome::RetryTransient(
                                "Exit timeout but cannot check balance".to_string(),
                            );
                        }
                    }
                    VerificationKind::Entry => {
                        return VerificationOutcome::RetryTransient(
                            "Entry transaction not found (timeout)".to_string(),
                        );
                    }
                }
            } else {
                return VerificationOutcome::RetryTransient(
                    "Transaction not found (propagation)".to_string(),
                );
            }
        }
        Err(e) => {
            let error_msg = format!("Error getting transaction: {}", e);

            // Event-aware fallback: if events show a decisive outcome, act accordingly
            if let Ok(events) = crate::events::search_events(
                Some("transaction"),
                None,
                Some(&item.signature),
                Some(24),
                5,
            )
            .await
            {
                for ev in events {
                    if let Some(cs) = ev
                        .payload
                        .get("confirmation_status")
                        .and_then(|v| v.as_str())
                    {
                        match cs {
                            "confirmed" => {
                                return VerificationOutcome::RetryTransient(
                                    "Transaction confirmed by events; awaiting RPC indexing"
                                        .to_string(),
                                );
                            }
                            "failed" => {
                                // Map failure as above
                                match item.kind {
                                    VerificationKind::Entry => {
                                        return VerificationOutcome::PermanentFailure(
                                            PositionTransition::RemoveOrphanEntry {
                                                position_id: item.position_id.unwrap_or(0),
                                            },
                                        );
                                    }
                                    VerificationKind::Exit => {
                                        if let (Ok(wallet_address), Some(position_id)) =
                                            (get_wallet_address(), item.position_id)
                                        {
                                            match get_total_token_balance(
                                                &wallet_address,
                                                &item.mint,
                                            )
                                            .await
                                            {
                                                Ok(balance) => {
                                                    if residual_balance_requires_retry(
                                                        Some(position_id),
                                                        balance,
                                                    )
                                                    .await
                                                    {
                                                        return VerificationOutcome::Transition(
                                                            PositionTransition::ExitFailedClearForRetry {
                                                                position_id,
                                                            }
                                                        );
                                                    } else {
                                                        return VerificationOutcome::PermanentFailure(
                                                            PositionTransition::ExitPermanentFailureSynthetic {
                                                                position_id,
                                                                exit_time: Utc::now(),
                                                            }
                                                        );
                                                    }
                                                }
                                                Err(_) => {
                                                    return VerificationOutcome::PermanentFailure(
                                                        PositionTransition::ExitPermanentFailureSynthetic {
                                                            position_id,
                                                            exit_time: Utc::now(),
                                                        }
                                                    );
                                                }
                                            }
                                        } else {
                                            return VerificationOutcome::PermanentFailure(
                                                PositionTransition::ExitPermanentFailureSynthetic {
                                                    position_id: item.position_id.unwrap_or(0),
                                                    exit_time: Utc::now(),
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Enhanced error classification for immediate verification optimization
            if error_msg.to_lowercase().contains("not found")
                || error_msg.to_lowercase().contains("not yet indexed")
                || error_msg.to_lowercase().contains("rpc error")
            {
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "🔄 RPC indexing delay for {}: {}",
                        item.signature, error_msg
                    ),
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

    // Get swap analysis
    // Prefer attached swap analysis if present; otherwise defer
    let swap_info = if let Some(pnl) = transaction.swap_pnl_info.clone() {
        pnl
    } else {
        return VerificationOutcome::RetryTransient("No valid swap analysis".to_string());
    };

    // Verify token mint matches
    if swap_info.token_mint != item.mint {
        return VerificationOutcome::RetryTransient("Token mint mismatch".to_string());
    }

    let position_id = item.position_id.unwrap_or(0);

    match item.kind {
        VerificationKind::Entry => {
            if swap_info.swap_type != "Buy" {
                return VerificationOutcome::RetryTransient("Expected Buy transaction".to_string());
            }

            // Convert token amount to integer units with rounding
            let (mut token_amount_units, decimals_opt) =
                if let Some(decimals) = get_decimals(&item.mint).await {
                    let scale = (10_f64).powi(decimals as i32);
                    let units = (swap_info.token_amount.abs() * scale).round();
                    (units.max(0.0) as u64, Some(decimals))
                } else {
                    (0u64, None)
                };

            // Enforce decimals presence as per tokens system contract
            if decimals_opt.is_none() {
                return VerificationOutcome::RetryTransient(
                    "Token decimals not cached".to_string(),
                );
            }

            // Prefer authoritative on-chain balance immediately after entry finalization, if available.
            // IMPORTANT: Only ever reduce token_amount_units to the on-chain balance if it's smaller.
            // Never increase to an aggregated wallet balance as that may include subsequent buys and
            // incorrectly attribute tokens to this entry (causing duplicate-buys to be merged).
            if let Ok(wallet_address) = get_wallet_address() {
                // Throttle token accounts query to reduce RPC load
                if should_throttle_token_accounts(&item.mint).await {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "⏳ Throttling token accounts check (entry verify) for mint {}",
                            item.mint
                        ),
                    );
                    return VerificationOutcome::RetryTransient(
                        "Token accounts check throttled".to_string(),
                    );
                }
                if let Ok(actual_units) = get_total_token_balance(&wallet_address, &item.mint).await
                {
                    if actual_units > 0 && actual_units < token_amount_units {
                        logger::debug(
                            LogTag::Positions,
                            &format!(
                                "Reduced token units to on-chain balance for mint {}: tx-derived={} actual={}",
                                &item.mint,
                                token_amount_units,
                                actual_units
                            ),
                        );
                        token_amount_units = actual_units;
                    }
                }
            }

            // Calculate effective entry price using authoritative units when possible
            let effective_price = match (decimals_opt, token_amount_units) {
                (Some(decimals), units) if units > 0 && swap_info.effective_sol_spent > 0.0 => {
                    let scale = (10_f64).powi(decimals as i32);
                    let token_amount_float = (units as f64) / scale;
                    swap_info.effective_sol_spent / token_amount_float
                }
                _ => swap_info.calculated_price_sol,
            };

            VerificationOutcome::Transition(PositionTransition::EntryVerified {
                position_id,
                effective_entry_price: effective_price,
                token_amount_units,
                fee_lamports: sol_to_lamports(swap_info.fee_sol),
                sol_size: swap_info.sol_amount,
            })
        }
        VerificationKind::Exit => {
            if swap_info.swap_type != "Sell" {
                return VerificationOutcome::RetryTransient(
                    "Expected Sell transaction".to_string(),
                );
            }

            let exit_time = if let Some(block_time) = transaction.block_time {
                chrono::DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now())
            } else {
                Utc::now()
            };

            // Calculate exit amount from transaction
            let exit_amount = if let Some(decimals) = get_decimals(&item.mint).await {
                let scale = (10_f64).powi(decimals as i32);
                let units = (swap_info.token_amount.abs() * scale).round();
                units.max(0.0) as u64
            } else {
                return VerificationOutcome::RetryTransient(
                    "Token decimals not cached for exit".to_string(),
                );
            };

            // CRITICAL: For PARTIAL exits, we expect remaining balance
            // For FULL exits, we must ensure complete closure (ATA closable)
            if let Ok(wallet_address) = get_wallet_address() {
                // Throttle token accounts query to reduce RPC load
                if should_throttle_token_accounts(&item.mint).await {
                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "⏳ Throttling token accounts check (exit residual) for mint {}",
                            item.mint
                        ),
                    );
                    return VerificationOutcome::RetryTransient(
                        "Token accounts check throttled".to_string(),
                    );
                }
                match get_total_token_balance(&wallet_address, &item.mint).await {
                    Ok(remaining_balance) => {
                        // PARTIAL EXIT: Verify expected amount was sold, balance check is informational
                        if item.is_partial_exit {
                            if let Some(expected) = item.expected_exit_amount {
                                let tolerance = std::cmp::max(expected / 1000, 10); // 0.1% tolerance or 10 units
                                if exit_amount < expected.saturating_sub(tolerance)
                                    || exit_amount > expected + tolerance
                                {
                                    logger::warning(
                                        LogTag::Positions,
                                        &format!(
                                            "⚠️ Partial exit amount mismatch for mint {}: expected={} actual={} tolerance={}",
                                            item.mint, expected, exit_amount, tolerance
                                        ),
                                    );
                                    return VerificationOutcome::RetryTransient(
                                        "Partial exit amount mismatch - will verify again".to_string(),
                                    );
                                }
                            }

                            logger::info(
                                LogTag::Positions,
                                &format!(
                                    "✅ Partial exit verified for mint {}: sold={} remaining={}",
                                    item.mint, exit_amount, remaining_balance
                                ),
                            );

                            return VerificationOutcome::Transition(
                                PositionTransition::PartialExitVerified {
                                    position_id,
                                    exit_amount,
                                    sol_received: swap_info.effective_sol_received.abs(),
                                    effective_exit_price: swap_info.calculated_price_sol,
                                    fee_lamports: sol_to_lamports(swap_info.fee_sol),
                                    exit_time,
                                },
                            );
                        }

                        // FULL EXIT: Ensure complete closure (check for residual)
                        if residual_balance_requires_retry(item.position_id, remaining_balance)
                            .await
                        {
                            logger::warning(
                                LogTag::Positions,
                                &format!(
                                    "⚠️ Exit residual {} units for mint {} → will retry another close",
                                    remaining_balance,
                                    item.mint
                                ),
                            );

                            crate::events::record_safe(crate::events::Event::new(
                                crate::events::EventCategory::Position,
                                Some("exit_residual_detected".to_string()),
                                crate::events::Severity::Warn,
                                Some(item.mint.clone()),
                                item.position_id.map(|id| id.to_string()),
                                serde_json::json!({
                                    "position_id": item.position_id,
                                    "remaining_balance": remaining_balance
                                }),
                            ))
                            .await;

                            return VerificationOutcome::Transition(
                                PositionTransition::ExitFailedClearForRetry { position_id },
                            );
                        } else {
                            logger::info(
                                LogTag::Positions,
                                &format!(
                                    "✅ Exit verified with zero residual for mint {}",
                                    item.mint
                                ),
                            );
                        }
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Positions,
                            &format!("⚠️ Could not verify residual balance after exit: {}", e),
                        );
                        // Be conservative, retry later
                        return VerificationOutcome::RetryTransient(
                            "Residual check failed after exit".to_string(),
                        );
                    }
                }
            }

            // FULL EXIT: Standard verification
            VerificationOutcome::Transition(PositionTransition::ExitVerified {
                position_id,
                effective_exit_price: swap_info.calculated_price_sol,
                sol_received: swap_info.effective_sol_received.abs(),
                fee_lamports: sol_to_lamports(swap_info.fee_sol),
                exit_time,
            })
        }
    }
}
