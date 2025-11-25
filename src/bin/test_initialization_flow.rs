/// Test initialization flow - mimics the exact flow from webserver initialization
///
/// This tests the complete validation + license verification flow that happens
/// during bot initialization via the web UI.
use screenerbot::license;
use screenerbot::logger;
use screenerbot::rpc;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() {
    // Initialize logger with debug enabled
    logger::init();

    println!("🔐 ScreenerBot Initialization Flow Test");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Test credentials - replace with your actual credentials
    let private_key_str =
        "YOUR_WALLET_PRIVATE_KEY_BASE58_HERE";
    let rpc_urls = vec![
        "https://mainnet.helius-rpc.com/?api-key=YOUR_API_KEY_HERE".to_string(),
    ];

    println!("📋 Test Configuration:");
    println!(
        "   Private Key: {}...{}",
        &private_key_str[..10],
        &private_key_str[private_key_str.len() - 10..]
    );
    println!("   RPC URLs: {} endpoint(s)", rpc_urls.len());
    for (i, url) in rpc_urls.iter().enumerate() {
        let safe_url = if url.contains("api-key=") {
            url.split("api-key=").next().unwrap().to_string() + "api-key=***"
        } else {
            url.clone()
        };
        println!("      [{}] {}", i + 1, safe_url);
    }
    println!();

    // ============================================================================
    // STEP 1: Validate Wallet Private Key
    // ============================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 1: Validate Wallet Private Key");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let keypair = match parse_wallet_private_key(private_key_str) {
        Ok(kp) => {
            println!("✅ Wallet private key valid");
            println!("   Public key: {}", kp.pubkey());
            println!();
            kp
        }
        Err(e) => {
            eprintln!("❌ Invalid wallet private key: {}", e);
            return;
        }
    };

    let wallet_address = keypair.pubkey();

    // ============================================================================
    // STEP 2: Test RPC Endpoints
    // ============================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 2: Test RPC Endpoints");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("⏱️  Starting RPC endpoint tests (this may take 30+ seconds)...");
    println!();

    let start = std::time::Instant::now();
    let rpc_test_results = rpc::test_rpc_endpoints(&rpc_urls).await;
    let duration = start.elapsed();

    println!("⏱️  RPC tests completed in {:.2}s", duration.as_secs_f64());
    println!();

    // Print detailed results
    for (i, result) in rpc_test_results.iter().enumerate() {
        println!("Endpoint [{}]:", i + 1);
        let safe_url = if result.url.contains("api-key=") {
            result.url.split("api-key=").next().unwrap().to_string() + "api-key=***"
        } else {
            result.url.clone()
        };
        println!("   URL: {}", safe_url);
        println!(
            "   Status: {}",
            if result.success {
                "✅ Success"
            } else {
                "❌ Failed"
            }
        );

        if result.success {
            println!("   Latency: {:.0}ms", result.latency_ms);
            println!(
                "   Premium: {}",
                if result.is_premium {
                    "✅ Yes"
                } else {
                    "⚠️  No"
                }
            );
            if let Some(is_mainnet) = result.is_mainnet {
                println!(
                    "   Mainnet: {}",
                    if is_mainnet { "✅ Yes" } else { "⚠️  No" }
                );
            }
        } else {
            if let Some(error) = &result.error {
                println!("   Error: {}", error);
            }
        }
        println!();
    }

    // Check if any endpoints succeeded
    let working_endpoints: Vec<_> = rpc_test_results
        .iter()
        .filter(|r| r.success)
        .map(|r| r.url.clone())
        .collect();

    if working_endpoints.is_empty() {
        eprintln!("❌ No working RPC endpoints found");
        eprintln!("   All RPC tests failed!");
        return;
    }

    println!(
        "✅ {} of {} RPC endpoint(s) working",
        working_endpoints.len(),
        rpc_urls.len()
    );
    println!();

    // ============================================================================
    // STEP 3: Verify License
    // ============================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 3: Verify License");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!(
        "🎫 Verifying ScreenerBot license for wallet: {}",
        wallet_address
    );
    println!(
        "   Using {} working RPC endpoint(s)",
        working_endpoints.len()
    );
    println!();

    let start = std::time::Instant::now();
    let license_status = match license::verify_license_for_wallet_with_endpoints(
        &wallet_address,
        &working_endpoints,
    )
    .await
    {
        Ok(status) => {
            let duration = start.elapsed();
            println!(
                "⏱️  License verification completed in {:.2}s",
                duration.as_secs_f64()
            );
            println!();
            status
        }
        Err(e) => {
            eprintln!("❌ License verification failed: {}", e);
            return;
        }
    };

    // Print license status
    println!("📊 License Status:");
    println!(
        "   Valid: {}",
        if license_status.valid {
            "✅ Yes"
        } else {
            "❌ No"
        }
    );

    if let Some(tier) = &license_status.tier {
        println!("   Tier: {}", tier);
    }

    if let Some(mint) = &license_status.mint {
        println!("   NFT Mint: {}", mint);
        println!("   Solscan: https://solscan.io/token/{}", mint);
    }

    if let Some(start_ts) = license_status.start_ts {
        let start_time = chrono::DateTime::from_timestamp(start_ts as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| format!("{}", start_ts));
        println!("   Start: {}", start_time);
    }

    if let Some(expiry_ts) = license_status.expiry_ts {
        let expiry_time = chrono::DateTime::from_timestamp(expiry_ts as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| format!("{}", expiry_ts));
        println!("   Expiry: {}", expiry_time);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let remaining = expiry_ts.saturating_sub(now);
        let days_remaining = remaining / 86400;
        println!("   Remaining: {} days", days_remaining);
    }

    if let Some(reason) = &license_status.reason {
        println!("   Reason: {}", reason);
    }

    println!();

    // ============================================================================
    // FINAL RESULT
    // ============================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if license_status.valid {
        println!("✅ INITIALIZATION WOULD SUCCEED");
        println!();
        println!("   The bot would successfully initialize with these credentials.");
        println!("   License is valid and all checks passed.");
    } else {
        println!("❌ INITIALIZATION WOULD FAIL");
        println!();
        println!(
            "   Reason: {}",
            license_status
                .reason
                .unwrap_or_else(|| "Unknown".to_string())
        );
        println!();
        println!("   The bot cannot start without a valid license.");
        println!("   Visit https://screenerbot.io to purchase a license.");
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Parse wallet private key from base58 string
fn parse_wallet_private_key(key: &str) -> Result<Keypair, String> {
    // Try base58 first
    match Keypair::from_base58_string(key) {
        kp => return Ok(kp),
    }
}
