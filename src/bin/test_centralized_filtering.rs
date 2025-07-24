/// Test binary to verify the centralized filtering system
use screenerbot::global::{ Token, LiquidityInfo };
use screenerbot::trader::MAX_OPEN_POSITIONS;
use screenerbot::filtering::{
    filter_token_for_trading,
    FilterResult,
    FilterReason,
    filter_eligible_tokens,
    get_filtering_stats,
};
use screenerbot::positions::{ Position, SAVED_POSITIONS };
use screenerbot::logger::{ log, LogTag };
use chrono::{ Utc, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Centralized Token Filtering System");
    println!("==============================================");

    // Test 1: Basic validation filters
    println!("\n📋 Test 1: Basic Validation Filters");
    println!("------------------------------------");

    test_basic_validation().await;

    // Test 2: Age-based filters
    println!("\n⏰ Test 2: Age-Based Filters");
    println!("-----------------------------");

    test_age_filters().await;

    // Test 3: Position constraint filters
    println!("\n🎯 Test 3: Position Constraint Filters");
    println!("--------------------------------------");

    test_position_filters().await;

    // Test 4: Bulk filtering operations
    println!("\n📊 Test 4: Bulk Filtering Operations");
    println!("------------------------------------");

    test_bulk_filtering().await;

    println!("\n✨ All tests completed!");
    Ok(())
}

async fn test_basic_validation() {
    // Valid token
    let valid_token = create_test_token(
        "ValidToken",
        "VALID",
        Some(Utc::now() - Duration::hours(24)),
        Some(0.000001),
        Some(50000.0)
    );

    // Empty symbol
    let empty_symbol = create_test_token(
        "",
        "EMPTY",
        Some(Utc::now() - Duration::hours(24)),
        Some(0.000001),
        Some(50000.0)
    );

    // No price
    let no_price = create_test_token(
        "NoPrice",
        "NOPRICE",
        Some(Utc::now() - Duration::hours(24)),
        None,
        Some(50000.0)
    );

    // Zero liquidity
    let zero_liquidity = create_test_token(
        "ZeroLiq",
        "ZEROLIQ",
        Some(Utc::now() - Duration::hours(24)),
        Some(0.000001),
        Some(0.0)
    );

    test_single_token("Valid Token", &valid_token);
    test_single_token("Empty Symbol", &empty_symbol);
    test_single_token("No Price", &no_price);
    test_single_token("Zero Liquidity", &zero_liquidity);
}

async fn test_age_filters() {
    // Too young (6 hours old)
    let young_token = create_test_token(
        "YoungToken",
        "YOUNG",
        Some(Utc::now() - Duration::hours(6)),
        Some(0.000001),
        Some(50000.0)
    );

    // Just old enough (12 hours old)
    let just_old_enough = create_test_token(
        "JustRight",
        "JUSTRIGHT",
        Some(Utc::now() - Duration::hours(12)),
        Some(0.000001),
        Some(50000.0)
    );

    // No creation date
    let no_date = create_test_token("NoDate", "NODATE", None, Some(0.000001), Some(50000.0));

    test_single_token("Too Young (6h)", &young_token);
    test_single_token("Just Right (12h)", &just_old_enough);
    test_single_token("No Creation Date", &no_date);
}

async fn test_position_filters() {
    // Create a test token
    let test_token = create_test_token(
        "TestToken",
        "TEST",
        Some(Utc::now() - Duration::hours(24)),
        Some(0.000001),
        Some(50000.0)
    );

    println!("Testing position constraints...");

    // Test with no existing positions
    test_single_token("No Existing Positions", &test_token);

    // Add a mock open position
    {
        let mut positions = SAVED_POSITIONS.lock().unwrap();
        positions.push(Position {
            mint: test_token.mint.clone(),
            symbol: test_token.symbol.clone(),
            name: test_token.name.clone(),
            entry_price: 0.000001,
            entry_time: Utc::now() - Duration::minutes(30),
            exit_price: None,
            exit_time: None,
            position_type: "buy".to_string(),
            entry_size_sol: 0.1,
            total_size_sol: 0.1,
            price_highest: 0.000001,
            price_lowest: 0.000001,
            entry_transaction_signature: Some("test_sig".to_string()),
            exit_transaction_signature: None,
            token_amount: Some(100000),
            effective_entry_price: Some(0.000001),
            effective_exit_price: None,
            sol_received: None,
        });
    }

    test_single_token("With Open Position", &test_token);

    // Test recently closed position (simulate a closed position)
    {
        let mut positions = SAVED_POSITIONS.lock().unwrap();
        // Close the position
        if let Some(position) = positions.last_mut() {
            position.exit_price = Some(0.000002);
            position.exit_time = Some(Utc::now() - Duration::minutes(10)); // Closed 10 minutes ago
        }
    }

    test_single_token("Recently Closed Position", &test_token);

    // Clean up test positions
    {
        let mut positions = SAVED_POSITIONS.lock().unwrap();
        positions.clear();
    }
}

