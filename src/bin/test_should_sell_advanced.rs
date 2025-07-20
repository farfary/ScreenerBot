use screenerbot::trader::*;
use screenerbot::utils::load_positions_from_file;
use chrono::{ DateTime, Utc, Duration as ChronoDuration };
use tabled::{ Tabled, Table, settings::{ Style, Alignment, object::Rows, Modify } };

#[derive(Tabled)]
struct TestResult {
    #[tabled(rename = "📊 Scenario")]
    scenario: String,

    #[tabled(rename = "💰 P&L %")]
    pnl_percent: String,

    #[tabled(rename = "⏱️ Duration")]
    duration: String,

    #[tabled(rename = "📉 Drawdown")]
    drawdown: String,

    #[tabled(rename = "🔥 Urgency")]
    urgency: String,

    #[tabled(rename = "📍 Decision")]
    decision: String,
}

fn create_test_position(
    entry_price: f64,
    current_price: f64,
    price_highest: f64,
    price_lowest: f64,
    minutes_passed: i64
) -> (Position, f64, DateTime<Utc>) {
    let now = Utc::now();
    let entry_time = now - ChronoDuration::minutes(minutes_passed);

    // Calculate P&L percentage
    let pnl_percent = if entry_price > 0.0 {
        ((current_price - entry_price) / entry_price) * 100.0
    } else {
        0.0
    };

    // Calculate drawdown
    let drawdown = if price_highest > 0.0 {
        ((price_highest - current_price) / price_highest) * 100.0
    } else {
        0.0
    };

    let pos = Position {
        mint: "test_mint".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price,
        entry_time,
        exit_price: None,
        exit_time: None,
        pnl_sol: None,
        pnl_percent: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.1,
        total_size_sol: 0.1,
        drawdown_percent: drawdown,
        price_highest,
        price_lowest,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
        token_amount: Some(1000000),
        effective_entry_price: Some(entry_price),
        effective_exit_price: None,
    };

    (pos, current_price, now)
}

fn run_test_scenario(
    scenario: &str,
    entry_price: f64,
    current_price: f64,
    price_highest: f64,
    price_lowest: f64,
    minutes_passed: i64
) -> TestResult {
    let (pos, current, now) = create_test_position(
        entry_price,
        current_price,
        price_highest,
        price_lowest,
        minutes_passed
    );

    let urgency = should_sell(&pos, current, now);
    let pnl_percent = if pos.entry_price > 0.0 {
        ((current_price - pos.entry_price) / pos.entry_price) * 100.0
    } else {
        0.0
    };
    // Calculate drawdown directly since the should_sell function doesn't modify pos
    let drawdown = if price_highest > 0.0 {
        ((price_highest - current_price) / price_highest) * 100.0
    } else {
        0.0
    };

    let decision = if urgency > 0.7 { "🔴 SELL" } else { "🟢 HOLD" };

    TestResult {
        scenario: scenario.to_string(),
        pnl_percent: format!("{:.1}%", pnl_percent),
        duration: format!("{}m", minutes_passed),
        drawdown: format!("{:.1}%", drawdown),
        urgency: format!("{:.3}", urgency),
        decision: decision.to_string(),
    }
}

