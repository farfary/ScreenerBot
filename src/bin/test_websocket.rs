use screenerbot::{
    configs::read_configs,
    logger::{ init_file_logging, log, LogTag },
    websocket,
    utils::get_wallet_address,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();
    log(LogTag::System, "INFO", "🧪 Testing WebSocket transaction monitoring");

    // Get wallet address
    let wallet_address = get_wallet_address().expect("Failed to get wallet address");

    log(LogTag::System, "INFO", &format!("Monitoring wallet: {}", &wallet_address[..8]));

    // Load WebSocket URL from config, fallback to default
    let ws_url = match read_configs() {
        Ok(config) => {
            log(LogTag::System, "INFO", &format!("📡 Using premium WebSocket URL from config"));
            config.rpc_urls.get(0).cloned().unwrap_or_default()
        }
        Err(e) => {
            log(
                LogTag::System,
                "INFO",
                &format!("⚠️ Failed to load config ({}), using default WebSocket", e)
            );
            websocket::SolanaWebSocketClient::get_default_ws_url()
        }
    };

    // Start WebSocket monitoring
    let mut receiver = websocket::start_websocket_monitoring(wallet_address, Some(ws_url)).await?;

    log(LogTag::System, "INFO", "🔌 WebSocket monitoring started - waiting for transactions...");

    // Listen for new transactions for 30 seconds
    let mut transaction_count = 0;
    let start_time = std::time::Instant::now();
    let test_duration = std::time::Duration::from_secs(230);

    while start_time.elapsed() < test_duration {
        tokio::select! {
            signature = receiver.recv() => {
                match signature {
                    Some(sig) => {
                        transaction_count += 1;
                        log(
                            LogTag::System,
                            "INFO",
                            &format!("🆕 Detected transaction #{}: {}", transaction_count, &sig[..8])
                        );
                    }
                    None => {
                        log(LogTag::System, "WARN", "WebSocket channel closed");
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("⏱️ Still listening... ({} transactions detected so far)", transaction_count)
                );
            }
        }
    }

    log(
        LogTag::System,
        "INFO",
        &format!(
            "🎉 Test completed! Detected {} transactions in {} seconds",
            transaction_count,
            test_duration.as_secs()
        )
    );

    Ok(())
}
