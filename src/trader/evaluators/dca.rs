//! Dollar Cost Averaging (DCA) implementation
//!
//! Merged DCA orchestration and evaluation logic.
//! Evaluates all open positions for DCA opportunities based on:
//! - Price drop threshold
//! - DCA count limits
//! - Cooldown periods
//! - DCA size percentage
//!
//! NOTE: This evaluator uses simple price-threshold logic and does NOT integrate
//! with the strategy system or OHLCV data. For OHLCV-based DCA decisions, consider
//! creating DCA-specific strategies in the strategy system instead.

use crate::logger::{self, LogTag};
use crate::positions;
use crate::positions::Position;
use crate::trader::config;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Configuration snapshot for DCA evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaConfigSnapshot {
  pub enabled: bool,
  pub max_count: u32,
  pub cooldown_minutes: i64,
  pub threshold_pct: f64,
  pub size_percentage: f64,
}

/// Calculated metrics for DCA evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaCalculations {
  pub current_dca_count: u32,
  pub minutes_since_last: Option<i64>,
  pub pnl_pct: f64,
  pub required_drop_pct: f64,
  pub dca_amount_sol: f64,
  pub entry_price: f64,
  pub current_price: f64,
}

/// Structured DCA evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcaEvaluation {
  pub should_trigger: bool,
  pub reasons: Vec<String>,
  pub config: DcaConfigSnapshot,
  pub calculations: DcaCalculations,
}

impl DcaEvaluation {
  /// Evaluate whether DCA should trigger for a position
  pub fn evaluate(position: &Position, config: DcaConfigSnapshot) -> Result<Self, String> {
    let mut reasons = Vec::new();
    let mut should_trigger = true;

    // Extract position data
    let current_price = match position.current_price {
      Some(price) if price > 0.0 && price.is_finite() => price,
      _ => {
        return Ok(Self {
          should_trigger: false,
          reasons: vec!["No valid current price".to_string()],
          config: config.clone(),
          calculations: DcaCalculations {
            current_dca_count: position.dca_count,
            minutes_since_last: None,
            pnl_pct: 0.0,
            required_drop_pct: config.threshold_pct.abs(),
            dca_amount_sol: 0.0,
            entry_price: position.average_entry_price,
            current_price: 0.0,
          },
        });
      }
    };

    let entry_price = position.average_entry_price;
    if entry_price <= 0.0 || !entry_price.is_finite() {
      return Ok(Self {
        should_trigger: false,
        reasons: vec!["No valid entry price".to_string()],
        config: config.clone(),
        calculations: DcaCalculations {
          current_dca_count: position.dca_count,
          minutes_since_last: None,
          pnl_pct: 0.0,
          required_drop_pct: config.threshold_pct.abs(),
          dca_amount_sol: 0.0,
          entry_price,
          current_price,
        },
      });
    }

    // Calculate metrics
    let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;
    let required_drop_pct = config.threshold_pct.abs();
    let dca_amount_sol = position.entry_size_sol * (config.size_percentage / 100.0);

    let minutes_since_last = position
      .last_dca_time
      .map(|t| (Utc::now() - t).num_minutes());

    let calculations = DcaCalculations {
      current_dca_count: position.dca_count,
      minutes_since_last,
      pnl_pct,
      required_drop_pct,
      dca_amount_sol,
      entry_price,
      current_price,
    };

    // Evaluate conditions
    if !config.enabled {
      should_trigger = false;
      reasons.push("DCA disabled in config".to_string());
    }

    if calculations.current_dca_count >= config.max_count {
      should_trigger = false;
      reasons.push(format!(
        "DCA count limit reached ({}/{})",
        calculations.current_dca_count, config.max_count
      ));
    }

    if let Some(minutes) = calculations.minutes_since_last {
      if minutes < config.cooldown_minutes {
        should_trigger = false;
        reasons.push(format!(
          "DCA cooldown active ({}/{} minutes)",
          minutes, config.cooldown_minutes
        ));
      }
    }

    // Check if price has dropped enough to trigger DCA
    // IMPORTANT: This comparison logic is intentionally inverted for negative percentages
    //
    // Configuration: threshold_pct is NEGATIVE (e.g., -10.0 means "trigger at 10% loss")
 // Current P&L: pnl_pct is NEGATIVE when losing money (e.g., -15.0 means 15% loss)
    //
    // Example scenarios:
 // - Config threshold: -10.0 (trigger at 10% loss)
 // - Current loss: -15.0 (losing 15%)
 // - Check: -15.0 >= -10.0? NO → -15.0 < -10.0 → Trigger DCA 
    //
 // - Config threshold: -10.0 (trigger at 10% loss)
 // - Current loss: -5.0 (losing only 5%)
 // - Check: -5.0 >= -10.0? YES → Not losing enough → Skip DCA 
    //
    // The >= comparison works because more negative = bigger loss
    if pnl_pct >= config.threshold_pct {
      should_trigger = false;
      reasons.push(format!(
        "Price drop insufficient: {:.2}% P&L (need < {:.2}%)",
        pnl_pct, config.threshold_pct
      ));
    }

    // Check if DCA amount is valid and above minimum trade size
    const MIN_TRADE_SIZE_SOL: f64 = 0.001; // Minimum 0.001 SOL (~$0.20 at $200/SOL)
    if calculations.dca_amount_sol < MIN_TRADE_SIZE_SOL {
      should_trigger = false;
      reasons.push(format!(
        "DCA amount {:.6} SOL below minimum {:.6} SOL",
        calculations.dca_amount_sol, MIN_TRADE_SIZE_SOL
      ));
    }

    if should_trigger {
      reasons.push(format!(
        "DCA triggered: {:.2}% loss exceeds {:.2}% threshold (price: {:.9} → {:.9} SOL)",
        pnl_pct, config.threshold_pct, entry_price, current_price
      ));
    }

    Ok(Self {
      should_trigger,
      reasons,
      config,
      calculations,
    })
  }