fn main() {
    println!("🧪 Testing Advanced Should Sell Function\n");

    // Create various test scenarios
    let mut test_results = Vec::new();

    // PROFIT SCENARIOS
    // Small profit, short time
    test_results.push(
        run_test_scenario(
            "Small profit, short time",
            0.0001, // entry
            0.00011, // current (10% profit)
            0.00011, // highest (no drawdown)
            0.0001, // lowest
            5 // 5 minutes
        )
    );

    // Small profit, medium time
    test_results.push(
        run_test_scenario(
            "Small profit, medium time",
            0.0001, // entry
            0.00011, // current (10% profit)
            0.00011, // highest (no drawdown)
            0.0001, // lowest
            15 // 15 minutes
        )
    );

    // Good profit, short time
    test_results.push(
        run_test_scenario(
            "Good profit, short time",
            0.0001, // entry
            0.00013, // current (30% profit)
            0.00013, // highest
            0.0001, // lowest
            5 // 5 minutes
        )
    );

    // Good profit, medium time
    test_results.push(
        run_test_scenario(
            "Good profit, medium time",
            0.0001, // entry
            0.00013, // current (30% profit)
            0.00013, // highest
            0.0001, // lowest
            15 // 15 minutes
        )
    );

    // Excellent profit, short time with some drawdown
    test_results.push(
        run_test_scenario(
            "Excellent profit, short time",
            0.0001, // entry
            0.0002, // current (100% profit)
            0.00022, // highest (9% drawdown)
            0.0001, // lowest
            3 // 3 minutes
        )
    );

    // Great profit but heavy drawdown
    test_results.push(
        run_test_scenario(
            "Great profit with heavy drawdown",
            0.0001, // entry
            0.00015, // current (50% profit)
            0.0003, // highest (50% drawdown)
            0.0001, // lowest
            8 // 8 minutes
        )
    );

    // LOSS SCENARIOS
    // Small loss, short time
    test_results.push(
        run_test_scenario(
            "Small loss, short time",
            0.0001, // entry
            0.00009, // current (-10% loss)
            0.0001, // highest
            0.00009, // lowest
            5 // 5 minutes
        )
    );

    // Big loss, short time
    test_results.push(
        run_test_scenario(
            "Big loss, short time",
            0.0001, // entry
            0.00006, // current (-40% loss)
            0.0001, // highest
            0.00006, // lowest
            5 // 5 minutes
        )
    );

    // Big loss, medium time
    test_results.push(
        run_test_scenario(
            "Big loss, medium time",
            0.0001, // entry
            0.00006, // current (-40% loss)
            0.0001, // highest
            0.00006, // lowest
            15 // 15 minutes
        )
    );

    // Big loss, long time
    test_results.push(
        run_test_scenario(
            "Big loss, long time",
            0.0001, // entry
            0.00006, // current (-40% loss)
            0.0001, // highest
            0.00006, // lowest
            60 // 60 minutes
        )
    );

    // Catastrophic loss
    test_results.push(
        run_test_scenario(
            "Catastrophic loss",
            0.0001, // entry
            0.00003, // current (-70% loss)
            0.0001, // highest
            0.00003, // lowest
            10 // 10 minutes
        )
    );

    // TIME-BASED SCENARIOS
    // Breakeven, long time
    test_results.push(
        run_test_scenario(
            "Breakeven, long time",
            0.0001, // entry
            0.0001, // current (0% profit)
            0.00011, // highest (9% drawdown)
            0.000095, // lowest
            90 // 90 minutes
        )
    );

    // Small loss, very long time
    test_results.push(
        run_test_scenario(
            "Small loss, very long time",
            0.0001, // entry
            0.000095, // current (-5% loss)
            0.00011, // highest
            0.000095, // lowest
            120 // 2 hours
        )
    );

    // SPECIAL SCENARIOS
    // Recovery from big loss
    test_results.push(
        run_test_scenario(
            "Recovery from big loss",
            0.0001, // entry
            0.000095, // current (-5% loss)
            0.0001, // highest
            0.00006, // lowest (was -40% at worst)
            25 // 25 minutes
        )
    );

    // Falling knife
    test_results.push(
        run_test_scenario(
            "Falling knife scenario",
            0.0001, // entry
            0.00008, // current (-20% loss)
            0.0001, // highest
            0.00008, // lowest
            2 // 2 minutes - very fresh position
        )
    );

    // Display results
    let mut table = Table::new(test_results);
    table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));

    println!("📊 Testing various market scenarios:");
    println!("=====================================");
    println!("{}", table);

    println!("\n✅ Advanced should_sell function tests complete!");
    println!("🎯 Key behaviors:");
    println!("   - Quick profit-taking for significant gains");
    println!("   - Loss tolerance early, less tolerance over time");
    println!("   - Drawdown-aware decision making");
    println!("   - Time-based holding strategy that evolves");
    println!("   - Patience with fresh positions");
}
