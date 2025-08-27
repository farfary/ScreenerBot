use std::time::Duration;
use tokio::time::sleep;
use chrono::{ DateTime, Utc };
use screener_bot::tokens::pool::{ get_pool_service, get_price, PriceOptions, PriceResult };
use screener_bot::logger::{ log, LogTag };

/// Test token address to monitor
const TEST_TOKEN: &str = "A8C3xuqscfmyLrte3VmTqrAq8kgMASius9AFNANwpump";

/// Price monitoring test structure
#[derive(Debug, Clone)]
struct PriceSnapshot {
    timestamp: DateTime<Utc>,
    price_sol: Option<f64>,
    price_usd: Option<f64>,
    pool_address: Option<String>,
    dex_id: Option<String>,
    pool_type: Option<String>,
    liquidity_usd: Option<f64>,
    volume_24h: Option<f64>,
    source: String,
}

impl PriceSnapshot {
    fn from_price_result(result: &PriceResult) -> Self {
        Self {
            timestamp: result.calculated_at,
            price_sol: result.best_sol_price(),
            price_usd: result.price_usd,
            pool_address: result.pool_address.clone(),
            dex_id: result.dex_id.clone(),
            pool_type: result.pool_type.clone(),
            liquidity_usd: result.liquidity_usd,
            volume_24h: result.volume_24h,
            source: result.source.clone(),
        }
    }

    fn print_summary(&self) {
        println!("📊 Price Snapshot at {}", self.timestamp.format("%H:%M:%S"));

        if let Some(price_sol) = self.price_sol {
            println!("   💰 SOL Price: {:.8} SOL", price_sol);
        } else {
            println!("   💰 SOL Price: N/A");
        }

        if let Some(price_usd) = self.price_usd {
            println!("   💵 USD Price: ${:.6}", price_usd);
        } else {
            println!("   💵 USD Price: N/A");
        }

        println!("   📈 Source: {}", self.source);

        if let Some(pool_addr) = &self.pool_address {
            println!("   🏊 Pool: {}...{}", &pool_addr[..8], &pool_addr[pool_addr.len() - 8..]);
        }

        if let Some(dex) = &self.dex_id {
            println!("   🔄 DEX: {}", dex);
        }

        if let Some(pool_type) = &self.pool_type {
            println!("   🔧 Pool Type: {}", pool_type);
        }

        if let Some(liquidity) = self.liquidity_usd {
            println!("   💧 Liquidity: ${:.2}", liquidity);
        }

        if let Some(volume) = self.volume_24h {
            println!("   📊 24h Volume: ${:.2}", volume);
        }

        println!();
    }
}

/// Price change detector
#[derive(Debug)]
struct PriceChangeDetector {
    snapshots: Vec<PriceSnapshot>,
    last_price_sol: Option<f64>,
    price_change_threshold: f64, // Minimum percentage change to report
}

impl PriceChangeDetector {
    fn new(price_change_threshold: f64) -> Self {
        Self {
            snapshots: Vec::new(),
            last_price_sol: None,
            price_change_threshold,
        }
    }

    fn add_snapshot(&mut self, snapshot: PriceSnapshot) -> bool {
        let has_significant_change = if
            let (Some(current_price), Some(last_price)) = (snapshot.price_sol, self.last_price_sol)
        {
            let change_pct = (((current_price - last_price) / last_price) * 100.0).abs();
            change_pct >= self.price_change_threshold
        } else {
            true // First price or price became available/unavailable
        };

        if has_significant_change {
            if let Some(last_price) = self.last_price_sol {
                if let Some(current_price) = snapshot.price_sol {
                    let change_pct = ((current_price - last_price) / last_price) * 100.0;
                    let change_indicator = if change_pct > 0.0 { "📈" } else { "📉" };
                    println!("🚨 PRICE CHANGE ALERT! {} {:.2}%", change_indicator, change_pct);
                }
            }
        }

        self.snapshots.push(snapshot.clone());
        self.last_price_sol = snapshot.price_sol;

        // Keep only last 100 snapshots
        if self.snapshots.len() > 100 {
            self.snapshots.remove(0);
        }

        has_significant_change
    }

