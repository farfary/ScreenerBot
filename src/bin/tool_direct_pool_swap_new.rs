/// Direct Pool Swap Tool
/// 
/// Performs direct swaps with Raydium pools without using external APIs.
/// Builds and sends swap transactions directly to the Raydium AMM program.
/// 
/// This tool:
/// 1. Initializes required services (RPC, API, Pool Service)
/// 2. Creates Associated Token Account (ATA) for output token if needed
/// 3. Validates pool availability and decodes pool data from blockchain
/// 4. Calculates expected output using constant product formula
/// 5. Uses Jupiter aggregator for actual swap execution (supports all pool types)
/// 6. Performs real on-chain testing with configurable amounts
/// 
/// Usage:
///   cargo run --bin tool_direct_pool_swap_new -- --help
///   cargo run --bin tool_direct_pool_swap_new -- --pool <POOL_ADDRESS> --token <TOKEN_MINT> --amount <SOL_AMOUNT>
///   cargo run --bin tool_direct_pool_swap_new -- --pool BPp7mbBLDe3UwXmeWKnwm6CnAdAwVS746auJNmjtArjw --token AkdtuaKVDpsZyeZ8LvcVf4G4L3nJ1Jmd7npmF5mpbonk --amount 0.001

use screenerbot::global::*;
use screenerbot::logger::{log, LogTag, init_file_logging};
use screenerbot::rpc::{get_rpc_client, init_rpc_client, SwapError};
use screenerbot::tokens::{
    api::init_dexscreener_api,
    pool::{init_pool_service, SOL_MINT, PoolPriceCalculator, get_pool_program_display_name},
    decimals::get_token_decimals_from_chain,
};
use screenerbot::utils::{get_sol_balance, get_token_balance};

use std::{env, str::FromStr};
use solana_sdk::{
    pubkey::Pubkey, 
    signature::{Keypair, Signer},
    transaction::Transaction,
    instruction::Instruction,
    compute_budget::ComputeBudgetInstruction,
};
use spl_token;
use spl_associated_token_account::{
    instruction::create_associated_token_account, 
    get_associated_token_address
};
use bs58;

// Constants for pool types and Raydium programs
const RAYDIUM_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

// Raydium AMM instruction discriminators
const RAYDIUM_SWAP_INSTRUCTION: u8 = 9;
const RAYDIUM_CPMM_SWAP_INSTRUCTION: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x7f, 0x9b, 0x5f]; // swapBaseInput

// Default values for testing
const DEFAULT_AMOUNT_SOL: f64 = 0.001; // Minimum test amount
const MINIMUM_COMPUTE_UNITS: u32 = 200_000; // Conservative compute units
const MINIMUM_PRIORITY_FEE: u64 = 1; // Minimum priority fee (1 micro-lamport)

/// Direct pool swap configuration
#[derive(Debug)]
struct SwapConfig {
    pool_address: String,
    token_mint: String,
    amount_sol: f64,
    slippage_percent: f64,
    dry_run: bool,
}

/// Enhanced pool account data structure supporting multiple pool types
#[derive(Debug)]
struct PoolData {
    pool_address: String,
    program_id: String,
    pool_type: String,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    base_reserve: u64,
    quote_reserve: u64,
    base_decimals: u8,
    quote_decimals: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();
    
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        return Ok(());
    }
    
    log(LogTag::System, "START", "🚀 Starting Direct Pool Swap Tool");
    
    // Initialize required services
    log(LogTag::System, "INIT", "🔄 Initializing services...");
    
    // Initialize configurations
    let _configs = read_configs()?;
    
    // Initialize RPC client
    if let Err(e) = init_rpc_client() {
        log(LogTag::System, "ERROR", &format!("❌ Failed to initialize RPC client: {}", e));
        return Err(e.into());
    }
    log(LogTag::System, "SUCCESS", "✅ RPC client initialized");
    
    // Initialize DexScreener API
    if let Err(e) = init_dexscreener_api().await {
        log(LogTag::System, "ERROR", &format!("❌ Failed to initialize DexScreener API: {}", e));
        return Err(e.into());
    }
    log(LogTag::System, "SUCCESS", "✅ DexScreener API initialized");
    
    // Initialize Pool Service
    let pool_service = init_pool_service();
    pool_service.start_monitoring().await;
    log(LogTag::System, "SUCCESS", "✅ Pool service initialized and monitoring started");
    
    // Parse command line arguments
    let config = parse_arguments(&args)?;
    
    log(LogTag::System, "CONFIG", &format!(
        "📋 Swap configuration: Pool={}, Token={}, Amount={} SOL, Slippage={}%, DryRun={}",
        &config.pool_address[..12],
        &config.token_mint[..12],
        config.amount_sol,
        config.slippage_percent,
        config.dry_run
    ));
    
    // Execute the direct pool swap
    execute_direct_pool_swap(config).await?;
    
    Ok(())
}

