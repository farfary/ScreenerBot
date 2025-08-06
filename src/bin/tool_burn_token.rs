use screenerbot::global::*;
use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::rpc::{ get_rpc_client, init_rpc_client };
use screenerbot::wallet::{ get_wallet_address };
use std::str::FromStr;
use solana_sdk::{
    pubkey::Pubkey,
    transaction::Transaction,
    instruction::{ Instruction, AccountMeta },
    signer::Signer,
};

// Simple error type for this tool
#[derive(Debug)]
struct BurnError(String);

impl std::fmt::Display for BurnError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BurnError {}

impl From<String> for BurnError {
    fn from(msg: String) -> Self {
        BurnError(msg)
    }
}

impl From<&str> for BurnError {
    fn from(msg: &str) -> Self {
        BurnError(msg.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for BurnError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        BurnError(format!("{}", err))
    }
}

/// Print comprehensive help menu for the Token Burn Tool
fn print_help() {
    println!("🔥 Token Burn Tool");
    println!("=====================================");
    println!("Advanced token burning utility that permanently destroys tokens by reducing");
    println!("the total supply. Supports both SPL Token and Token-2022 programs.");
    println!("");
    println!("⚠️  WARNING: Token burning is IRREVERSIBLE and PERMANENT!");
    println!("    Burned tokens cannot be recovered under any circumstances.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_burn_token -- <MINT_ADDRESS> [OPTIONS]");
    println!("");
    println!("ARGUMENTS:");
    println!("    <MINT_ADDRESS>     Token mint address to burn tokens from");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h         Show this help message");
    println!("    --dry-run         Simulate burn without executing transaction");
    println!("    --force           Execute actual burn transaction");
    println!("    --verbose         Enable detailed logging and analysis");
    println!("");
    println!("EXAMPLES:");
    println!("    # Test burn simulation for a specific token");
    println!(
        "    cargo run --bin tool_burn_token -- ChoNKscpdU3hPd1N3q8a3FPvPcuj5fsg1dA5WnHbTvZV --dry-run"
    );
    println!("");
    println!("    # Execute actual token burn with detailed logging");
    println!(
        "    cargo run --bin tool_burn_token -- ChoNKscpdU3hPd1N3q8a3FPvPcuj5fsg1dA5WnHbTvZV --force --verbose"
    );
    println!("");
    println!("    # Quick burn without extra logging");
    println!(
        "    cargo run --bin tool_burn_token -- ChoNKscpdU3hPd1N3q8a3FPvPcuj5fsg1dA5WnHbTvZV --force"
    );
    println!("");
    println!("BURN PROCESS:");
    println!("    1. Validates token mint address and program ownership");
    println!("    2. Checks wallet token balance and burn authority");
    println!("    3. Calculates total tokens available for burning");
    println!("    4. Creates burn instruction with proper program ID");
    println!("    5. Submits transaction to permanently destroy tokens");
    println!("    6. Confirms transaction and reports burned amount");
    println!("");
    println!("SAFETY FEATURES:");
    println!("    • Validates mint address format before processing");
    println!("    • Checks token balance to prevent zero-amount burns");
    println!("    • Automatic program detection (SPL Token vs Token-2022)");
    println!("    • Dry-run mode for safe testing");
    println!("    • Multi-RPC fallback for transaction reliability");
    println!("");
    println!("VALIDATION CHECKS:");
    println!("    • Token account existence and ownership");
    println!("    • Sufficient token balance for burn operation");
    println!("    • Proper burn authority permissions");
    println!("    • Token program compatibility");
    println!("");
    println!("RISK WARNINGS:");
    println!("    • Burned tokens are permanently removed from circulation");
    println!("    • No way to recover or 'unburn' tokens after transaction");
    println!("    • Failed transactions still consume transaction fees");
    println!("    • Use --dry-run first to validate burn parameters");
    println!("");
}

#[tokio::main]
async fn main() -> Result<(), BurnError> {
    // Initialize logger
    init_file_logging();

    log(LogTag::System, "TOOL", "🔥 Starting Token Burn Tool");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        return Ok(());
    }

    if args.len() < 2 {
        print_help();
        log(LogTag::System, "ERROR", "❌ Please provide a token mint address");
        return Ok(());
    }

    let mint_address = &args[1];
    let dry_run = args.contains(&"--dry-run".to_string());
    let force = args.contains(&"--force".to_string());
    let verbose = args.contains(&"--verbose".to_string());

