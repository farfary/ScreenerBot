/// Test to show exactly what's happening with time filtering
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let now = chrono::Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);

    println!("🕐 TIME ANALYSIS:");
    println!("  Current UTC time: {}", now);
    println!("  One hour ago: {}", one_hour_ago);

    println!("\n📊 DATABASE TIME COMPARISON:");

    // Most recent token update from our database query
    let last_token_update = chrono::DateTime
        ::parse_from_rfc3339("2025-09-10T23:38:48.479718+00:00")
        .unwrap()
        .with_timezone(&chrono::Utc);

    println!("  Last token update: {}", last_token_update);

    let time_diff = now - last_token_update;
    println!(
        "  Time difference: {} hours, {} minutes",
        time_diff.num_hours(),
        time_diff.num_minutes() % 60
    );

    let is_fresh = last_token_update >= one_hour_ago;
    println!("  Is fresh (< 1 hour): {}", is_fresh);

    if !is_fresh {
        println!(
            "\n❌ PROBLEM: Last token update is {} hours old, but filtering requires < 1 hour",
            time_diff.num_hours()
        );
        println!("🔧 SOLUTION: Either update tokens more frequently OR increase freshness window");
    }

    Ok(())
}
