/// Events MCP Integration Tool
///
/// This tool provides MCP (Model Context Protocol) compatible functions for
/// querying and analyzing the events system. It can be used as a standalone
/// debug tool or integrated into the MCP server.

use screenerbot::events::{ self, Event, EventCategory, Severity };
use screenerbot::logger::{ init_file_logging };
use serde_json::{ json, Value };
use std::collections::HashMap;
use chrono::{ Utc, Duration, Timelike };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    // Initialize events system
    if let Err(e) = events::init().await {
        eprintln!("Failed to initialize events system: {}", e);
        return Ok(());
    }

    println!("🚀 Events MCP Integration Tool");
    println!("==============================");

    // Get command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    match args[1].as_str() {
        "get_events" => {
            let category = args.get(2).map(|s| s.as_str());
            let limit = args
                .get(3)
                .and_then(|s| s.parse().ok())
                .unwrap_or(50);
            get_events_mcp(category, limit).await?;
        }
        "search_events" => {
            let query = args.get(2).map(|s| s.as_str());
            let category = args.get(3).map(|s| s.as_str());
            let severity = args.get(4).map(|s| s.as_str());
            let limit = args
                .get(5)
                .and_then(|s| s.parse().ok())
                .unwrap_or(50);
            search_events_mcp(query, category, severity, limit).await?;
        }
        "get_events_summary" => {
            let hours = args
                .get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(24);
            get_events_summary_mcp(hours).await?;
        }
        "get_events_by_mint" => {
            if args.len() < 3 {
                eprintln!("❌ Mint address required");
                return Ok(());
            }
            let mint = &args[2];
            let limit = args
                .get(3)
                .and_then(|s| s.parse().ok())
                .unwrap_or(50);
            get_events_by_mint_mcp(mint, limit).await?;
        }
        "get_events_by_reference" => {
            if args.len() < 3 {
                eprintln!("❌ Reference ID required");
                return Ok(());
            }
            let reference_id = &args[2];
            let limit = args
                .get(3)
                .and_then(|s| s.parse().ok())
                .unwrap_or(50);
            get_events_by_reference_mcp(reference_id, limit).await?;
        }
        "analyze_events" => {
            let hours = args
                .get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(24);
            analyze_events_mcp(hours).await?;
        }
        "test_mcp_functions" => {
            test_all_mcp_functions().await?;
        }
        "--help" | "-h" => print_help(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_help();
        }
    }

    Ok(())
}

fn print_help() {
    println!("Events MCP Integration Tool");
    println!("");
    println!("Usage: debug_events_mcp <command> [args...]");
    println!("");
    println!("Commands:");
    println!(
        "  get_events [category] [limit]              - Get recent events, optionally filtered by category"
    );
    println!("  search_events [query] [category] [severity] [limit] - Search events with filters");
    println!("  get_events_summary [hours]                 - Get event statistics for time period");
    println!("  get_events_by_mint <mint> [limit]          - Get events for specific token mint");
    println!("  get_events_by_reference <ref_id> [limit]   - Get events for specific reference ID");
    println!("  analyze_events [hours]                     - Analyze event patterns and trends");
    println!("  test_mcp_functions                         - Test all MCP functions");
    println!("");
    println!("Examples:");
    println!("  debug_events_mcp get_events swap 20");
    println!("  debug_events_mcp search_events jupiter");
    println!("  debug_events_mcp get_events_by_mint So11111111111111111111111111111111111111112");
    println!("  debug_events_mcp analyze_events 48");
}

