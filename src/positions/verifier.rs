use crate::{
    transactions::{ get_transaction, get_global_transaction_manager },
    transactions_types::{ Transaction, TransactionStatus },
    tokens::get_token_decimals,
    rpc::sol_to_lamports,
    arguments::is_debug_positions_enabled,
    logger::{ log, LogTag },
    utils::{ get_token_balance, get_wallet_address, get_total_token_balance },
};
use super::{
    transitions::PositionTransition,
    queue::{ VerificationItem, VerificationKind },
    state::get_mint_by_signature,
};
use chrono::Utc;

/// Classify transient (retryable) verification errors
fn is_transient_verification_error(msg: &str) -> bool {
    let m = msg.to_lowercase();
    m.contains("within propagation grace") ||
        m.contains("still pending") ||
        m.contains("within propagation") ||
        m.contains("not found in system") ||
        m.contains("will retry") ||
        m.contains("no valid swap analysis") ||
        m.contains("error getting transaction") ||
        m.contains("transaction manager not available") ||
        m.contains("transaction manager not initialized") ||
        m.contains("transaction not found (propagation)") ||
        m.contains("not yet indexed") ||
        m.contains("transaction not found") ||
        m.contains("failed to fetch transaction details") ||
        m.contains("rpc error") ||
        m.contains("transaction not available") ||
        m.contains("blockchain transaction not found")
}

#[derive(Debug)]
pub enum VerificationOutcome {
    Transition(PositionTransition),
    RetryTransient(String),
    PermanentFailure(PositionTransition),
}