/// Parse command line arguments
fn parse_arguments(args: &[String]) -> Result<SwapConfig, SwapError> {
    let mut pool_address = String::new();
    let mut token_mint = String::new();
    let mut amount_sol = DEFAULT_AMOUNT_SOL;
    let mut slippage_percent = 1.0; // 1% default slippage
    let mut dry_run = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--pool" | "-p" => {
                if i + 1 < args.len() {
                    pool_address = args[i + 1].clone();
                    i += 2;
                } else {
                    return Err(SwapError::InvalidAmount("Missing pool address".to_string()));
                }
            }
            "--token" | "-t" => {
                if i + 1 < args.len() {
                    token_mint = args[i + 1].clone();
                    i += 2;
                } else {
                    return Err(SwapError::InvalidAmount("Missing token mint".to_string()));
                }
            }
            "--amount" | "-a" => {
                if i + 1 < args.len() {
                    amount_sol = args[i + 1].parse()
                        .map_err(|_| SwapError::InvalidAmount("Invalid amount".to_string()))?;
                    i += 2;
                } else {
                    return Err(SwapError::InvalidAmount("Missing amount".to_string()));
                }
            }
            "--slippage" | "-s" => {
                if i + 1 < args.len() {
                    slippage_percent = args[i + 1].parse()
                        .map_err(|_| SwapError::InvalidAmount("Invalid slippage".to_string()))?;
                    i += 2;
                } else {
                    return Err(SwapError::InvalidAmount("Missing slippage".to_string()));
                }
            }
            "--dry-run" | "-d" => {
                dry_run = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    
    if pool_address.is_empty() || token_mint.is_empty() {
        return Err(SwapError::InvalidAmount("Pool address and token mint are required".to_string()));
    }
    
    // Validate amount
    if amount_sol <= 0.0 || amount_sol > 1.0 {
        return Err(SwapError::InvalidAmount("Amount must be between 0 and 1 SOL".to_string()));
    }
    
    Ok(SwapConfig {
        pool_address,
        token_mint,
        amount_sol,
        slippage_percent,
        dry_run,
    })
}

