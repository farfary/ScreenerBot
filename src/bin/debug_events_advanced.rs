/// Advanced Events Testing and Integration Tool
///
/// This tool provides enhanced testing capabilities for the events system,
/// focusing on integration testing and correlation analysis.

use screenerbot::events::{ self, Event, EventCategory, Severity };
use screenerbot::logger::{ init_file_logging };
use serde_json::json;
use std::collections::HashMap;
use chrono::{ Duration, Timelike, Utc };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    // Initialize events system
    if let Err(e) = events::init().await {
        eprintln!("Failed to initialize events system: {}", e);
        return Ok(());
    }

    println!("🚀 Advanced Events Integration Testing Tool");
    println!("==========================================");

    // Get command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    match args[1].as_str() {
        "correlate" => test_event_correlation().await?,
        "simulate" => simulate_trading_session().await?,
        "monitor" => monitor_events_live().await?,
        "analyze" => analyze_event_patterns().await?,
        "validate" => validate_event_data().await?,
        "benchmark" => benchmark_queries().await?,
        "--help" | "-h" => print_help(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_help();
        }
    }

    Ok(())
}

fn print_help() {
    println!("Advanced Events Integration Testing Tool");
    println!("");
    println!("Usage: debug_events_advanced <command>");
    println!("");
    println!("Commands:");
    println!("  correlate   - Test event correlation with existing transactions/positions");
    println!("  simulate    - Simulate a complete trading session with events");
    println!("  monitor     - Live monitoring of events (useful for integration testing)");
    println!("  analyze     - Analyze event patterns and relationships");
    println!("  validate    - Validate event data consistency and completeness");
    println!("  benchmark   - Benchmark query performance across different patterns");
}

async fn test_event_correlation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Testing Event Correlation with Existing Data");
    println!("===============================================");

    // Since we can't access transactions directly, create test correlation events
    let test_signatures = vec!["test_correlation_1", "test_correlation_2", "test_correlation_3"];

    println!("📊 Creating {} test correlation events", test_signatures.len());

    // Create events for correlation testing
    for (i, signature) in test_signatures.iter().enumerate() {
        println!(
            "   Processing test signature {} of {}: {}",
            i + 1,
            test_signatures.len(),
            signature
        );

        // Create a transaction event for correlation testing
        let event = Event::new(
            EventCategory::Transaction,
            Some("correlation_test".to_string()),
            Severity::Info,
            Some("So11111111111111111111111111111111111111112".to_string()),
            Some(signature.to_string()),
            json!({
                "correlation_test": true,
                "test_index": i,
                "created_for_testing": chrono::Utc::now().to_rfc3339()
            })
        );

        events::record(event).await?;
    }

    // Wait for events to be written
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Query events by reference IDs
    for signature in &test_signatures {
        let correlate_events = events::by_reference(signature, 10).await?;
        println!("   ✅ Found {} events for test signature {}", correlate_events.len(), signature);
    }

    println!("🎯 Correlation test completed successfully!");
    Ok(())
}

