/// WebSocket Debug Tool - Comprehensive WebSocket Connection Testing
///
/// This tool performs detailed diagnostics of WebSocket connectivity:
/// - Tests connection to Helius WebSocket endpoint
/// - Validates subscription to wallet logs
/// - Monitors for incoming messages and notifications
/// - Tests ping/pong heartbeat mechanism
/// - Displays raw WebSocket messages for troubleshooting
///
/// Usage:
///   cargo run --bin debug_websocket
///   cargo run --bin debug_websocket -- --duration 60  (run for 60 seconds)
///   cargo run --bin debug_websocket -- --wallet YOUR_WALLET_ADDRESS

use chrono::Utc;
use futures_util::{ SinkExt, StreamExt };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ Duration, timeout };
use tokio_tungstenite::{ connect_async, tungstenite::Message };

#[derive(Serialize)]
struct LogsSubscribe {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WebSocketResponse {
    jsonrpc: String,
    id: Option<u64>,
    result: Option<serde_json::Value>,
    method: Option<String>,
    params: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

fn print_header(title: &str) {
    println!("\n{}", "=".repeat(80));
    println!("  {}", title);
    println!("{}\n", "=".repeat(80));
}

fn print_step(step: &str, status: &str) {
    let status_symbol = match status {
        "SUCCESS" => "✅",
        "RUNNING" => "🔄",
        "ERROR" => "❌",
        "WARNING" => "⚠️",
        "INFO" => "ℹ️",
        _ => "▪️",
    };
    println!("{} {} {}", status_symbol, step, if status != "RUNNING" && status != "INFO" {
        format!("- {}", status)
    } else {
        String::new()
    });
}

#[tokio::main]
async fn main() {
    print_header("🔍 WEBSOCKET DEBUG TOOL - COMPREHENSIVE CONNECTION DIAGNOSTICS");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut test_duration = 30; // Default 30 seconds
    let mut wallet_address: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--duration" => {
                if i + 1 < args.len() {
                    test_duration = args[i + 1].parse().unwrap_or(30);
                    i += 1;
                }
            }
            "--wallet" => {
                if i + 1 < args.len() {
                    wallet_address = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--help" => {
                println!("Usage: cargo run --bin debug_websocket [OPTIONS]\n");
                println!("Options:");
                println!("  --duration <seconds>    Duration to monitor (default: 30)");
                println!(
                    "  --wallet <address>      Wallet address to monitor (default: from config)"
                );
                println!("  --help                  Show this help message");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    // Load wallet from config if not provided
    let wallet = if let Some(addr) = wallet_address {
        addr
    } else {
        match screenerbot::utils::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => {
                print_step("Load wallet from config", "ERROR");
                println!("   Error: {}", e);
                return;
            }
        }
    };

    println!("📋 Configuration:");
    println!("   Wallet:   {}", wallet);
    println!("   Duration: {}s", test_duration);
    println!();

    // Step 1: Load RPC configuration
    print_step("Loading RPC configuration", "RUNNING");

    // Load config first
    if let Err(e) = screenerbot::config::load_config() {
        print_step(&format!("Failed to load config: {}", e), "ERROR");
        return;
    }

    let ws_url = {
        let rpc_urls = screenerbot::config::with_config(|cfg| cfg.rpc.urls.clone());
        if let Some(first_rpc_url) = rpc_urls.first() {
            let ws = first_rpc_url.replace("https://", "wss://").replace("http://", "ws://");
            print_step(&format!("RPC URL: {}", first_rpc_url), "INFO");
            print_step(&format!("WebSocket URL: {}", ws), "SUCCESS");
            ws
        } else {
            print_step("No RPC URLs in config", "ERROR");
            return;
        }
    };

    println!();

    // Step 2: Test WebSocket Connection
    print_header("🔌 STEP 1: TESTING WEBSOCKET CONNECTION");
    print_step(&format!("Connecting to {}", ws_url), "RUNNING");

    let connect_start = std::time::Instant::now();
    let connection_result = timeout(Duration::from_secs(10), connect_async(&ws_url)).await;

    let ws_stream = match connection_result {
        Ok(Ok((stream, resp))) => {
            let elapsed = connect_start.elapsed();
            print_step(&format!("Connection established in {:?}", elapsed), "SUCCESS");
            print_step(&format!("Response status: {:?}", resp.status()), "INFO");
            println!();
            stream
        }
        Ok(Err(e)) => {
            print_step(&format!("Connection failed: {}", e), "ERROR");
            println!("\n🔍 Troubleshooting:");
            println!("   1. Check if the RPC URL is correct in configs.json");
            println!("   2. Verify network connectivity");
            println!("   3. Check if Helius API key is valid");
            println!("   4. Try: curl -I {}", ws_url.replace("wss://", "https://"));
            return;
        }
        Err(_) => {
            print_step("Connection timeout (10s)", "ERROR");
            println!("\n🔍 Possible causes:");
            println!("   - Network firewall blocking WebSocket connections");
            println!("   - RPC endpoint not responding");
            println!("   - DNS resolution issues");
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Step 3: Send Subscription Request
    print_header("📡 STEP 2: SUBSCRIBING TO WALLET LOGS");

    let subscribe_message = LogsSubscribe {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "logsSubscribe".to_string(),
        params: vec![
            serde_json::json!({
                "mentions": [wallet]
            }),
            serde_json::json!({
                "commitment": "confirmed"
            })
        ],
    };

    let subscribe_text = match serde_json::to_string(&subscribe_message) {
        Ok(text) => {
            print_step("Subscription request serialized", "SUCCESS");
            println!("   Request: {}", text);
            text
        }
        Err(e) => {
            print_step(&format!("Failed to serialize: {}", e), "ERROR");
            return;
        }
    };

    println!();
    print_step("Sending subscription request", "RUNNING");

    if let Err(e) = ws_sender.send(Message::Text(subscribe_text)).await {
        print_step(&format!("Failed to send: {}", e), "ERROR");
        return;
    }

    print_step("Subscription request sent", "SUCCESS");
    println!();

    // Step 4: Wait for Subscription Confirmation
    print_header("⏳ STEP 3: WAITING FOR SUBSCRIPTION CONFIRMATION");

    let mut subscription_id: Option<u64> = None;
    let mut message_count = 0;
    let mut notification_count = 0;
    let mut ping_count = 0;
    let mut pong_count = 0;

    print_step("Waiting for server response (10s timeout)", "RUNNING");

    match timeout(Duration::from_secs(10), ws_receiver.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            message_count += 1;
            print_step("Received response", "SUCCESS");
            println!("\n📨 Raw Response:");
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(json_val) => {
                    println!("{}", serde_json::to_string_pretty(&json_val).unwrap_or(text.clone()));
                }
                Err(_) => {
                    println!("{}", text);
                }
            }

            // Parse response
            match serde_json::from_str::<WebSocketResponse>(&text) {
                Ok(resp) => {
                    if let Some(error) = resp.error {
                        print_step(&format!("Subscription error: {:?}", error), "ERROR");
                        println!("\n🔍 Common errors:");
                        println!("   - Invalid wallet address format");
                        println!("   - logsSubscribe not supported by endpoint");
                        println!("   - Rate limiting");
                        return;
                    }

                    if let Some(result) = resp.result {
                        subscription_id = result.as_u64();
                        print_step(
                            &format!("Subscription confirmed! ID: {:?}", subscription_id),
                            "SUCCESS"
                        );
                    }
                }
                Err(e) => {
                    print_step(&format!("Failed to parse response: {}", e), "WARNING");
                }
            }
        }
        Ok(Some(Ok(msg))) => {
            print_step(&format!("Received unexpected message type: {:?}", msg), "WARNING");
        }
        Ok(Some(Err(e))) => {
            print_step(&format!("WebSocket error: {}", e), "ERROR");
            return;
        }
        Ok(None) => {
            print_step("Connection closed by server", "ERROR");
            return;
        }
        Err(_) => {
            print_step("No response received (timeout)", "ERROR");
            println!("\n🔍 Possible causes:");
            println!("   - Server not responding to logsSubscribe");
            println!("   - Wrong subscription method for this endpoint");
            println!("   - Try using 'accountSubscribe' instead");
            return;
        }
    }

    println!();

    // Step 5: Monitor for Messages
    print_header(&format!("👀 STEP 4: MONITORING FOR MESSAGES ({}s)", test_duration));

    if subscription_id.is_some() {
        print_step("Listening for notifications", "SUCCESS");
        println!("   💡 You can send a test transaction to see it appear here");
    } else {
        print_step("No subscription ID - monitoring anyway", "WARNING");
    }

    println!();

    let start_time = std::time::Instant::now();
    let mut last_heartbeat = std::time::Instant::now();
    let heartbeat_interval = Duration::from_secs(5);

    // Statistics
    let mut last_stats_time = std::time::Instant::now();
    let stats_interval = Duration::from_secs(5);

    loop {
        let elapsed = start_time.elapsed().as_secs();
        if elapsed >= test_duration {
            break;
        }

        // Send heartbeat ping
        if last_heartbeat.elapsed() >= heartbeat_interval {
            if let Err(e) = ws_sender.send(Message::Ping(vec![])).await {
                print_step(&format!("Heartbeat failed: {}", e), "ERROR");
                break;
            }
            ping_count += 1;
            last_heartbeat = std::time::Instant::now();
        }

        // Print statistics
        if last_stats_time.elapsed() >= stats_interval {
            println!("\n📊 Statistics ({}/{}s):", elapsed, test_duration);
            println!("   Messages received: {}", message_count);
            println!("   Notifications:     {}", notification_count);
            println!("   Pings sent:        {}", ping_count);
            println!("   Pongs received:    {}", pong_count);
            println!("   Subscription ID:   {:?}", subscription_id);
            last_stats_time = std::time::Instant::now();
        }

        // Wait for next message with timeout
        match timeout(Duration::from_millis(500), ws_receiver.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                message_count += 1;
                let timestamp = Utc::now().format("%H:%M:%S%.3f");

                // Try to parse as structured response
                match serde_json::from_str::<WebSocketResponse>(&text) {
                    Ok(resp) => {
                        if resp.method.as_deref() == Some("logsNotification") {
                            notification_count += 1;
                            println!("\n🔔 [{}] LOG NOTIFICATION RECEIVED!", timestamp);
                            match serde_json::to_string_pretty(&resp) {
                                Ok(pretty) => println!("{}", pretty),
                                Err(_) => println!("{}", text),
                            }

                            // Extract signature if available
                            if let Some(params) = resp.params {
                                if let Some(result) = params.get("result") {
                                    if
                                        let Some(signature) = result
                                            .get("value")
                                            .and_then(|v| v.get("signature"))
                                    {
                                        println!("\n   🎯 Transaction Signature: {}", signature);
                                    }
                                }
                            }
                        } else if resp.result.is_some() {
                            println!("\n📬 [{}] Response message:", timestamp);
                            match serde_json::to_string_pretty(&resp) {
                                Ok(pretty) => println!("{}", pretty),
                                Err(_) => println!("{}", text),
                            }
                        } else {
                            println!("\n📨 [{}] Unknown message:", timestamp);
                            println!("{}", text);
                        }
                    }
                    Err(_) => {
                        println!("\n📨 [{}] Raw message:", timestamp);
                        println!("{}", text);
                    }
                }
            }
            Ok(Some(Ok(Message::Ping(payload)))) => {
                // Respond to server ping
                if let Err(e) = ws_sender.send(Message::Pong(payload)).await {
                    print_step(&format!("Failed to respond to ping: {}", e), "ERROR");
                    break;
                }
                println!("🏓 Server pinged, responded with pong");
            }
            Ok(Some(Ok(Message::Pong(_)))) => {
                pong_count += 1;
                // Connection is alive - silent success
            }
            Ok(Some(Ok(Message::Close(frame)))) => {
                print_step(&format!("Server closed connection: {:?}", frame), "WARNING");
                break;
            }
            Ok(Some(Ok(msg))) => {
                println!("\n❓ Unexpected message type: {:?}", msg);
            }
            Ok(Some(Err(e))) => {
                print_step(&format!("WebSocket error: {}", e), "ERROR");
                break;
            }
            Ok(None) => {
                print_step("Connection closed", "WARNING");
                break;
            }
            Err(_) => {
                // Timeout - normal, continue
            }
        }
    }

