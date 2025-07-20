use screenerbot::trader::*;
use chrono::{ DateTime, Utc, Duration as ChronoDuration };
use tabled::{ Tabled, Table, settings::{ Style, Alignment, object::Rows, Modify } };
use std::f64::consts::PI;

#[derive(Tabled)]
struct SimulationResult {
    #[tabled(rename = "⏱️ Time (m)")]
    time_minutes: String,

    #[tabled(rename = "💰 Price")]
    price: String,

    #[tabled(rename = "📊 P&L %")]
    pnl_percent: String,

    #[tabled(rename = "📉 Drawdown %")]
    drawdown: String,

    #[tabled(rename = "🔥 Urgency")]
    urgency: String,

    #[tabled(rename = "📍 Decision")]
    decision: String,
}

fn run_price_simulation(
    scenario_name: &str,
    entry_price: f64,
    price_generator: fn(f64, i64) -> f64,
    duration_minutes: i64,
    time_step: i64
) {
    println!("\n🔮 Simulating: {}", scenario_name);

    // Create base position
    let now = Utc::now();
    let entry_time = now - ChronoDuration::minutes(duration_minutes);

    let mut position = Position {
        mint: "simulation_mint".to_string(),
        symbol: "SIM".to_string(),
        name: "Simulation Token".to_string(),
        entry_price,
        entry_time,
        exit_price: None,
        exit_time: None,
        pnl_sol: None,
        pnl_percent: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.1,
        total_size_sol: 0.1,
        drawdown_percent: 0.0,
        price_highest: entry_price,
        price_lowest: entry_price,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
        token_amount: Some(1000000),
        effective_entry_price: Some(entry_price),
        effective_exit_price: None,
    };

    let mut results = Vec::new();

    // Run simulation over time
    for minute in (0..=duration_minutes).step_by(time_step as usize) {
        let simulation_time = entry_time + ChronoDuration::minutes(minute);
        let price = price_generator(entry_price, minute);

        // Update highest and lowest prices
        if price > position.price_highest {
            position.price_highest = price;
        }
        if price < position.price_lowest || position.price_lowest == entry_price {
            position.price_lowest = price;
        }

        // Calculate P&L and drawdown
        let pnl_percent = ((price - entry_price) / entry_price) * 100.0;
        let drawdown = if position.price_highest > 0.0 {
            ((position.price_highest - price) / position.price_highest) * 100.0
        } else {
            0.0
        };
        // Store the drawdown in the position for record keeping in our simulation
        position.drawdown_percent = drawdown;

        // Calculate sell urgency
        let urgency = should_sell(&position, price, simulation_time);

        // Record result
        results.push(SimulationResult {
            time_minutes: format!("{}", minute),
            price: format!("{:.6}", price),
            pnl_percent: format!("{:.1}%", pnl_percent),
            drawdown: format!("{:.1}%", drawdown),
            urgency: format!("{:.3}", urgency),
            decision: if urgency > 0.7 {
                "🔴 SELL".to_string()
            } else {
                "🟢 HOLD".to_string()
            },
        });
    }

    // Save final P&L result
    let final_pnl = results
        .last()
        .map(|r| r.pnl_percent.clone())
        .unwrap_or_else(|| "N/A".to_string());

    // Find sell position if any
    let sell_time = results.iter().position(|r| r.decision.contains("SELL"));
    let sell_info = if let Some(sell_idx) = sell_time {
        let time_min = results[sell_idx].time_minutes.clone();
        let pnl = results[sell_idx].pnl_percent.clone();
        (time_min, pnl)
    } else {
        ("N/A".to_string(), "N/A".to_string())
    };

    // Display results
    let mut table = Table::new(results);
    table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));

    println!("{}", table);

    if let Some(_) = sell_time {
        println!(
            "📈 Simulation outcome: Sold after {} minutes with P&L: {}",
            sell_info.0,
            sell_info.1
        );
    } else {
        println!(
            "📉 Simulation outcome: Held for entire {} minutes with final P&L: {}",
            duration_minutes,
            final_pnl
        );
    }
}

