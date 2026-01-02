use super::db::{
  force_database_sync, save_entry_record, save_exit_record, update_position,
  update_position_price_fields,
};
use super::{
  loss_detection::process_position_loss_detection,
  state::{
    clear_pending_dca_swap, get_position_by_id, get_position_by_mint,
    release_global_position_permit, remove_position, remove_signature_from_index,
    update_position_state, POSITIONS,
  },
  transitions::PositionTransition,
};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::telegram::{queue_notification, Notification};
use chrono::Utc;

#[derive(Debug)]
pub struct ApplyEffects {
  pub db_updated: bool,
  pub position_removed: bool,
  pub position_closed: bool,
}

/// Apply a position transition to state and database
pub async fn apply_transition(transition: PositionTransition) -> Result<ApplyEffects, String> {
  let mut effects = ApplyEffects {
    db_updated: false,
    position_removed: false,
    position_closed: false,
  };

  let requires_db_update = transition.requires_db_update();

  match transition {
    PositionTransition::EntryVerified {
      position_id,
      effective_entry_price,
      token_amount_units,
      fee_lamports,
      sol_size,
    } => {
      let updated =
        update_position_state(&find_mint_by_position_id(position_id).await?, |pos| {
          pos.transaction_entry_verified = true;
          pos.effective_entry_price = Some(effective_entry_price);
          pos.total_size_sol = sol_size;
          pos.token_amount = Some(token_amount_units);
          pos.entry_fee_lamports = Some(fee_lamports);
          pos.entry_size_sol = sol_size;
          pos.remaining_token_amount = Some(token_amount_units);
          pos.average_entry_price = effective_entry_price;
        })
        .await;

      if updated && requires_db_update {
        if let Some(position) = get_position_by_id(position_id).await {
          match update_position(&position).await {
            Ok(_) => {
              effects.db_updated = true;
              let _ = force_database_sync().await;
              // Record an entry verified event
              crate::events::record_position_event(
                &position_id.to_string(),
                &position.mint,
                "entry_verified",
                position.entry_transaction_signature.as_deref(),
                None,
                sol_size,
                token_amount_units,
                None,
                None,
              )
              .await;

              if let Some(entry_sig) = position.entry_transaction_signature.as_deref()
              {
                if let Err(err) = save_entry_record(
                  position_id,
                  position.entry_time,
                  token_amount_units,
                  effective_entry_price,
                  sol_size,
                  entry_sig,
                  false,
                  Some(fee_lamports),
                )
                .await
                {
                  logger::error(
                    LogTag::Positions,
                    &format!(
                      "Failed to persist entry history for position {}: {}",
                      position_id, err
                    ),
                  );
                }
              }

              // Queue Telegram notification for position opened
              if with_config(|c| c.telegram.enabled && c.telegram.notify_position_opened) {
                queue_notification(Notification::position_opened(
                  position.symbol.clone(),
                  position.mint.clone(),
                  sol_size,
                  effective_entry_price,
                ));
              }
            }
            Err(e) => {
              return Err(format!("Failed to update database: {}", e));
            }
          }
        }
      }
    }

    PositionTransition::ExitVerified {
      position_id,
      effective_exit_price,
      sol_received,
      fee_lamports,
      exit_time,
    } => {
      let updated =
        update_position_state(&find_mint_by_position_id(position_id).await?, |pos| {
          pos.transaction_exit_verified = true;
          pos.effective_exit_price = Some(effective_exit_price);
          pos.sol_received = Some(sol_received);
          pos.exit_fee_lamports = Some(fee_lamports);
          pos.exit_time = Some(exit_time);

 // CRITICAL FIX: Update closed_reason to remove pending verification suffix
          // This ensures database state matches verification status
          if let Some(reason) = &pos.closed_reason {
            if reason.ends_with(super::PENDING_VERIFICATION_SUFFIX) {
              pos.closed_reason =
                Some(reason.trim_end_matches(super::PENDING_VERIFICATION_SUFFIX).to_string());
            }
          }

          // Note: exit_price is already set by close_position_direct to market price
        })
        .await;

      if updated && requires_db_update {
        if let Some(position) = get_position_by_id(position_id).await {
          // Calculate final P&L for closed position BEFORE any database operations
          let (pnl_sol, pnl_pct) =
            crate::positions::calculate_position_pnl(&position, None).await;

          // Atomically update position with PnL in a single operation
          let pnl_updated = update_position_state(&position.mint, |pos| {
            pos.pnl = Some(pnl_sol);
            pos.pnl_percent = Some(pnl_pct);
            // Clear unrealized PnL (position is now closed)
            pos.unrealized_pnl = None;
            pos.unrealized_pnl_percent = None;
          })
          .await;

          if !pnl_updated {
            logger::error(
              LogTag::Positions,
              &format!(
                "Failed to update PnL for closed position {}",
                position.symbol
              ),
            );
            // Continue anyway - position is closed, PnL is secondary
          }

          // Refresh position after PnL update for loss detection
          if let Some(position) = get_position_by_id(position_id).await {
            // Process loss detection and potential blacklisting
            if let Err(e) = process_position_loss_detection(&position).await {
              logger::error(
                LogTag::Positions,
                &format!(
                  "Failed to process loss detection for {}: {}",
                  position.symbol, e
                ),
              );
            }

            // Record realized loss for loss limit tracking (full exit only)
            // pnl_sol was calculated above via calculate_position_pnl
            if pnl_sol < 0.0 {
              crate::trader::safety::loss_limit::record_realized_loss(pnl_sol.abs());
            }

            match update_position(&position).await {
              Ok(_) => {
                effects.db_updated = true;
                effects.position_closed = true;
                let _ = force_database_sync().await;

                // CRITICAL: Release global position permit when position is verified closed
                // This allows new positions to be opened, fixing the MAX_OPEN_POSITIONS limit
                release_global_position_permit();

                // Record an exit verified event with basic P&L if computable
                let pnl_sol =
                  position.sol_received.map(|s| s - position.total_size_sol);
                let pnl_pct = position.effective_entry_price.and_then(|ep| {
                  position.effective_exit_price.map(|xp| {
                    if ep > 0.0 {
                      ((xp - ep) / ep) * 100.0
                    } else {
                      0.0
                    }
                  })
                });
                crate::events::record_position_event(
                  &position_id.to_string(),
                  &position.mint,
                  "exit_verified",
                  position.entry_transaction_signature.as_deref(),
                  position.exit_transaction_signature.as_deref(),
                  position.total_size_sol,
                  position.token_amount.unwrap_or(0),
                  pnl_sol,
                  pnl_pct,
                )
                .await;

                logger::info(
                  LogTag::Positions,
                  &format!(
 "Released position slot for verified exit (ID: {})",
                    position_id
                  ),
                );

                // Queue Telegram notification for position closed
                if with_config(|c| c.telegram.enabled && c.telegram.notify_position_closed) {
                  let exit_reason = position.closed_reason.clone().unwrap_or_else(|| "exit".to_string());
                  // Use position.pnl and position.pnl_percent which were set in the state update above
                  let final_pnl_sol = position.pnl.unwrap_or(0.0);
                  let final_pnl_pct = position.pnl_percent.unwrap_or(0.0);
                  queue_notification(Notification::position_closed(
                    position.symbol.clone(),
                    position.mint.clone(),
                    final_pnl_sol,
                    final_pnl_pct,
                    exit_reason,
                  ));
                }
              }
              Err(e) => {
                return Err(format!("Failed to update database: {}", e));
              }
            }
          }
        }
      }
    }

    PositionTransition::ExitFailedClearForRetry { position_id } => {
      let mint = find_mint_by_position_id(position_id).await?;
      // Capture old signature to purge index entry (prevent stale sig->mint mapping)
      let mut old_sig: Option<String> = None;
      let updated = update_position_state(&mint, |pos| {
        if let Some(sig) = pos.exit_transaction_signature.clone() {
          old_sig = Some(sig);
        }
        pos.exit_transaction_signature = None;
        pos.transaction_exit_verified = false;
        pos.closed_reason = Some("exit_retry_pending".to_string());
      })
      .await;

      if let Some(sig) = old_sig {
        remove_signature_from_index(&sig).await;
        crate::events::record_position_event_flexible(
          "exit_retry_cleared",
          crate::events::Severity::Warn,
          None,
          Some(&sig),
          serde_json::json!({
            "position_id": position_id
          }),
        )
        .await;
      }

      if updated && requires_db_update {
        if let Some(position) = get_position_by_id(position_id).await {
          match update_position(&position).await {
            Ok(_) => {
              effects.db_updated = true;
            }
            Err(e) => {
              return Err(format!("Failed to update database: {}", e));
            }
          }
        }
      }
    }

    PositionTransition::ExitPermanentFailureSynthetic {
      position_id,
      exit_time,
    } => {
      let updated =
        update_position_state(&find_mint_by_position_id(position_id).await?, |pos| {
          pos.synthetic_exit = true;
          pos.transaction_exit_verified = true;
          pos.exit_time = Some(exit_time);
          pos.closed_reason = Some("synthetic_exit_permanent_failure".to_string());
        })
        .await;

      if updated && requires_db_update {
        if let Some(position) = get_position_by_id(position_id).await {
          // Record synthetic exit event
          crate::events::record_position_event(
            &position_id.to_string(),
            &position.mint,
            "exit_synthetic",
            position.entry_transaction_signature.as_deref(),
            position.exit_transaction_signature.as_deref(),
            position.total_size_sol,
            position.remaining_token_amount.unwrap_or(0),
            None,
            None,
          )
          .await;

          match update_position(&position).await {
            Ok(_) => {
              effects.db_updated = true;
              effects.position_closed = true;
              // Release global slot for synthetic exits as well
              release_global_position_permit();
              logger::debug(
                LogTag::Positions,
                &format!(
 "Released position slot for synthetic exit (ID: {})",
                  position_id
                ),
              );
            }
            Err(e) => {
              return Err(format!("Failed to update database: {}", e));
            }
          }
        }
      }
    }

    PositionTransition::RemoveOrphanEntry { position_id } => {
      if let Ok(mint) = find_mint_by_position_id(position_id).await {
        if let Some(_) = remove_position(&mint).await {
          effects.position_removed = true;
          crate::events::record_position_event_flexible(
            "orphan_entry_removed",
            crate::events::Severity::Warn,
            Some(&mint),
            None,
            serde_json::json!({
              "position_id": position_id
            }),
          )
          .await;

          logger::debug(
            LogTag::Positions,
 &format!("Removed orphan entry position {}", position_id),
          );

          // Orphan entries also occupied a slot originally; free it now
          release_global_position_permit();
          logger::debug(
            LogTag::Positions,
            &format!(
 "Released position slot after orphan removal (ID: {})",
              position_id
            ),
          );
          // NOTE: position removal already purged signature indexes. Optionally we could
          // attempt to prune per-mint lock map here if implemented in state.
        }
      }
    }

    // ==================== PARTIAL EXIT TRANSITIONS ====================
    PositionTransition::PartialExitSubmitted {
      position_id,
      exit_signature,
      exit_amount,
      exit_percentage,
      market_price,
    } => {
      // Record partial exit submitted event
      if let Some(position) = get_position_by_id(position_id).await {
        let sol_estimate = (exit_amount as f64 / 10_f64.powi(9)) * market_price;
        crate::events::record_position_event(
          &position_id.to_string(),
          &position.mint,
          "partial_exit_submitted",
          position.entry_transaction_signature.as_deref(),
          Some(&exit_signature),
          sol_estimate,
          exit_amount,
          None,
          Some(exit_percentage),
        )
        .await;
      }

      logger::info(
        LogTag::Positions,
        &format!(
          "Partial exit submitted for position {}: {}% ({} tokens) at price {:.11}",
          position_id, exit_percentage, exit_amount, market_price
        ),
      );
    }

    PositionTransition::PartialExitVerified {
      position_id,
      exit_amount,
      sol_received,
      effective_exit_price,
      fee_lamports,
      exit_time,
      exit_signature,
      exit_percentage,
    } => {
      let updated =
        update_position_state(&find_mint_by_position_id(position_id).await?, |pos| {
          // Update remaining token amount
          if let Some(remaining) = pos.remaining_token_amount {
            pos.remaining_token_amount = Some(remaining.saturating_sub(exit_amount));
          }

          // Update total exited amount
          pos.total_exited_amount += exit_amount;

          // Calculate new average exit price (weighted average)
          let total_exited = pos.total_exited_amount;
          if let Some(prev_avg) = pos.average_exit_price {
            let prev_weight = (total_exited - exit_amount) as f64 / total_exited as f64;
            let new_weight = exit_amount as f64 / total_exited as f64;
            pos.average_exit_price =
              Some((prev_avg * prev_weight) + (effective_exit_price * new_weight));
          } else {
            pos.average_exit_price = Some(effective_exit_price);
          }

          // Increment partial exit count
          pos.partial_exit_count += 1;

          // Update SOL received (cumulative)
          pos.sol_received = Some(pos.sol_received.unwrap_or(0.0) + sol_received);

          // CRITICAL: Do NOT set exit_time or exit_signature - position still open!
        })
        .await;

      if updated && requires_db_update {
        if let Some(mut position) = get_position_by_id(position_id).await {
          // Calculate unrealized PnL immediately after partial exit
          // Don't wait for price updater (eliminates up to 1 second delay)
          if let Some(current_price) = position.current_price {
            let (pnl_sol, pnl_pct) = crate::positions::calculate_position_pnl(
              &position,
              Some(current_price),
            )
            .await;

            // Update unrealized PnL in memory
            update_position_state(&position.mint, |pos| {
              pos.unrealized_pnl = Some(pnl_sol);
              pos.unrealized_pnl_percent = Some(pnl_pct);
            })
            .await;

            // Refresh position to get updated PnL
            if let Some(updated_pos) = get_position_by_id(position_id).await {
              position = updated_pos;
            }
          } else {
            logger::debug(
              LogTag::Positions,
              &format!("No current price available for {} after partial exit, PnL will update on next price tick", position.symbol),
            );
          }

          match update_position(&position).await {
            Ok(_) => {
              effects.db_updated = true;
              let _ = force_database_sync().await;

              if let Err(err) = save_exit_record(
                position_id,
                exit_time,
                exit_amount,
                effective_exit_price,
                sol_received,
                &exit_signature,
                true,
                exit_percentage,
                Some(fee_lamports),
              )
              .await
              {
                logger::error(
                  LogTag::Positions,
                  &format!(
                    "Failed to persist partial exit record for position {}: {}",
                    position_id, err
                  ),
                );
              }

              if let Err(err) =
                super::state::clear_pending_partial_exit(&exit_signature).await
              {
                return Err(format!(
                  "Failed to clear pending partial exit {} for position {}: {}",
                  exit_signature, position_id, err
                ));
              }

              crate::events::record_position_event(
                &position_id.to_string(),
                &position.mint,
                "partial_exit_verified",
                position.entry_transaction_signature.as_deref(),
                None,
                sol_received,
                exit_amount,
                Some(
                  sol_received
                    - (exit_amount as f64 / 10_f64.powi(9)
                      * position.average_entry_price),
                ), // Quick P&L estimate
                None,
              )
              .await;

              logger::info(
                LogTag::Positions,
                &format!(
 "Partial exit verified for position {}: {} tokens sold, {} remaining",
                  position_id,
                  exit_amount,
                  position.remaining_token_amount.unwrap_or(0)
                ),
              );
              // Clear pending mark
              super::state::clear_partial_exit_pending(&position.mint).await;

              // Queue Telegram notification for partial exit
              if with_config(|c| c.telegram.enabled && c.telegram.notify_partial_exit) {
                // Calculate remaining percentage
                let remaining_pct = if let (Some(remaining), Some(total)) = (position.remaining_token_amount, position.token_amount) {
                  if total > 0 { (remaining as f64 / total as f64) * 100.0 } else { 0.0 }
                } else {
                  100.0 - exit_percentage
                };
                // Calculate realized PnL for this partial exit
                let partial_pnl = sol_received - (exit_amount as f64 / 10_f64.powi(9) * position.average_entry_price);
                queue_notification(Notification::partial_exit(
                  position.symbol.clone(),
                  position.mint.clone(),
                  exit_percentage,
                  partial_pnl,
                  remaining_pct,
                ));
              }

              // IMPORTANT: Do NOT release semaphore permit - position still open!
            }
            Err(e) => {
              return Err(format!("Failed to update database: {}", e));
            }
          }
        }
      }
    }

    PositionTransition::PartialExitFailed {
      position_id,
      reason,
    } => {
      // Record partial exit failure event
      if let Some(position) = get_position_by_id(position_id).await {
        crate::events::record_position_event(
          &position_id.to_string(),
          &position.mint,
          "partial_exit_failed",
          position.entry_transaction_signature.as_deref(),
          position.exit_transaction_signature.as_deref(),
          position.total_size_sol,
          position.remaining_token_amount.unwrap_or(0),
          None,
          None,
        )
        .await;
      }

      logger::error(
        LogTag::Positions,
        &format!(
          "Partial exit failed for position {}: {}",
          position_id, reason
        ),
      );
      if let Some(position) = get_position_by_id(position_id).await {
        if let Some(exit_sig) = position.exit_transaction_signature.clone() {
          if let Err(err) = super::state::clear_pending_partial_exit(&exit_sig).await {
            logger::error(
              LogTag::Positions,
              &format!(
                "Failed to clear pending partial exit {} during failure handling for position {}: {}",
                exit_sig, position_id, err
              ),
            );
          }
        }
        super::state::clear_partial_exit_pending(&position.mint).await;
      }
      // TODO: Implement retry logic if needed
    }

    // ==================== DCA TRANSITIONS ====================
    PositionTransition::DcaSubmitted {
      position_id,
      dca_signature,
      dca_amount_sol,
      market_price,
    } => {
      // Record DCA submitted event
      if let Some(position) = get_position_by_id(position_id).await {
        let token_estimate = (dca_amount_sol / market_price) * 10_f64.powi(9);
        crate::events::record_position_event(
          &position_id.to_string(),
          &position.mint,
          "dca_submitted",
          position.entry_transaction_signature.as_deref(),
          Some(&dca_signature),
          dca_amount_sol,
          token_estimate as u64,
          None,
          None,
        )
        .await;
      }

      logger::info(
        LogTag::Positions,
        &format!(
          "DCA submitted for position {}: {} SOL at price {:.11}",
          position_id, dca_amount_sol, market_price
        ),
      );
      // No state update needed for submission - just logging
    }

    PositionTransition::DcaVerified {
      position_id,
      tokens_bought,
      sol_spent,
      effective_price,
      fee_lamports,
      dca_time,
      dca_signature,
    } => {
      let mint = find_mint_by_position_id(position_id).await?;

      // Get token decimals for accurate price calculation
      let decimals = crate::tokens::get_decimals(&mint).await.unwrap_or(9); // Default to 9 if not found

      let updated =
        update_position_state(&mint, |pos| {
          // Update remaining token amount (add new tokens)
          if let Some(remaining) = pos.remaining_token_amount {
            pos.remaining_token_amount = Some(remaining + tokens_bought);
          } else {
            pos.remaining_token_amount = Some(tokens_bought);
          }

          // Update total SOL invested
          pos.total_size_sol += sol_spent;

          // Recalculate average entry price (weighted average) with actual decimals
          // CRITICAL: Validate all inputs to prevent division by zero or invalid calculations
          let remaining_tokens = pos.remaining_token_amount.unwrap_or(0);
          if remaining_tokens > 0 && pos.total_size_sol > 0.0 && pos.total_size_sol.is_finite() {
            let total_tokens_normalized = remaining_tokens as f64
              / 10_f64.powi(decimals as i32);
            if total_tokens_normalized > 0.0 && total_tokens_normalized.is_finite() {
              pos.average_entry_price = pos.total_size_sol / total_tokens_normalized;
            } else {
              logger::error(
                LogTag::Positions,
                &format!(
 "DCA: Invalid token normalization for position {} (remaining={}, decimals={})",
                  position_id, remaining_tokens, decimals
                ),
              );
            }
          } else {
            // Edge case: Invalid state for average price calculation
            logger::error(
              LogTag::Positions,
              &format!(
 "DCA: Invalid position state for average price calculation - position_id={}, remaining_tokens={}, total_size_sol={}",
                position_id, remaining_tokens, pos.total_size_sol
              ),
            );
          }

          // Increment DCA count
          pos.dca_count += 1;

          // Update last DCA time
          pos.last_dca_time = Some(dca_time);
        })
        .await;

      if updated && requires_db_update {
        if let Some(position) = get_position_by_id(position_id).await {
          match update_position(&position).await {
            Ok(_) => {
              effects.db_updated = true;
              let _ = force_database_sync().await;

              if let Err(err) = save_entry_record(
                position_id,
                dca_time,
                tokens_bought,
                effective_price,
                sol_spent,
                &dca_signature,
                true,
                Some(fee_lamports),
              )
              .await
              {
                logger::error(
                  LogTag::Positions,
                  &format!(
                    "Failed to persist DCA entry history for position {}: {}",
                    position_id, err
                  ),
                );
              }

              if let Err(err) = clear_pending_dca_swap(&dca_signature).await {
                return Err(format!(
                  "Failed to clear pending DCA {} for position {}: {}",
                  dca_signature, position_id, err
                ));
              }

              crate::events::record_position_event(
                &position_id.to_string(),
                &position.mint,
                "dca_verified",
                position.entry_transaction_signature.as_deref(),
                None,
                sol_spent,
                tokens_bought,
                None,
                None,
              )
              .await;

              logger::info(
                LogTag::Positions,
                &format!(
 "DCA verified for position {}: {} tokens bought, new average entry: {:.11}",
                  position_id,
                  tokens_bought,
                  position.average_entry_price
                ),
              );

              // Queue Telegram notification for DCA executed
              if with_config(|c| c.telegram.enabled && c.telegram.notify_dca_executed) {
                queue_notification(Notification::dca_executed(
                  position.symbol.clone(),
                  position.mint.clone(),
                  sol_spent,
                  position.total_size_sol,
                  position.dca_count,
                ));
              }

              // IMPORTANT: Do NOT consume another semaphore permit - same position!
            }
            Err(e) => {
              return Err(format!("Failed to update database: {}", e));
            }
          }
        }
      }
    }

    PositionTransition::DcaFailed {
      position_id,
      dca_signature,
      reason,
    } => {
      // Record DCA failure event
      if let Some(position) = get_position_by_id(position_id).await {
        crate::events::record_position_event(
          &position_id.to_string(),
          &position.mint,
          "dca_failed",
          position.entry_transaction_signature.as_deref(),
          Some(&dca_signature),
          position.total_size_sol,
          position.remaining_token_amount.unwrap_or(0),
          None,
          None,
        )
        .await;
      }

      logger::error(
        LogTag::Positions,
        &format!("DCA failed for position {}: {}", position_id, reason),
      );

      if let Err(err) = clear_pending_dca_swap(&dca_signature).await {
        return Err(format!(
          "Failed to clear pending DCA {} after failure: {}",
          dca_signature, err
        ));
      }
      // TODO: Implement retry logic if needed
    }

    PositionTransition::UpdatePriceTracking {
      mint,
      current_price,
      highest,
      lowest,
    } => {
      let updated = update_position_state(&mint, |pos| {
        let now = Utc::now();
        pos.current_price = Some(current_price);
        pos.current_price_updated = Some(now);
        if let Some(high) = highest {
          pos.price_highest = high;
        }
        if let Some(low) = lowest {
          pos.price_lowest = low;
        }
      })
      .await;

      if updated {
        if let Some(position) = get_position_by_mint(&mint).await {
          match update_position_price_fields(&position).await {
            Ok(_) => {
              effects.db_updated = true;
            }
            Err(err) => {
              logger::error(
                LogTag::Positions,
                &format!(
                  "Failed to persist price update for mint {} (id={:?}): {}",
                  mint, position.id, err
                ),
              );
            }
          }
        } else {
          logger::debug(
            LogTag::Positions,
            &format!(
              "Price update transition applied but position missing from state (mint={})",
              mint
            ),
          );
        }
      }
    }
  }

  Ok(effects)
}

async fn find_mint_by_position_id(position_id: i64) -> Result<String, String> {
  let positions = POSITIONS.read().await;
  positions
    .iter()
    .find(|p| p.id == Some(position_id))
    .map(|p| p.mint.clone())
    .ok_or_else(|| format!("Position not found: {}", position_id))
}
