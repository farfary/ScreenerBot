use screenerbot::global::Token;
use screenerbot::trader::{ validate_token_age, MIN_TOKEN_AGE_HOURS, MAX_TOKEN_AGE_HOURS };
use chrono::{ Utc, Duration };

fn main() {
    println!("Testing token age validation...");
    println!("Min age requirement: {} hours", MIN_TOKEN_AGE_HOURS);
    println!("Max age limit: {} hours", MAX_TOKEN_AGE_HOURS);
    println!();

    // Test token that's too young (12 hours old)
    let young_token = Token {
        mint: "young_token_mint".to_string(),
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
        created_at: Some(Utc::now() - Duration::hours(12)), // 12 hours ago
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
        liquidity: None,
        info: None,
        boosts: None,
    };

    // Test token that's in the acceptable age range (48 hours old)
    let good_token = Token {
        mint: "good_token_mint".to_string(),
        symbol: "GOOD".to_string(),
        name: "Good Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(48)), // 48 hours ago
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
        liquidity: None,
        info: None,
        boosts: None,
    };

    // Test token that's too old (4 days = 96 hours old)
    let old_token = Token {
        mint: "old_token_mint".to_string(),
        symbol: "OLD".to_string(),
        name: "Old Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(96)), // 96 hours ago (4 days)
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
        liquidity: None,
        info: None,
        boosts: None,
    };

    // Test token without creation timestamp
    let no_timestamp_token = Token {
        mint: "no_timestamp_mint".to_string(),
        symbol: "NOTIME".to_string(),
        name: "No Timestamp Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: None, // No creation timestamp
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
        liquidity: None,
        info: None,
        boosts: None,
    };

    println!("Testing young token (12 hours old) - should be REJECTED:");
    let result1 = validate_token_age(&young_token);
    println!("Result: {}\n", result1);

    println!("Testing good token (48 hours old) - should be ACCEPTED:");
    let result2 = validate_token_age(&good_token);
    println!("Result: {}\n", result2);

    println!("Testing old token (96 hours old) - should be REJECTED:");
    let result3 = validate_token_age(&old_token);
    println!("Result: {}\n", result3);

    println!("Testing token without timestamp - should be ACCEPTED (with warning):");
    let result4 = validate_token_age(&no_timestamp_token);
    println!("Result: {}\n", result4);

    println!("Summary:");
    println!("Young token (12h): {} (expected: false)", result1);
    println!("Good token (48h): {} (expected: true)", result2);
    println!("Old token (96h): {} (expected: false)", result3);
    println!("No timestamp: {} (expected: true)", result4);
}