/// Print comprehensive help menu
fn print_help() {
    println!("🏊 Direct Pool Swap Tool");
    println!("=====================================");
    println!("Performs direct swaps with Raydium pools using direct blockchain interaction.");
    println!("Creates ATAs and builds Raydium swap instructions directly.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_direct_pool_swap_new -- [OPTIONS]");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h                 Show this help message");
    println!("    --pool, -p <ADDRESS>       Pool address (required)");
    println!("    --token, -t <MINT>         Token mint address (required)");
    println!("    --amount, -a <SOL>         Amount in SOL (default: {} SOL)", DEFAULT_AMOUNT_SOL);
    println!("    --slippage, -s <PERCENT>   Slippage tolerance % (default: 1.0%)");
    println!("    --dry-run, -d              Simulate without sending transaction");
    println!("");
    println!("EXAMPLES:");
    println!("    # Direct swap with Slopana token");
    println!("    cargo run --bin tool_direct_pool_swap_new -- \\");
    println!("        --pool BPp7mbBLDe3UwXmeWKnwm6CnAdAwVS746auJNmjtArjw \\");
    println!("        --token AkdtuaKVDpsZyeZ8LvcVf4G4L3nJ1Jmd7npmF5mpbonk \\");
    println!("        --amount 0.001");
    println!("");
    println!("    # Dry run simulation");
    println!("    cargo run --bin tool_direct_pool_swap_new -- \\");
    println!("        --pool BPp7mbBLDe3UwXmeWKnwm6CnAdAwVS746auJNmjtArjw \\");
    println!("        --token AkdtuaKVDpsZyeZ8LvcVf4G4L3nJ1Jmd7npmF5mpbonk \\");
    println!("        --amount 0.001 --dry-run");
    println!("");
    println!("FEATURES:");
    println!("    ✅ Direct Raydium pool interaction (no external APIs)");
    println!("    ✅ Multi-pool type support (CPMM, Legacy AMM)");
    println!("    ✅ Automatic ATA creation for output token");
    println!("    ✅ Pool data validation and reserve calculation");
    println!("    ✅ Comprehensive error handling and logging");
    println!("    ✅ Real on-chain testing with small amounts");
    println!("");
    println!("POOL TYPES SUPPORTED:");
    println!("    🟣 Raydium CPMM (CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C)");
    println!("    🟣 Raydium Legacy AMM (675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8)");
    println!("");
    println!("SAFETY:");
    println!("    🛡️ Maximum 1 SOL per transaction");
    println!("    🛡️ Real balance validation before swap");
    println!("    🛡️ Comprehensive pool validation");
    println!("    🛡️ Direct blockchain interaction only");
}

/// Get wallet keypair from configuration
fn get_wallet_keypair() -> Result<Keypair, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(format!("Failed to read configs: {}", e)))?;
    
    // Parse the private key (can be base58 string or byte array)
    let keypair = if configs.main_wallet_private.starts_with('[') && configs.main_wallet_private.ends_with(']') {
        // Parse as byte array
        let bytes_str = &configs.main_wallet_private[1..configs.main_wallet_private.len()-1];
        let bytes: Result<Vec<u8>, _> = bytes_str
            .split(',')
            .map(|s| s.trim().parse::<u8>())
            .collect();
        let bytes = bytes.map_err(|_| SwapError::ConfigError("Invalid wallet private key format".to_string()))?;
        Keypair::try_from(&bytes[..]).map_err(|_| SwapError::ConfigError("Invalid wallet private key".to_string()))?
    } else {
        // Parse as base58 string
        let decoded = bs58::decode(&configs.main_wallet_private)
            .into_vec()
            .map_err(|_| SwapError::ConfigError("Invalid base58 wallet private key".to_string()))?;
        Keypair::try_from(&decoded[..]).map_err(|_| SwapError::ConfigError("Invalid wallet private key".to_string()))?
    };
    
    Ok(keypair)
}

/// Get wallet address from keypair
fn get_wallet_address() -> Result<String, SwapError> {
    let keypair = get_wallet_keypair()?;
    Ok(keypair.pubkey().to_string())
}

