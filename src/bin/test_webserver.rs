/// Test binary for webserver-only startup
/// This allows testing events and WebSocket functionality without running the full bot

use screenerbot::{
    config::{ load_config, get_config_clone },
    events,
    logger::{ log, LogTag },
    webserver,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger (no init function, just use it directly)
    log(LogTag::System, "INFO", "🧪 Starting webserver-only test...");

    // Load configuration
    load_config().expect("Failed to load config");
    let config = get_config_clone();
    let webserver_config = config.webserver.clone();

    // Initialize events system
    log(LogTag::System, "INFO", "Initializing events system...");
    events::init().await?;
    log(LogTag::System, "SUCCESS", "✅ Events system initialized");

    // Start webserver
    log(LogTag::System, "INFO", "Starting webserver...");
    let webserver_cfg = webserver_config.clone();
    tokio::spawn(async move {
        if let Err(e) = webserver::start_server(webserver_cfg).await {
            log(LogTag::System, "ERROR", &format!("Webserver error: {}", e));
        }
    });

    // Wait for webserver to start
    sleep(Duration::from_millis(500)).await;

    log(LogTag::System, "SUCCESS", "✅ Webserver test started on http://127.0.0.1:8080");
    log(LogTag::System, "INFO", "Open http://127.0.0.1:8080/events to test");
    log(LogTag::System, "INFO", "Press Ctrl+C to stop");

    // Simulate some test events every 5 seconds
    tokio::spawn(async {
        let mut counter = 1;
        loop {
            sleep(Duration::from_secs(5)).await;

            let event = events::Event::new(
                events::EventCategory::System,
                Some("TestEvent".to_string()),
                events::Severity::Info,
                None,
                None,
                serde_json::json!({
                    "test_number": counter,
                    "message": format!("Test event #{}", counter),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            );

            if let Err(e) = events::record(event).await {
                log(LogTag::System, "ERROR", &format!("Failed to record test event: {}", e));
            } else {
                log(LogTag::System, "DEBUG", &format!("✅ Recorded test event #{}", counter));
            }

            counter += 1;
        }
    });

    // Keep running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
