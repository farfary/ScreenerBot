/// Comprehensive LP Lock Verification Test with Real Database Tokens
/// Tests LP lock verification using actual tokens from the ScreenerBot database

use screenerbot::logger::{ log, LogTag };
use screenerbot::tokens::lp_lock::{ check_lp_lock_status, LpLockStatus };
use screenerbot::tokens::dexscreener::init_dexscreener_api;
use std::time::Instant;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct TestToken {
    mint: String,
    symbol: String,
    name: String,
    dex_id: String,
    liquidity_usd: f64,
    market_cap: Option<f64>,
}

#[tokio::main]
async fn main() {
    // Initialize DexScreener API
    if let Err(e) = init_dexscreener_api().await {
        println!("⚠️  Failed to initialize DexScreener API: {}", e);
        return;
    }

    println!("🧪 Comprehensive LP Lock Verification Test with Database Tokens");
    println!("{}", "=".repeat(80));

    // Get real tokens from database
    let test_tokens = get_test_tokens_from_database().await;

    if test_tokens.is_empty() {
        println!("❌ No tokens found in database for testing");
        return;
    }

    println!("📊 Testing {} tokens from database...\n", test_tokens.len());

    let mut results = Vec::new();
    let mut status_distribution = HashMap::new();
    let mut dex_performance = HashMap::new();
    let start_time = Instant::now();

    for (i, token) in test_tokens.iter().enumerate() {
        println!(
            "🔍 [{}/{}] Testing: {} ({})",
            i + 1,
            test_tokens.len(),
            token.symbol,
            token.dex_id
        );
        println!("   Mint: {}", token.mint);
        println!("   Liquidity: ${:.0}", token.liquidity_usd);

        let test_start = Instant::now();

        match check_lp_lock_status(&token.mint).await {
            Ok(analysis) => {
                let test_duration = test_start.elapsed();

                println!("   ✅ Status: {:?}", analysis.status);
                println!("   📝 Description: {}", analysis.status.description());
                println!("   ⚡ Duration: {}ms", test_duration.as_millis());
                println!("   🔒 Lock Score: {}/100", analysis.lock_score);

                // Show key details
                if !analysis.details.is_empty() {
                    println!("   📋 Key Details:");
                    for detail in analysis.details.iter().take(3) {
                        println!("      • {}", detail);
                    }
                }

                results.push((
                    token.clone(),
                    analysis.status.clone(),
                    test_duration,
                    analysis.lock_score,
                ));

                // Track statistics
                let status_key = format!("{:?}", analysis.status)
                    .split('{')
                    .next()
                    .unwrap_or("Unknown")
                    .trim()
                    .to_string();
                *status_distribution.entry(status_key).or_insert(0) += 1;

                let dex_stats = dex_performance
                    .entry(token.dex_id.clone())
                    .or_insert((0, 0u128, 0));
                dex_stats.0 += 1;
                dex_stats.1 += test_duration.as_millis();
                dex_stats.2 += analysis.lock_score as usize;
            }
            Err(e) => {
                let test_duration = test_start.elapsed();
                println!("   ❌ Error: {}", e);
                println!("   ⚡ Duration: {}ms", test_duration.as_millis());

                *status_distribution.entry("Error".to_string()).or_insert(0) += 1;
            }
        }

        println!();

        // Small delay to avoid overwhelming the system
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    let total_duration = start_time.elapsed();

    // Generate comprehensive report
    generate_test_report(&results, &status_distribution, &dex_performance, total_duration);
}

async fn get_test_tokens_from_database() -> Vec<TestToken> {
    use rusqlite::{ Connection, Result };

    let db_path = "data/tokens.db";
    let conn = match Connection::open(db_path) {
        Ok(conn) => conn,
        Err(e) => {
            println!("❌ Failed to open database: {}", e);
            return Vec::new();
        }
    };

    let mut tokens = Vec::new();

    // Get tokens from different DEX types and liquidity ranges
    let queries = vec![
        // High liquidity tokens from different DEXes
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'raydium' AND liquidity_usd > 50000 ORDER BY RANDOM() LIMIT 2",
            "High Liquidity Raydium",
        ),
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'orca' AND liquidity_usd > 10000 ORDER BY RANDOM() LIMIT 2",
            "High Liquidity Orca",
        ),
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'meteora' AND liquidity_usd > 10000 ORDER BY RANDOM() LIMIT 2",
            "Meteora",
        ),

        // PumpFun tokens (different liquidity ranges)
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'pumpswap' AND liquidity_usd > 20000 ORDER BY RANDOM() LIMIT 2",
            "High Liquidity PumpFun",
        ),
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'pumpswap' AND liquidity_usd BETWEEN 1000 AND 10000 ORDER BY RANDOM() LIMIT 2",
            "Medium Liquidity PumpFun",
        ),

        // Other DEX types
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'fluxbeam' AND liquidity_usd > 1000 ORDER BY RANDOM() LIMIT 1",
            "FluxBeam",
        ),
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE dex_id = 'heaven' AND liquidity_usd > 1000 ORDER BY RANDOM() LIMIT 1",
            "Heaven",
        ),

        // Low liquidity tokens for edge case testing
        (
            "SELECT mint, symbol, name, dex_id, liquidity_usd, market_cap FROM tokens WHERE liquidity_usd BETWEEN 100 AND 1000 ORDER BY RANDOM() LIMIT 2",
            "Low Liquidity",
        )
    ];

    for (query, category) in queries {
        println!("🔍 Fetching {} tokens...", category);

        let mut stmt = match conn.prepare(query) {
            Ok(stmt) => stmt,
            Err(e) => {
                println!("❌ Failed to prepare query for {}: {}", category, e);
                continue;
            }
        };

        let token_iter = stmt.query_map([], |row| {
            Ok(TestToken {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                dex_id: row.get(3)?,
                liquidity_usd: row.get(4)?,
                market_cap: row.get::<_, Option<f64>>(5).unwrap_or(None),
            })
        });

        match token_iter {
            Ok(iter) => {
                for token_result in iter {
                    match token_result {
                        Ok(token) => {
                            println!(
                                "   ✅ {}: {} (${:.0})",
                                token.symbol,
                                token.dex_id,
                                token.liquidity_usd
                            );
                            tokens.push(token);
                        }
                        Err(e) => println!("   ❌ Error parsing token: {}", e),
                    }
                }
            }
            Err(e) => println!("❌ Failed to execute query for {}: {}", category, e),
        }
    }

    println!("📊 Total tokens selected for testing: {}\n", tokens.len());
    tokens
}

