use std::time::{ Duration, Instant };
use std::collections::HashMap;
use screenerbot::tokens::lp_lock::{ check_lp_lock_status, LpLockStatus, LpLockAnalysis };
use screenerbot::tokens::dexscreener::init_dexscreener_api;
use screenerbot::logger::{ log, LogTag };

#[derive(Debug, Clone)]
struct TestToken {
    mint: String,
    symbol: String,
    name: String,
    dex_id: String,
    liquidity_usd: Option<f64>,
    market_cap: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Initialize logger manually since we just need basic logging
    println!("📝 Logger initialized");

    // Initialize DexScreener API first (CRITICAL!)
    println!("🔧 Initializing DexScreener API...");
    match init_dexscreener_api().await {
        Ok(()) => println!("✅ DexScreener API initialized successfully"),
        Err(e) => {
            println!("❌ Failed to initialize DexScreener API: {}", e);
            return Err(format!("DexScreener API initialization failed: {}", e).into());
        }
    }

    println!("🔍 COMPREHENSIVE LP LOCK DEBUG TEST WITH FULL LOGGING");
    println!("{}", "=".repeat(80));

    // Check if specific tokens are provided via command line (skip any flags starting with --)
    let tokens = if args.len() > 1 {
        let provided: Vec<String> = args.iter().skip(1).cloned().collect();
        let mut mints: Vec<String> = Vec::new();
        let mut flags: Vec<String> = Vec::new();
        for a in provided {
            if a.starts_with("--") {
                flags.push(a);
            } else {
                mints.push(a);
            }
        }
        println!("🎯 Testing {} specific token addresses from command line...", mints.len());
        if !flags.is_empty() {
            println!("⚙️  Detected flags: {}", flags.join(", "));
        }

        // Attempt to enrich each mint using DexScreener cached pools (single call per mint)
        let mut specific_tokens = Vec::new();
        for (i, mint_address) in mints.iter().enumerate() {
            let (symbol, name, dex_id, liq, mc) = match
                screenerbot::tokens::dexscreener::get_token_pools_from_dexscreener(
                    mint_address
                ).await
            {
                Ok(pools) if !pools.is_empty() => {
                    // Prefer pool whose baseToken matches the mint
                    let primary = pools
                        .iter()
                        .find(|p| p.base_token.address == *mint_address)
                        .unwrap_or(&pools[0]);
                    let sym = primary.base_token.symbol.clone();
                    let nm = primary.base_token.name.clone();
                    let dex = primary.dex_id.clone();
                    let liq_usd = primary.liquidity.as_ref().map(|l| l.usd);
                    let mc_v = primary.market_cap;
                    (sym, nm, dex, liq_usd, mc_v)
                }
                _ =>
                    (
                        format!("TOKEN{}", i + 1),
                        format!("Token {}", i + 1),
                        "unknown".to_string(),
                        None,
                        None,
                    ),
            };
            specific_tokens.push(TestToken {
                mint: mint_address.clone(),
                symbol,
                name,
                dex_id,
                liquidity_usd: liq,
                market_cap: mc,
            });
        }
        specific_tokens
    } else {
        // Fall back to database sample
        println!("🎯 Loading comprehensive token sample from database...");
        load_comprehensive_token_sample().await?
    };

    println!("📊 Testing {} tokens", tokens.len());

    // Analyze token distribution only if loaded from database
    if args.len() <= 1 {
        analyze_token_distribution(&tokens);
    }

    println!("\n🧪 STARTING COMPREHENSIVE LP LOCK VERIFICATION TEST");
    println!("{}", "=".repeat(80));

    let mut results: Vec<(TestToken, LpLockAnalysis, Duration)> = Vec::new();
    let start_time = Instant::now();