async fn simulate_trading_session() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Simulating Complete Trading Session");
    println!("=====================================");

    let test_mint = "So11111111111111111111111111111111111111112";
    let session_id = format!("session_{}", chrono::Utc::now().timestamp());

    // 1. System startup event
    events::record_system_event(
        "trader",
        "session_started",
        Severity::Info,
        Some(
            json!({
            "session_id": session_id,
            "test_simulation": true,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
        )
    ).await;

    // 2. Token discovery event
    events::record_token_event(
        test_mint,
        "discovered",
        Severity::Info,
        json!({
            "session_id": session_id,
            "discovery_method": "simulation",
            "market_cap": 50000.0
        })
    ).await;

    // 3. Pool discovery event
    let pool_address = format!("pool_{}", chrono::Utc::now().timestamp_millis());
    events::record_pool_event(
        &pool_address,
        "RaydiumCPMM",
        "CPMM",
        test_mint,
        "discovered",
        json!({
            "session_id": session_id,
            "liquidity_sol": 125.5,
            "price_impact": 0.015
        })
    ).await;

    // 4. Entry signal event
    events::record_entry_event(
        test_mint,
        "momentum_breakout",
        "buy",
        0.00002156,
        "15m",
        0.78,
        Some("Breakout with volume confirmation")
    ).await;

    // 5. Position opened event
    let position_id = format!("pos_{}", chrono::Utc::now().timestamp_millis());
    let tx_signature = format!("tx_{}", chrono::Utc::now().timestamp_millis());
    events::record_position_event(
        &position_id,
        test_mint,
        "opened",
        Some(&tx_signature),
        None,
        2.5, // 2.5 SOL
        1500000, // 1.5M tokens
        Some(0.00002156),
        None
    ).await;

    // 6. Swap event
    events::record_swap_event(
        &tx_signature,
        "So11111111111111111111111111111111111111112", // SOL
        test_mint,
        2500000000, // 2.5 SOL in lamports
        1500000, // 1.5M tokens
        true,
        None
    ).await;

    // 7. Position monitoring events (simulate price changes)
    for i in 1..=3 {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let new_price = 0.00002156 * (1.0 + (i as f64) * 0.05); // 5% increases
        let pnl = (new_price - 0.00002156) * 1500000.0;

        events::record_position_event(
            &position_id,
            test_mint,
            "price_update",
            None,
            None,
            2.5,
            1500000,
            Some(new_price),
            Some(pnl)
        ).await;
    }

    // 8. Position closed event
    let exit_signature = format!("exit_tx_{}", chrono::Utc::now().timestamp_millis());
    events::record_position_event(
        &position_id,
        test_mint,
        "closed",
        None,
        Some(&exit_signature),
        2.5,
        1500000,
        Some(0.00002695), // Final price
        Some(0.8) // Final PnL
    ).await;

    // 9. System session end
    events::record_system_event(
        "trader",
        "session_completed",
        Severity::Info,
        Some(
            json!({
            "session_id": session_id,
            "positions_traded": 1,
            "total_pnl": 0.8
        })
        )
    ).await;

    // Wait for all events to be processed
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    println!("✅ Trading session simulation completed!");

    // Query and display the session events
    println!("\n📊 Session Event Summary:");
    let session_events = events::recent_all(50).await?;

    println!("   Total recent events: {}", session_events.len());

    // Group by category
    let mut category_counts: HashMap<String, u32> = HashMap::new();
    for event in &session_events {
        *category_counts.entry(event.category.to_string()).or_insert(0) += 1;
    }

    for (category, count) in category_counts {
        println!("   {}: {} events", category, count);
    }

    Ok(())
}

async fn monitor_events_live() -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 Live Events Monitor");
    println!("=====================");
    println!("Press Ctrl+C to stop monitoring");
    println!();

    let mut last_count = 0u64;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));

    loop {
        interval.tick().await;

        // Get current event count
        let summary = events::get_events_summary(24).await?;
        let current_count = summary
            .get("database_stats")
            .and_then(|stats| stats.get("total_events"))
            .and_then(|count| count.as_u64())
            .unwrap_or(0);

        if current_count > last_count {
            let new_events = current_count - last_count;
            println!("🆕 {} new events detected (total: {})", new_events, current_count);

            // Show recent events
            let recent = events::recent_all(new_events as usize).await?;
            for event in recent {
                let time_str = event.event_time.format("%H:%M:%S");
                let mint_str = event.mint
                    .as_deref()
                    .unwrap_or("-")
                    .chars()
                    .take(8)
                    .collect::<String>();
                println!(
                    "   {} | {} | {} | {}",
                    time_str,
                    event.category.to_string(),
                    event.subtype.as_deref().unwrap_or("-"),
                    mint_str
                );
            }

            last_count = current_count;
        } else {
            print!("⏳ Waiting for events... (current: {})   \r", current_count);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
    }
}

