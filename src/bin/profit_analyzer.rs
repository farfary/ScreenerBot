//! Comprehensive profit system analyzer
//! Deep analysis of profit decision logic, parameter sensitivity, and edge cases

use screenerbot::positions_types::Position;
use screenerbot::profit::*;
use chrono::{ Utc, Duration as ChronoDuration };

// Quick capture windows (from profit.rs - not exposed)
const QUICK_WINDOWS: &[(f64, f64)] = &[
    (1.0, 30.0), // 30% profit in 1 minute = instant exit
    (5.0, 50.0), // 50% profit in 5 minutes = instant exit
    (15.0, 80.0), // 80% profit in 15 minutes = instant exit
];

#[tokio::main]
async fn main() {
    println!("=== PROFIT SYSTEM ANALYZER ===\n");

    // Run all analysis modules
    analyze_decision_breakdown().await;
    analyze_parameter_sensitivity().await;
    analyze_edge_cases().await;
    analyze_timing_windows().await;
    analyze_trailing_behavior().await;

    println!("\n=== ANALYSIS COMPLETE ===");
}

/// Analyze decision breakdown for key scenarios
async fn analyze_decision_breakdown() {
    println!("--- DECISION BREAKDOWN ANALYSIS ---");

    let test_cases = vec![
        ("Quick 30% in 1min", 1.0, 1.0, 1.3, 1.0),
        ("Slow 50% in 30min", 30.0, 1.0, 1.5, 1.0),
        ("Peak 100% now 80%", 45.0, 1.0, 2.0, 1.8),
        ("Loss -30%", 20.0, 1.0, 1.0, 0.7),
        ("Stale 25% in 90min", 90.0, 1.0, 1.25, 1.25)
    ];

    for (name, minutes, entry, peak, current) in test_cases {
        let pos = create_test_position(entry, minutes, peak, entry.min(current));
        let decision = should_sell(&pos, current).await;

        let profit_pct = ((current - entry) / entry) * 100.0;
        let peak_profit = ((peak - entry) / entry) * 100.0;
        let drawdown = peak_profit - profit_pct;
        let gap = trailing_gap(peak_profit, minutes);
        let odds = continuation_odds(profit_pct, minutes);

        println!(
            "{}: {} | Profit: {:.1}% | Peak: {:.1}% | DD: {:.1}% | Gap: {:.1}% | Odds: {:.2} | Time: {:.0}m",
            name,
            if decision {
                "SELL"
            } else {
                "HOLD"
            },
            profit_pct,
            peak_profit,
            drawdown,
            gap,
            odds,
            minutes
        );

        // Analyze why decision was made
        analyze_decision_reasons(&pos, current, decision).await;
        println!();
    }
}

/// Analyze parameter sensitivity
async fn analyze_parameter_sensitivity() {
    println!("--- PARAMETER SENSITIVITY ANALYSIS ---");

    // Test trailing gap sensitivity
    println!("Trailing Gap Analysis (70% peak profit):");
    for minutes in [10.0, 30.0, 45.0, 60.0, 90.0, 120.0] {
        let gap = trailing_gap(70.0, minutes);
        println!("  {}min: {:.1}% gap", minutes, gap);
    }

    // Test odds sensitivity
    println!("\nContinuation Odds Analysis:");
    for (profit, time) in [
        (20.0, 30.0),
        (50.0, 60.0),
        (80.0, 90.0),
        (100.0, 120.0),
    ] {
        let odds = continuation_odds(profit, time);
        println!("  {:.0}% profit at {:.0}min: {:.2} odds", profit, time, odds);
    }

    // Test critical thresholds
    println!("\nCritical Threshold Tests:");
    test_threshold_sensitivity().await;
}