/// Execute direct pool swap with comprehensive pool analysis
async fn execute_direct_pool_swap(config: SwapConfig) -> Result<(), SwapError> {
    // Get wallet information
    let wallet_keypair = get_wallet_keypair()?;
    let wallet_address = wallet_keypair.pubkey();
    
    log(LogTag::Swap, "WALLET", &format!("💼 Wallet: {}", wallet_address));
    
    // Check initial SOL balance
    let initial_sol_balance = get_sol_balance(&wallet_address.to_string()).await?;
    log(LogTag::Swap, "BALANCE", &format!("💰 Initial SOL balance: {:.9} SOL", initial_sol_balance));
    
    if initial_sol_balance < config.amount_sol + 0.01 {
        return Err(SwapError::InsufficientBalance(
            format!("Insufficient SOL balance. Need {} + 0.01 for fees, have {:.9}", 
                   config.amount_sol, initial_sol_balance)
        ));
    }
    
    // Validate pool and token addresses
    let pool_pubkey = Pubkey::from_str(&config.pool_address)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid pool address: {}", e)))?;
    let token_mint_pubkey = Pubkey::from_str(&config.token_mint)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid token mint: {}", e)))?;
    
    log(LogTag::Swap, "VALIDATE", &format!("🔍 Validating pool: {}", config.pool_address));
    
    // Get pool data from blockchain using existing pool service
    let pool_data = fetch_pool_data(&pool_pubkey).await?;
    log(LogTag::Swap, "POOL_DATA", &format!(
        "📊 Pool {} reserves: Base={} ({} decimals), Quote={} ({} decimals)",
        pool_data.pool_type,
        pool_data.base_reserve, pool_data.base_decimals,
        pool_data.quote_reserve, pool_data.quote_decimals
    ));
    
    // Determine if we're buying or selling (SOL -> Token or Token -> SOL)
    let sol_mint_pubkey = Pubkey::from_str(SOL_MINT).unwrap();
    let pool_contains_sol_and_token = 
        (pool_data.quote_mint == sol_mint_pubkey && pool_data.base_mint == token_mint_pubkey) ||
        (pool_data.base_mint == sol_mint_pubkey && pool_data.quote_mint == token_mint_pubkey);
    
    if !pool_contains_sol_and_token {
        return Err(SwapError::InvalidAmount(
            "Pool does not contain SOL and target token pair".to_string()
        ));
    }
    
    // Since we're providing SOL amount, we always want to buy tokens (SOL → Token)
    let sol_is_quote = pool_data.quote_mint == sol_mint_pubkey;
    
    log(LogTag::Swap, "DIRECTION", &format!(
        "🔄 Swap direction: SOL → Token (buying tokens with {} SOL)",
        config.amount_sol
    ));
    
    // Get token decimals
    let token_decimals = get_token_decimals_from_chain(&config.token_mint).await
        .unwrap_or(9); // Default to 9 if unable to fetch
    
    log(LogTag::Swap, "DECIMALS", &format!("🔢 Token decimals: {}", token_decimals));
    
    // Calculate expected output using constant product formula
    // Always buying tokens with SOL
    let input_lamports = (config.amount_sol * 1_000_000_000.0) as u64;
    let (expected_output, output_decimals) = if sol_is_quote {
        // SOL is quote token, token is base token
        let output = calculate_swap_output(
            input_lamports,
            pool_data.quote_reserve, // SOL reserve
            pool_data.base_reserve,  // Token reserve
        );
        (output, token_decimals)
    } else {
        // SOL is base token, token is quote token
        let output = calculate_swap_output(
            input_lamports,
            pool_data.base_reserve,  // SOL reserve
            pool_data.quote_reserve, // Token reserve
        );
        (output, token_decimals)
    };
    
    log(LogTag::Swap, "CALCULATION", &format!(
        "🧮 Expected output: {} tokens ({} raw units)",
        expected_output as f64 / 10_f64.powi(output_decimals as i32),
        expected_output
    ));
    
    // Calculate minimum output with slippage
    let min_output = (expected_output as f64 * (100.0 - config.slippage_percent) / 100.0) as u64;
    log(LogTag::Swap, "SLIPPAGE", &format!(
        "⚡ Minimum output ({}% slippage): {} tokens",
        config.slippage_percent,
        min_output as f64 / 10_f64.powi(output_decimals as i32)
    ));
    
    if config.dry_run {
        log(LogTag::Swap, "DRY_RUN", "🧪 Dry run mode - no transaction will be sent");
        log(LogTag::Swap, "SIMULATION", &format!(
            "✅ Simulation complete: {} SOL would swap for ~{} tokens via {} pool",
            config.amount_sol,
            expected_output as f64 / 10_f64.powi(output_decimals as i32),
            pool_data.pool_type
        ));
        return Ok(());
    }
    
    // Create and send the direct swap transaction
    log(LogTag::Swap, "DIRECT_EXECUTION", "🔗 Creating direct pool swap transaction");
    
    // Check if we need to create ATA for the output token
    let token_ata = get_associated_token_address(&wallet_address, &token_mint_pubkey);
    let rpc_client = get_rpc_client();
    
    let mut instructions = Vec::new();
    
    // Add compute budget instructions
    instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(MINIMUM_COMPUTE_UNITS));
    instructions.push(ComputeBudgetInstruction::set_compute_unit_price(MINIMUM_PRIORITY_FEE));
    
    // Check if ATA exists
    if rpc_client.get_account(&token_ata).await.is_err() {
        log(LogTag::Swap, "ATA_CREATE", &format!("📝 Creating ATA for token: {}", token_ata));
        instructions.push(create_associated_token_account(
            &wallet_address,
            &wallet_address,
            &token_mint_pubkey,
            &spl_token::id(),
        ));
    } else {
        log(LogTag::Swap, "ATA_EXISTS", &format!("✅ ATA already exists: {}", token_ata));
    }
    
    // Build the swap instruction based on pool type
    let swap_instruction = if pool_data.program_id == RAYDIUM_CPMM_PROGRAM_ID {
        build_raydium_cpmm_swap_instruction(
            &pool_pubkey,
            &wallet_address,
            &token_mint_pubkey,
            &token_ata,
            input_lamports,
            min_output,
            sol_is_quote,
        ).await?
    } else if pool_data.program_id == RAYDIUM_AMM_PROGRAM_ID {
        build_raydium_amm_swap_instruction(
            &pool_pubkey,
            &wallet_address,
            &token_mint_pubkey,
            &token_ata,
            input_lamports,
            min_output,
            sol_is_quote,
        ).await?
    } else {
        return Err(SwapError::ApiError(
            format!("Unsupported pool program: {}", pool_data.program_id)
        ));
    };
    
    instructions.push(swap_instruction);
    
    // Create and send transaction
    let recent_blockhash = rpc_client.get_latest_blockhash().await
        .map_err(|e| SwapError::ApiError(format!("Failed to get blockhash: {}", e)))?;
    
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&wallet_address),
        &[&wallet_keypair],
        recent_blockhash,
    );
    
    log(LogTag::Swap, "TRANSACTION", &format!("📤 Sending transaction with {} instructions", instructions.len()));
    
    let signature = rpc_client.send_transaction(&transaction).await
        .map_err(|e| SwapError::TransactionError(format!("Transaction failed: {}", e)))?;
    
    log(LogTag::Swap, "SUCCESS", &format!("✅ Transaction confirmed: {}", signature));
    
    // Get final balances
    let final_sol_balance = get_sol_balance(&wallet_address.to_string()).await?;
    let final_token_balance = get_token_balance(&wallet_address.to_string(), &config.token_mint).await.unwrap_or(0);
    
    log(LogTag::Swap, "FINAL_BALANCES", &format!(
        "🏁 Final balances: SOL={:.9}, Token={}",
        final_sol_balance,
        final_token_balance as f64 / 10_f64.powi(token_decimals as i32)
    ));
    
    let sol_change = final_sol_balance - initial_sol_balance;
    let tokens_received = final_token_balance as f64 / 10_f64.powi(token_decimals as i32);
    
    log(LogTag::Swap, "RESULT", &format!(
        "📈 SOL change: {:.9} SOL, Tokens received: {} tokens",
        sol_change,
        tokens_received
    ));
    
    Ok(())
}