    for (i, token) in tokens.iter().enumerate() {
        println!("\n🔍 [{}/{}] TESTING: {} ({})", i + 1, tokens.len(), token.symbol, token.dex_id);
        println!("   Mint: {}", token.mint);
        println!(
            "   Liquidity: ${}",
            token.liquidity_usd.map_or("N/A".to_string(), |l| format!("{:.0}", l))
        );
        println!(
            "   Market Cap: ${}",
            token.market_cap.map_or("N/A".to_string(), |m| format!("{:.0}", m))
        );

        // Add detailed logging before calling the function
        log(
            LogTag::Security,
            "DEBUG_TEST",
            &format!(
                "Starting LP lock analysis for {} (FULL_MINT: {}) - DEX: {}",
                token.symbol,
                token.mint,
                token.dex_id
            )
        );

        let test_start = Instant::now();

        // Call LP lock verification with full logging enabled
        let analysis = match check_lp_lock_status(&token.mint).await {
            Ok(analysis) => {
                log(
                    LogTag::Security,
                    "DEBUG_TEST",
                    &format!(
                        "LP lock analysis completed for {} - Status: {:?}, Score: {}",
                        token.symbol,
                        analysis.status,
                        analysis.lock_score
                    )
                );
                analysis
            }
            Err(e) => {
                log(
                    LogTag::Security,
                    "DEBUG_ERROR",
                    &format!("LP lock analysis failed for {}: {}", token.symbol, e)
                );
                println!("   ❌ Error: {}", e);
                continue;
            }
        };

        let duration = test_start.elapsed();

        // Print detailed result
        print_detailed_result(&token, &analysis.status, analysis.lock_score as u32, duration);

        results.push((token.clone(), analysis, duration));

        // Add small delay to avoid overwhelming logs
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let total_duration = start_time.elapsed();

    // Generate comprehensive analysis
    println!("\n📊 COMPREHENSIVE ANALYSIS REPORT");
    println!("{}", "=".repeat(80));

    generate_comprehensive_analysis(&results, total_duration);

    Ok(())
}

async fn load_comprehensive_token_sample() -> Result<Vec<TestToken>, Box<dyn std::error::Error>> {
    use rusqlite::Connection;

    let conn = Connection::open("data/tokens.db")?;
    let mut tokens = Vec::new();

    // Load high liquidity tokens from each DEX type
    let queries = vec![
        // High liquidity Raydium tokens (reduced to 3)
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'raydium' AND liquidity_usd > 100000 ORDER BY liquidity_usd DESC LIMIT 3",
            "High Liquidity Raydium",
        ),

        // PumpFun tokens (should be safer) (reduced to 5)
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'pumpfun' ORDER BY liquidity_usd DESC LIMIT 5",
            "PumpFun Tokens",
        ),

        // PumpSwap tokens (graduated from PumpFun) (reduced to 5)
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'pumpswap' ORDER BY liquidity_usd DESC LIMIT 5",
            "PumpSwap Tokens",
        ),

        // Orca tokens (reduced to 3)
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'orca' ORDER BY liquidity_usd DESC LIMIT 3",
            "Orca Tokens",
        ),

        // Meteora tokens (reduced to 3)
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'meteora' ORDER BY liquidity_usd DESC LIMIT 3",
            "Meteora Tokens",
        )
    ];

    for (query, category) in queries {
        println!("🔍 Loading {}...", category);

        let mut stmt = conn.prepare(query)?;
        let token_iter = stmt.query_map([], |row| {
            Ok(TestToken {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                dex_id: row.get(3)?,
                liquidity_usd: row.get(4).ok(),
                market_cap: row.get(5).ok(),
            })
        })?;

        let mut category_tokens = Vec::new();
        for token in token_iter {
            match token {
                Ok(t) => category_tokens.push(t),
                Err(e) => println!("   ⚠️  Error loading token: {}", e),
            }
        }

        println!("   ✅ Loaded {} tokens", category_tokens.len());
        tokens.extend(category_tokens);
    }

    // Remove duplicates by mint
    tokens.sort_by(|a, b| a.mint.cmp(&b.mint));
    tokens.dedup_by(|a, b| a.mint == b.mint);

    Ok(tokens)
}

fn analyze_token_distribution(tokens: &[TestToken]) {
    println!("\n📊 TOKEN DISTRIBUTION ANALYSIS:");

    // By DEX
    let mut dex_counts: HashMap<String, usize> = HashMap::new();
    let mut liquidity_ranges: HashMap<String, usize> = HashMap::new();

    for token in tokens {
        *dex_counts.entry(token.dex_id.clone()).or_insert(0) += 1;

        // Liquidity ranges
        let range = match token.liquidity_usd {
            Some(l) if l > 1_000_000.0 => "Very High (>$1M)",
            Some(l) if l > 100_000.0 => "High ($100K-$1M)",
            Some(l) if l > 10_000.0 => "Medium ($10K-$100K)",
            Some(l) if l > 1_000.0 => "Low ($1K-$10K)",
            Some(_) => "Very Low (<$1K)",
            None => "Unknown",
        };
        *liquidity_ranges.entry(range.to_string()).or_insert(0) += 1;
    }

    println!("🏪 By DEX:");
    for (dex, count) in dex_counts {
        println!("   • {}: {} tokens", dex, count);
    }

    println!("💰 By Liquidity:");
    for (range, count) in liquidity_ranges {
        println!("   • {}: {} tokens", range, count);
    }
}