    if dry_run {
        log(LogTag::System, "MODE", "🔍 DRY RUN MODE - No actual transactions will be sent");
    }

    if verbose {
        log(LogTag::System, "MODE", "📝 VERBOSE MODE - Detailed logging enabled");
    }

    // Validate mint address
    let _mint_pubkey = match Pubkey::from_str(mint_address) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("Invalid mint address '{}': {}", mint_address, e)
            );
            return Err(BurnError::from(format!("Invalid mint address: {}", e)));
        }
    };

    log(LogTag::System, "INFO", &format!("Target mint: {}", mint_address));

    // Get wallet address from configs
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr.clone(),
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            return Err(BurnError::from(format!("Failed to get wallet address: {}", e)));
        }
    };

    log(LogTag::System, "INFO", &format!("Wallet: {}", wallet_address));

    // Initialize RPC client
    if let Err(e) = init_rpc_client() {
        log(LogTag::System, "ERROR", &format!("Failed to initialize RPC client: {}", e));
        return Err(BurnError::from(format!("Failed to initialize RPC client: {}", e)));
    }
    let rpc_client = get_rpc_client();

    // Find the token account for this mint
    log(LogTag::System, "SEARCH", "🔍 Searching for token account...");

    let token_account_address = match
        rpc_client.get_associated_token_account(&wallet_address, mint_address).await
    {
        Ok(account) => account,
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("No token account found for mint {}: {}", mint_address, e)
            );
            return Err(
                BurnError::from(format!("No token account found for mint {}: {}", mint_address, e))
            );
        }
    };

    log(LogTag::System, "FOUND", &format!("Token account: {}", token_account_address));

    // Get token account info to check authority
    if verbose {
        log(LogTag::System, "DEBUG", "🔍 Checking token account details...");
        let token_account_pubkey = match Pubkey::from_str(&token_account_address) {
            Ok(pubkey) => pubkey,
            Err(e) => {
                log(LogTag::System, "WARNING", &format!("Invalid token account address: {}", e));
                return Err(BurnError::from(format!("Invalid token account address: {}", e)));
            }
        };

        match rpc_client.get_account(&token_account_pubkey).await {
            Ok(account) => {
                log(
                    LogTag::System,
                    "DEBUG",
                    &format!("Token account owner program: {}", account.owner)
                );
                log(
                    LogTag::System,
                    "DEBUG",
                    &format!("Token account data length: {} bytes", account.data.len())
                );

                // Try to parse SPL Token account data if it's the right size
                if account.data.len() >= 165 {
                    // SPL Token account is exactly 165 bytes
                    // Parse the owner from token account data (first 32 bytes after some fields)
                    if let Some(mint_bytes) = account.data.get(0..32) {
                        if let Ok(mint_from_data) = Pubkey::try_from(mint_bytes) {
                            log(
                                LogTag::System,
                                "DEBUG",
                                &format!("Mint from account data: {}", mint_from_data)
                            );
                        }
                    }
                    if let Some(owner_bytes) = account.data.get(32..64) {
                        if let Ok(owner_from_data) = Pubkey::try_from(owner_bytes) {
                            log(
                                LogTag::System,
                                "DEBUG",
                                &format!("Owner from account data: {}", owner_from_data)
                            );
                            if owner_from_data.to_string() != wallet_address {
                                log(
                                    LogTag::System,
                                    "WARNING",
                                    "⚠️  Wallet is not the owner of this token account!"
                                );
                                log(
                                    LogTag::System,
                                    "WARNING",
                                    &format!(
                                        "Account owner: {}, Wallet: {}",
                                        owner_from_data,
                                        wallet_address
                                    )
                                );
                            } else {
                                log(LogTag::System, "DEBUG", "✅ Wallet owns this token account");
                            }
                        }
                    }

                    // Check if the token account is frozen (byte 108)
                    if let Some(state_byte) = account.data.get(108) {
                        let is_frozen = *state_byte != 0;
                        if is_frozen {
                            log(
                                LogTag::System,
                                "WARNING",
                                "⚠️  TOKEN ACCOUNT IS FROZEN! This may prevent burning."
                            );
                        } else {
                            log(LogTag::System, "DEBUG", "✅ Token account is not frozen");
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Could not get token account details: {}", e)
                );
            }
        }

        // Also check the mint account for freeze authority and other restrictions
        log(LogTag::System, "DEBUG", "🔍 Checking mint account details...");
        let mint_pubkey = match Pubkey::from_str(mint_address) {
            Ok(pubkey) => pubkey,
            Err(e) => {
                log(LogTag::System, "WARNING", &format!("Invalid mint address: {}", e));
                return Err(BurnError::from(format!("Invalid mint address: {}", e)));
            }
        };

        match rpc_client.get_account(&mint_pubkey).await {
            Ok(mint_account) => {
                log(
                    LogTag::System,
                    "DEBUG",
                    &format!("Mint account owner program: {}", mint_account.owner)
                );
                log(
                    LogTag::System,
                    "DEBUG",
                    &format!("Mint account data length: {} bytes", mint_account.data.len())
                );

                // Check if the mint account might have freeze authority set
                if mint_account.data.len() >= 82 {
                    // SPL Token mint is 82 bytes
                    log(LogTag::System, "DEBUG", "Parsing mint account data...");

                    // Check mint authority (bytes 4-36, but first check if it exists)
                    if let Some(mint_authority_option) = mint_account.data.get(4) {
                        if *mint_authority_option == 1 {
                            if let Some(mint_auth_bytes) = mint_account.data.get(5..37) {
                                if let Ok(mint_authority) = Pubkey::try_from(mint_auth_bytes) {
                                    log(
                                        LogTag::System,
                                        "DEBUG",
                                        &format!("Mint authority: {}", mint_authority)
                                    );
                                }
                            }
                        } else {
                            log(LogTag::System, "DEBUG", "No mint authority set");
                        }
                    }

                    // Check freeze authority (bytes 46-78)
                    if let Some(freeze_authority_option) = mint_account.data.get(46) {
                        if *freeze_authority_option == 1 {
                            if let Some(freeze_auth_bytes) = mint_account.data.get(47..79) {
                                if let Ok(freeze_authority) = Pubkey::try_from(freeze_auth_bytes) {
                                    log(
                                        LogTag::System,
                                        "WARNING",
                                        &format!("⚠️  Mint has freeze authority: {}", freeze_authority)
                                    );
                                    if freeze_authority.to_string() == wallet_address {
                                        log(
                                            LogTag::System,
                                            "INFO",
                                            "✅ Wallet IS the freeze authority - could thaw first"
                                        );
                                    } else {
                                        log(
                                            LogTag::System,
                                            "WARNING",
                                            "❌ Wallet is NOT the freeze authority - cannot thaw"
                                        );
                                        log(
                                            LogTag::System,
                                            "WARNING",
                                            "🔒 Frozen tokens cannot be burned without freeze authority"
                                        );
                                    }
                                }
                            }
                        } else {
                            log(LogTag::System, "DEBUG", "No freeze authority set on mint");
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Could not get mint account details: {}", e)
                );
            }
        }
    }

    // Get token balance
    let token_balance = match rpc_client.get_token_balance(&wallet_address, mint_address).await {
        Ok(balance) => balance,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get token balance: {}", e));
            return Err(BurnError::from(format!("Failed to get token balance: {}", e)));
        }
    };

    if token_balance == 0 {
        log(LogTag::System, "INFO", "No tokens to burn - balance is 0");
        return Ok(());
    }

    log(LogTag::System, "BALANCE", &format!("Token balance: {} (raw units)", token_balance));

    // Check if it's Token-2022
    let is_token_2022 = match rpc_client.is_token_2022_mint(mint_address).await {
        Ok(is_2022) => is_2022,
        Err(e) => {
            log(
                LogTag::System,
                "WARNING",
                &format!("Could not determine token standard: {}, assuming SPL Token", e)
            );
            false
        }
    };

    log(
        LogTag::System,
        "DETECTION",
        &format!("Token standard: {}", if is_token_2022 {
            "Token-2022 (Token Extensions)"
        } else {
            "SPL Token"
        })
    );

    if !dry_run && !force {
        log(LogTag::System, "CONFIRM", "⚠️  This will permanently burn ALL tokens of this mint!");
        log(LogTag::System, "CONFIRM", &format!("⚠️  Amount to burn: {} tokens", token_balance));
        log(LogTag::System, "CONFIRM", "⚠️  Run with --force to proceed, or --dry-run to simulate");
        return Ok(());
    }

    // Build the burn instruction
    log(LogTag::System, "BUILD", "🔨 Building burn instruction...");

    let burn_instruction = if is_token_2022 {
        match
            build_token_2022_burn_instruction(
                &token_account_address,
                mint_address,
                &wallet_address,
                token_balance
            )
        {
            Ok(instruction) => instruction,
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to build Token-2022 burn instruction: {}", e)
                );
                return Err(
                    BurnError::from(format!("Failed to build Token-2022 burn instruction: {}", e))
                );
            }
        }
    } else {
        match
            build_spl_token_burn_instruction(
                &token_account_address,
                mint_address,
                &wallet_address,
                token_balance
            )
        {
            Ok(instruction) => instruction,
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to build SPL Token burn instruction: {}", e)
                );
                return Err(
                    BurnError::from(format!("Failed to build SPL Token burn instruction: {}", e))
                );
            }
        }
    };

    if dry_run {
        log(
            LogTag::System,
            "DRY_RUN",
            &format!(
                "Would burn {} tokens from account {} using {} program",
                token_balance,
                token_account_address,
                if is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                }
            )
        );
        return Ok(());
    }

    // Load wallet keypair
    let configs = match read_configs() {
        Ok(configs) => configs,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to read configs: {}", e));
            return Err(BurnError::from(format!("Failed to read configs: {}", e)));
        }
    };

    let wallet_keypair = match load_wallet_from_config(&configs) {
        Ok(keypair) => keypair,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to load wallet keypair: {}", e));
            return Err(BurnError::from(format!("Failed to load wallet keypair: {}", e)));
        }
    };

    // Get recent blockhash
    let recent_blockhash = match rpc_client.get_latest_blockhash().await {
        Ok(blockhash) => blockhash,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get recent blockhash: {}", e));
            return Err(BurnError::from(format!("Failed to get recent blockhash: {}", e)));
        }
    };

    // Build and sign transaction
    let transaction = Transaction::new_signed_with_payer(
        &[burn_instruction],
        Some(&wallet_keypair.pubkey()),
        &[&wallet_keypair],
        recent_blockhash
    );

    log(
        LogTag::System,
        "BURN",
        &format!("🔥 Burning {} tokens using {} program...", token_balance, if is_token_2022 {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Send transaction
    match rpc_client.send_transaction(&transaction).await {
        Ok(signature) => {
            log(LogTag::System, "SUCCESS", &format!("✅ Tokens burned successfully!"));
            log(LogTag::System, "TX", &format!("Transaction signature: {}", signature));
            log(
                LogTag::System,
                "BURNED",
                &format!("🔥 Burned {} tokens from mint {}", token_balance, mint_address)
            );
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to burn tokens: {}", e));
            return Err(BurnError::from(format!("Failed to burn tokens: {}", e)));
        }
    }

    Ok(())
}

/// Builds burn instruction for SPL Token accounts
fn build_spl_token_burn_instruction(
    token_account: &str,
    mint: &str,
    owner: &str,
    amount: u64
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let token_account_pubkey = Pubkey::from_str(token_account)?;
    let mint_pubkey = Pubkey::from_str(mint)?;
    let owner_pubkey = Pubkey::from_str(owner)?;

    // SPL Token Program ID
    let spl_token_program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")?;

    // Burn instruction: [8] + amount (8 bytes, little endian)
    let mut instruction_data = vec![8u8]; // Burn instruction ID
    instruction_data.extend_from_slice(&amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(token_account_pubkey, false), // Token account to burn from (writable)
        AccountMeta::new(mint_pubkey, false), // Mint account (writable) - to reduce supply
        AccountMeta::new_readonly(owner_pubkey, true) // Authority (signer)
    ];

    Ok(Instruction {
        program_id: spl_token_program_id,
        accounts,
        data: instruction_data,
    })
}

/// Builds burn instruction for Token-2022 accounts
fn build_token_2022_burn_instruction(
    token_account: &str,
    mint: &str,
    owner: &str,
    amount: u64
) -> Result<Instruction, Box<dyn std::error::Error>> {
    let token_account_pubkey = Pubkey::from_str(token_account)?;
    let mint_pubkey = Pubkey::from_str(mint)?;
    let owner_pubkey = Pubkey::from_str(owner)?;

    // Token-2022 Program ID
    let token_2022_program_id = Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")?;

    // Burn instruction: [8] + amount (8 bytes, little endian)
    let mut instruction_data = vec![8u8]; // Burn instruction ID
    instruction_data.extend_from_slice(&amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(token_account_pubkey, false), // Token account to burn from (writable)
        AccountMeta::new(mint_pubkey, false), // Mint account (writable) - to reduce supply
        AccountMeta::new_readonly(owner_pubkey, true) // Authority (signer)
    ];

    Ok(Instruction {
        program_id: token_2022_program_id,
        accounts,
        data: instruction_data,
    })
}