    // Final Statistics
    print_header("📊 FINAL STATISTICS");

    println!("Test Duration:        {}s", start_time.elapsed().as_secs());
    println!("Messages Received:    {}", message_count);
    println!("Notifications:        {}", notification_count);
    println!("Pings Sent:           {}", ping_count);
    println!("Pongs Received:       {}", pong_count);
    println!("Subscription ID:      {:?}", subscription_id);

    println!();

    // Send close message
    print_step("Closing connection", "RUNNING");
    if let Err(e) = ws_sender.send(Message::Close(None)).await {
        print_step(&format!("Failed to send close: {}", e), "WARNING");
    } else {
        print_step("Connection closed gracefully", "SUCCESS");
    }

    println!();
    print_header("🎯 DIAGNOSIS");

    if subscription_id.is_some() {
        println!("✅ WebSocket connection: WORKING");
        println!("✅ Subscription:         CONFIRMED");

        if notification_count > 0 {
            println!("✅ Notifications:        RECEIVED ({} notifications)", notification_count);
            println!("\n🎉 WebSocket is fully operational!");
        } else {
            println!("⚠️  Notifications:        NONE RECEIVED");
            println!("\n🔍 This means:");
            println!("   - WebSocket connection works");
            println!("   - Subscription was accepted");
            println!("   - But no transactions occurred during test");
            println!("\n💡 To verify:");
            println!("   1. Run this tool again with --duration 60");
            println!("   2. Send a test transaction to your wallet");
            println!("   3. You should see a notification appear");
        }
    } else {
        println!("❌ WebSocket connection: UNKNOWN");
        println!("❌ Subscription:         FAILED");
        println!("\n🔍 Review the errors above to diagnose the issue");
    }

    println!("\n{}\n", "=".repeat(80));
}