fn print_detailed_result(token: &TestToken, status: &LpLockStatus, score: u32, duration: Duration) {
    let status_emoji = match status {
        LpLockStatus::Burned => "🔥",
        LpLockStatus::TimeLocked { .. } => "⏰",
        LpLockStatus::ProgramLocked { .. } => "🏦",
        LpLockStatus::Locked { .. } => "🔒",
        LpLockStatus::NotLocked { .. } => "🔓",
        LpLockStatus::CreatorHeld => "👤",
        LpLockStatus::PositionNft { .. } => "🎫",
        LpLockStatus::BondingCurve { .. } => "📈",
        LpLockStatus::Unknown => "❓",
        LpLockStatus::NoPool => "🚫",
    };

    let risk_level = if score >= 70 {
        "🟢 SAFE"
    } else if score >= 50 {
        "🟡 MODERATE"
    } else if score >= 30 {
        "🟠 RISKY"
    } else {
        "🔴 HIGH RISK"
    };

    println!("   {} Status: {:?}", status_emoji, status);
    println!("   📝 Description: {}", get_status_description(status));
    println!("   ⚡ Duration: {}ms", duration.as_millis());
    println!("   🔒 Lock Score: {}/100 ({})", score, risk_level);

    // Additional context based on token properties
    if let Some(market_cap) = token.market_cap {
        println!("   � Market Cap: ${:.0}", market_cap);
    }
}

fn get_status_description(status: &LpLockStatus) -> String {
    match status {
        LpLockStatus::Burned => "LP tokens are burned (permanent lock)".to_string(),
        LpLockStatus::TimeLocked { unlock_date, .. } => {
            if let Some(date) = unlock_date {
                format!("LP tokens time-locked until {}", date.format("%Y-%m-%d"))
            } else {
                "LP tokens time-locked".to_string()
            }
        }
        LpLockStatus::ProgramLocked { program, .. } =>
            format!("LP tokens locked by program: {}", program),
        LpLockStatus::Locked { confidence, .. } =>
            format!("LP tokens are properly locked (confidence: {}%)", confidence),
        LpLockStatus::NotLocked { confidence } =>
            format!("LP tokens not locked (confidence: {}%)", confidence),
        LpLockStatus::CreatorHeld => "LP tokens held by creator (risky)".to_string(),
        LpLockStatus::PositionNft { dex, mechanism } =>
            format!("Uses {} {} mechanism (safe)", dex, mechanism),
        LpLockStatus::BondingCurve { dex } =>
            format!("Uses {} bonding curve (no LP tokens to rug)", dex),
        LpLockStatus::Unknown => "Unable to determine lock status".to_string(),
        LpLockStatus::NoPool => "No liquidity pool found".to_string(),
    }
}

