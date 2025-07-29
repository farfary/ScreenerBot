/// Pool Price Calculator Tool
///
/// This tool calculates token prices directly from Solana pool reserves.
/// It supports different pool program IDs with dedicated decoders for each.
///
/// Usage: cargo run --bin tool_pool_price -- --pool <POOL_ADDRESS> --token <TOKEN_MINT>

use screenerbot::tokens::pool::{ PoolPriceCalculator, get_pool_price_from_address };
use screenerbot::tokens::api::DexScreenerApi;
use screenerbot::logger::{ log, LogTag, init_file_logging };
use clap::{ Arg, Command };
use std::process;

#[tokio::main]
async fn main() {
    // Initialize logger
    init_file_logging();

    let matches = Command::new("Pool Price Calculator")
        .version("1.0")
        .about("Calculate token prices from Solana pool reserves")
        .arg(
            Arg::new("pool")
                .short('p')
                .long("pool")
                .value_name("POOL_ADDRESS")
                .help("Pool address to calculate price from")
                .required(true)
        )
        .arg(
            Arg::new("token")
                .short('t')
                .long("token")
                .value_name("TOKEN_MINT")
                .help("Token mint address")
                .required(true)
        )
        .arg(
            Arg::new("rpc")
                .short('r')
                .long("rpc")
                .value_name("RPC_URL")
                .help("Custom RPC URL (optional)")
                .required(false)
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug output")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    let pool_address = matches.get_one::<String>("pool").unwrap();
    let token_mint = matches.get_one::<String>("token").unwrap();
    let rpc_url = matches.get_one::<String>("rpc");
    let debug = matches.get_flag("debug");

    log(LogTag::System, "START", "Pool Price Calculator Tool");
    log(LogTag::System, "INFO", &format!("Pool Address: {}", pool_address));
    log(LogTag::System, "INFO", &format!("Token Mint: {}", token_mint));

    // Initialize pool calculator
    let mut calculator = match PoolPriceCalculator::new_with_rpc(rpc_url).await {
        Ok(calc) => calc,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to initialize pool calculator: {}", e));
            process::exit(1);
        }
    };

    // Enable debug mode if requested
    if debug {
        calculator.enable_debug();
    }

    // Calculate price from pool
    match get_pool_price_from_address(pool_address, token_mint).await {
        Ok(Some(price_info)) => {
            log(LogTag::System, "SUCCESS", "Pool price calculation completed!");

            println!("\n=== POOL PRICE CALCULATION RESULTS ===");
            println!("Pool Address: {}", pool_address);
            println!("Token Mint: {}", token_mint);
            println!("Pool Program ID: {}", price_info.pool_program_id);
            println!("Pool Type: {}", price_info.pool_type);
            println!("Token Price: {:.12} SOL", price_info.price_sol);
            println!("Token Reserve: {}", price_info.token_reserve);
            println!("SOL Reserve: {}", price_info.sol_reserve);
            println!("Liquidity (USD): ${:.2}", price_info.liquidity_usd.unwrap_or(0.0));

            // Fetch price from DexScreener API for comparison
            log(LogTag::System, "API", "Fetching price from DexScreener API...");
            let mut api = DexScreenerApi::new();

            match api.initialize().await {
                Ok(_) => {
                    // First try to get detailed token data for debugging
                    match api.get_token_data(token_mint).await {
                        Ok(Some(token_data)) => {
                            if let Some(api_price) = token_data.price_sol {
                                println!("\n=== API PRICE COMPARISON ===");
                                println!("DexScreener API Price: {:.12} SOL", api_price);
                                println!("Pool Calculator Price: {:.12} SOL", price_info.price_sol);

                                let difference = (
                                    ((price_info.price_sol - api_price) / api_price) *
                                    100.0
                                ).abs();
                                println!("Price Difference: {:.4}%", difference);

                                if difference < 1.0 {
                                    println!("✅ Prices match closely (< 1% difference)");
                                } else if difference < 5.0 {
                                    println!("⚠️  Moderate price difference (1-5%)");
                                } else {
                                    println!("❌ Significant price difference (> 5%)");
                                }

                                // Calculate which price is higher
                                if price_info.price_sol > api_price {
                                    let premium =
                                        ((price_info.price_sol - api_price) / api_price) * 100.0;
                                    println!("Pool price is {:.4}% higher than API price", premium);
                                } else {
                                    let discount =
                                        ((api_price - price_info.price_sol) / api_price) * 100.0;
                                    println!("Pool price is {:.4}% lower than API price", discount);
                                }

                                if debug {
                                    println!("\n=== API DEBUG INFO ===");
                                    println!("Token Symbol: {}", token_data.symbol);
                                    println!("Token Name: {}", token_data.name);
                                    println!("Pair Address: {}", token_data.pair_address);
                                    println!("DEX ID: {}", token_data.dex_id);
                                    if token_data.price_usd > 0.0 {
                                        println!("Price USD: ${:.12}", token_data.price_usd);
                                    }
                                    if let Some(liquidity) = &token_data.liquidity {
                                        if let Some(liq_usd) = liquidity.usd {
                                            println!("API Liquidity: ${:.2}", liq_usd);
                                        }
                                    }
                                }
                            } else {
                                println!("\n=== API PRICE COMPARISON ===");
                                println!(
                                    "⚠️  Token found in DexScreener but no SOL price available"
                                );
                                println!("Pool Calculator Price: {:.12} SOL", price_info.price_sol);

                                if debug {
                                    println!("\n=== API DEBUG INFO ===");
                                    println!("Token Symbol: {}", token_data.symbol);
                                    println!("Token Name: {}", token_data.name);
                                    println!("Pair Address: {}", token_data.pair_address);
                                    println!("DEX ID: {}", token_data.dex_id);
                                    println!("Price Native: {:.12}", token_data.price_native);
                                    if token_data.price_usd > 0.0 {
                                        println!("Price USD: ${:.12}", token_data.price_usd);
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            println!("\n=== API PRICE COMPARISON ===");
                            println!("ℹ️  Token not found in DexScreener database");
                            println!("Pool Calculator Price: {:.12} SOL", price_info.price_sol);
                            println!(
                                "This could be a very new token or one not tracked by DexScreener"
                            );
                        }
                        Err(e) => {
                            println!("\n=== API PRICE COMPARISON ===");
                            println!("❌ Error fetching token data from DexScreener: {}", e);
                            println!("Pool Calculator Price: {:.12} SOL", price_info.price_sol);
                        }
                    }
                }
                Err(e) => {
                    println!("\n=== API PRICE COMPARISON ===");
                    println!("❌ Failed to initialize DexScreener API: {}", e);
                    println!("Pool Calculator Price: {:.12} SOL", price_info.price_sol);
                }
            }

            if debug {
                println!("\n=== DEBUG INFO ===");
                println!("Token Decimals: {}", price_info.token_decimals);
                println!("SOL Decimals: {}", price_info.sol_decimals);
                println!("Raw Token Reserve: {}", price_info.token_reserve);
                println!("Raw SOL Reserve: {}", price_info.sol_reserve);

                // Add hex and raw account data view
                match calculator.get_raw_pool_data(pool_address).await {
                    Ok(Some(raw_data)) => {
                        println!("\n=== RAW POOL DATA ===");
                        println!("Account Data Size: {} bytes", raw_data.len());

                        // Show first 256 bytes in hex format
                        let hex_data = raw_data
                            .iter()
                            .take(256)
                            .map(|b| format!("{:02x}", b))
                            .collect::<Vec<String>>()
                            .join(" ");

                        println!("First 256 bytes (hex):");
                        // Format hex in 16-byte rows
                        for (i, chunk) in hex_data
                            .split_whitespace()
                            .collect::<Vec<&str>>()
                            .chunks(16)
                            .enumerate() {
                            println!("{:04x}: {}", i * 16, chunk.join(" "));
                        }

                        // Show raw bytes as ASCII where printable
                        println!("\nASCII view (first 256 bytes, '.' for non-printable):");
                        let ascii_data = raw_data
                            .iter()
                            .take(256)
                            .map(|&b| {
                                if b >= 32 && b <= 126 { b as char } else { '.' }
                            })
                            .collect::<String>();

                        // Format ASCII in 64-char rows
                        for (i, chunk) in ascii_data
                            .chars()
                            .collect::<Vec<char>>()
                            .chunks(64)
                            .enumerate() {
                            println!("{:04x}: {}", i * 64, chunk.iter().collect::<String>());
                        }

                        // Show specific offset data for Raydium CPMM
                        if price_info.pool_type == "Raydium CPMM" {
                            println!("\n=== RAYDIUM CPMM STRUCTURE ===");
                            if raw_data.len() >= 312 {
                                // Minimum size for Raydium CPMM
                                println!(
                                    "Pool Creator (offset 40): {:?}",
                                    &raw_data[40..72]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join("")
                                );
                                println!(
                                    "Token 0 Vault (offset 72): {:?}",
                                    &raw_data[72..104]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join("")
                                );
                                println!(
                                    "Token 1 Vault (offset 104): {:?}",
                                    &raw_data[104..136]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join("")
                                );
                                println!(
                                    "LP Mint (offset 136): {:?}",
                                    &raw_data[136..168]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join("")
                                );
                                println!(
                                    "Token 0 Mint (offset 168): {:?}",
                                    &raw_data[168..200]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join("")
                                );
                                println!(
                                    "Token 1 Mint (offset 200): {:?}",
                                    &raw_data[200..232]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<String>>()
                                        .join("")
                                );

                                // Decimals at offset 264-266
                                if raw_data.len() > 266 {
                                    println!("LP Mint Decimals (offset 264): {}", raw_data[264]);
                                    println!("Token 0 Decimals (offset 265): {}", raw_data[265]);
                                    println!("Token 1 Decimals (offset 266): {}", raw_data[266]);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        println!("\n=== RAW POOL DATA ===");
                        println!("No raw pool data available");
                    }
                    Err(e) => {
                        println!("\n=== RAW POOL DATA ===");
                        println!("Error fetching raw pool data: {}", e);
                    }
                }
            }
        }
        Ok(None) => {
            log(LogTag::System, "WARN", "No price data available for this pool");
            println!("No price data available for pool: {}", pool_address);
            process::exit(1);
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Pool price calculation failed: {}", e));
            println!("Error: {}", e);
            process::exit(1);
        }
    }

    log(LogTag::System, "FINISH", "Pool Price Calculator Tool completed");
}
