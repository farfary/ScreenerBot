/// Test initialization flow - mimics the exact flow from webserver initialization
///
/// This tests the complete validation flow that happens
/// during bot initialization via the web UI.
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
    let private_key_str = "YOUR_WALLET_PRIVATE_KEY_BASE58_HERE";
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

    let _wallet_address = keypair.pubkey();

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
    // FINAL RESULT
    // ============================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ INITIALIZATION WOULD SUCCEED");
    println!();
    println!("   The bot would successfully initialize with these credentials.");
    println!("   Wallet is valid and RPC endpoints are working.");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Parse wallet private key from base58 string
fn parse_wallet_private_key(key: &str) -> Result<Keypair, String> {
    // Try base58 first
    match Keypair::from_base58_string(key) {
        kp => return Ok(kp),
    }
}