fn generate_comprehensive_analysis(
    results: &[(TestToken, LpLockAnalysis, Duration)],
    total_duration: Duration
) {
    println!("📊 Overall Statistics:");
    println!("   • Total Tests: {}", results.len());
    println!("   • Total Duration: {:.2}s", total_duration.as_secs_f64());
    println!(
        "   • Average Duration: {}ms",
        results
            .iter()
            .map(|(_, _, d)| d.as_millis() as u64)
            .sum::<u64>() / (results.len() as u64)
    );

    // Status distribution
    let mut status_counts: HashMap<String, usize> = HashMap::new();
    let mut score_ranges: HashMap<String, usize> = HashMap::new();
    let mut dex_performance: HashMap<String, Vec<u8>> = HashMap::new();

    for (token, analysis, _) in results {
        let status_str = format!("{:?}", analysis.status)
            .split('{')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        *status_counts.entry(status_str).or_insert(0) += 1;

        let score = analysis.lock_score;
        let range = if score >= 70 {
            "Safe (70-100)"
        } else if score >= 50 {
            "Moderate (50-69)"
        } else if score >= 30 {
            "Risky (30-49)"
        } else {
            "High Risk (0-29)"
        };
        *score_ranges.entry(range.to_string()).or_insert(0) += 1;

        dex_performance.entry(token.dex_id.clone()).or_insert_with(Vec::new).push(score);
    }

    println!("\n🔒 LP Lock Status Distribution:");
    for (status, count) in status_counts {
        let percentage = ((count as f64) / (results.len() as f64)) * 100.0;
        println!("   • {}: {} ({:.1}%)", status, count, percentage);
    }

    println!("\n⚠️  Risk Level Distribution:");
    for (range, count) in score_ranges {
        let percentage = ((count as f64) / (results.len() as f64)) * 100.0;
        println!("   • {}: {} ({:.1}%)", range, count, percentage);
    }

    println!("\n🏪 DEX Performance Analysis:");
    for (dex, scores) in dex_performance {
        let avg_score =
            scores
                .iter()
                .map(|&s| s as f64)
                .sum::<f64>() / (scores.len() as f64);
        let safe_count = scores
            .iter()
            .filter(|&&s| s >= 70)
            .count();
        let safe_percentage = ((safe_count as f64) / (scores.len() as f64)) * 100.0;
        println!(
            "   • {}: {} tests, {:.1} avg score, {:.1}% safe",
            dex,
            scores.len(),
            avg_score,
            safe_percentage
        );
    }

    // Identify patterns
    println!("\n🔍 PATTERN ANALYSIS:");

    // Check if any tokens are actually safe
    let safe_tokens: Vec<_> = results
        .iter()
        .filter(|(_, analysis, _)| analysis.lock_score >= 70)
        .collect();
    if safe_tokens.is_empty() {
        println!("   ⚠️  WARNING: NO SAFE TOKENS FOUND - This suggests a potential issue with:");
        println!("      • LP lock verification logic");
        println!("      • Scoring algorithm");
        println!("      • Token selection criteria");
        println!("      • Pool discovery mechanism");
    } else {
        println!("   ✅ Found {} safe tokens:", safe_tokens.len());
        for (token, analysis, _) in safe_tokens {
            println!(
                "      • {} (DEX: {}) - Score: {}",
                token.symbol,
                token.dex_id,
                analysis.lock_score
            );
            println!("        └─ FULL_MINT: {}", token.mint);
            if let Some(pool_addr) = &analysis.pool_address {
                println!("        └─ Pool: {}", pool_addr);
            }
            println!("        └─ Status: {:?}", analysis.status);
        }
    }

    // Check PumpFun vs other DEX performance
    let pumpfun_scores: Vec<u8> = results
        .iter()
        .filter(|(token, _, _)| (token.dex_id == "pumpfun" || token.dex_id == "pumpswap"))
        .map(|(_, analysis, _)| analysis.lock_score)
        .collect();

    let other_scores: Vec<u8> = results
        .iter()
        .filter(|(token, _, _)| token.dex_id != "pumpfun" && token.dex_id != "pumpswap")
        .map(|(_, analysis, _)| analysis.lock_score)
        .collect();

    if !pumpfun_scores.is_empty() && !other_scores.is_empty() {
        let pumpfun_avg =
            pumpfun_scores
                .iter()
                .map(|&s| s as f64)
                .sum::<f64>() / (pumpfun_scores.len() as f64);
        let other_avg =
            other_scores
                .iter()
                .map(|&s| s as f64)
                .sum::<f64>() / (other_scores.len() as f64);

        println!("\n🎯 PumpFun vs Other DEX Comparison:");
        println!("   • PumpFun/PumpSwap Average Score: {:.1}", pumpfun_avg);
        println!("   • Other DEX Average Score: {:.1}", other_avg);

        if pumpfun_avg <= other_avg {
            println!(
                "   ⚠️  WARNING: PumpFun tokens not scoring higher than traditional DEX tokens!"
            );
            println!(
                "      This suggests the LP lock logic may not be properly detecting bonding curves."
            );
        }
    }

    // Detailed breakdown by status
    println!("\n📋 Detailed Results by Status:");
    let mut by_status: HashMap<
        String,
        Vec<&(TestToken, LpLockAnalysis, Duration)>
    > = HashMap::new();
    for result in results {
        let status_key = format!("{:?}", result.1.status)
            .split('{')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        by_status.entry(status_key).or_insert_with(Vec::new).push(result);
    }

    for (status, tokens) in by_status {
        println!("\n   🔹 {} ({} tokens):", status, tokens.len());
        for (token, analysis, duration) in tokens.iter().take(5) {
            // Show first 5
            println!(
                "      • {} (FULL_MINT: {}) - Score: {} - {}ms",
                token.symbol,
                token.mint,
                analysis.lock_score,
                duration.as_millis()
            );
            // Print pool address if available
            if let Some(pool_addr) = &analysis.pool_address {
                println!("        └─ Pool: {}", pool_addr);
            }
        }
        if tokens.len() > 5 {
            println!("      ... and {} more", tokens.len() - 5);
        }
    }

    println!("\n✅ Comprehensive test completed!");
}