async fn get_events_mcp(
    category_str: Option<&str>,
    limit: usize
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 MCP Function: get_events");
    println!("Category: {}", category_str.unwrap_or("all"));
    println!("Limit: {}", limit);

    let events = if let Some(cat_str) = category_str {
        let category = EventCategory::from_string(cat_str);
        events::recent(category, limit).await?
    } else {
        events::recent_all(limit).await?
    };

    let result =
        json!({
        "events": events.iter().map(|e| event_to_json(e)).collect::<Vec<_>>(),
        "count": events.len(),
        "category_filter": category_str,
        "limit": limit
    });

    println!("✅ Result:");
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn search_events_mcp(
    query: Option<&str>,
    category_str: Option<&str>,
    severity_str: Option<&str>,
    limit: usize
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 MCP Function: search_events");
    println!("Query: {}", query.unwrap_or("none"));
    println!("Category: {}", category_str.unwrap_or("all"));
    println!("Severity: {}", severity_str.unwrap_or("all"));
    println!("Limit: {}", limit);

    // For now, implement basic search using the available functions
    // In a full implementation, this would use more sophisticated search
    let all_events = events::recent_all(1000).await?;

    let filtered_events: Vec<Event> = all_events
        .into_iter()
        .filter(|event| {
            // Filter by category
            if let Some(cat_str) = category_str {
                let category = EventCategory::from_string(cat_str);
                if std::mem::discriminant(&event.category) != std::mem::discriminant(&category) {
                    return false;
                }
            }

            // Filter by severity
            if let Some(sev_str) = severity_str {
                let severity = Severity::from_string(sev_str);
                if std::mem::discriminant(&event.severity) != std::mem::discriminant(&severity) {
                    return false;
                }
            }

            // Filter by query text (search in subtype and payload)
            if let Some(q) = query {
                let q_lower = q.to_lowercase();
                let matches_subtype = event.subtype
                    .as_ref()
                    .map_or(false, |s| s.to_lowercase().contains(&q_lower));
                let matches_payload = event.payload.to_string().to_lowercase().contains(&q_lower);

                return matches_subtype || matches_payload;
            }

            true
        })
        .take(limit)
        .collect();

    let result =
        json!({
        "events": filtered_events.iter().map(|e| event_to_json(e)).collect::<Vec<_>>(),
        "count": filtered_events.len(),
        "filters": {
            "query": query,
            "category": category_str,
            "severity": severity_str
        },
        "limit": limit
    });

    println!("✅ Result:");
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn get_events_summary_mcp(hours: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 MCP Function: get_events_summary");
    println!("Time range: {} hours", hours);

    let summary = events::get_events_summary(hours).await?;

    println!("✅ Result:");
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

async fn get_events_by_mint_mcp(
    mint: &str,
    limit: usize
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🪙 MCP Function: get_events_by_mint");
    println!("Mint: {}", mint);
    println!("Limit: {}", limit);

    let events = events::by_mint(mint, limit).await?;

    let result =
        json!({
        "events": events.iter().map(|e| event_to_json(e)).collect::<Vec<_>>(),
        "count": events.len(),
        "mint": mint,
        "limit": limit
    });

    println!("✅ Result:");
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn get_events_by_reference_mcp(
    reference_id: &str,
    limit: usize
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 MCP Function: get_events_by_reference");
    println!("Reference ID: {}", reference_id);
    println!("Limit: {}", limit);

    let events = events::by_reference(reference_id, limit).await?;

    let result =
        json!({
        "events": events.iter().map(|e| event_to_json(e)).collect::<Vec<_>>(),
        "count": events.len(),
        "reference_id": reference_id,
        "limit": limit
    });

    println!("✅ Result:");
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn analyze_events_mcp(hours: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 MCP Function: analyze_events");
    println!("Analysis window: {} hours", hours);

    // Get events for analysis
    let events = events::recent_all(1000).await?;
    let cutoff_time = Utc::now() - Duration::hours(hours as i64);
    let recent_events: Vec<Event> = events
        .into_iter()
        .filter(|e| e.event_time >= cutoff_time)
        .collect();

    if recent_events.is_empty() {
        let result =
            json!({
            "analysis": "no_data",
            "message": "No events found in the specified time range",
            "time_range_hours": hours
        });
        println!("✅ Result:");
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Category analysis
    let mut category_counts: HashMap<String, u32> = HashMap::new();
    let mut severity_counts: HashMap<String, u32> = HashMap::new();
    let mut mint_counts: HashMap<String, u32> = HashMap::new();
    let mut hourly_counts: HashMap<u32, u32> = HashMap::new();

    for event in &recent_events {
        *category_counts.entry(event.category.to_string()).or_insert(0) += 1;
        *severity_counts.entry(event.severity.to_string()).or_insert(0) += 1;

        if let Some(mint) = &event.mint {
            *mint_counts.entry(mint.clone()).or_insert(0) += 1;
        }

        let hour = event.event_time.hour();
        *hourly_counts.entry(hour).or_insert(0) += 1;
    }

    // Find most active elements
    let mut category_vec: Vec<(&String, &u32)> = category_counts.iter().collect();
    category_vec.sort_by(|a, b| b.1.cmp(a.1));

    let mut mint_vec: Vec<(&String, &u32)> = mint_counts.iter().collect();
    mint_vec.sort_by(|a, b| b.1.cmp(a.1));

    // Calculate trends (simplified)
    let total_events = recent_events.len();
    let error_events = severity_counts.get("error").unwrap_or(&0);
    let error_rate = ((*error_events as f64) / (total_events as f64)) * 100.0;

    let result =
        json!({
        "analysis": {
            "time_range_hours": hours,
            "total_events": total_events,
            "error_rate_percent": error_rate,
            "categories": category_counts,
            "severities": severity_counts,
            "top_categories": category_vec.iter().take(5).map(|(k, v)| json!({"category": k, "count": v})).collect::<Vec<_>>(),
            "top_mints": mint_vec.iter().take(5).map(|(k, v)| json!({"mint": k, "count": v})).collect::<Vec<_>>(),
            "hourly_distribution": hourly_counts,
            "insights": generate_insights(&category_counts, &severity_counts, error_rate, total_events)
        }
    });

    println!("✅ Result:");
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

async fn test_all_mcp_functions() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing All MCP Functions");
    println!("============================");

    // Test 1: get_events
    println!("\n1️⃣ Testing get_events...");
    let _ = get_events_mcp(Some("swap"), 5).await;

    // Test 2: search_events
    println!("\n2️⃣ Testing search_events...");
    let _ = search_events_mcp(Some("test"), None, None, 5).await;

    // Test 3: get_events_summary
    println!("\n3️⃣ Testing get_events_summary...");
    let _ = get_events_summary_mcp(24).await;

    // Test 4: get_events_by_mint
    println!("\n4️⃣ Testing get_events_by_mint...");
    let _ = get_events_by_mint_mcp("So11111111111111111111111111111111111111112", 5).await;

    // Test 5: get_events_by_reference
    println!("\n5️⃣ Testing get_events_by_reference...");
    let _ = get_events_by_reference_mcp("test_signature_123456789", 5).await;

    // Test 6: analyze_events
    println!("\n6️⃣ Testing analyze_events...");
    let _ = analyze_events_mcp(168).await;

    println!("\n✅ All MCP function tests completed!");
    Ok(())
}

fn event_to_json(event: &Event) -> Value {
    json!({
        "id": event.id,
        "event_time": event.event_time.to_rfc3339(),
        "category": event.category.to_string(),
        "subtype": event.subtype,
        "severity": event.severity.to_string(),
        "mint": event.mint,
        "reference_id": event.reference_id,
        "payload": event.payload,
        "created_at": event.created_at.map(|dt| dt.to_rfc3339())
    })
}

fn generate_insights(
    categories: &HashMap<String, u32>,
    _severities: &HashMap<String, u32>,
    error_rate: f64,
    total_events: usize
) -> Vec<String> {
    let mut insights = Vec::new();

    // Error rate insight
    if error_rate > 10.0 {
        insights.push(format!("High error rate detected: {:.1}% of events are errors", error_rate));
    } else if error_rate < 1.0 {
        insights.push("Low error rate: System appears to be running smoothly".to_string());
    }

    // Activity level insight
    if total_events > 100 {
        insights.push("High event activity detected".to_string());
    } else if total_events < 10 {
        insights.push(
            "Low event activity - system may be idle or events not being recorded".to_string()
        );
    }

    // Category insights
    if let Some(swap_count) = categories.get("swap") {
        let swap_percentage = ((*swap_count as f64) / (total_events as f64)) * 100.0;
        if swap_percentage > 50.0 {
            insights.push(
                format!("High swap activity: {:.1}% of events are swaps", swap_percentage)
            );
        }
    }

    if categories.get("position").unwrap_or(&0) > &0 {
        insights.push("Position management events detected - trading activity present".to_string());
    }

    if categories.get("system").unwrap_or(&0) > &5 {
        insights.push(
            "Multiple system events - check for restarts or configuration changes".to_string()
        );
    }

    if insights.is_empty() {
        insights.push("No significant patterns detected in event data".to_string());
    }

    insights
}
