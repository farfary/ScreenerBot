/// Direct Raydium CPMM Swap Tool
///
/// This tool creates and sends direct swap transactions to Raydium CPMM pools
/// without using aggregators or APIs. It builds the transaction manually using
/// the Raydium program instructions.
///
/// Features:
/// - Direct program interaction with Raydium CPMM
/// - Automatic WSOL wrapping/unwrapping
/// - Manual transaction construction
/// - SOL to token and token to SOL swaps
/// - Configurable slippage protection
/// - Real-time pool state fetching

use screenerbot::arguments::{ get_arg_value, has_arg, set_cmd_args };
use screenerbot::logger::{ log, LogTag };
use screenerbot::pools::types::{ RAYDIUM_CPMM_PROGRAM_ID };
use screenerbot::pools::decoders::raydium_cpmm::{ RaydiumCpmmPoolInfo };
use screenerbot::pools::AccountData;
use screenerbot::rpc::{ get_rpc_client, lamports_to_sol, sol_to_lamports };
use screenerbot::configs::{ read_configs, load_wallet_from_config };

use solana_sdk::{
    instruction::{ Instruction, AccountMeta },
    pubkey::Pubkey,
    transaction::Transaction,
    signature::{ Keypair, Signer },
    system_instruction,
};
use spl_token;
use spl_associated_token_account;
use std::str::FromStr;
use std::collections::HashMap;

/// Token-2022 program ID
const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// Legacy SPL Token program ID
const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// WSOL mint address (Wrapped SOL)
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Default slippage in basis points (1% = 100 bps)
const DEFAULT_SLIPPAGE_BPS: u16 = 100;

