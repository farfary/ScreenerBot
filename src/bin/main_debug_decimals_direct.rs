use screenerbot::rpc::get_rpc_client;
use std::env;
use tokio;

// Import Solana dependencies for direct blockchain access
use solana_sdk::pubkey::Pubkey;
use solana_program::program_pack::Pack;
use spl_token::state::Mint;
use std::str::FromStr;

/// Direct decimals extraction debugging tool - bypasses all caching
///
/// Usage:
/// --test-direct <mint> : Test single token with full extraction debugging
/// --test-batch <mints> : Test multiple tokens with full debugging
/// --compare <mint>     : Compare with GeckoTerminal API

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize RPC client
    use screenerbot::rpc::init_rpc_client;
    if let Err(e) = init_rpc_client() {
        eprintln!("❌ Failed to initialize RPC client: {}", e);
        return Err(e.into());
    }

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "--test-direct" => {
            if args.len() < 3 {
                eprintln!("❌ Please provide mint address");
                return Ok(());
            }
            test_direct_extraction(&args[2]).await?;
        }
        "--test-batch" => {
            if args.len() < 3 {
                eprintln!("❌ Please provide comma-separated mint addresses");
                return Ok(());
            }
            let mints: Vec<String> = args[2]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            for mint in &mints {
                println!("\n{}", "═".repeat(80));
                test_direct_extraction(mint).await?;
            }
        }
        "--compare" => {
            if args.len() < 3 {
                eprintln!("❌ Please provide mint address");
                return Ok(());
            }
            compare_with_geckoterminal(&args[2]).await?;
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("🔍 ScreenerBot Direct Decimals Extraction Tool");
    println!("{}", "═".repeat(60));
    println!("Usage: cargo run --bin main_debug_decimals_direct [OPTION] <MINT>");
    println!();
    println!("Options:");
    println!("  --test-direct <mint>  Test single token with full debugging");
    println!("  --test-batch <mints>  Test multiple tokens (comma-separated)");
    println!("  --compare <mint>      Compare with GeckoTerminal API");
    println!();
    println!("Examples:");
    println!(
        "  cargo run --bin main_debug_decimals_direct -- --test-direct So11111111111111111111111111111111111111112"
    );
    println!("  cargo run --bin main_debug_decimals_direct -- --test-batch \"MINT1,MINT2\"");
    println!(
        "  cargo run --bin main_debug_decimals_direct -- --compare bv2Rv7uyiEQxxjjsLxABjcw6mH8XzUDnm5oNuZDpump"
    );
}

/// Directly extract decimals from blockchain with comprehensive debugging
async fn test_direct_extraction(mint_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 DIRECT DECIMALS EXTRACTION DEBUG");
    println!("{}", "═".repeat(60));
    println!("🎯 Token: {}", mint_str);

    // Step 1: Parse mint address
    println!("\n📝 Step 1: Parsing mint address...");
    let mint_pubkey = match Pubkey::from_str(mint_str) {
        Ok(pubkey) => {
            println!("  ✅ Successfully parsed mint pubkey: {}", pubkey);
            pubkey
        }
        Err(e) => {
            println!("  ❌ Failed to parse mint address: {}", e);
            return Err(e.into());
        }
    };

    // Step 2: Get account info from blockchain
    println!("\n📡 Step 2: Fetching account info from blockchain...");
    let rpc_client = get_rpc_client();

    let account_info = match rpc_client.get_account(&mint_pubkey).await {
        Ok(account) => {
            println!("  ✅ Account found on blockchain");
            println!("  📊 Account Details:");
            println!("     💰 Lamports: {}", account.lamports);
            println!("     📦 Data length: {} bytes", account.data.len());
            println!("     👤 Owner: {}", account.owner);
            println!("     🏠 Executable: {}", account.executable);
            println!("     🏷️ Rent Epoch: {}", account.rent_epoch);

            // Show raw data in hex (first 100 bytes)
            let hex_data = account.data
                .iter()
                .take(100)
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" ");
            println!("     🔢 Raw data (first 100 bytes): {}", hex_data);

            account
        }
        Err(e) => {
            println!("  ❌ RPC error fetching account: {}", e);
            return Err(e.into());
        }
    };

    // Step 3: Determine token program type
    println!("\n🔍 Step 3: Determining token program type...");
    let spl_token_program = Pubkey::from_str(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    ).unwrap();
    let spl_token_2022_program = Pubkey::from_str(
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    ).unwrap();

    println!("  🔍 Account owner: {}", account_info.owner);
    println!("  🔍 SPL Token Program: {}", spl_token_program);
    println!("  🔍 SPL Token-2022 Program: {}", spl_token_2022_program);

    if account_info.owner == spl_token_program {
        println!("  ✅ Identified as SPL Token (original)");
        extract_spl_token_decimals(&account_info.data, mint_str).await?;
    } else if account_info.owner == spl_token_2022_program {
        println!("  ✅ Identified as SPL Token-2022");
        extract_spl_token_2022_decimals(&account_info.data, mint_str).await?;
    } else {
        println!("  ❌ Unknown token program owner: {}", account_info.owner);
        println!("  ℹ️  Expected one of:");
        println!("     - SPL Token: {}", spl_token_program);
        println!("     - SPL Token-2022: {}", spl_token_2022_program);
        return Err("Unknown token program".into());
    }

    Ok(())
}

