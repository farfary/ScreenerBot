/// Loss prevention system to analyze token history and prevent buying tokens with poor performance
use crate::positions::{ Position, calculate_position_pnl, SAVED_POSITIONS };
use crate::logger::{ log, LogTag };
use std::collections::HashMap;

/// Configuration for loss prevention system
pub const LOSS_PREVENTION_ENABLED: bool = true;
pub const MIN_CLOSED_POSITIONS_FOR_ANALYSIS: usize = 2; // Need at least 2 closed positions to analyze
pub const MAX_LOSS_RATE_PERCENT: f64 = 70.0; // Don't buy if more than 70% of positions were losses
pub const MAX_AVERAGE_LOSS_PERCENT: f64 = -15.0; // Don't buy if average loss is worse than -15%
pub const LOOKBACK_HOURS: i64 = 168; // Look back 7 days (168 hours) for position history

/// Statistics for a token's trading history
#[derive(Debug, Clone)]
pub struct TokenLossStats {
    pub mint: String,
    pub symbol: String,
    pub total_closed_positions: usize,
    pub losing_positions: usize,
    pub winning_positions: usize,
    pub loss_rate_percent: f64,
    pub average_pnl_percent: f64,
    pub total_pnl_sol: f64,
    pub worst_loss_percent: f64,
    pub best_gain_percent: f64,
}

/// Check if a token should be avoided based on its historical performance
/// Returns true if the token is safe to buy, false if it should be avoided
pub fn should_allow_token_purchase(mint: &str, symbol: &str) -> bool {
    if !LOSS_PREVENTION_ENABLED {
        return true;
    }

    let stats = analyze_token_loss_history(mint, symbol);

    // If we don't have enough data, allow the purchase (benefit of the doubt)
    if stats.total_closed_positions < MIN_CLOSED_POSITIONS_FOR_ANALYSIS {
        log(
            LogTag::Trader,
            "LOSS_PREVENTION",
            &format!(
                "Allowing {} purchase - insufficient history ({} positions < {} required)",
                symbol,
                stats.total_closed_positions,
                MIN_CLOSED_POSITIONS_FOR_ANALYSIS
            )
        );
        return true;
    }

    // Check loss rate threshold
    if stats.loss_rate_percent > MAX_LOSS_RATE_PERCENT {
        log(
            LogTag::Trader,
            "BUY_BLOCKED",
            &format!(
                "❌ Blocking {} purchase - high loss rate: {:.1}% losses ({}/{} positions) exceeds {:.1}% threshold",
                symbol,
                stats.loss_rate_percent,
                stats.losing_positions,
                stats.total_closed_positions,
                MAX_LOSS_RATE_PERCENT
            )
        );
        return false;
    }

    // Check average loss threshold
    if stats.average_pnl_percent < MAX_AVERAGE_LOSS_PERCENT {
        log(
            LogTag::Trader,
            "BUY_BLOCKED",
            &format!(
                "❌ Blocking {} purchase - poor average P&L: {:.1}% < {:.1}% threshold (Total: {:.6} SOL)",
                symbol,
                stats.average_pnl_percent,
                MAX_AVERAGE_LOSS_PERCENT,
                stats.total_pnl_sol
            )
        );
        return false;
    }

    // Token passes all checks
    log(
        LogTag::Trader,
        "LOSS_PREVENTION",
        &format!(
            "✅ Allowing {} purchase - good history: {:.1}% loss rate, {:.1}% avg P&L, {:.6} SOL total",
            symbol,
            stats.loss_rate_percent,
            stats.average_pnl_percent,
            stats.total_pnl_sol
        )
    );

    true
}

