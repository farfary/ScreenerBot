use screenerbot::*;
use chrono::{ DateTime, Utc };
use std::collections::HashMap;

fn main() {
    println!("🔍 Debugging Dip Detection Logic");
    println!("============================================================");

    // Create a test token with price history
    let mint = "test_mint_debug";
    let current_time = Utc::now();

    // Build price history that should allow dip detection
    let mut price_history = Vec::new();
    price_history.push((current_time - chrono::Duration::minutes(120), 0.00012)); // $0.00012
    price_history.push((current_time - chrono::Duration::minutes(100), 0.000114)); // -5%
    price_history.push((current_time - chrono::Duration::minutes(80), 0.000108)); // -5%
    price_history.push((current_time - chrono::Duration::minutes(60), 0.000102)); // -5.5%
    price_history.push((current_time - chrono::Duration::minutes(40), 0.000096)); // -6%
    price_history.push((current_time - chrono::Duration::minutes(20), 0.00009)); // -6%
    price_history.push((current_time - chrono::Duration::minutes(10), 0.000085)); // -5.5%
    price_history.push((current_time, 0.00008)); // Current: -6% from last

    println!("📈 Price History:");
    for (i, (time, price)) in price_history.iter().enumerate() {
        if i > 0 {
            let prev_price = price_history[i - 1].1;
            let change = ((price - prev_price) / prev_price) * 100.0;
            println!("   {:2}: {:.8} ({:+.1}%)", i, price, change);
        } else {
            println!("   {:2}: {:.8} (start)", i, price);
        }
    }

    // Calculate price moves manually to understand what the function sees
    let mut price_moves = Vec::new();
    for i in 1..price_history.len() {
        let prev_price = price_history[i - 1].1;
        let curr_price = price_history[i].1;
        if prev_price > 0.0 {
            let change_percent = ((curr_price - prev_price) / prev_price) * 100.0;
            price_moves.push(change_percent);
        }
    }

    println!("\n📊 Price Moves:");
    for (i, move_pct) in price_moves.iter().enumerate() {
        println!("   Move {}: {:+.2}%", i + 1, move_pct);
    }

    // Calculate metrics manually
    let current_price = 0.00008;
    let recent_prices: Vec<f64> = price_history
        .iter()
        .rev()
        .take(10)
        .map(|(_, price)| *price)
        .collect();
    let recent_avg = recent_prices.iter().sum::<f64>() / (recent_prices.len() as f64);

    println!("\n🔍 Analysis:");
    println!("   Current Price: {:.8}", current_price);
    println!("   Recent Average: {:.8}", recent_avg);
    println!("   Price vs Avg: {:.2}% of avg", (current_price / recent_avg) * 100.0);

    // Test support level calculation
    let mut local_minima = Vec::new();
    for i in 1..recent_prices.len() - 1 {
        if recent_prices[i] < recent_prices[i - 1] && recent_prices[i] < recent_prices[i + 1] {
            local_minima.push(recent_prices[i]);
        }
    }

    let support_level = if !local_minima.is_empty() {
        Some(local_minima.iter().sum::<f64>() / (local_minima.len() as f64))
    } else {
        None
    };

    println!("   Support Level: {:?}", support_level);
    if let Some(support) = support_level {
        println!("   Price vs Support: {:.2}% of support", (current_price / support) * 100.0);
    }

    // Test recent moves
    let recent_moves: Vec<f64> = price_moves.iter().rev().take(5).cloned().collect();
    let downward_moves = recent_moves
        .iter()
        .filter(|&m| *m < -0.5)
        .count();

    println!("   Recent 5 moves: {:?}", recent_moves);
    println!("   Downward moves (< -0.5%): {}", downward_moves);

    // Calculate volatility scale
    let mut abs_moves: Vec<f64> = price_moves
        .iter()
        .map(|m| m.abs())
        .collect();
    abs_moves.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let percentile_75_index = ((abs_moves.len() as f64) * 0.75) as usize;
    let volatility_scale = if percentile_75_index < abs_moves.len() {
        abs_moves[percentile_75_index]
    } else {
        abs_moves[abs_moves.len() - 1]
    };
    let volatility_scale = f64::max(volatility_scale, 1.0);

    println!("   Volatility Scale: {:.2}%", volatility_scale);

    // Test drop significance
    if let Some(last_price) = recent_prices.get(1) {
        let current_drop = (((current_price - last_price) / last_price) * 100.0).abs();
        let min_required_drop = volatility_scale * 0.1;
        println!("   Current Drop: {:.2}%", current_drop);
        println!("   Min Required Drop: {:.2}%", min_required_drop);
        println!("   Drop Significant?: {}", current_drop >= min_required_drop);
    }

    // Manual checks
    println!("\n✅ Manual Dip Checks:");

    // Check 1: Below recent average
    let check1 = current_price <= recent_avg * 1.15;
    println!(
        "   1. Below 115% of recent avg: {} ({:.8} <= {:.8})",
        check1,
        current_price,
        recent_avg * 1.15
    );

    // Check 2: Support level
    let check2 = if let Some(support) = support_level {
        let result = current_price <= support * 1.5;
        println!(
            "   2. Below 150% of support: {} ({:.8} <= {:.8})",
            result,
            current_price,
            support * 1.5
        );
        result
    } else {
        println!("   2. No support level found: true (pass)");
        true
    };

    // Check 3: Recent moves
    let check3 = !(recent_moves.len() >= 5 && downward_moves == 0);
    println!(
        "   3. Has downward moves: {} (moves: {}, down: {})",
        check3,
        recent_moves.len(),
        downward_moves
    );

    // Check 4: Drop significance
    let check4 = if let Some(last_price) = recent_prices.get(1) {
        let current_drop = (((current_price - last_price) / last_price) * 100.0).abs();
        let min_drop = volatility_scale * 0.1;
        let result = current_drop >= min_drop;
        println!("   4. Drop significant: {} ({:.2}% >= {:.2}%)", result, current_drop, min_drop);
        result
    } else {
        println!("   4. No previous price: true (pass)");
        true
    };

    let is_genuine_dip = check1 && check2 && check3 && check4;

    println!("\n🎯 Final Result:");
    println!("   Is Genuine Dip: {}", is_genuine_dip);
    if !is_genuine_dip {
        println!("   Blocked by: {}", if !check1 {
            "price too high vs avg"
        } else if !check2 {
            "price too high vs support"
        } else if !check3 {
            "no downward momentum"
        } else if !check4 {
            "drop too small"
        } else {
            "unknown"
        });
    }
}
