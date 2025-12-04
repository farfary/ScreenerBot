/// Debug tool for testing and demonstrating the events system
///
/// This tool allows you to:
/// - Record test events across all categories
/// - Query events by various criteria
/// - View database statistics
/// - Test event recording performance
use screenerbot::events::{self, Event, EventCategory, Severity};
use screenerbot::logger::{self as logger, LogTag};
use serde_json::json;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logging
  logger::init();

  // Initialize events system
  if let Err(e) = events::init().await {
    eprintln!("Failed to initialize events system: {}", e);
    return Ok(());
  }

 println!("Events System Debug Tool");
  println!("==========================");

  // Get command line arguments
  let args: Vec<String> = std::env::args().collect();

  if args.len() < 2 {
    print_help();
    return Ok(());
  }

  match args[1].as_str() {
 "test"=> test_event_recording().await?,
 "query"=> query_events(&args[2..]).await?,
 "stats"=> show_stats().await?,
 "performance"=> test_performance().await?,
 "cleanup"=> force_cleanup().await?,
 "--help"| "-h"=> print_help(),
    _ => {
      eprintln!("Unknown command: {}", args[1]);
      print_help();
    }
  }

  Ok(())
}

fn print_help() {
  println!("Events System Debug Tool");
  println!("");
  println!("Usage: debug_events <command> [options]");
  println!("");
  println!("Commands:");
 println!("test - Record test events across all categories");
 println!("query <type> - Query events (recent, by-category, by-mint, by-ref)");
 println!("stats - Show database statistics and event counts");
 println!("performance - Test event recording performance");
 println!("cleanup - Force cleanup of old events");
  println!("");
  println!("Query examples:");
 println!("debug_events query recent");
 println!("debug_events query by-category swap");
 println!("debug_events query by-mint <mint_address>");
 println!("debug_events query by-ref <transaction_signature>");
}

async fn test_event_recording() -> Result<(), Box<dyn std::error::Error>> {
 println!("Recording test events...");

  let test_mint = "So11111111111111111111111111111111111111112"; // SOL mint
  let test_signature = "test_signature_123456789";
  let test_pool = "test_pool_address_123456789";

  // Test swap event
  events::record_swap_event(
    test_signature,
    test_mint,
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
 1000000000, // 1 SOL
 1000000, // 1 USDC (6 decimals)
    true,
    None,
  )
  .await;

  // Test pool event
  events::record_pool_event(
    test_pool,
    "RaydiumProgram123",
    "CPMM",
    test_mint,
    "discovered",
    json!({
      "liquidity_sol": 100.5,
      "price_impact": 0.02,
      "reserves": {
        "token_a": 1000000,
        "token_b": 2000000
      }
    }),
  )
  .await;

  // Test position event
  events::record_position_event(
    "pos_123",
    test_mint,
    "opened",
    Some(test_signature),
    None,
 1.5, // 1.5 SOL
    1000000, // 1M tokens
    None,
    None,
  )
  .await;

  // Test trader signal event
  events::record_trader_event(
    "momentum_spike",
    Severity::Info,
    Some(test_mint),
    None,
    json!({
      "decision": "buy",
      "price_sol": 0.00001234,
      "timeframe": "5m",
      "strength": 0.85,
      "reason": "Strong momentum with volume confirmation",
    }),
  )
  .await;

  // Test system event
  events::record_system_event(
    "trader",
    "started",
    Severity::Info,
    Some(json!({
      "version": "1.0.0",
      "config": {
        "dry_run": false,
        "max_positions": 10
      }
    })),
  )
  .await;

  // Test token event
  events::record_token_event(
    test_mint,
    "blacklisted",
    Severity::Warn,
    json!({
      "reason": "suspicious_activity",
      "details": "Rapid mint authority changes detected",
      "analyzer": "security_scanner"
    }),
  )
  .await;

  // Test security event
  events::record_security_event(
    test_mint,
    "rugcheck_analysis",
    "medium",
    json!({
      "mint_authority": "disabled",
      "freeze_authority": "enabled",
      "lp_locked": false,
      "holder_count": 1250,
      "risk_factors": ["freeze_authority_enabled", "lp_not_locked"]
    }),
  )
  .await;

  // Test error event
  let error_event = Event::error(
    EventCategory::Rpc,
    Some("get_account_info".to_string()),
    None,
    Some("failed_account_fetch".to_string()),
    json!({
      "account": test_pool,
      "error": "RPC timeout after 30s",
      "retry_count": 3
    }),
  );
  events::record(error_event).await?;

 println!("Test events recorded successfully!");

  // Give the background writer time to process
  tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

 println!("Recent events summary:");
  let summary = events::get_events_summary(1).await?;
  println!("{}", serde_json::to_string_pretty(&summary)?);

  Ok(())
}