/// Extract decimals from SPL Token (original) mint data
async fn extract_spl_token_decimals(
    data: &[u8],
    mint_str: &str
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧬 Step 4: Extracting SPL Token decimals...");

    // Check minimum data length for SPL Token mint
    println!("  📏 Data length: {} bytes", data.len());
    if data.len() < Mint::LEN {
        println!(
            "  ❌ Data too short for SPL Token mint (expected {} bytes, got {})",
            Mint::LEN,
            data.len()
        );
        return Err("Invalid SPL Token mint data length".into());
    }

    println!("  ✅ Data length is sufficient for SPL Token mint");

    // Parse the mint data using solana program pack
    println!("  🔄 Attempting to unpack mint data...");
    match Mint::unpack(data) {
        Ok(mint) => {
            println!("  ✅ Successfully unpacked SPL Token mint data");
            println!("  📊 Mint Details:");
            println!("     🔢 Decimals: {}", mint.decimals);
            println!("     💰 Supply: {}", mint.supply);
            println!("     🎯 Mint Authority: {:?}", mint.mint_authority);
            println!("     ❄️  Freeze Authority: {:?}", mint.freeze_authority);
            println!("     🔄 Is Initialized: {}", mint.is_initialized);

            println!("\n🎉 EXTRACTION SUCCESSFUL!");
            println!("  Token: {}", mint_str);
            println!("  Decimals: {}", mint.decimals);
        }
        Err(e) => {
            println!("  ❌ Failed to unpack SPL Token mint data: {}", e);

            // Try manual parsing for debugging
            println!("  🔍 Attempting manual parsing for debugging...");
            manual_parse_spl_token(data);

            return Err(e.into());
        }
    }

    Ok(())
}

/// Extract decimals from SPL Token-2022 mint data
async fn extract_spl_token_2022_decimals(
    data: &[u8],
    mint_str: &str
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧬 Step 4: Extracting SPL Token-2022 decimals...");

    // SPL Token-2022 has variable-length data due to extensions
    println!("  📏 Data length: {} bytes", data.len());

    // Basic mint data should be at least 82 bytes
    if data.len() < 82 {
        println!(
            "  ❌ Data too short for SPL Token-2022 mint (minimum 82 bytes, got {})",
            data.len()
        );
        return Err("Invalid SPL Token-2022 mint data length".into());
    }

    println!("  ✅ Data length is sufficient for SPL Token-2022 mint");

    // For Token-2022, we need to parse differently due to extensions
    // The basic mint data is in the first 82 bytes, same format as original SPL Token
    let basic_mint_data = &data[..Mint::LEN];

    println!("  🔄 Attempting to unpack basic mint data from Token-2022...");
    match Mint::unpack(basic_mint_data) {
        Ok(mint) => {
            println!("  ✅ Successfully unpacked SPL Token-2022 mint data");
            println!("  📊 Mint Details:");
            println!("     🔢 Decimals: {}", mint.decimals);
            println!("     💰 Supply: {}", mint.supply);
            println!("     🎯 Mint Authority: {:?}", mint.mint_authority);
            println!("     ❄️  Freeze Authority: {:?}", mint.freeze_authority);
            println!("     🔄 Is Initialized: {}", mint.is_initialized);

            // Check for extensions
            if data.len() > Mint::LEN {
                println!("     🔧 Has Extensions: {} extra bytes", data.len() - Mint::LEN);
                analyze_token_2022_extensions(&data[Mint::LEN..]);
            }

            println!("\n🎉 EXTRACTION SUCCESSFUL!");
            println!("  Token: {}", mint_str);
            println!("  Decimals: {}", mint.decimals);
        }
        Err(e) => {
            println!("  ❌ Failed to unpack SPL Token-2022 mint data: {}", e);

            // Try manual parsing for debugging
            println!("  🔍 Attempting manual parsing for debugging...");
            manual_parse_spl_token(basic_mint_data);

            return Err(e.into());
        }
    }

    Ok(())
}

