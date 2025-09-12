/// Debug tool for testing chain-based pool discovery
///
/// This tool discovers pools for a given token by scanning directly on-chain
/// without using external APIs like DexScreener. It searches through all major
/// DEX program accounts to find pools containing the specified token.
///
/// Usage Examples:
/// cargo run --bin debug_chain_pool_discovery -- --token cZrk3wMM36vxDALBRJwhSsbqkr5KM7MvvTazM4Gdaos
/// cargo run --bin debug_chain_pool_discovery -- --token <MINT> --program raydium-cpmm
/// cargo run --bin debug_chain_pool_discovery -- --token <MINT> --dry-run

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::pools::chain_discovery::{ ChainPoolDiscovery, ChainPoolInfo };
use screenerbot::pools::types::ProgramKind;
use screenerbot::rpc::get_rpc_client;
use screenerbot::logger::{ log, LogTag };
use std::str::FromStr;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "debug_chain_pool_discovery",
    about = "Test chain-based pool discovery for a token"
)]
struct Args {
    /// Token mint address to discover pools for
    #[arg(short, long)]
    token: String,

    /// Specific program to scan (optional)
    /// Valid values: raydium-cpmm, raydium-legacy, raydium-clmm, orca-whirlpool,
    /// meteora-dlmm, meteora-damm, pumpfun-amm, pumpfun-legacy, moonit
    #[arg(short, long)]
    program: Option<String>,

    /// Dry run mode - validate inputs without making RPC calls
    #[arg(long)]
    dry_run: bool,

    /// Enable debug logging for RPC calls
    #[arg(long)]
    debug_rpc: bool,

    /// Timeout for RPC calls in seconds (default: 120)
    #[arg(long, default_value = "120")]
    timeout: u64,

    /// Maximum pools to discover per program (default: 100)
    #[arg(long, default_value = "100")]
    max_pools: usize,
}

fn parse_program_kind(program: &str) -> Option<ProgramKind> {
    match program.to_lowercase().as_str() {
        "raydium-cpmm" => Some(ProgramKind::RaydiumCpmm),
        "raydium-legacy" => Some(ProgramKind::RaydiumLegacyAmm),
        "raydium-clmm" => Some(ProgramKind::RaydiumClmm),
        "orca-whirlpool" => Some(ProgramKind::OrcaWhirlpool),
        "meteora-dlmm" => Some(ProgramKind::MeteoraDlmm),
        "meteora-damm" => Some(ProgramKind::MeteoraDamm),
        "meteora-dbc" => Some(ProgramKind::MeteoraDbc),
        "pumpfun-amm" => Some(ProgramKind::PumpFunAmm),
        "pumpfun-legacy" => Some(ProgramKind::PumpFunLegacy),
        "moonit" => Some(ProgramKind::Moonit),
        _ => None,
    }
}

fn validate_token_mint(token_mint: &str) -> Result<(), String> {
    solana_sdk::pubkey::Pubkey
        ::from_str(token_mint)
        .map_err(|e| format!("Invalid token mint address: {}", e))?;
    Ok(())
}