/// Test threshold sensitivity around critical values
async fn test_threshold_sensitivity() {
    let base_pos = create_test_position(1.0, 45.0, 1.5, 1.0);

    // Test around BASE_MIN_PROFIT_PERCENT
    println!("  Base Min Profit Threshold ({:.0}%):", BASE_MIN_PROFIT_PERCENT);
    for offset in [-2.0, -1.0, 0.0, 1.0, 2.0] {
        let test_profit = BASE_MIN_PROFIT_PERCENT + offset;
        let price = 1.0 * (1.0 + test_profit / 100.0);
        let decision = should_sell(&base_pos, price).await;
        println!("    {:.1}%: {}", test_profit, if decision { "SELL" } else { "HOLD" });
    }

    // Test around INSTANT_EXIT_LEVEL_1
    println!("  Instant Exit Level 1 ({:.0}%):", INSTANT_EXIT_LEVEL_1);
    for offset in [-10.0, -5.0, 0.0, 5.0, 10.0] {
        let test_profit = INSTANT_EXIT_LEVEL_1 + offset;
        let price = 1.0 * (1.0 + test_profit / 100.0);
        let decision = should_sell(&base_pos, price).await;
        println!("    {:.1}%: {}", test_profit, if decision { "SELL" } else { "HOLD" });
    }
}

/// Analyze edge cases and potential issues
async fn analyze_edge_cases() {
    println!("--- EDGE CASE ANALYSIS ---");

    let edge_cases = vec![
        ("Zero profit at max time", 120.0, 1.0, 1.0, 1.0),
        ("Tiny profit long hold", 100.0, 1.0, 1.02, 1.02),
        ("Massive instant gain", 1.0, 1.0, 3.0, 3.0),
        ("Deep loss recovery", 60.0, 1.0, 1.2, 0.6),
        ("Price equal to entry", 30.0, 1.0, 1.3, 1.0),
        ("Negative price (invalid)", 30.0, 1.0, 1.3, -0.5),
        ("Zero price (invalid)", 30.0, 1.0, 1.3, 0.0)
    ];

    for (name, minutes, entry, peak, current) in edge_cases {
        let pos = create_test_position(entry, minutes, peak, entry.min(current));
        let decision = should_sell(&pos, current).await;

        if current <= 0.0 || !current.is_finite() {
            println!("{}: {} (Invalid price handled correctly)", name, if decision {
                "SELL"
            } else {
                "HOLD"
            });
        } else {
            let profit_pct = ((current - entry) / entry) * 100.0;
            println!(
                "{}: {} | Profit: {:.1}%",
                name,
                if decision {
                    "SELL"
                } else {
                    "HOLD"
                },
                profit_pct
            );
        }
    }
}

/// Analyze timing windows and quick capture
async fn analyze_timing_windows() {
    println!("\n--- TIMING WINDOW ANALYSIS ---");

    println!("Quick Capture Windows:");
    for (window_min, required_profit) in QUICK_WINDOWS {
        println!("  {:.0}min window requires {:.0}% profit", window_min, required_profit);

        // Test just below and above threshold
        for test_profit in [required_profit - 5.0, required_profit + 5.0] {
            let pos = create_test_position(
                1.0,
                *window_min - 0.1,
                1.0 + test_profit / 100.0,
                1.0 + test_profit / 100.0
            );
            let price = 1.0 + test_profit / 100.0;
            let decision = should_sell(&pos, price).await;
            println!("    {:.0}% at {:.1}min: {}", test_profit, window_min - 0.1, if decision {
                "SELL"
            } else {
                "HOLD"
            });
        }
    }
}

/// Analyze trailing stop behavior in detail
async fn analyze_trailing_behavior() {
    println!("\n--- TRAILING STOP BEHAVIOR ---");

    // Simulate a position that peaks then retraces
    let entry_price = 1.0;
    let peak_price = 1.8; // 80% gain
    let peak_profit = 80.0;

    println!("Peak profit scenario (80% peak):");
    println!("Time, CurrentPrice, Profit%, Drawdown%, Gap%, Decision");

    for minutes in [30.0, 45.0, 60.0, 75.0, 90.0, 105.0] {
        for current_profit in [75.0, 70.0, 65.0, 60.0, 55.0, 50.0] {
            let current_price = entry_price * (1.0 + current_profit / 100.0);
            let drawdown = peak_profit - current_profit;
            let gap = trailing_gap(peak_profit, minutes);

            let pos = create_test_position(
                entry_price,
                minutes,
                peak_price,
                current_price.min(entry_price)
            );
            let decision = should_sell(&pos, current_price).await;

            if drawdown >= gap {
                println!(
                    "{:.0}, {:.3}, {:.0}%, {:.1}%, {:.1}%, {} *TRIGGERED*",
                    minutes,
                    current_price,
                    current_profit,
                    drawdown,
                    gap,
                    if decision {
                        "SELL"
                    } else {
                        "HOLD"
                    }
                );
            }
        }
    }
}