/// Manual parsing for debugging purposes
fn manual_parse_spl_token(data: &[u8]) {
    println!("  🔍 Manual parsing analysis:");

    if data.len() >= 82 {
        // SPL Token mint structure:
        // 0-3: mint_authority option (4 bytes)
        // 4-35: mint_authority pubkey (32 bytes, if Some)
        // 36-43: supply (8 bytes, little endian)
        // 44: decimals (1 byte)
        // 45: is_initialized (1 byte)
        // 46-49: freeze_authority option (4 bytes)
        // 50-81: freeze_authority pubkey (32 bytes, if Some)

        let supply = u64::from_le_bytes([
            data[36],
            data[37],
            data[38],
            data[39],
            data[40],
            data[41],
            data[42],
            data[43],
        ]);
        let decimals = data[44];
        let is_initialized = data[45] != 0;

        println!("     📊 Manual parsing results:");
        println!("        💰 Supply: {}", supply);
        println!("        🔢 Decimals: {}", decimals);
        println!("        🔄 Initialized: {}", is_initialized);
        println!("        🔢 Raw decimals byte: 0x{:02x}", data[44]);

        // Show bytes around decimals field
        if data.len() > 50 {
            println!("        🔍 Bytes 40-50: {:?}", &data[40..=50]);
        }
    } else {
        println!("     ❌ Data too short for manual parsing");
    }
}

/// Analyze Token-2022 extensions
fn analyze_token_2022_extensions(extension_data: &[u8]) {
    println!("     🔧 Analyzing Token-2022 extensions:");
    println!("        📏 Extension data length: {} bytes", extension_data.len());

    // Show first few bytes of extension data
    if !extension_data.is_empty() {
        let hex_preview = extension_data
            .iter()
            .take(32)
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join(" ");
        println!("        🔢 Extension data preview: {}", hex_preview);
    }
}

/// Compare with GeckoTerminal API
async fn compare_with_geckoterminal(mint_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 COMPARISON WITH GECKOTERMINAL API");
    println!("{}", "═".repeat(60));
    println!("🎯 Token: {}", mint_str);

    // First, extract directly from blockchain
    println!("\n🔗 Extracting from blockchain...");
    let _blockchain_result = test_direct_extraction(mint_str).await;

    // Then fetch from GeckoTerminal
    println!("\n🦎 Fetching from GeckoTerminal API...");
    match fetch_geckoterminal_decimals(mint_str).await {
        Ok(gecko_decimals) => {
            println!("  ✅ GeckoTerminal decimals: {}", gecko_decimals);
        }
        Err(e) => {
            println!("  ❌ GeckoTerminal error: {}", e);
        }
    }

    Ok(())
}

/// Fetch decimals from GeckoTerminal API
async fn fetch_geckoterminal_decimals(mint_str: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let url = format!("https://api.geckoterminal.com/api/v2/networks/solana/tokens/{}", mint_str);

    println!("  🌐 Requesting: {}", url);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    println!("  📡 Response status: {}", response.status());

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    let json: serde_json::Value = response.json().await?;

    // Print full response for debugging
    println!("  📄 Full response: {}", serde_json::to_string_pretty(&json)?);

    // Extract decimals from response
    if let Some(decimals) = json["data"]["attributes"]["decimals"].as_u64() {
        Ok(decimals as u8)
    } else {
        Err("Could not find decimals in response".into())
    }
}