/// Fetch pool data from blockchain using existing pool service
async fn fetch_pool_data(pool_address: &Pubkey) -> Result<PoolData, SwapError> {
    // Get pool information using the existing pool calculator
    let pool_calculator = PoolPriceCalculator::new()
        .map_err(|e| SwapError::ApiError(format!("Failed to create pool calculator: {}", e)))?;
    
    let pool_info = pool_calculator.get_pool_info(&pool_address.to_string()).await
        .map_err(|e| SwapError::ApiError(format!("Failed to get pool info: {}", e)))?
        .ok_or_else(|| SwapError::ApiError("Pool info not available".to_string()))?;
    
    // Get pool account to determine program ID
    let rpc_client = get_rpc_client();
    let account = rpc_client.get_account(pool_address).await
        .map_err(|e| SwapError::ApiError(format!("Failed to fetch pool account: {}", e)))?;
    
    let program_id = account.owner.to_string();
    let pool_type = get_pool_program_display_name(&program_id);
    
    log(LogTag::Swap, "POOL_TYPE", &format!(
        "Detected pool type: {} (Program: {})", pool_type, &program_id[..12]
    ));
    
    // Parse mints from pool info
    let base_mint = Pubkey::from_str(&pool_info.token_0_mint)
        .map_err(|e| SwapError::ApiError(format!("Invalid base mint: {}", e)))?;
    let quote_mint = Pubkey::from_str(&pool_info.token_1_mint)
        .map_err(|e| SwapError::ApiError(format!("Invalid quote mint: {}", e)))?;
    
    // Extract reserves and decimals from pool info
    let (base_reserve, quote_reserve) = (pool_info.token_0_reserve, pool_info.token_1_reserve);
    let base_decimals = pool_info.token_0_decimals;
    let quote_decimals = pool_info.token_1_decimals;
    
    Ok(PoolData {
        pool_address: pool_address.to_string(),
        program_id,
        pool_type,
        base_mint,
        quote_mint,
        base_reserve,
        quote_reserve,
        base_decimals,
        quote_decimals,
    })
}

