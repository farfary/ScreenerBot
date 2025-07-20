use screenerbot::trader::*;
use screenerbot::utils::load_positions_from_file;
use chrono::{ DateTime, Utc };

/// Improved sell urgency calculation that considers current P&L
pub fn calculate_sell_urgency_improved(
    pos: &Position,
    current_price: f64,
    now: DateTime<Utc>
) -> f64 {
    // === PARAMETERS ===
    let min_time_secs = 30.0; // Give positions more time to develop
    let max_time_secs = 1800.0; // 30 minutes instead of 10
    let profit_target = 20.0; // Target 20% profit
    let max_loss_tolerance = -50.0; // Start getting urgent at -50% loss

    // === TIME CALC ===
    let time_secs = (now - pos.entry_time).num_seconds().max(min_time_secs as i64) as f64;

    // === CURRENT P&L CALC (not highest) ===
    let current_pnl_percent = if pos.entry_price > 0.0 {
        ((current_price - pos.entry_price) / pos.entry_price) * 100.0
    } else {
        0.0
    };

    // === URGENCY LOGIC ===
    let mut urgency = 0.0;

    // 1. Time-based urgency (gentle ramp up)
    let norm_time = ((time_secs - min_time_secs) / (max_time_secs - min_time_secs)).clamp(0.0, 1.0);
    let time_urgency = norm_time * 0.3; // Max 30% urgency from time alone

    // 2. Loss-based urgency (aggressive on big losses)
    let loss_urgency = if current_pnl_percent < 0.0 {
        let loss_severity = (current_pnl_percent / max_loss_tolerance).clamp(0.0, 1.0);
        loss_severity * 0.8 // Up to 80% urgency from losses
    } else {
        0.0
    };

    // 3. Profit-taking urgency (take profits when time + profit is right)
    let profit_urgency = if current_pnl_percent > 0.0 {
        let profit_factor = (current_pnl_percent / profit_target).clamp(0.0, 1.0);
        profit_factor * norm_time * 0.5 // Only urgent if both profitable AND time has passed
    } else {
        0.0
    };

    // 4. Drawdown urgency (sell if we're down significantly from peak)
    let drawdown_urgency = if pos.drawdown_percent > 30.0 {
        ((pos.drawdown_percent - 30.0) / 50.0).clamp(0.0, 1.0) * 0.6
    } else {
        0.0
    };

    // Combine all urgency factors
    urgency = (time_urgency + loss_urgency + profit_urgency + drawdown_urgency).clamp(0.0, 1.0);

    urgency
}