async fn query_events(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
  if args.is_empty() {
 println!("Query type required. Use: recent, by-category, by-mint, or by-ref");
    return Ok(());
  }

  match args[0].as_str() {
 "recent"=> {
      let limit = if args.len() > 1 {
        args[1].parse().unwrap_or(20)
      } else {
        20
      };
 println!("Recent {} events:", limit);

      let events = events::recent_all(limit).await?;
      print_events(&events);
    }

 "by-category"=> {
      if args.len() < 2 {
 println!("Category required. Examples: swap, pool, position, system");
        return Ok(());
      }

      let category = EventCategory::from_string(&args[1]);
      let limit = if args.len() > 2 {
        args[2].parse().unwrap_or(20)
      } else {
        20
      };

 println!("Recent {} events in category '{}':", limit, args[1]);
      let events = events::recent(category, limit).await?;
      print_events(&events);
    }

 "by-mint"=> {
      if args.len() < 2 {
 println!("Mint address required");
        return Ok(());
      }

      let limit = if args.len() > 2 {
        args[2].parse().unwrap_or(20)
      } else {
        20
      };

 println!("Recent {} events for mint {}:", limit, args[1]);
      let events = events::by_mint(&args[1], limit).await?;
      print_events(&events);
    }

 "by-ref"=> {
      if args.len() < 2 {
 println!("Reference ID required (e.g., transaction signature)");
        return Ok(());
      }

      let limit = if args.len() > 2 {
        args[2].parse().unwrap_or(20)
      } else {
        20
      };

 println!("Recent {} events for reference {}:", limit, args[1]);
      let events = events::by_reference(&args[1], limit).await?;
      print_events(&events);
    }

    _ => {
 println!("Unknown query type: {}", args[0]);
      println!("Available: recent, by-category, by-mint, by-ref");
    }
  }

  Ok(())
}

async fn show_stats() -> Result<(), Box<dyn std::error::Error>> {
 println!("Events Database Statistics");
  println!("=============================");

  // Get summary for different time periods
  for hours in [1, 24, 168] {
    // 1 hour, 1 day, 1 week
    println!("\n Last {} hours:", hours);
    let summary = events::get_events_summary(hours).await?;

    if let Some(counts) = summary.get("counts_by_category") {
 println!("Events by category:");
      if let serde_json::Value::Object(map) = counts {
        for (category, count) in map {
 println!("{}: {}", category, count);
        }
      }
    }
  }

  // Get overall database stats
  println!("\n Database Info:");
  let summary = events::get_events_summary(24).await?;
  if let Some(stats) = summary.get("database_stats") {
    if let serde_json::Value::Object(map) = stats {
      for (key, value) in map {
        match key.as_str() {
 "db_size_bytes"=> {
            let mb = value.as_i64().unwrap_or(0) / 1024 / 1024;
 println!("Database size: {} MB", mb);
          }
 _ => println!("{}: {}", key, value),
        }
      }
    }
  }

  Ok(())
}

async fn test_performance() -> Result<(), Box<dyn std::error::Error>> {
 println!("Testing event recording performance...");

  let test_count = 1000;
  let start = Instant::now();

  for i in 0..test_count {
    let event = Event::info(
      EventCategory::System,
      Some("performance_test".to_string()),
      None,
      None,
      json!({
        "test_id": i,
        "batch": "performance_test",
        "timestamp": chrono::Utc::now().to_rfc3339()
      }),
    );

    events::record_safe(event).await;
  }

  let duration = start.elapsed();
 println!("Recorded {} events in {:?}", test_count, duration);
  println!(
 "Rate: {:.2} events/second",
    (test_count as f64) / duration.as_secs_f64()
  );

  // Wait for background processing
 println!("Waiting for background processing...");
  tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

  // Verify events were recorded
  let recent_events = events::recent_all(50).await?;
  let test_events = recent_events
    .iter()
    .filter(|e| {
      e.subtype
        .as_ref()
        .map_or(false, |s| s == "performance_test")
    })
    .count();

 println!("Verified {} test events in database", test_events);

  Ok(())
}

async fn force_cleanup() -> Result<(), Box<dyn std::error::Error>> {
 println!("Forcing cleanup of old events...");

  let deleted_count = events::cleanup_old_events().await?;
 println!("Cleaned up {} old events", deleted_count);

  Ok(())
}

fn print_events(events: &[Event]) {
  if events.is_empty() {
 println!("No events found");
    return;
  }

  for event in events {
    let time_str = event.event_time.format("%Y-%m-%d %H:%M:%S UTC");
    let subtype_str = event.subtype.as_deref().unwrap_or("-");
    let mint_str = event.mint.as_deref().unwrap_or("-");
    let ref_str = event.reference_id.as_deref().unwrap_or("-");

    println!(
 "{} | {} | {} | {} | {} | {}",
      time_str,
      event.category.to_string(),
      subtype_str,
      event.severity.to_string(),
      mint_str.chars().take(8).collect::<String>(),
      ref_str.chars().take(8).collect::<String>()
    );
  }
}