fn print_pool_summary(pools: &[ChainPoolInfo]) {
    if pools.is_empty() {
        println!("🔍 No pools discovered");
        return;
    }

    println!("\n📊 DISCOVERY SUMMARY");
    println!("═══════════════════════════════════════════════════════════════");
    println!("Total pools found: {}", pools.len());

    // Group by program kind
    let mut by_program = std::collections::HashMap::new();
    for pool in pools {
        *by_program.entry(pool.program_kind).or_insert(0usize) += 1;
    }

    println!("\nBy DEX Program:");
    for (program, count) in by_program {
        println!("  {} → {} pools", program.display_name(), count);
    }

    println!("\n🏊 POOL DETAILS");
    println!("═══════════════════════════════════════════════════════════════");

    for (i, pool) in pools.iter().enumerate() {
        println!("{}. {}", i + 1, pool.program_kind.display_name());
        println!("   Address: {}", pool.address);
        println!("   Token A: {}", pool.token_a);
        println!("   Token B: {}", pool.token_b);
        println!("   Data Size: {} bytes", pool.account_data.len());
        println!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Set up debug flags
    let mut debug_args = vec!["debug_chain_pool_discovery".to_string()];
    if args.debug_rpc {
        debug_args.push("--debug-rpc".to_string());
    }
    // Always enable pool discovery debug for this tool
    debug_args.push("--debug-pool-discovery".to_string());
    set_cmd_args(debug_args);

    log(LogTag::System, "INFO", "🔍 Chain Pool Discovery Debug Tool");
    log(LogTag::System, "INFO", &format!("Target token: {}", args.token));

    // Validate token mint
    if let Err(e) = validate_token_mint(&args.token) {
        eprintln!("❌ Error: {}", e);
        std::process::exit(1);
    }

    if args.dry_run {
        println!("✅ Dry run mode - token mint is valid");
        if let Some(program) = &args.program {
            if let Some(program_kind) = parse_program_kind(program) {
                println!(
                    "✅ Program filter: {} ({})",
                    program_kind.display_name(),
                    program_kind.program_id()
                );
            } else {
                eprintln!("❌ Invalid program: {}", program);
                std::process::exit(1);
            }
        }
        println!("✅ All validations passed");
        return Ok(());
    }

    // Initialize RPC client
    log(LogTag::System, "INFO", "🔌 Initializing RPC client...");
    let rpc_client_ref = get_rpc_client();
    let rpc_urls = rpc_client_ref.get_all_urls();
    let rpc_client = Arc::new(
        screenerbot::rpc::RpcClient
            ::new_with_urls(rpc_urls)
            .map_err(|e| format!("Failed to create RPC client: {}", e))?
    );

    // Create chain discovery service
    let discovery = ChainPoolDiscovery::new(rpc_client);

    // Discover pools
    log(LogTag::System, "INFO", "🔍 Starting chain pool discovery...");

    let discovered_pools = if let Some(program) = &args.program {
        if let Some(program_kind) = parse_program_kind(program) {
            log(
                LogTag::System,
                "INFO",
                &format!("🎯 Scanning only {} program", program_kind.display_name())
            );

            // Scan specific program only
            match
                discovery.scan_program_for_token(
                    program_kind,
                    program_kind.program_id(),
                    &args.token
                ).await
            {
                Ok(pools) => pools,
                Err(e) => {
                    eprintln!("❌ Error scanning {}: {}", program_kind.display_name(), e);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("❌ Invalid program: {}", program);
            std::process::exit(1);
        }
    } else {
        // Discover across all programs
        match discovery.discover_pools_for_token(&args.token).await {
            Ok(pools) => pools,
            Err(e) => {
                eprintln!("❌ Discovery error: {}", e);
                std::process::exit(1);
            }
        }
    };

    // Limit results if requested
    let limited_pools: Vec<_> = discovered_pools.into_iter().take(args.max_pools).collect();

    // Print results
    print_pool_summary(&limited_pools);

    // Convert to PoolDescriptor format for testing
    log(LogTag::System, "INFO", "🔄 Converting to PoolDescriptor format...");
    match discovery.convert_to_pool_descriptors(limited_pools.clone()) {
        Ok(descriptors) => {
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "✅ Successfully converted {} pools to PoolDescriptor format",
                    descriptors.len()
                )
            );

            // Show sample descriptor details
            if let Some(first_descriptor) = descriptors.first() {
                println!("\n📋 SAMPLE POOL DESCRIPTOR");
                println!("═══════════════════════════════════════════════════════════════");
                println!("Pool ID: {}", first_descriptor.pool_id);
                println!("Program: {}", first_descriptor.program_kind.display_name());
                println!("Base Mint: {}", first_descriptor.base_mint);
                println!("Quote Mint: {}", first_descriptor.quote_mint);
                println!(
                    "Reserve Accounts: {} (populated by analyzer)",
                    first_descriptor.reserve_accounts.len()
                );
                println!(
                    "Liquidity USD: ${:.2} (requires price calc)",
                    first_descriptor.liquidity_usd
                );
                println!(
                    "Volume 24h USD: ${:.2} (not available from chain)",
                    first_descriptor.volume_h24_usd
                );
            }
        }
        Err(e) => {
            eprintln!("❌ Error converting to PoolDescriptor: {}", e);
            std::process::exit(1);
        }
    }

    log(LogTag::System, "INFO", "✅ Chain pool discovery completed successfully");

    Ok(())
}