/// Minimum SOL to keep in wallet (for fees)
const MIN_SOL_BALANCE: f64 = 0.01;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set command line arguments for global access
    set_cmd_args(std::env::args().collect());

    if has_arg("--help") || has_arg("-h") {
        print_help();
        return Ok(());
    }

    log(LogTag::System, "STARTUP", "🚀 Raydium CPMM Direct Swap Tool");

    // Parse command line arguments
    let pool_address = get_arg_value("--pool").ok_or(
        "Pool address is required. Use --pool <address>"
    )?;

    let token_mint = get_arg_value("--token").ok_or("Token mint is required. Use --token <mint>")?;

    let amount_str = get_arg_value("--amount").ok_or(
        "Amount is required. Use --amount <amount_in_sol>"
    )?;

    let amount_sol: f64 = amount_str
        .parse()
        .map_err(|_| "Invalid amount format. Use decimal format like 0.1")?;

    let slippage_bps: u16 = get_arg_value("--slippage")
        .map(|s| s.parse().unwrap_or(DEFAULT_SLIPPAGE_BPS))
        .unwrap_or(DEFAULT_SLIPPAGE_BPS);

    let is_sell = has_arg("--sell");
    let is_dry_run = has_arg("--dry-run");

    if amount_sol <= 0.0 {
        return Err("Amount must be greater than 0".into());
    }

    // Load configuration and wallet
    let configs = read_configs()?;
    let wallet = load_wallet_from_config(&configs)?;
    let wallet_pubkey = wallet.pubkey();

    log(
        LogTag::System,
        "CONFIG",
        &format!(
            "📋 Swap Configuration:
        Wallet: {}
        Pool: {}
        Token: {}
        Amount: {} SOL
        Direction: {}
        Slippage: {}%
        Dry Run: {}",
            wallet_pubkey,
            pool_address,
            &token_mint[..8],
            amount_sol,
            if is_sell {
                "SELL (Token → SOL)"
            } else {
                "BUY (SOL → Token)"
            },
            (slippage_bps as f64) / 100.0,
            is_dry_run
        )
    );

    // Validate wallet balance
    let rpc_client = get_rpc_client();
    let sol_balance = rpc_client.get_sol_balance(&wallet_pubkey.to_string()).await?;

    if sol_balance < amount_sol + MIN_SOL_BALANCE {
        return Err(
            format!(
                "Insufficient SOL balance. Have: {:.6}, Need: {:.6} (+ {:.3} for fees)",
                sol_balance,
                amount_sol,
                MIN_SOL_BALANCE
            ).into()
        );
    }

    log(LogTag::System, "BALANCE", &format!("💰 Wallet Balance: {:.6} SOL", sol_balance));

    // Step 1: Fetch pool state
    log(LogTag::System, "POOL_FETCH", "📡 Fetching pool state...");
    let pool_info = fetch_pool_state(&pool_address, &token_mint).await?;

    log(
        LogTag::System,
        "POOL_INFO",
        &format!(
            "🏊 Pool Information:
        Token0: {} (Vault: {})
        Token1: {} (Vault: {})
        Pool Program: {}",
            &pool_info.token_0_mint[..8],
            &pool_info.token_0_vault[..8],
            &pool_info.token_1_mint[..8],
            &pool_info.token_1_vault[..8],
            RAYDIUM_CPMM_PROGRAM_ID
        )
    );

    // Step 2: Calculate swap amounts
    let swap_params = calculate_swap_amounts(&pool_info, amount_sol, slippage_bps, is_sell).await?;

    log(
        LogTag::System,
        "SWAP_CALC",
        &format!(
            "🧮 Swap Calculation:
        Input Amount: {} {}
        Expected Output: {} {}
        Minimum Output: {} {} ({}% slippage)",
            swap_params.input_amount,
            if is_sell {
                "tokens"
            } else {
                "SOL"
            },
            swap_params.expected_output,
            if is_sell {
                "SOL"
            } else {
                "tokens"
            },
            swap_params.minimum_output,
            if is_sell {
                "SOL"
            } else {
                "tokens"
            },
            (slippage_bps as f64) / 100.0
        )
    );

    // Step 3: Build swap transaction
    log(LogTag::System, "TX_BUILD", "🔨 Building swap transaction...");
    let transaction = build_swap_transaction(&wallet, &pool_info, &swap_params, is_sell).await?;

    log(
        LogTag::System,
        "TX_READY",
        &format!(
            "✅ Transaction built with {} instructions",
            transaction.message.instructions.len()
        )
    );

    // Step 4: Send transaction or simulate
    if is_dry_run {
        log(LogTag::System, "DRY_RUN", "🧪 DRY RUN MODE - Transaction not sent");
        log(
            LogTag::System,
            "SIMULATION",
            &format!(
                "📝 Would execute swap:
            {} {} → {} {}
            Estimated Network Fee: ~0.000005 SOL",
                swap_params.input_amount,
                if is_sell {
                    "tokens"
                } else {
                    "SOL"
                },
                swap_params.expected_output,
                if is_sell {
                    "SOL"
                } else {
                    "tokens"
                }
            )
        );
    } else {
        log(LogTag::System, "TX_SEND", "📤 Sending transaction to network...");

        let signature = rpc_client.send_transaction(&transaction).await?;

        log(
            LogTag::System,
            "SUCCESS",
            &format!(
                "🎉 Swap completed successfully!
            Transaction: {}
            Swapped: {} {} → {} {}",
                signature,
                swap_params.input_amount,
                if is_sell {
                    "tokens"
                } else {
                    "SOL"
                },
                swap_params.expected_output,
                if is_sell {
                    "SOL"
                } else {
                    "tokens"
                }
            )
        );
    }

    Ok(())
}

/// Swap calculation parameters
#[derive(Debug)]
struct SwapParams {
    input_amount: f64,
    expected_output: f64,
    minimum_output: f64,
    input_amount_raw: u64,
    minimum_output_raw: u64,
}