async fn test_bulk_filtering() {
    let tokens = vec![
        create_test_token(
            "Good1",
            "GOOD1",
            Some(Utc::now() - Duration::hours(24)),
            Some(0.000001),
            Some(50000.0)
        ),
        create_test_token(
            "Good2",
            "GOOD2",
            Some(Utc::now() - Duration::hours(18)),
            Some(0.000002),
            Some(75000.0)
        ),
        create_test_token(
            "",
            "BAD1",
            Some(Utc::now() - Duration::hours(24)),
            Some(0.000001),
            Some(50000.0)
        ), // Empty symbol
        create_test_token(
            "Young",
            "YOUNG",
            Some(Utc::now() - Duration::hours(6)),
            Some(0.000001),
            Some(50000.0)
        ), // Too young
        create_test_token(
            "NoLiq",
            "NOLIQ",
            Some(Utc::now() - Duration::hours(24)),
            Some(0.000001),
            Some(0.0)
        ) // No liquidity
    ];

    println!("Testing bulk filtering with {} tokens:", tokens.len());

    let eligible = filter_eligible_tokens(&tokens);
    let (total, passed, pass_rate) = get_filtering_stats(&tokens);

    println!("Results:");
    println!("  Total tokens: {}", total);
    println!("  Eligible tokens: {}", passed);
    println!("  Pass rate: {:.1}%", pass_rate);
    println!(
        "  Eligible token symbols: {:?}",
        eligible
            .iter()
            .map(|t| &t.symbol)
            .collect::<Vec<_>>()
    );
}

fn create_test_token(
    symbol: &str,
    mint: &str,
    created_at: Option<chrono::DateTime<Utc>>,
    price: Option<f64>,
    liquidity_usd: Option<f64>
) -> Token {
    Token {
        mint: mint.to_string(),
        symbol: symbol.to_string(),
        name: format!("{} Token", symbol),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at,
        price_dexscreener_sol: price,
        price_dexscreener_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: liquidity_usd.map(|usd| LiquidityInfo {
            usd: Some(usd),
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    }
}

fn test_single_token(description: &str, token: &Token) {
    print!("  {} ... ", description);

    match filter_token_for_trading(token) {
        FilterResult::Approved => {
            println!("✅ APPROVED");
        }
        FilterResult::Rejected(reason) => {
            let reason_str = match reason {
                FilterReason::EmptySymbol => "Empty symbol",
                FilterReason::EmptyMint => "Empty mint",
                FilterReason::InvalidPrice => "Invalid price",
                FilterReason::ZeroLiquidity => "Zero liquidity",
                FilterReason::MissingLiquidityData => "Missing liquidity data",
                FilterReason::MissingPriceData => "Missing price data",
                FilterReason::TooYoung { age_hours, min_required } =>
                    &format!("Too young: {}h < {}h", age_hours, min_required),
                FilterReason::TooOld { age_hours, max_allowed } =>
                    &format!("Too old: {}h > {}h", age_hours, max_allowed),
                FilterReason::NoCreationDate => "No creation date",
                FilterReason::ExistingOpenPosition => "Existing open position",
                FilterReason::RecentlyClosed { minutes_ago, cooldown_minutes } =>
                    &format!(
                        "Recently closed: {}min ago (cooldown: {}min)",
                        minutes_ago,
                        cooldown_minutes
                    ),
                FilterReason::MaxPositionsReached { current, max } =>
                    &format!("Max positions: {}/{}", current, max),
                FilterReason::PoorHistoricalPerformance { .. } => "Poor historical performance",
                FilterReason::LockAcquisitionFailed => "Lock acquisition failed",
            };
            println!("❌ REJECTED: {}", reason_str);
        }
    }
}
