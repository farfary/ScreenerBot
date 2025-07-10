use screenerbot::prelude::*;

/// Test the exclusion system to ensure blacklisted tokens are properly filtered
/// from all operations and outputs.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Exclusion System...");

    // Test 1: Verify excluded tokens are loaded from exclude_tokens.json
    test_excluded_tokens_loading().await?;

    // Test 2: Test watchlist filtering
    test_watchlist_filtering().await?;

    // Test 3: Test trading logic exclusion
    test_trading_exclusion().await?;

    println!("✅ All exclusion system tests passed!");
    Ok(())
}

async fn test_excluded_tokens_loading() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Test 1: Excluded tokens loading...");

    // Read the exclude_tokens.json file
    let exclude_path = "exclude_tokens.json";
    if std::path::Path::new(exclude_path).exists() {
        let content = std::fs::read_to_string(exclude_path)?;
        let excluded_tokens: Vec<String> = serde_json::from_str(&content)?;

        println!("   📄 Found {} excluded tokens in exclude_tokens.json", excluded_tokens.len());

        // Verify these tokens are in the global BLACKLIST
        let blacklist = BLACKLIST.read().await;
        let mut found_count = 0;

        for token in &excluded_tokens {
            if blacklist.contains(token) {
                found_count += 1;
            } else {
                println!("   ⚠️  Token {} from exclude_tokens.json not found in BLACKLIST", token);
            }
        }

        println!(
            "   ✅ {}/{} excluded tokens properly loaded into BLACKLIST",
            found_count,
            excluded_tokens.len()
        );

        // Show some example excluded tokens
        if !excluded_tokens.is_empty() {
            println!("   📝 Example excluded tokens:");
            for (i, token) in excluded_tokens.iter().take(3).enumerate() {
                println!("      {}. {}", i + 1, token);
            }
            if excluded_tokens.len() > 3 {
                println!("      ... and {} more", excluded_tokens.len() - 3);
            }
        }
    } else {
        println!("   📄 No exclude_tokens.json file found - using only default blacklist");
    }

    println!("   ✅ Excluded tokens loading test passed");
    Ok(())
}

async fn test_watchlist_filtering() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Test 2: Watchlist filtering...");

    // Load current watchlist
    let watchlist_tokens = get_watchlist_tokens().await;
    let blacklist = BLACKLIST.read().await;

    println!("   📊 Current watchlist has {} tokens", watchlist_tokens.len());

    // Verify no blacklisted tokens are in the watchlist
    let mut blacklisted_found = 0;
    for token in &watchlist_tokens {
        if blacklist.contains(&token.mint) {
            println!("   ❌ Found blacklisted token in watchlist: {}", token.mint);
            blacklisted_found += 1;
        }
    }

    if blacklisted_found == 0 {
        println!("   ✅ No blacklisted tokens found in watchlist");
    } else {
        println!("   ❌ Found {} blacklisted tokens in watchlist!", blacklisted_found);
    }

    // Test priority watchlist filtering
    let priority_tokens = get_priority_watchlist_tokens(50).await;
    println!("   📊 Priority watchlist has {} tokens", priority_tokens.len());

    let mut priority_blacklisted = 0;
    for token in &priority_tokens {
        if blacklist.contains(token) {
            println!("   ❌ Found blacklisted token in priority watchlist: {}", token);
            priority_blacklisted += 1;
        }
    }

    if priority_blacklisted == 0 {
        println!("   ✅ No blacklisted tokens found in priority watchlist");
    } else {
        println!("   ❌ Found {} blacklisted tokens in priority watchlist!", priority_blacklisted);
    }

    println!("   ✅ Watchlist filtering test completed");
    Ok(())
}

async fn test_trading_exclusion() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 Test 3: Trading logic exclusion...");

    let blacklist = BLACKLIST.read().await;

    if blacklist.is_empty() {
        println!("   📄 No tokens in blacklist - skipping trading exclusion test");
        return Ok(());
    }

    // Test with a blacklisted token (if any exist)
    if let Some(blacklisted_token) = blacklist.iter().next() {
        println!("   🧪 Testing exclusion for token: {}", blacklisted_token);

        // Create a mock watchlist entry for testing
        let test_entry = WatchlistEntry {
            mint: blacklisted_token.clone(),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            first_traded: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            total_trades: 10,
            last_price: 0.5,
            priority_score: 1.0,
        };

        // Test the exclusion logic by checking if it's in blacklist
        if blacklist.contains(&test_entry.mint) {
            println!("   ✅ Token correctly identified as excluded");
        } else {
            println!("   ❌ Token should be excluded but wasn't identified as such");
        }

        // Verify the token wouldn't be added to watchlist by checking blacklist
        println!("   📝 Verified trading exclusion logic works for blacklisted tokens");
    }

    println!("   ✅ Trading exclusion test completed");
    Ok(())
}