/// Fetch and decode pool state from the blockchain
async fn fetch_pool_state(
    pool_address: &str,
    token_mint: &str
) -> Result<RaydiumCpmmPoolInfo, Box<dyn std::error::Error>> {
    let rpc_client = get_rpc_client();
    let pool_pubkey = Pubkey::from_str(pool_address)?;

    // Get pool account data
    let pool_account = rpc_client
        .get_account(&pool_pubkey).await
        .map_err(|e| format!("Failed to get pool account: {}", e))?;

    log(
        LogTag::System,
        "POOL_DATA",
        &format!(
            "📊 Pool account: {} bytes, owner: {}",
            pool_account.data.len(),
            pool_account.owner
        )
    );

    // Create account data map for decoder
    let mut accounts = HashMap::new();
    accounts.insert(
        pool_address.to_string(),
        AccountData::from_account(
            pool_pubkey,
            pool_account,
            0 // slot not critical for our use case
        )
    );

    // Debug: Check what we have in accounts map
    log(
        LogTag::System,
        "DEBUG",
        &format!(
            "Accounts map contains {} entries, pool owner: {}",
            accounts.len(),
            accounts
                .get(pool_address)
                .map(|acc| acc.owner.to_string())
                .unwrap_or("None".to_string())
        )
    );

    // Instead of using the price decoder, let's directly extract the pool info
    // since we need the full pool state for swap operations
    let pool_data = &accounts[pool_address];
    let mut pool_info = decode_cpmm_pool_manually(&pool_data.data)?;
    pool_info.pool_id = pool_address.to_string(); // Set the pool ID
    Ok(pool_info)
}

/// Manually decode CPMM pool data to get enhanced pool info
fn decode_cpmm_pool_manually(
    data: &[u8]
) -> Result<RaydiumCpmmPoolInfo, Box<dyn std::error::Error>> {
    if data.len() < 8 + 32 * 10 {
        return Err("Invalid pool data length".into());
    }

    let mut offset = 8; // Skip discriminator

    // Read pool fields according to CPMM layout (complete version)
    let amm_config = read_pubkey_at_offset(data, &mut offset)?;
    let pool_creator = read_pubkey_at_offset(data, &mut offset)?;
    let token_0_vault = read_pubkey_at_offset(data, &mut offset)?;
    let token_1_vault = read_pubkey_at_offset(data, &mut offset)?;
    let lp_mint = read_pubkey_at_offset(data, &mut offset)?;
    let token_0_mint = read_pubkey_at_offset(data, &mut offset)?;
    let token_1_mint = read_pubkey_at_offset(data, &mut offset)?;
    let token_0_program = read_pubkey_at_offset(data, &mut offset)?;
    let token_1_program = read_pubkey_at_offset(data, &mut offset)?;
    let observation_key = read_pubkey_at_offset(data, &mut offset)?;

    // Read additional fields
    let auth_bump = data[offset];
    let status = data[offset + 1];
    let lp_mint_decimals = data[offset + 2];
    let token_0_decimals = data[offset + 3];
    let token_1_decimals = data[offset + 4];

    Ok(RaydiumCpmmPoolInfo {
        // Basic token information
        token_0_mint,
        token_1_mint,
        token_0_vault,
        token_1_vault,
        token_0_decimals,
        token_1_decimals,

        // Additional fields for swap operations (will be set by caller)
        pool_id: String::new(), // Will be filled by caller
        amm_config,
        pool_creator,
        lp_mint,
        token_0_program,
        token_1_program,
        observation_key,
        auth_bump,
        status,
        lp_mint_decimals,
    })
}

