use anyhow::Result;
use screenerbot::{ config::Config, trader::{ FastProfitStrategy, Position, PositionStatus } };

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Fast Profit Strategy");
    println!("═══════════════════════════════");

    // Load config
    let config = Config::load("configs.json").expect("Failed to load config");

    // Create fast profit strategy
    let mut strategy = FastProfitStrategy::new(config.trader.clone());

    // Test profit targets
    let targets = FastProfitStrategy::get_profit_targets();
    println!("\n📊 Profit Targets:");
    for (i, target) in targets.iter().enumerate() {
        println!(
            "  {}. {:.1}% profit → Sell {:.0}% within {}s",
            i + 1,
            target.percentage,
            target.sell_portion * 100.0,
            target.time_threshold_seconds
        );
    }

    // Create test position with different profit scenarios
    let test_scenarios = vec![
        (15.0, "Quick 15% gain"),
        (35.0, "Medium 35% gain"),
        (75.0, "High 75% gain"),
        (150.0, "Very high 150% gain"),
        (300.0, "Massive 300% gain"),
        (600.0, "Extreme 600% gain")
    ];

    println!("\n🎯 Testing Profit Scenarios:");
    println!("─────────────────────────────");

    for (profit_percent, description) in test_scenarios {
        println!("\n{}", description);

        let mut position = Position::new("test_token_address".to_string(), "TEST".to_string());

        // Set up position as if it just bought
        position.add_buy_trade(0.01, 1000.0, 0.00001, None); // Bought 1000 tokens for 0.01 SOL
        position.status = PositionStatus::Active;

        // Calculate current price for the desired profit
        let target_price = position.average_buy_price * (1.0 + profit_percent / 100.0);
        position.update_price(target_price);

        println!("  💰 Position P&L: {:.2}%", position.unrealized_pnl_percent);

        // Test if position should use fast strategy
        let should_use_fast = strategy.should_use_fast_strategy(&position);
        println!("  🚀 Use fast strategy: {}", should_use_fast);

        // Analyze position for signals
        let signals = strategy.analyze_position_fast(&position, target_price);

        if signals.is_empty() {
            println!("  📊 No signals generated");
        } else {
            for signal in signals {
                match &signal.signal_type {
                    screenerbot::trader::TradeSignalType::FastProfit {
                        profit_percentage,
                        sell_portion,
                        reason,
                    } => {
                        println!(
                            "  ✅ Signal: Sell {:.0}% at {:.1}% profit ({})",
                            sell_portion * 100.0,
                            profit_percentage,
                            reason
                        );

                        let sell_amount = strategy.calculate_fast_sell_amount(
                            &position,
                            *sell_portion
                        );
                        println!("  📤 Sell amount: {:.2} tokens", sell_amount);
                    }
                    screenerbot::trader::TradeSignalType::EmergencyStopLoss => {
                        println!("  🚨 Signal: Emergency stop loss");
                    }
                    _ => {
                        println!("  📊 Signal: {:?}", signal.signal_type);
                    }
                }
            }
        }
    }

    // Test price momentum detection
    println!("\n📈 Testing Price Momentum Detection:");
    println!("─────────────────────────────────────");

    let token_address = "momentum_test_token";

    // Simulate rising prices followed by decline
    let price_sequence = vec![
        0.00001,
        0.00002,
        0.00003,
        0.00004,
        0.00005, // Rising
        0.00004,
        0.00003,
        0.00002,
        0.00001 // Declining (reversal)
    ];

    for (i, price) in price_sequence.iter().enumerate() {
        strategy.update_price_history(token_address, *price);
        println!("  Step {}: Price = {:.5}, Momentum detected: {}", i + 1, price, if i >= 6 {
            // Start checking after we have enough data
            strategy.detect_momentum_reversal(token_address)
        } else {
            false
        });
    }

    // Test dynamic check intervals
    println!("\n⏱️  Dynamic Check Intervals:");
    println!("──────────────────────────────");

    let profit_levels = vec![0.0, 5.0, 15.0, 60.0, 120.0, 300.0];
    for profit in profit_levels {
        let interval = strategy.get_price_check_interval(profit);
        println!("  {:.1}% profit → Check every {}s", profit, interval);
    }

    println!("\n✅ Fast Profit Strategy Test Complete");
    println!("═══════════════════════════════════════");

    Ok(())
}