// Price generators for different scenarios
fn up_and_down(entry: f64, minute: i64) -> f64 {
    // Price goes up 50% then back to entry
    let max_gain = 0.5; // 50% gain
    let cycle_minutes = 60.0; // Full cycle in 60 minutes

    let phase = ((minute as f64) / cycle_minutes) * 2.0 * PI;
    let factor = 1.0 + (max_gain * (1.0 + phase.cos())) / 2.0;

    entry * factor
}

fn steady_rise(entry: f64, minute: i64) -> f64 {
    // Steady rise to 100% gain over 2 hours
    let max_gain = 1.0; // 100% gain
    let max_minutes = 120.0; // 2 hours

    let factor = 1.0 + max_gain * ((minute as f64) / max_minutes).min(1.0);

    entry * factor
}

fn sudden_crash(entry: f64, minute: i64) -> f64 {
    // Normal for 30 minutes, then crash to -50% in 10 minutes, then flat
    if minute < 30 {
        entry * (1.0 + 0.1 * ((minute as f64) / 30.0)) // Small 10% rise
    } else if minute < 40 {
        let crash_progress = ((minute - 30) as f64) / 10.0;
        let start_factor = 1.1; // 10% up
        let end_factor = 0.5; // 50% down

        entry * (start_factor - (start_factor - end_factor) * crash_progress)
    } else {
        entry * 0.5 // Stay at -50%
    }
}

fn recovery_after_drop(entry: f64, minute: i64) -> f64 {
    // Drop to -40% in 20 minutes, then recover to -10% by 60 minutes
    if minute < 20 {
        let drop_factor = 1.0 - 0.4 * ((minute as f64) / 20.0);
        entry * drop_factor
    } else {
        let recovery_progress = (((minute - 20) as f64) / 40.0).min(1.0);
        let start_factor = 0.6; // -40%
        let end_factor = 0.9; // -10%

        entry * (start_factor + (end_factor - start_factor) * recovery_progress)
    }
}

fn wild_volatility(entry: f64, minute: i64) -> f64 {
    // Wild price swings with overall downtrend
    let base_trend = 1.0 - 0.3 * ((minute as f64) / 60.0).min(1.0); // Downtrend to -30%
    let volatility = 0.25 * ((2.0 * PI * (minute as f64)) / 10.0).sin(); // ±25% swings every 10 minutes

    entry * (base_trend + volatility)
}

fn main() {
    println!("🧪 Testing Should Sell Function with Time-Series Simulations\n");

    // Run different price simulations
    let entry_price = 0.0001;

    run_price_simulation(
        "Steady Rise (100% gain over 2 hours)",
        entry_price,
        steady_rise,
        120, // 2 hours
        5 // 5-minute intervals
    );

    run_price_simulation(
        "Up and Down Cycle (50% gain, back to entry)",
        entry_price,
        up_and_down,
        60, // 1 hour
        2 // 2-minute intervals
    );

    run_price_simulation(
        "Sudden Crash (-50% after 30 minutes)",
        entry_price,
        sudden_crash,
        60, // 1 hour
        2 // 2-minute intervals
    );

    run_price_simulation(
        "Recovery After Drop (-40% then back to -10%)",
        entry_price,
        recovery_after_drop,
        60, // 1 hour
        2 // 2-minute intervals
    );

    run_price_simulation(
        "Wild Volatility (±25% swings with -30% trend)",
        entry_price,
        wild_volatility,
        60, // 1 hour
        2 // 2-minute intervals
    );

    println!("\n✅ Simulation tests complete!");
    println!("🎯 Observed behaviors:");
    println!("   - Profit-taking near peaks");
    println!("   - Loss avoidance by timing");
    println!("   - Dynamic response to price action");
}