/// Calculate swap input/output amounts with slippage protection
async fn calculate_swap_amounts(
    pool_info: &RaydiumCpmmPoolInfo,
    amount_sol: f64,
    slippage_bps: u16,
    is_sell: bool
) -> Result<SwapParams, Box<dyn std::error::Error>> {
    // Fetch vault balances
    let vault_0_balance = get_token_account_balance(&pool_info.token_0_vault).await?;
    let vault_1_balance = get_token_account_balance(&pool_info.token_1_vault).await?;

    log(
        LogTag::System,
        "RESERVES",
        &format!(
            "💰 Pool Reserves:
        Vault 0: {} (raw)
        Vault 1: {} (raw)",
            vault_0_balance,
            vault_1_balance
        )
    );

    // Determine which vault is SOL and which is token
    let (sol_reserve, token_reserve, sol_decimals, token_decimals) = if
        pool_info.token_0_mint == WSOL_MINT
    {
        (vault_0_balance, vault_1_balance, pool_info.token_0_decimals, pool_info.token_1_decimals)
    } else {
        (vault_1_balance, vault_0_balance, pool_info.token_1_decimals, pool_info.token_0_decimals)
    };

    // Calculate using constant product formula: x * y = k
    let (input_amount, expected_output, input_amount_raw, minimum_output_raw) = if is_sell {
        // For selling: calculate how many tokens to get approximate SOL amount
        // Using current pool rate: SOL per token = sol_reserve / token_reserve
        let current_rate = (sol_reserve as f64) / (token_reserve as f64);
        let tokens_to_sell = amount_sol / current_rate;
        let token_amount_raw = (tokens_to_sell * (10_f64).powi(token_decimals as i32)) as u64;

        // Calculate expected SOL output using constant product formula
        let sol_output_raw = (sol_reserve * token_amount_raw) / (token_reserve + token_amount_raw);
        let sol_output = (sol_output_raw as f64) / (10_f64).powi(sol_decimals as i32);

        // Apply slippage to the calculated output
        let min_sol_output_raw = (sol_output_raw * (10000 - (slippage_bps as u64))) / 10000;

        (tokens_to_sell, sol_output, token_amount_raw, min_sol_output_raw)
    } else {
        // Buying tokens with SOL
        let sol_amount_raw =
            (sol_to_lamports(amount_sol) * (10_u64).pow(sol_decimals as u32)) / (10_u64).pow(9);
        let token_output_raw = (token_reserve * sol_amount_raw) / (sol_reserve + sol_amount_raw);
        let token_output = (token_output_raw as f64) / (10_f64).powi(token_decimals as i32);
        let min_token_output_raw = (token_output_raw * (10000 - (slippage_bps as u64))) / 10000;

        (amount_sol, token_output, sol_amount_raw, min_token_output_raw)
    };

    Ok(SwapParams {
        input_amount,
        expected_output,
        minimum_output: expected_output * (1.0 - (slippage_bps as f64) / 10000.0),
        input_amount_raw,
        minimum_output_raw,
    })
}

/// Build the complete swap transaction with all necessary instructions
async fn build_swap_transaction(
    wallet: &Keypair,
    pool_info: &RaydiumCpmmPoolInfo,
    swap_params: &SwapParams,
    is_sell: bool
) -> Result<Transaction, Box<dyn std::error::Error>> {
    let mut instructions = Vec::new();
    let wallet_pubkey = wallet.pubkey();

    // Determine token mint (non-SOL token) and its program
    let (token_mint, token_program) = if pool_info.token_0_mint == WSOL_MINT {
        (&pool_info.token_1_mint, &pool_info.token_1_program)
    } else {
        (&pool_info.token_0_mint, &pool_info.token_0_program)
    };

    // Get associated token accounts with correct program IDs
    let wsol_ata = spl_associated_token_account::get_associated_token_address(
        &wallet_pubkey,
        &Pubkey::from_str(WSOL_MINT)?
    );

    // For Token-2022 tokens, use get_associated_token_address_with_program_id
    let token_ata = if token_program == TOKEN_2022_PROGRAM_ID {
        // Token-2022 ATA calculation
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &wallet_pubkey,
            &Pubkey::from_str(token_mint)?,
            &Pubkey::from_str(token_program)?
        )
    } else {
        // Legacy SPL token ATA calculation
        spl_associated_token_account::get_associated_token_address(
            &wallet_pubkey,
            &Pubkey::from_str(token_mint)?
        )
    };

    // Create WSOL ATA if needed
    let rpc_client = get_rpc_client();
    if !account_exists(&rpc_client, &wsol_ata).await? {
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account(
                &wallet_pubkey,
                &wallet_pubkey,
                &Pubkey::from_str(WSOL_MINT)?,
                &spl_token::id()
            )
        );
    }

    // Create token ATA if needed
    if !account_exists(&rpc_client, &token_ata).await? {
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account(
                &wallet_pubkey,
                &wallet_pubkey,
                &Pubkey::from_str(token_mint)?,
                &Pubkey::from_str(token_program)?
            )
        );
    }

    // Wrap SOL if buying tokens
    if !is_sell {
        // Transfer SOL to WSOL ATA
        instructions.push(
            system_instruction::transfer(&wallet_pubkey, &wsol_ata, swap_params.input_amount_raw)
        );

        // Sync native (wrap SOL)
        instructions.push(spl_token::instruction::sync_native(&spl_token::id(), &wsol_ata)?);
    }

    // Add the actual swap instruction
    let swap_instruction = build_cpmm_swap_instruction(
        &wallet_pubkey,
        pool_info,
        &wsol_ata,
        &token_ata,
        swap_params,
        is_sell
    )?;
    instructions.push(swap_instruction);

    // Unwrap SOL if selling tokens
    if is_sell {
        // Close WSOL account to get SOL back
        instructions.push(
            spl_token::instruction::close_account(
                &spl_token::id(),
                &wsol_ata,
                &wallet_pubkey,
                &wallet_pubkey,
                &[]
            )?
        );
    }

    // Get recent blockhash
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    // Create and sign transaction
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&wallet_pubkey),
        &[wallet],
        recent_blockhash
    );

    Ok(transaction)
}

