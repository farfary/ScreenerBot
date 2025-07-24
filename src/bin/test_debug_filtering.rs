/// Test binary to verify debug filtering functionality
use screenerbot::global::{ is_debug_filtering_enabled, Token, LiquidityInfo };
use screenerbot::filtering::{ filter_token_for_trading, FilterResult };
use screenerbot::logger::{ log, LogTag };
use chrono::{ Utc, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Debug Filtering System");
    println!("=====================================");

    // Test 1: Check if debug filtering is enabled
    let debug_enabled = is_debug_filtering_enabled();
    println!("Debug filtering enabled: {}", debug_enabled);

    if debug_enabled {
        println!("✅ --debug-filtering flag detected! Filtering logs will be visible.");
    } else {
        println!("❌ --debug-filtering flag NOT detected. Filtering logs will be hidden.");
        println!("💡 Run with: cargo run --bin test_debug_filtering -- --debug-filtering");
    }

    println!();

    // Test 2: Create test tokens with various filtering conditions
    println!("🔍 Testing Token Filtering Conditions:");
    println!("======================================");

    // Test token 1: Valid token (should pass all filters)
    let valid_token = Token {
        mint: "ValidTokenMint123".to_string(),
        symbol: "VALID".to_string(),
        name: "Valid Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(24)), // 24 hours old
        price_dexscreener_sol: Some(0.000001),
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
        liquidity: Some(LiquidityInfo {
            usd: Some(50000.0),
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    // Test token 2: Too young (should fail age filter)
    let young_token = Token {
        mint: "YoungTokenMint456".to_string(),
        symbol: "YOUNG".to_string(),
        name: "Young Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(6)), // Only 6 hours old
        price_dexscreener_sol: Some(0.000001),
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
        liquidity: Some(LiquidityInfo {
            usd: Some(50000.0),
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    // Test token 3: No liquidity data (should fail liquidity filter)
    let no_liquidity_token = Token {
        mint: "NoLiquidityMint789".to_string(),
        symbol: "NOLIQ".to_string(),
        name: "No Liquidity Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(24)),
        price_dexscreener_sol: Some(0.000001),
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
        liquidity: None, // No liquidity data
        info: None,
        boosts: None,
    };

    // Test token 4: Empty symbol (should fail info validation)
    let empty_symbol_token = Token {
        mint: "EmptySymbolMint000".to_string(),
        symbol: "".to_string(), // Empty symbol
        name: "Empty Symbol Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(24)),
        price_dexscreener_sol: Some(0.000001),
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
        liquidity: Some(LiquidityInfo {
            usd: Some(50000.0),
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    // Run tests using centralized filtering
    println!("🟢 Testing VALID token:");
    let valid_result = filter_token_for_trading(&valid_token);
    match valid_result {
        FilterResult::Approved => println!("   ✅ Token approved for trading"),
        FilterResult::Rejected(reason) => println!("   ❌ Token rejected: {:?}", reason),
    }

    println!("\n🟡 Testing YOUNG token:");
    let young_result = filter_token_for_trading(&young_token);
    match young_result {
        FilterResult::Approved => println!("   ✅ Token approved for trading"),
        FilterResult::Rejected(reason) => println!("   ❌ Token rejected: {:?}", reason),
    }

    println!("\n🔴 Testing NO LIQUIDITY token:");
    let no_liq_result = filter_token_for_trading(&no_liquidity_token);
    match no_liq_result {
        FilterResult::Approved => println!("   ✅ Token approved for trading"),
        FilterResult::Rejected(reason) => println!("   ❌ Token rejected: {:?}", reason),
    }

    println!("\n🔴 Testing EMPTY SYMBOL token:");
    let empty_result = filter_token_for_trading(&empty_symbol_token);
    match empty_result {
        FilterResult::Approved => println!("   ✅ Token approved for trading"),
        FilterResult::Rejected(reason) => println!("   ❌ Token rejected: {:?}", reason),
    }

    println!("\n✨ Test Complete!");
    println!("================");

    if debug_enabled {
        println!("✅ All filtering debug logs above should be visible with --debug-filtering flag");
    } else {
        println!("❌ No filtering logs should be visible without --debug-filtering flag");
        println!("💡 Try running: cargo run --bin test_debug_filtering -- --debug-filtering");
    }

    Ok(())
}