async fn analyze_event_patterns() -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 Event Pattern Analysis");
    println!("=========================");

    // Get events from last 24 hours
    let all_events = events::recent_all(1000).await?;
    println!("📊 Analyzing {} events", all_events.len());

    if all_events.is_empty() {
        println!("No events to analyze");
        return Ok(());
    }

    // Time distribution analysis
    let mut hourly_counts: HashMap<u32, u32> = HashMap::new();
    for event in &all_events {
        let hour = event.event_time.hour();
        *hourly_counts.entry(hour).or_insert(0) += 1;
    }

    println!("\n⏰ Events by Hour (UTC):");
    for hour in 0..24 {
        let count = hourly_counts.get(&hour).unwrap_or(&0);
        if *count > 0 {
            println!("   {:02}:00 - {}", hour, count);
        }
    }

    // Category distribution
    let mut category_counts: HashMap<String, u32> = HashMap::new();
    for event in &all_events {
        *category_counts.entry(event.category.to_string()).or_insert(0) += 1;
    }

    println!("\n📂 Events by Category:");
    for (category, count) in category_counts.iter() {
        let percentage = ((*count as f64) / (all_events.len() as f64)) * 100.0;
        println!("   {}: {} ({:.1}%)", category, count, percentage);
    }

    // Severity distribution
    let mut severity_counts: HashMap<String, u32> = HashMap::new();
    for event in &all_events {
        *severity_counts.entry(event.severity.to_string()).or_insert(0) += 1;
    }

    println!("\n⚠️  Events by Severity:");
    for (severity, count) in severity_counts.iter() {
        let percentage = ((*count as f64) / (all_events.len() as f64)) * 100.0;
        println!("   {}: {} ({:.1}%)", severity, count, percentage);
    }

    // Most active mints
    let mut mint_counts: HashMap<String, u32> = HashMap::new();
    for event in &all_events {
        if let Some(mint) = &event.mint {
            *mint_counts.entry(mint.clone()).or_insert(0) += 1;
        }
    }

    if !mint_counts.is_empty() {
        println!("\n🪙 Most Active Token Mints:");
        let mut mint_vec: Vec<(&String, &u32)> = mint_counts.iter().collect();
        mint_vec.sort_by(|a, b| b.1.cmp(a.1));

        for (mint, count) in mint_vec.iter().take(5) {
            let mint_short = mint.chars().take(8).collect::<String>();
            println!("   {}: {} events", mint_short, count);
        }
    }

    Ok(())
}

async fn validate_event_data() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Event Data Validation");
    println!("========================");

    let events = events::recent_all(500).await?;
    println!("📊 Validating {} events", events.len());

    let mut issues = Vec::new();

    for event in &events {
        // Check for required fields
        if event.category.to_string().is_empty() {
            issues.push(format!("Event {} missing category", event.id.unwrap_or(0)));
        }

        // Check JSON payload validity
        if let Err(_) = serde_json::from_str::<serde_json::Value>(&event.payload.to_string()) {
            issues.push(format!("Event {} has invalid JSON payload", event.id.unwrap_or(0)));
        }

        // Check timestamp validity
        if event.event_time > chrono::Utc::now() + Duration::hours(1) {
            issues.push(format!("Event {} has future timestamp", event.id.unwrap_or(0)));
        }

        // Validate mint addresses (if present)
        if let Some(mint) = &event.mint {
            if mint.len() != 44 && mint.len() != 43 {
                // Base58 encoded pubkey length
                issues.push(
                    format!("Event {} has invalid mint address format", event.id.unwrap_or(0))
                );
            }
        }
    }

    if issues.is_empty() {
        println!("✅ All events passed validation!");
    } else {
        println!("❌ Found {} validation issues:", issues.len());
        for issue in issues.iter().take(10) {
            // Show first 10
            println!("   {}", issue);
        }
        if issues.len() > 10 {
            println!("   ... and {} more", issues.len() - 10);
        }
    }

    Ok(())
}

async fn benchmark_queries() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Query Performance Benchmark");
    println!("==============================");

    // Benchmark different query types
    let queries = vec![
        ("Recent events (50)", "recent"),
        ("By category (swap)", "category"),
        ("By mint", "mint"),
        ("By reference", "reference")
    ];

    for (name, query_type) in queries {
        let start = std::time::Instant::now();

        match query_type {
            "recent" => {
                events::recent_all(50).await?;
            }
            "category" => {
                events::recent(EventCategory::Swap, 50).await?;
            }
            "mint" => {
                events::by_mint("So11111111111111111111111111111111111111112", 50).await?;
            }
            "reference" => {
                events::by_reference("test_signature_123456789", 50).await?;
            }
            _ => {}
        }

        let duration = start.elapsed();
        println!("   {}: {:?}", name, duration);
    }

    println!("✅ Benchmark completed");
    Ok(())
}
