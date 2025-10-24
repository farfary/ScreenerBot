/// Test binary for webserver-only startup
/// This allows testing events and WebSocket functionality without running the full bot
use screenerbot::{
    config::load_config,
    events,
    logger::{self as logger, LogTag},
    webserver,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    logger::init();
    logger::info(LogTag::System, "🧪 Starting webserver-only test...");

    // Load configuration (global config is required by various modules)
    load_config().expect("Failed to load config");

    // Initialize events system
    logger::info(LogTag::System, "Initializing events system...");
    events::init().await?;
    logger::info(LogTag::System, "✅ Events system initialized");

    // Start webserver
    logger::info(LogTag::System, "Starting webserver...");
    tokio::spawn(async move {
        if let Err(e) = webserver::start_server().await {
            logger::error(LogTag::System, &format!("Webserver error: {}", e));
        }
    });

    // Wait for webserver to start
    sleep(Duration::from_millis(500)).await;

    logger::info(
        LogTag::System,
        "✅ Webserver test started on http://127.0.0.1:8080",
    );
    logger::info(LogTag::System, "Open http://127.0.0.1:8080/events to test");
    logger::info(LogTag::System, "Press Ctrl+C to stop");

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
                }),
            );

            if let Err(e) = events::record(event).await {
                logger::error(
                    LogTag::System,
                    &format!("Failed to record test event: {}", e),
                );
            } else {
                logger::debug(
                    LogTag::System,
                    &format!("✅ Recorded test event #{}", counter),
                );
            }

            counter += 1;
        }
    });

    // Keep running
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