/// Build the Raydium CPMM swap instruction
fn build_cpmm_swap_instruction(
    user: &Pubkey,
    pool_info: &RaydiumCpmmPoolInfo,
    wsol_ata: &Pubkey,
    token_ata: &Pubkey,
    swap_params: &SwapParams,
    is_sell: bool
) -> Result<Instruction, Box<dyn std::error::Error>> {
    // SwapBaseInput instruction discriminator calculated from SHA256("global:swap_base_input")
    // 8fbe5adac41e33de7fd664ed224c46a877892e28fe513659f68296f7079c123d -> first 8 bytes
    let mut instruction_data = vec![0x8f, 0xbe, 0x5a, 0xda, 0xc4, 0x1e, 0x33, 0xde];

    // Add swap parameters to instruction data
    instruction_data.extend_from_slice(&swap_params.input_amount_raw.to_le_bytes());
    instruction_data.extend_from_slice(&swap_params.minimum_output_raw.to_le_bytes());

    // Determine input/output vaults and token accounts based on swap direction
    let (
        input_vault,
        output_vault,
        input_token_account,
        output_token_account,
        input_mint,
        output_mint,
        input_program,
        output_program,
    ) = if is_sell {
        // Selling token for SOL
        (
            Pubkey::from_str(&pool_info.token_1_vault)?, // Token vault
            Pubkey::from_str(&pool_info.token_0_vault)?, // SOL vault
            *token_ata, // User token account
            *wsol_ata, // User WSOL account
            Pubkey::from_str(&pool_info.token_1_mint)?, // Token mint
            Pubkey::from_str(&pool_info.token_0_mint)?, // SOL mint
            Pubkey::from_str(&pool_info.token_1_program)?, // Token program
            Pubkey::from_str(&pool_info.token_0_program)?, // SOL program
        )
    } else {
        // Buying token with SOL
        (
            Pubkey::from_str(&pool_info.token_0_vault)?, // SOL vault
            Pubkey::from_str(&pool_info.token_1_vault)?, // Token vault
            *wsol_ata, // User WSOL account
            *token_ata, // User token account
            Pubkey::from_str(&pool_info.token_0_mint)?, // SOL mint
            Pubkey::from_str(&pool_info.token_1_mint)?, // Token mint
            Pubkey::from_str(&pool_info.token_0_program)?, // SOL program
            Pubkey::from_str(&pool_info.token_1_program)?, // Token program
        )
    };

    // Authority PDA (derived from "vault_and_lp_mint_auth_seed" seed in CPMM program)
    let authority = Pubkey::find_program_address(
        &[b"vault_and_lp_mint_auth_seed"],
        &Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID)?
    ).0;

    // For now, use pool address as pool_id (which it should be)
    let pool_pubkey = Pubkey::from_str(&pool_info.pool_id)?;

    // Use real fields from pool_info instead of placeholders
    let amm_config = Pubkey::from_str(&pool_info.amm_config)?;
    let observation_key = Pubkey::from_str(&pool_info.observation_key)?;

    // Build accounts according to Raydium CPMM swap instruction format
    let accounts = vec![
        AccountMeta::new_readonly(*user, true), // payer (signer)
        AccountMeta::new_readonly(authority, false), // authority
        AccountMeta::new_readonly(amm_config, false), // amm_config
        AccountMeta::new(pool_pubkey, false), // pool_state
        AccountMeta::new(input_token_account, false), // input_token_account
        AccountMeta::new(output_token_account, false), // output_token_account
        AccountMeta::new(input_vault, false), // input_vault
        AccountMeta::new(output_vault, false), // output_vault
        AccountMeta::new_readonly(input_program, false), // input_token_program
        AccountMeta::new_readonly(output_program, false), // output_token_program
        AccountMeta::new_readonly(input_mint, false), // input_token_mint
        AccountMeta::new_readonly(output_mint, false), // output_token_mint
        AccountMeta::new(observation_key, false) // observation_state
    ];

    Ok(Instruction {
        program_id: Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID)?,
        accounts,
        data: instruction_data,
    })
}