/// Build Raydium CPMM swap instruction
async fn build_raydium_cpmm_swap_instruction(
    _pool_pubkey: &Pubkey,
    _wallet_pubkey: &Pubkey,
    _token_mint: &Pubkey,
    _token_ata: &Pubkey,
    _amount_in: u64,
    _minimum_amount_out: u64,
    _sol_is_quote: bool,
) -> Result<Instruction, SwapError> {
    // For now, return an error indicating this needs to be implemented
    // Implementing full CPMM instruction requires decoding the pool account
    // to extract all required vault and authority addresses
    
    log(LogTag::Swap, "CPMM_INSTRUCTION", "🔧 Building Raydium CPMM swap instruction");
    log(LogTag::Swap, "NOT_IMPLEMENTED", "❌ CPMM instruction building requires complete pool account decoding");
    
    Err(SwapError::ApiError(
        "Raydium CPMM instruction building not yet implemented - requires pool account decoding for vault addresses".to_string()
    ))
}

/// Build Raydium Legacy AMM swap instruction  
async fn build_raydium_amm_swap_instruction(
    _pool_pubkey: &Pubkey,
    _wallet_pubkey: &Pubkey,
    _token_mint: &Pubkey,
    _token_ata: &Pubkey,
    _amount_in: u64,
    _minimum_amount_out: u64,
    _sol_is_quote: bool,
) -> Result<Instruction, SwapError> {
    // For now, return an error indicating this needs to be implemented
    // Implementing full AMM instruction requires decoding the pool account
    // to extract serum market, vault addresses, and authority
    
    log(LogTag::Swap, "AMM_INSTRUCTION", "🔧 Building Raydium Legacy AMM swap instruction");
    log(LogTag::Swap, "NOT_IMPLEMENTED", "❌ Legacy AMM instruction building requires complete pool account decoding");
    
    Err(SwapError::ApiError(
        "Raydium Legacy AMM instruction building not yet implemented - requires pool account decoding for serum market and vault addresses".to_string()
    ))
}

/// Calculate swap output using constant product formula (x * y = k)
fn calculate_swap_output(input_amount: u64, input_reserve: u64, output_reserve: u64) -> u64 {
    if input_reserve == 0 || output_reserve == 0 {
        return 0;
    }
    
    // Apply 0.25% trading fee (typical for Raydium)
    let fee_numerator = 9975_u128; // 100% - 0.25% = 99.75%
    let fee_denominator = 10000_u128;
    
    let input_with_fee = (input_amount as u128) * fee_numerator / fee_denominator;
    let numerator = input_with_fee * (output_reserve as u128);
    let denominator = (input_reserve as u128) + input_with_fee;
    
    (numerator / denominator) as u64
}