fn generate_test_report(
    results: &[(TestToken, LpLockStatus, std::time::Duration, u8)],
    status_distribution: &HashMap<String, i32>,
    dex_performance: &HashMap<String, (i32, u128, usize)>,
    total_duration: std::time::Duration
) {
    println!("📈 COMPREHENSIVE TEST REPORT");
    println!("{}", "=".repeat(80));

    // Overall Statistics
    println!("📊 Overall Statistics:");
    println!("   • Total Tests: {}", results.len());
    println!("   • Total Duration: {:.2}s", total_duration.as_secs_f64());
    println!(
        "   • Average Duration: {:.0}ms",
        (total_duration.as_millis() as f64) / (results.len() as f64)
    );
    println!(
        "   • Success Rate: {:.1}%",
        ((results.len() as f64) / (results.len() as f64)) * 100.0
    );
    println!();

    // Status Distribution
    println!("🔒 LP Lock Status Distribution:");
    for (status, count) in status_distribution {
        let percentage = ((*count as f64) / (results.len() as f64)) * 100.0;
        println!("   • {}: {} ({:.1}%)", status, count, percentage);
    }
    println!();

    // DEX Performance Analysis
    println!("🏪 DEX Performance Analysis:");
    for (dex, (count, total_ms, total_score)) in dex_performance {
        let avg_duration = (*total_ms as f64) / (*count as f64);
        let avg_score = (*total_score as f64) / (*count as f64);
        println!(
            "   • {}: {} tests, {:.0}ms avg, {:.1} avg score",
            dex,
            count,
            avg_duration,
            avg_score
        );
    }
    println!();

    // Detailed Results by DEX
    println!("📋 Detailed Results by DEX:");
    let mut dex_groups: HashMap<
        String,
        Vec<&(TestToken, LpLockStatus, std::time::Duration, u8)>
    > = HashMap::new();
    for result in results {
        dex_groups.entry(result.0.dex_id.clone()).or_default().push(result);
    }

    for (dex, dex_results) in dex_groups {
        println!("\n   🏪 {} ({} tokens):", dex.to_uppercase(), dex_results.len());
        for (token, status, duration, score) in dex_results {
            let status_emoji = match status {
                LpLockStatus::Burned => "🔥",
                LpLockStatus::TimeLocked { .. } => "⏰",
                LpLockStatus::ProgramLocked { .. } => "🔒",
                LpLockStatus::Locked { .. } => "🔐",
                LpLockStatus::NotLocked { .. } => "🔓",
                LpLockStatus::CreatorHeld => "👤",
                LpLockStatus::PositionNft { .. } => "🎫",
                LpLockStatus::BondingCurve { .. } => "📈",
                LpLockStatus::Unknown => "❓",
                LpLockStatus::NoPool => "🚫",
            };

            println!(
                "      {} {} ({}) - {} - {}ms - Score: {}/100",
                status_emoji,
                token.symbol,
                token.mint[..8].to_string(),
                status.description(),
                duration.as_millis(),
                score
            );
        }
    }

    // Risk Assessment Summary
    println!("\n🛡️  Risk Assessment Summary:");
    let safe_count = results
        .iter()
        .filter(|(_, status, _, _)| status.is_safe())
        .count();
    let unsafe_count = results.len() - safe_count;

    println!(
        "   • Safe Tokens: {} ({:.1}%)",
        safe_count,
        ((safe_count as f64) / (results.len() as f64)) * 100.0
    );
    println!(
        "   • Risky Tokens: {} ({:.1}%)",
        unsafe_count,
        ((unsafe_count as f64) / (results.len() as f64)) * 100.0
    );

    // Performance Insights
    println!("\n⚡ Performance Insights:");
    let fastest = results.iter().min_by_key(|(_, _, duration, _)| duration);
    let slowest = results.iter().max_by_key(|(_, _, duration, _)| duration);

    if let Some((token, _, duration, _)) = fastest {
        println!("   • Fastest: {} ({}ms)", token.symbol, duration.as_millis());
    }
    if let Some((token, _, duration, _)) = slowest {
        println!("   • Slowest: {} ({}ms)", token.symbol, duration.as_millis());
    }

    let high_score_count = results
        .iter()
        .filter(|(_, _, _, score)| *score >= 70)
        .count();
    println!("   • High Confidence (≥70): {} tokens", high_score_count);

    println!("\n✅ Test completed successfully!");
}