    fn get_stats(&self) -> (usize, Option<f64>, Option<f64>, Option<f64>) {
        let count = self.snapshots.len();

        let prices: Vec<f64> = self.snapshots
            .iter()
            .filter_map(|s| s.price_sol)
            .collect();

        if prices.is_empty() {
            return (count, None, None, None);
        }

        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg_price = prices.iter().sum::<f64>() / (prices.len() as f64);

        (count, Some(min_price), Some(max_price), Some(avg_price))
    }

    fn print_stats(&self) {
        let (count, min_price, max_price, avg_price) = self.get_stats();

        println!("📊 MONITORING STATISTICS:");
        println!("   Total snapshots: {}", count);

        if let (Some(min), Some(max), Some(avg)) = (min_price, max_price, avg_price) {
            println!("   Min price: {:.8} SOL", min);
            println!("   Max price: {:.8} SOL", max);
            println!("   Avg price: {:.8} SOL", avg);

            if max > min {
                let volatility = ((max - min) / avg) * 100.0;
                println!("   Volatility: {:.2}%", volatility);
            }
        } else {
            println!("   No price data available");
        }
        println!();
    }
}

/// Main price monitoring test function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Starting Price Monitor Test for token: {}", TEST_TOKEN);
    println!("===============================================");
    println!();

    // Initialize the pool service
    let pool_service = get_pool_service();

    // Start background monitoring if not already active
    if !pool_service.is_monitoring_active().await {
        println!("🚀 Starting pool monitoring service...");
        pool_service.start_monitoring().await;
        sleep(Duration::from_secs(2)).await; // Give it time to start
    } else {
        println!("✅ Pool monitoring service already active");
    }

    let mut detector = PriceChangeDetector::new(1.0); // 1% change threshold
    let mut iteration = 0;

    println!("🎯 Monitoring price changes (Ctrl+C to stop)...");
    println!("📊 Will report changes >= 1.0%");
    println!();

    loop {
        iteration += 1;

        println!("🔄 Iteration {} - {}", iteration, Utc::now().format("%Y-%m-%d %H:%M:%S"));

        // Test different price options
        let test_scenarios = vec![
            ("📊 Comprehensive (Pool + API)", PriceOptions::comprehensive()),
            ("🏊 Pool Only", PriceOptions::pool_only()),
            ("🌐 API Only", PriceOptions::api_only()),
            ("⚡ Simple (Cached)", PriceOptions::simple())
        ];

        let mut best_result: Option<PriceResult> = None;

        for (scenario_name, options) in test_scenarios {
            println!("   Testing: {}", scenario_name);

            match get_price(TEST_TOKEN, Some(options), false).await {
                Some(result) => {
                    let snapshot = PriceSnapshot::from_price_result(&result);

                    // Use the first successful result as our main snapshot
                    if best_result.is_none() {
                        best_result = Some(result.clone());
                    }

                    // Print condensed info
                    if let Some(price) = snapshot.price_sol {
                        println!("      ✅ Price: {:.8} SOL ({})", price, snapshot.source);
                    } else {
                        println!("      ❌ No price available");
                    }
                }
                None => {
                    println!("      ❌ Failed to get price");
                }
            }
        }

        // Process the best result
        if let Some(result) = best_result {
            let snapshot = PriceSnapshot::from_price_result(&result);
            snapshot.print_summary();

            let has_change = detector.add_snapshot(snapshot);

            if has_change {
                detector.print_stats();
            }
        } else {
            println!("❌ No price data available from any source");
            println!();
        }

        // Check pool availability
        let has_pools = pool_service.check_token_availability(TEST_TOKEN).await;
        println!("🏊 Pool availability: {}", if has_pools {
            "✅ Available"
        } else {
            "❌ No pools"
        });

        // Get cached pools info
        if let Some(pools) = pool_service.get_cached_pools_infos(TEST_TOKEN).await {
            println!("📋 Cached pools: {} found", pools.len());
            for (i, pool) in pools.iter().take(3).enumerate() {
                println!("   {}. {} (${:.0} liquidity)", i + 1, pool.dex_id, pool.liquidity_usd);
            }
        } else {
            println!("📋 No cached pools info");
        }

        // Get price history
        let history = pool_service.get_recent_price_history(TEST_TOKEN).await;
        if !history.is_empty() {
            println!("📈 Price history: {} entries", history.len());
            if let Some((timestamp, price)) = history.last() {
                println!("   Latest: {:.8} SOL at {}", price, timestamp.format("%H:%M:%S"));
            }
        } else {
            println!("📈 No price history available");
        }

        // Get service statistics
        let stats = pool_service.get_enhanced_stats().await;
        println!(
            "📊 Service stats: {:.1}% success, {:.1}% cache hits",
            stats.get_success_rate() * 100.0,
            stats.get_cache_hit_rate() * 100.0
        );

        println!("═══════════════════════════════════════════════");

        // Print summary stats every 10 iterations
        if iteration % 10 == 0 {
            detector.print_stats();
        }

        // Wait before next iteration
        sleep(Duration::from_secs(10)).await; // 10 second intervals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_price_snapshot_creation() {
        // Create a mock price result
        let result = PriceResult {
            token_address: TEST_TOKEN.to_string(),
            price_sol: Some(0.00001234),
            price_usd: Some(0.002),
            api_price_sol: Some(0.000012),
            pool_price_sol: Some(0.00001234),
            pool_address: Some("DummyPoolAddress123".to_string()),
            dex_id: Some("raydium".to_string()),
            pool_type: Some("RAYDIUM CPMM".to_string()),
            liquidity_usd: Some(15000.0),
            volume_24h: Some(50000.0),
            source: "pool".to_string(),
            calculated_at: Utc::now(),
            is_cached: false,
        };

        let snapshot = PriceSnapshot::from_price_result(&result);

        assert_eq!(snapshot.price_sol, Some(0.00001234));
        assert_eq!(snapshot.source, "pool");
        assert!(snapshot.pool_address.is_some());
    }

    #[tokio::test]
    async fn test_price_change_detector() {
        let mut detector = PriceChangeDetector::new(5.0); // 5% threshold

        // First snapshot
        let snapshot1 = PriceSnapshot {
            timestamp: Utc::now(),
            price_sol: Some(0.00001),
            price_usd: None,
            pool_address: None,
            dex_id: None,
            pool_type: None,
            liquidity_usd: None,
            volume_24h: None,
            source: "test".to_string(),
        };

        let change1 = detector.add_snapshot(snapshot1);
        assert!(change1); // First snapshot should always be significant

        // Second snapshot with small change (< 5%)
        let snapshot2 = PriceSnapshot {
            timestamp: Utc::now(),
            price_sol: Some(0.0000102), // 2% increase
            price_usd: None,
            pool_address: None,
            dex_id: None,
            pool_type: None,
            liquidity_usd: None,
            volume_24h: None,
            source: "test".to_string(),
        };

        let change2 = detector.add_snapshot(snapshot2);
        assert!(!change2); // Should not be significant

        // Third snapshot with large change (> 5%)
        let snapshot3 = PriceSnapshot {
            timestamp: Utc::now(),
            price_sol: Some(0.000011), // ~7.8% increase from last
            price_usd: None,
            pool_address: None,
            dex_id: None,
            pool_type: None,
            liquidity_usd: None,
            volume_24h: None,
            source: "test".to_string(),
        };

        let change3 = detector.add_snapshot(snapshot3);
        assert!(change3); // Should be significant

        let (count, min_price, max_price, avg_price) = detector.get_stats();
        assert_eq!(count, 3);
        assert!(min_price.is_some());
        assert!(max_price.is_some());
        assert!(avg_price.is_some());
    }

    #[tokio::test]
    async fn test_pool_service_availability() {
        let pool_service = get_pool_service();

        // Test token availability check
        let has_pools = pool_service.check_token_availability(TEST_TOKEN).await;
        println!("Token has pools: {}", has_pools);

        // This test just ensures the function doesn't panic
        // The actual result depends on network conditions and token state
    }

    #[tokio::test]
    async fn test_price_retrieval() {
        // Test simple price retrieval
        let result = get_price(TEST_TOKEN, Some(PriceOptions::simple()), false).await;

        // Print result for debugging
        if let Some(price_result) = &result {
            println!("Price result: {:?}", price_result);
        } else {
            println!("No price result obtained");
        }

        // This test mainly ensures the function doesn't panic
        // Success depends on network conditions and token availability
    }
}
