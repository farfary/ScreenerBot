//! Profit system simulation
//! Simulates price paths to evaluate profiting logic.

use screenerbot::positions::Position;
use screenerbot::profit::should_sell;
use chrono::{ Utc, Duration as ChronoDuration };

#[tokio::main]
async fn main() {
    println!("=== Profiting System Simulation ===");
    println!("Testing price path: rapid rise to 150%, retracement, and slow decline");
    println!();

    // Test multiple scenarios
    let scenarios = vec![
        (
            "Quick Moon",
            vec![
                (0.0, 1.0),
                (0.5, 1.15), // +15% in 30s
                (1.0, 1.35) // +35% in 1m - should trigger quick capture
            ],
        ),
        (
            "Slow Grind Up",
            vec![
                (0.0, 1.0),
                (10.0, 1.25), // +25% in 10m
                (20.0, 1.45), // +45% in 20m
                (30.0, 1.65), // +65% in 30m
                (45.0, 1.8), // +80% in 45m
                (60.0, 1.75), // Retrace to +75%
                (90.0, 1.6) // Further retrace to +60%
            ],
        ),
        (
            "Mega Pump",
            vec![
                (0.0, 1.0),
                (2.0, 1.8), // +80% in 2m
                (5.0, 2.6) // +160% in 5m - should trigger instant exit
            ],
        ),
        (
            "Loss Scenario",
            vec![
                (0.0, 1.0),
                (10.0, 0.85), // -15%
                (20.0, 0.7), // -30%
                (30.0, 0.55) // -45% - should trigger stop loss
            ],
        )
    ];

    for (scenario_name, path) in scenarios {
        println!("--- {} ---", scenario_name);

        // Create synthetic position
        let mut pos = Position {
            id: None,
            mint: "SIMULATED".into(),
            symbol: "SIM".into(),
            name: "SimToken".into(),
            entry_price: 1.0,
            entry_time: Utc::now(),
            exit_price: None,
            exit_time: None,
            position_type: "buy".into(),
            entry_size_sol: 0.01,
            total_size_sol: 0.01,
            price_highest: 1.0,
            price_lowest: 1.0,
            entry_transaction_signature: None,
            exit_transaction_signature: None,
            token_amount: Some(1_000_000_000),
            effective_entry_price: Some(1.0),
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
        };

        println!("minute,price,profit%,peak%,drawdown%,decision");

        for (minute, price) in path {
            // Update position highest/lowest prices
            if price > pos.price_highest {
                pos.price_highest = price;
            }
            if price < pos.price_lowest {
                pos.price_lowest = price;
            }

            // Adjust entry_time to simulate elapsed minutes
            pos.entry_time = Utc::now() - ChronoDuration::minutes(minute as i64);

            let decision = should_sell(&pos, price).await;
            let profit_pct = (price / pos.entry_price - 1.0) * 100.0;
            let peak_profit = ((pos.price_highest - pos.entry_price) / pos.entry_price) * 100.0;
            let drawdown = peak_profit - profit_pct;

            println!(
                "{:.1},{:.4},{:.2},{:.2},{:.2},{}",
                minute,
                price,
                profit_pct,
                peak_profit,
                drawdown,
                if decision {
                    "SELL"
                } else {
                    "HOLD"
                }
            );

            if decision {
                println!("-> EXIT at {:.1} minutes with {:.2}% profit", minute, profit_pct);
                break;
            }
        }
        println!();
    }
}
