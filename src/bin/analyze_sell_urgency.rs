use screenerbot::trader::*;
use screenerbot::utils::load_positions_from_file;

fn main() {
    println!("🔍 Analyzing Sell Urgency for Closed Positions\n");

    // Load positions from file
    let positions = load_positions_from_file();

    // Filter closed positions that were sold at a loss
    let loss_positions: Vec<_> = positions
        .iter()
        .filter(|p| { p.exit_time.is_some() && p.pnl_percent.unwrap_or(0.0) < 0.0 })
        .collect();

    println!("📊 Found {} closed positions with losses:\n", loss_positions.len());

    for (i, pos) in loss_positions.iter().enumerate() {
        let entry_time = pos.entry_time;
        let exit_time = pos.exit_time.unwrap();
        let duration_secs = (exit_time - entry_time).num_seconds();
        let pnl_percent = pos.pnl_percent.unwrap_or(0.0);

        // Calculate what the sell urgency was at exit time (using old logic for comparison)
        // Note: The old function signature was different, so we'll simulate it
        let sell_urgency = {
            // Simulate old logic
            let min_time_secs = 10.0;
            let max_time_secs = 600.0;
            let min_profit = 0.0;
            let max_profit = 500.0;

            let time_secs = (exit_time - entry_time).num_seconds().max(min_time_secs as i64) as f64;
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

        // Check if it was an emergency exit (stop loss)
        let was_emergency_exit = pnl_percent <= STOP_LOSS_PERCENT;

        // Calculate profit at highest point
        let highest_profit = if pos.entry_price > 0.0 {
            ((pos.price_highest - pos.entry_price) / pos.entry_price) * 100.0
        } else {
            0.0
        };

        println!("{}. {} ({}):", i + 1, pos.symbol, pos.mint);
        println!("   📈 Entry Price: {:.8} SOL", pos.entry_price);
        println!("   📉 Exit Price:  {:.8} SOL", pos.exit_price.unwrap_or(0.0));
        println!("   🔝 Highest:     {:.8} SOL", pos.price_highest);
        println!(
            "   📊 Duration:    {}s ({:.1} minutes)",
            duration_secs,
            (duration_secs as f64) / 60.0
        );
        println!("   💰 Final P&L:   {:.2}%", pnl_percent);
        println!("   🎯 Max Profit:  {:.2}%", highest_profit);
        println!("   🚨 Sell Urgency: {:.3}", sell_urgency);
        println!("   ⚠️  Emergency Exit: {}", was_emergency_exit);
        println!("   📉 Drawdown: {:.2}%", pos.drawdown_percent);

        // Analyze why it sold
        if was_emergency_exit {
            println!("   ❌ REASON: Emergency exit (stop loss at -70%)");
        } else if sell_urgency > 0.7 {
            println!("   ❌ REASON: High sell urgency (> 0.7)");

            // Break down the urgency calculation
            let min_time_secs = 10.0;
            let max_time_secs = 600.0; // 10 minutes
            let min_profit = 0.0;
            let max_profit = 500.0;

            let time_secs = duration_secs.max(min_time_secs as i64) as f64;
            let profit = highest_profit; // Use highest profit seen

            let norm_time = ((time_secs - min_time_secs) / (max_time_secs - min_time_secs)).clamp(
                0.0,
                1.0
            );
            let norm_profit = ((profit - min_profit) / (max_profit - min_profit)).clamp(0.0, 1.0);

            println!("     📏 Normalized Time: {:.3} ({}s / 600s max)", norm_time, time_secs);
            println!("     💹 Normalized Profit: {:.3} ({:.2}% / 500% max)", norm_profit, profit);
            println!("     🧮 Urgency = profit*(1-time) + time*(1-profit)");
            println!(
                "     🧮 Urgency = {:.3}*{:.3} + {:.3}*{:.3} = {:.3}",
                norm_profit,
                1.0 - norm_time,
                norm_time,
                1.0 - norm_profit,
                sell_urgency
            );
        } else {
            println!("   ❓ REASON: Unknown (urgency was only {:.3})", sell_urgency);
        }

        println!();
    }

    // Summary analysis
    println!("🎯 Summary Analysis:");
    println!("====================");

    let emergency_exits = loss_positions
        .iter()
        .filter(|p| p.pnl_percent.unwrap_or(0.0) <= STOP_LOSS_PERCENT)
        .count();

    let urgency_exits = loss_positions
        .iter()
        .filter(|p| {
            // Simulate old urgency calculation since function signature changed
            let exit_time = p.exit_time.unwrap();
            let min_time_secs = 10.0;
            let max_time_secs = 600.0;
            let min_profit = 0.0;
            let max_profit = 500.0;

            let time_secs = (exit_time - p.entry_time)
                .num_seconds()
                .max(min_time_secs as i64) as f64;
            let profit = if p.entry_price > 0.0 {
                ((p.price_highest - p.entry_price) / p.entry_price) * 100.0
            } else {
                0.0
            };

            let norm_time = ((time_secs - min_time_secs) / (max_time_secs - min_time_secs)).clamp(
                0.0,
                1.0
            );
            let norm_profit = ((profit - min_profit) / (max_profit - min_profit)).clamp(0.0, 1.0);

            let urgency = norm_profit * (1.0 - norm_time) + norm_time * (1.0 - norm_profit);
            let urgency = urgency.clamp(0.0, 1.0);

            p.pnl_percent.unwrap_or(0.0) > STOP_LOSS_PERCENT && urgency > 0.7
        })
        .count();

    println!("📊 Emergency exits (stop loss): {}", emergency_exits);
    println!("📊 Urgency-based exits: {}", urgency_exits);
    println!("📊 Other exits: {}", loss_positions.len() - emergency_exits - urgency_exits);

    // Analyze the urgency function behavior
    println!("\n🔬 Urgency Function Analysis:");
    println!("=============================");
    println!("Current parameters:");
    println!("- Min time: 10 seconds");
    println!("- Max time: 600 seconds (10 minutes)");
    println!("- Min profit: 0%");
    println!("- Max profit: 500%");
    println!("- Sell threshold: urgency > 0.7");
    println!("\nProblem: The urgency function causes early sells even when in profit!");
    println!("- After 10 minutes, time factor becomes 1.0");
    println!("- If profit is low (< 350%), urgency quickly exceeds 0.7");
    println!("- This forces sells even when position could recover");
}