  /// Get a human-readable summary of the evaluation
  pub fn summary(&self) -> String {
    if self.should_trigger {
      format!(
        "DCA #{}: {:.2}% loss, amount: {:.4} SOL",
        self.calculations.current_dca_count + 1,
        self.calculations.pnl_pct,
        self.calculations.dca_amount_sol
      )
    } else {
      self.reasons.join(", ")
    }
  }
}

/// Process DCA opportunities for eligible positions
pub async fn process_dca_opportunities() -> Result<Vec<TradeDecision>, String> {
  // Build config snapshot (batch read)
  let dca_config = DcaConfigSnapshot {
    enabled: config::is_dca_enabled(),
    max_count: config::get_dca_max_count() as u32,
    cooldown_minutes: config::get_dca_cooldown_minutes(),
    threshold_pct: config::get_dca_threshold_pct(),
    size_percentage: config::get_dca_size_percentage(),
  };

  // Early exit if DCA is disabled
  if !dca_config.enabled {
    return Ok(Vec::new());
  }

  // Get all open positions
  let open_positions = positions::get_open_positions().await;
  if open_positions.is_empty() {
    return Ok(Vec::new());
  }

  let mut dca_decisions = Vec::new();

  for position in open_positions {
    // Skip if position doesn't have ID
    let position_id = match position.id {
      Some(id) => id,
      None => continue,
    };

    // Evaluate DCA opportunity using structured evaluation
    let evaluation = match DcaEvaluation::evaluate(&position, dca_config.clone()) {
      Ok(eval) => eval,
      Err(e) => {
        logger::error(
          LogTag::Trader,
          &format!("DCA evaluation failed for {}: {}", position.symbol, e),
        );
        continue;
      }
    };

    if evaluation.should_trigger {
      logger::info(
        LogTag::Trader,
        &format!(
 "DCA opportunity: {} | {}",
          position.symbol,
          evaluation.summary()
        ),
      );

      dca_decisions.push(TradeDecision {
        position_id: Some(position_id.to_string()),
        mint: position.mint.clone(),
        action: TradeAction::DCA,
        reason: TradeReason::DCAScheduled,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::Normal,
        price_sol: Some(evaluation.calculations.current_price),
        size_sol: Some(evaluation.calculations.dca_amount_sol),
      });
    } else {
      logger::debug(
        LogTag::Trader,
        &format!(
          "DCA not triggered for {}: {}",
          position.symbol,
          evaluation.summary()
        ),
      );
    }
  }

  if !dca_decisions.is_empty() {
    logger::info(
      LogTag::Trader,
      &format!("Found {} DCA opportunities", dca_decisions.len()),
    );
  }

  Ok(dca_decisions)
}