/// Verify a transaction and produce the appropriate transition
pub async fn verify_transaction(item: &VerificationItem) -> VerificationOutcome {
    if is_debug_positions_enabled() {
        log(LogTag::Positions, "DEBUG", &format!("🔍 Verifying transaction: {}", item.signature));
    }

    // Get transaction
    let transaction = match get_transaction(&item.signature).await {
        Ok(Some(tx)) => {
            if !tx.success {
                let error_msg = tx.error_message.unwrap_or("Unknown error".to_string());
                if error_msg.contains("[PERMANENT]") {
                    return match item.kind {
                        VerificationKind::Entry => {
                            VerificationOutcome::PermanentFailure(
                                PositionTransition::RemoveOrphanEntry {
                                    position_id: item.position_id.unwrap_or(0),
                                }
                            )
                        }
                        VerificationKind::Exit => {
                            // For exit permanent failures, check wallet balance
                            if
                                let (Ok(wallet_address), Some(position_id)) = (
                                    get_wallet_address(),
                                    item.position_id,
                                )
                            {
                                match get_total_token_balance(&wallet_address, &item.mint).await {
                                    Ok(balance) => {
                                        if balance > 0 {
                                            // Clear exit signature for retry
                                            VerificationOutcome::Transition(
                                                PositionTransition::ExitFailedClearForRetry {
                                                    position_id,
                                                }
                                            )
                                        } else {
                                            // Synthetic exit
                                            VerificationOutcome::PermanentFailure(
                                                PositionTransition::ExitPermanentFailureSynthetic {
                                                    position_id,
                                                    exit_time: Utc::now(),
                                                }
                                            )
                                        }
                                    }
                                    Err(_) => {
                                        VerificationOutcome::PermanentFailure(
                                            PositionTransition::ExitPermanentFailureSynthetic {
                                                position_id,
                                                exit_time: Utc::now(),
                                            }
                                        )
                                    }
                                }
                            } else {
                                VerificationOutcome::PermanentFailure(
                                    PositionTransition::ExitPermanentFailureSynthetic {
                                        position_id: item.position_id.unwrap_or(0),
                                        exit_time: Utc::now(),
                                    }
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
                        "Transaction still pending".to_string()
                    );
                }
                TransactionStatus::Failed(err) => {
                    return VerificationOutcome::RetryTransient(
                        format!("Transaction failed: {}", err)
                    );
                }
            }
        }
        Ok(None) => {
            // Progressive timeout logic - different timeouts for entry vs exit
            let timeout_threshold = match item.kind {
                VerificationKind::Exit => 60, // 1 minute for exit transactions
                VerificationKind::Entry => 90, // 1.5 minutes for entry transactions
            };

            if item.age_seconds() > timeout_threshold {
                // Handle timeout based on transaction type
                match item.kind {
                    VerificationKind::Exit => {
                        // For exit timeouts, check wallet balance before giving up
                        if
                            let (Ok(wallet_address), Some(position_id)) = (
                                get_wallet_address(),
                                item.position_id,
                            )
                        {
                            match get_total_token_balance(&wallet_address, &item.mint).await {
                                Ok(balance) => {
                                    if balance > 0 {
                                        // Tokens still in wallet - clear exit signature for retry
                                        if is_debug_positions_enabled() {
                                            log(
                                                LogTag::Positions,
                                                "DEBUG",
                                                &format!(
                                                    "🔄 Exit timeout but tokens remain, clearing for retry: {}",
                                                    item.signature
                                                )
                                            );
                                        }
                                        return VerificationOutcome::Transition(
                                            PositionTransition::ExitFailedClearForRetry {
                                                position_id,
                                            }
                                        );
                                    } else {
                                        // No tokens - treat as synthetic exit
                                        return VerificationOutcome::PermanentFailure(
                                            PositionTransition::ExitPermanentFailureSynthetic {
                                                position_id,
                                                exit_time: Utc::now(),
                                            }
                                        );
                                    }
                                }
                                Err(_) => {
                                    // Balance check failed - be conservative and retry
                                    return VerificationOutcome::RetryTransient(
                                        "Exit timeout but balance check failed - will retry".to_string()
                                    );
                                }
                            }
                        } else {
                            return VerificationOutcome::RetryTransient(
                                "Exit timeout but cannot check balance".to_string()
                            );
                        }
                    }
                    VerificationKind::Entry => {
                        return VerificationOutcome::RetryTransient(
                            "Entry transaction not found (timeout)".to_string()
                        );
                    }
                }
            } else {
                return VerificationOutcome::RetryTransient(
                    "Transaction not found (propagation)".to_string()
                );
            }
        }
        Err(e) => {
            let error_msg = format!("Error getting transaction: {}", e);

            // Enhanced error classification for immediate verification optimization
            if
                error_msg.to_lowercase().contains("not found") ||
                error_msg.to_lowercase().contains("not yet indexed") ||
                error_msg.to_lowercase().contains("rpc error")
            {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("🔄 RPC indexing delay for {}: {}", item.signature, error_msg)
                    );
                }
                return VerificationOutcome::RetryTransient(
                    format!("RPC indexing delay: {}", error_msg)
                );
            }

            if is_transient_verification_error(&error_msg) {
                return VerificationOutcome::RetryTransient(error_msg);
            } else {
                return VerificationOutcome::RetryTransient(error_msg); // Be conservative with RPC errors
            }
        }
    };

    // Get swap analysis
    let swap_pnl_info = match get_global_transaction_manager().await {
        Some(manager_guard) => {
            let manager = manager_guard.lock().await;
            if let Some(ref manager) = *manager {
                let empty_cache = std::collections::HashMap::new();
                manager.convert_to_swap_pnl_info(&transaction, &empty_cache, false)
            } else {
                return VerificationOutcome::RetryTransient(
                    "Transaction manager not initialized".to_string()
                );
            }
        }
        None => {
            return VerificationOutcome::RetryTransient(
                "Transaction manager not available".to_string()
            );
        }
    };

    let swap_info = match swap_pnl_info {
        Some(info) => info,
        None => {
            return VerificationOutcome::RetryTransient("No valid swap analysis".to_string());
        }
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

            // Calculate effective entry price
            let effective_price = if
                swap_info.token_amount.abs() > 0.0 &&
                swap_info.effective_sol_spent > 0.0
            {
                swap_info.effective_sol_spent / swap_info.token_amount.abs()
            } else {
                swap_info.calculated_price_sol
            };

            // Convert token amount to units
            let token_amount_units = if let Some(decimals) = get_token_decimals(&item.mint).await {
                (swap_info.token_amount.abs() * (10_f64).powi(decimals as i32)) as u64
            } else {
                0
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
                return VerificationOutcome::RetryTransient("Expected Sell transaction".to_string());
            }

            let exit_time = if let Some(block_time) = transaction.block_time {
                chrono::DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now())
            } else {
                Utc::now()
            };

            // CRITICAL: Check if any tokens remain after exit verification
            if let Ok(wallet_address) = get_wallet_address() {
                match get_total_token_balance(&wallet_address, &item.mint).await {
                    Ok(remaining_balance) => {
                        if remaining_balance > 0 {
                            log(
                                LogTag::Positions,
                                "RESIDUAL_DETECTED",
                                &format!(
                                    "⚠️ Exit verified but {} tokens remain for mint {} - position not fully liquidated",
                                    remaining_balance,
                                    crate::utils::safe_truncate(&item.mint, 8)
                                )
                            );

                            // For now, we'll still mark as verified but log the issue
                            // TODO: Add residual cleanup mechanism
                        } else {
                            log(
                                LogTag::Positions,
                                "COMPLETE_EXIT",
                                &format!(
                                    "✅ Exit verified with zero residual for mint {}",
                                    crate::utils::safe_truncate(&item.mint, 8)
                                )
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Positions,
                            "RESIDUAL_CHECK_FAILED",
                            &format!("⚠️ Could not verify residual balance after exit: {}", e)
                        );
                    }
                }
            }

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