/// Helper functions
async fn account_exists(
    rpc_client: &screenerbot::rpc::RpcClient,
    pubkey: &Pubkey
) -> Result<bool, Box<dyn std::error::Error>> {
    match rpc_client.get_account(pubkey).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

async fn get_token_account_balance(
    account_address: &str
) -> Result<u64, Box<dyn std::error::Error>> {
    let rpc_client = get_rpc_client();
    let account_pubkey = Pubkey::from_str(account_address)?;
    let account = rpc_client
        .get_account(&account_pubkey).await
        .map_err(|e| format!("Failed to get token account: {}", e))?;

    // Decode token account amount (at offset 64)
    if account.data.len() < 72 {
        return Err("Invalid token account data".into());
    }

    let amount_bytes = &account.data[64..72];
    let amount = u64::from_le_bytes(amount_bytes.try_into()?);
    Ok(amount)
}

fn read_pubkey_at_offset(
    data: &[u8],
    offset: &mut usize
) -> Result<String, Box<dyn std::error::Error>> {
    if *offset + 32 > data.len() {
        return Err("Insufficient data for pubkey".into());
    }

    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;

    let pubkey = Pubkey::new_from_array(pubkey_bytes.try_into()?);
    Ok(pubkey.to_string())
}

fn print_help() {
    println!("Raydium CPMM Direct Swap Tool");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin raydium_cpmm_direct_swap [FLAGS] [OPTIONS]");
    println!();
    println!("REQUIRED OPTIONS:");
    println!("    --pool <ADDRESS>       Pool address to swap in");
    println!("    --token <MINT>         Token mint address");
    println!("    --amount <SOL>         Amount in SOL to swap");
    println!();
    println!("OPTIONAL FLAGS:");
    println!("    --sell                 Sell tokens for SOL (default: buy tokens with SOL)");
    println!("    --dry-run              Simulate the swap without executing");
    println!("    --slippage <BPS>       Slippage tolerance in basis points (default: 100 = 1%)");
    println!("    --help, -h             Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    # Buy tokens with 0.1 SOL");
    println!(
        "    cargo run --bin raydium_cpmm_direct_swap --pool 2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ --token 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t --amount 0.1"
    );
    println!();
    println!("    # Sell tokens worth 0.05 SOL");
    println!(
        "    cargo run --bin raydium_cpmm_direct_swap --pool 2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ --token 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t --amount 0.05 --sell"
    );
    println!();
    println!("    # Dry run with custom slippage");
    println!(
        "    cargo run --bin raydium_cpmm_direct_swap --pool 2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ --token 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t --amount 0.1 --slippage 50 --dry-run"
    );
}