/// Analyze the specific reasons a decision was made
async fn analyze_decision_reasons(pos: &Position, current_price: f64, decision: bool) {
    if current_price <= 0.0 || !current_price.is_finite() {
        println!("  -> Invalid price");
        return;
    }

    let entry = pos.effective_entry_price.unwrap_or(pos.entry_price);
    let pnl_percent = ((current_price - entry) / entry) * 100.0;
    let minutes_held = ((Utc::now() - pos.entry_time).num_seconds() as f64) / 60.0;
    let peak_price = pos.price_highest.max(current_price);
    let peak_profit = ((peak_price - entry) / entry) * 100.0;
    let drawdown = peak_profit - pnl_percent;

    let mut reasons = Vec::new();

    // Check each condition
    if pnl_percent <= EXTREME_LOSS_PERCENT {
        reasons.push("Extreme loss".to_string());
    }
    if pnl_percent <= STOP_LOSS_PERCENT && minutes_held >= 1.0 {
        reasons.push("Stop loss".to_string());
    }
    if minutes_held >= MAX_HOLD_MINUTES {
        reasons.push("Max hold time".to_string());
    }

    // Quick capture
    for (window_minutes, required_profit) in QUICK_WINDOWS {
        if minutes_held <= *window_minutes && pnl_percent >= *required_profit {
            reasons.push(
                format!("Quick capture ({:.0}min/{:.0}%)", window_minutes, required_profit)
            );
            break;
        }
    }

    // Instant exits
    if pnl_percent >= INSTANT_EXIT_LEVEL_2 {
        reasons.push("Instant exit level 2".to_string());
    }
    if pnl_percent >= INSTANT_EXIT_LEVEL_1 && (drawdown >= 10.0 || minutes_held > 10.0) {
        reasons.push("Instant exit level 1".to_string());
    }

    // Minimum profit gate
    if pnl_percent < BASE_MIN_PROFIT_PERCENT && decision {
        reasons.push("Below min profit but other trigger".to_string());
    }
    if pnl_percent < BASE_MIN_PROFIT_PERCENT && !decision {
        reasons.push("Below min profit - holding".to_string());
    }

    // Trailing stop
    if peak_profit >= BASE_MIN_PROFIT_PERCENT {
        let gap = trailing_gap(peak_profit, minutes_held);
        if drawdown >= gap {
            reasons.push(format!("Trailing stop (gap: {:.1}%)", gap));
        }
    }

    // Odds-based
    let odds = continuation_odds(pnl_percent, minutes_held);
    if odds < EXIT_ODDS_THRESHOLD {
        reasons.push(format!("Low odds ({:.2})", odds));
    }

    if reasons.is_empty() {
        reasons.push("No clear trigger - default hold".to_string());
    }

    println!("  -> Reasons: {}", reasons.join(", "));
}

/// Helper to create test positions
fn create_test_position(entry_price: f64, minutes_ago: f64, highest: f64, lowest: f64) -> Position {
    Position {
        id: None,
        mint: "SIM".into(),
        symbol: "SIM".into(),
        name: "SimToken".into(),
        entry_price,
        entry_time: Utc::now() - ChronoDuration::minutes(minutes_ago as i64),
        exit_price: None,
        exit_time: None,
        position_type: "buy".into(),
        entry_size_sol: 0.01,
        total_size_sol: 0.01,
        price_highest: highest,
        price_lowest: lowest,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
        token_amount: Some(1_000_000_000),
        effective_entry_price: Some(entry_price),
        effective_exit_price: None,
        sol_received: None,
        profit_target_min: None,
        profit_target_max: None,
        liquidity_tier: Some("MEDIUM".into()),
        transaction_entry_verified: true,
        transaction_exit_verified: false,
        entry_fee_lamports: None,
        exit_fee_lamports: None,
        current_price: None,
        current_price_updated: None,
        phantom_remove: false,
        phantom_confirmations: 0,
        phantom_first_seen: None,
        synthetic_exit: false,
        closed_reason: None,
    }
}