/// Analyze historical performance of a specific token
pub fn analyze_token_loss_history(mint: &str, symbol: &str) -> TokenLossStats {
    let mut stats = TokenLossStats {
        mint: mint.to_string(),
        symbol: symbol.to_string(),
        total_closed_positions: 0,
        losing_positions: 0,
        winning_positions: 0,
        loss_rate_percent: 0.0,
        average_pnl_percent: 0.0,
        total_pnl_sol: 0.0,
        worst_loss_percent: 0.0,
        best_gain_percent: 0.0,
    };

    let positions = match SAVED_POSITIONS.lock() {
        Ok(positions) => positions,
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Failed to acquire positions lock for loss analysis: {}", e)
            );
            return stats;
        }
    };

    let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(LOOKBACK_HOURS);
    let mut total_pnl_percent = 0.0;
    let mut total_pnl_sol = 0.0;
    let mut worst_loss = 0.0;
    let mut best_gain = 0.0;

    // Analyze closed positions for this specific token
    for position in positions.iter() {
        // Only analyze closed positions for this mint within the lookback period
        if
            position.mint == mint &&
            position.exit_price.is_some() &&
            position.entry_time >= cutoff_time
        {
            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);

            stats.total_closed_positions += 1;
            total_pnl_percent += pnl_percent;
            total_pnl_sol += pnl_sol;

            if pnl_percent < 0.0 {
                stats.losing_positions += 1;
                if pnl_percent < worst_loss {
                    worst_loss = pnl_percent;
                }
            } else {
                stats.winning_positions += 1;
                if pnl_percent > best_gain {
                    best_gain = pnl_percent;
                }
            }
        }
    }

    // Calculate statistics
    if stats.total_closed_positions > 0 {
        stats.loss_rate_percent =
            ((stats.losing_positions as f64) / (stats.total_closed_positions as f64)) * 100.0;
        stats.average_pnl_percent = total_pnl_percent / (stats.total_closed_positions as f64);
        stats.total_pnl_sol = total_pnl_sol;
        stats.worst_loss_percent = worst_loss;
        stats.best_gain_percent = best_gain;
    }

    stats
}

/// Get comprehensive loss analysis for all tokens with trading history
pub fn get_comprehensive_loss_analysis() -> HashMap<String, TokenLossStats> {
    let mut token_stats = HashMap::new();

    let positions = match SAVED_POSITIONS.lock() {
        Ok(positions) => positions,
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Failed to acquire positions lock for comprehensive analysis: {}", e)
            );
            return token_stats;
        }
    };

    let cutoff_time = chrono::Utc::now() - chrono::Duration::hours(LOOKBACK_HOURS);

    // Collect all unique mints with closed positions
    let mut unique_mints = std::collections::HashSet::new();
    for position in positions.iter() {
        if position.exit_price.is_some() && position.entry_time >= cutoff_time {
            unique_mints.insert((position.mint.clone(), position.symbol.clone()));
        }
    }

    // Analyze each token
    for (mint, symbol) in unique_mints {
        let stats = analyze_token_loss_history(&mint, &symbol);
        if stats.total_closed_positions > 0 {
            token_stats.insert(mint, stats);
        }
    }

    token_stats
}

/// Print detailed loss analysis report to console
pub fn print_loss_analysis_report() {
    let analysis = get_comprehensive_loss_analysis();

    if analysis.is_empty() {
        log(
            LogTag::Trader,
            "LOSS_ANALYSIS",
            "No closed positions found in the last week for analysis"
        );
        return;
    }

    log(
        LogTag::Trader,
        "LOSS_ANALYSIS",
        &format!("📊 Token Performance Analysis (Last {} hours)", LOOKBACK_HOURS)
    );

    // Sort by loss rate (worst first)
    let mut sorted_tokens: Vec<_> = analysis.values().collect();
    sorted_tokens.sort_by(|a, b| b.loss_rate_percent.partial_cmp(&a.loss_rate_percent).unwrap());

    for stats in sorted_tokens {
        let status = if
            stats.loss_rate_percent > MAX_LOSS_RATE_PERCENT ||
            stats.average_pnl_percent < MAX_AVERAGE_LOSS_PERCENT
        {
            "🚫 BLOCKED"
        } else {
            "✅ ALLOWED"
        };

        log(
            LogTag::Trader,
            "LOSS_ANALYSIS",
            &format!(
                "{} {} - {}/{} losses ({:.1}%), Avg P&L: {:.1}%, Total: {:.6} SOL, Range: {:.1}% to {:.1}%",
                status,
                stats.symbol,
                stats.losing_positions,
                stats.total_closed_positions,
                stats.loss_rate_percent,
                stats.average_pnl_percent,
                stats.total_pnl_sol,
                stats.worst_loss_percent,
                stats.best_gain_percent
            )
        );
    }
}