fn main() {
    println!("🔧 Testing Improved Sell Urgency Function\n");

    // Load positions from file
    let positions = load_positions_from_file();

    // Test with the closed loss positions
    let loss_positions: Vec<_> = positions
        .iter()
        .filter(|p| { p.exit_time.is_some() && p.pnl_percent.unwrap_or(0.0) < 0.0 })
        .collect();

    println!("📊 Testing improved urgency on {} loss positions:\n", loss_positions.len());

    for (i, pos) in loss_positions.iter().enumerate() {
        let exit_time = pos.exit_time.unwrap();
        let exit_price = pos.exit_price.unwrap();

        // Calculate urgency with original function (simulate old logic)
        let old_urgency = {
            let min_time_secs = 10.0;
            let max_time_secs = 600.0;
            let min_profit = 0.0;
            let max_profit = 500.0;

            let time_secs = (exit_time - pos.entry_time)
                .num_seconds()
                .max(min_time_secs as i64) as f64;
            let profit = if pos.entry_price > 0.0 {
                ((pos.price_highest - pos.entry_price) / pos.entry_price) * 100.0
            } else {
                0.0
            };

            let norm_time = ((time_secs - min_time_secs) / (max_time_secs - min_time_secs)).clamp(
                0.0,
                1.0
            );
            let norm_profit = ((profit - min_profit) / (max_profit - min_profit)).clamp(0.0, 1.0);

            let urgency = norm_profit * (1.0 - norm_time) + norm_time * (1.0 - norm_profit);
            urgency.clamp(0.0, 1.0)
        };

        // Calculate urgency with improved function
        let new_urgency = calculate_sell_urgency_improved(pos, exit_price, exit_time);

        // Calculate current P&L at exit time
        let exit_pnl_percent = if pos.entry_price > 0.0 {
            ((exit_price - pos.entry_price) / pos.entry_price) * 100.0
        } else {
            0.0
        };

        println!("{}. {} ({}):", i + 1, pos.symbol, pos.mint);
        println!("   📈 Entry: {:.8} → 📉 Exit: {:.8} SOL", pos.entry_price, exit_price);
        println!("   💰 Exit P&L: {:.2}%", exit_pnl_percent);
        println!("   📉 Drawdown: {:.2}%", pos.drawdown_percent);
        println!("   🕐 Duration: {}s", (exit_time - pos.entry_time).num_seconds());
        println!("   🚨 Old Urgency: {:.3} (sold because > 0.7)", old_urgency);
        println!("   ✨ New Urgency: {:.3}", new_urgency);

        if new_urgency <= 0.7 {
            println!("   ✅ WOULD NOT SELL with improved function!");
        } else {
            println!("   ❌ Would still sell with improved function");
        }
        println!();
    }

    // Test various scenarios
    println!("\n🧪 Testing Urgency in Different Scenarios:");
    println!("==========================================");

    // Create test position
    let test_pos = Position {
        mint: "test".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price: 0.00001,
        entry_time: Utc::now() - chrono::Duration::minutes(10),
        exit_price: None,
        exit_time: None,
        pnl_sol: None,
        pnl_percent: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.0005,
        total_size_sol: 0.0005,
        drawdown_percent: 0.0,
        price_highest: 0.00001,
        price_lowest: 0.00001,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
        token_amount: None,
        effective_entry_price: None,
        effective_exit_price: None,
    };

    let scenarios = vec![
        ("Small profit after 5 min", 0.000011, 5), // +10%
        ("Small profit after 15 min", 0.000011, 15), // +10%
        ("Good profit after 15 min", 0.000013, 15), // +30%
        ("Small loss after 5 min", 0.000009, 5), // -10%
        ("Big loss after 5 min", 0.000006, 5), // -40%
        ("Big loss after 15 min", 0.000006, 15) // -40%
    ];

    for (scenario, price, minutes) in scenarios {
        let test_time = test_pos.entry_time + chrono::Duration::minutes(minutes);
        // Simulate old urgency function since it changed
        let old_urgency = {
            let min_time_secs = 10.0;
            let max_time_secs = 600.0;
            let min_profit = 0.0;
            let max_profit = 500.0;

            let time_secs = (test_time - test_pos.entry_time)
                .num_seconds()
                .max(min_time_secs as i64) as f64;
            let profit = if test_pos.entry_price > 0.0 {
                ((test_pos.price_highest - test_pos.entry_price) / test_pos.entry_price) * 100.0
            } else {
                0.0
            };

            let norm_time = ((time_secs - min_time_secs) / (max_time_secs - min_time_secs)).clamp(
                0.0,
                1.0
            );
            let norm_profit = ((profit - min_profit) / (max_profit - min_profit)).clamp(0.0, 1.0);

            let urgency = norm_profit * (1.0 - norm_time) + norm_time * (1.0 - norm_profit);
            urgency.clamp(0.0, 1.0)
        };
        let new_urgency = calculate_sell_urgency_improved(&test_pos, price, test_time);
        let pnl = ((price - test_pos.entry_price) / test_pos.entry_price) * 100.0;

        println!("{}: P&L {:.1}%", scenario, pnl);
        println!(
            "  Old: {:.3} | New: {:.3} | Would sell: Old={}, New={}",
            old_urgency,
            new_urgency,
            if old_urgency > 0.7 {
                "YES"
            } else {
                "NO"
            },
            if new_urgency > 0.7 {
                "YES"
            } else {
                "NO"
            }
        );
    }

    println!("\n💡 Recommendation:");
    println!("==================");
    println!("Replace the current calculate_sell_urgency with the improved version.");
    println!("This should reduce unnecessary sells and improve profitability.");
}
