/// Raydium CPMM Direct Swap Implementation
/// Direct on-chain interaction with Raydium Constant Product Market Maker pools
/// Bypasses Jupiter/GMGN aggregators for direct pool access
/// 
/// Based on Raydium CPMM parameters from your research:
/// - Uses swap_base_input instruction
/// - Pool: 2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ
/// - Token: 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t

use crate::tokens::Token;
use crate::rpc::{SwapError, get_rpc_client, sol_to_lamports};
use crate::logger::{log, LogTag};
use crate::global::is_debug_swap_enabled;
use crate::utils::get_wallet_address;
use super::config::{SOL_MINT, TRANSACTION_CONFIRMATION_MAX_ATTEMPTS, TRANSACTION_CONFIRMATION_RETRY_DELAY_MS};

use solana_sdk::{
    instruction::{Instruction, AccountMeta},
    pubkey::Pubkey,
    transaction::{VersionedTransaction, Transaction},
    message::{v0::Message as MessageV0, VersionedMessage, Message},
    signer::Signer,
    system_instruction,
    compute_budget::ComputeBudgetInstruction,
};
use spl_token;
use spl_associated_token_account;
use std::str::FromStr;
use base64;
use bincode;

/// Raydium CPMM program IDs and constants
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"; // Mainnet
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"; // Token 2022 program
pub const RAYDIUM_CPMM_AUTHORITY_SEED: &str = "pool_authority";

/// Test pool configuration for SOL/5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t
/// Pool address: 2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ
pub const TEST_POOL_ADDRESS: &str = "2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ";
pub const TEST_TOKEN_MINT: &str = "5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t";

/// Note: These pool account addresses should be fetched dynamically from the pool state
/// For now, we'll implement a basic version that needs these addresses to be correct
/// TODO: Implement dynamic pool account fetching from on-chain data
/// 
/// IMPORTANT: These are placeholder addresses that MUST be replaced with actual pool accounts
/// The error "incorrect program id for instruction" indicates these addresses are wrong
pub const TEST_POOL_CONFIG: &str = "7YttLkHDoNj9wyDur5pM1ejNaAvT9X4eqaYcHQqtj2G5"; // TODO: Get real config
pub const TEST_POOL_VAULT_A: &str = "So11111111111111111111111111111111111111112"; // TODO: Get real WSOL vault
pub const TEST_POOL_VAULT_B: &str = "5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t"; // TODO: Get real token vault  
pub const TEST_OBSERVATION_STATE: &str = "11111111111111111111111111111111"; // TODO: Get real observation account

/// Raydium CPMM swap parameters for direct pool interaction
#[derive(Clone, Debug)]
pub struct RaydiumCpmmSwapParams {
    pub pool: Pubkey,                    // Pool address
    pub amm_config: Pubkey,              // AMM config account
    pub pool_authority: Pubkey,          // Pool authority PDA
    pub vault_a: Pubkey,                 // Token A vault
    pub vault_b: Pubkey,                 // Token B vault
    pub mint_a: Pubkey,                  // Token A mint
    pub mint_b: Pubkey,                  // Token B mint
    pub user_source_ata: Pubkey,         // User source token account
    pub user_dest_ata: Pubkey,           // User destination token account
    pub observation_state: Pubkey,       // Observation state account
    pub amount_in: u64,                  // Input amount
    pub min_amount_out: u64,             // Minimum output amount (slippage protection)
    pub wrap_sol: bool,                  // Whether to wrap SOL
    pub cu_price_micro_lamports: Option<u64>,
    pub cu_limit: Option<u32>,
}

/// Raydium CPMM swap result
#[derive(Debug)]
pub struct RaydiumCpmmSwapResult {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub input_amount: String,
    pub output_amount: String,
    pub price_impact: String,
    pub fee_lamports: u64,
    pub execution_time: f64,
    pub effective_price: Option<f64>,
    pub error: Option<String>,
}

/// Execute Raydium CPMM direct swap
pub async fn execute_raydium_cpmm_swap(params: RaydiumCpmmSwapParams) -> Result<RaydiumCpmmSwapResult, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_CPMM_START",
            &format!(
                "🔵 Starting Raydium CPMM direct swap:\n  • Pool: {}\n  • Amount In: {}\n  • Min Out: {}\n  • Wrap SOL: {}",
                &params.pool.to_string()[..8],
                params.amount_in,
                params.min_amount_out,
                params.wrap_sol
            )
        );
    }

    let start_time = std::time::Instant::now();
    let wallet_address = get_wallet_address()?;
    let payer = Pubkey::from_str(&wallet_address)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid wallet: {}", e)))?;

    // Build transaction instructions
    let mut instructions: Vec<Instruction> = vec![];

    // 1. Add compute budget instructions
    if let Some(limit) = params.cu_limit {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(limit));
    }
    if let Some(price) = params.cu_price_micro_lamports {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_price(price));
    }

    // 2. Create ATAs if needed
    let rpc_client = get_rpc_client();
    
    // Check if source ATA exists
    match rpc_client.get_account(&params.user_source_ata).await {
        Err(_) => {
            if is_debug_swap_enabled() {
                log(LogTag::Swap, "RAYDIUM_CPMM_CREATE_SOURCE_ATA", "Creating source ATA");
            }
            instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer,
                    &payer,
                    &params.mint_a,
                    &spl_token::id(),
                )
            );
        }
        Ok(_) => {
            if is_debug_swap_enabled() {
                log(LogTag::Swap, "RAYDIUM_CPMM_SOURCE_ATA_EXISTS", "Source ATA already exists");
            }
        }
    }

    // Check if destination ATA exists
    match rpc_client.get_account(&params.user_dest_ata).await {
        Err(_) => {
            if is_debug_swap_enabled() {
                log(LogTag::Swap, "RAYDIUM_CPMM_CREATE_DEST_ATA", "Creating destination ATA");
            }
            instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer,
                    &payer,
                    &params.mint_b,
                    &spl_token::id(),
                )
            );
        }
        Ok(_) => {
            if is_debug_swap_enabled() {
                log(LogTag::Swap, "RAYDIUM_CPMM_DEST_ATA_EXISTS", "Destination ATA already exists");
            }
        }
    }

    // 3. Wrap SOL if needed
    if params.wrap_sol && params.mint_a.to_string() == SOL_MINT {
        if is_debug_swap_enabled() {
            log(LogTag::Swap, "RAYDIUM_CPMM_WRAP_SOL", "Adding SOL wrap instruction");
        }
        
        // Transfer SOL to WSOL account
        instructions.push(
            system_instruction::transfer(
                &payer,
                &params.user_source_ata,
                params.amount_in,
            )
        );
        
        // Sync native
        instructions.push(
            spl_token::instruction::sync_native(
                &spl_token::id(),
                &params.user_source_ata,
            ).map_err(|e| SwapError::TransactionError(format!("Failed to create sync_native instruction: {}", e)))?
        );
    }

    // 4. Build Raydium CPMM swap instruction
    let swap_instruction = build_raydium_cpmm_swap_instruction(&params)?;
    instructions.push(swap_instruction);

    // 5. Unwrap SOL if needed
    if params.wrap_sol && params.mint_b.to_string() == SOL_MINT {
        if is_debug_swap_enabled() {
            log(LogTag::Swap, "RAYDIUM_CPMM_UNWRAP_SOL", "Adding SOL unwrap instruction");
        }
        
        // Close WSOL account to recover SOL
        instructions.push(
            spl_token::instruction::close_account(
                &spl_token::id(),
                &params.user_dest_ata,
                &payer,
                &payer,
                &[],
            ).map_err(|e| SwapError::TransactionError(format!("Failed to create close_account instruction: {}", e)))?
        );
    }

    // 6. Build and send transaction using legacy Transaction approach
    use super::execution::sign_and_send_transaction;
    
    let recent_blockhash = rpc_client.get_latest_blockhash().await
        .map_err(|e| SwapError::TransactionError(format!("Failed to get latest blockhash: {}", e)))?;

    // Build a message with the recent blockhash
    let message = Message::new_with_blockhash(&instructions, Some(&payer), &recent_blockhash);

    // Build an unsigned transaction
    let transaction = Transaction::new_unsigned(message);

    // 7. Serialize and send transaction directly
    let transaction_bytes = bincode::serialize(&transaction)
        .map_err(|e| SwapError::TransactionError(format!("Failed to serialize transaction: {}", e)))?;
    let transaction_base64 = base64::encode(transaction_bytes);
    
    let signature = sign_and_send_transaction(&transaction_base64).await
        .map_err(|e| SwapError::TransactionError(format!("Failed to send transaction: {}", e)))?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_CPMM_SENT",
            &format!("📤 Raydium CPMM transaction sent: {}", signature)
        );
    }

    // 8. Confirm transaction
    let confirmed = rpc_client.wait_for_transaction_confirmation_smart(
        &signature,
        TRANSACTION_CONFIRMATION_MAX_ATTEMPTS,
        TRANSACTION_CONFIRMATION_RETRY_DELAY_MS
    ).await.map_err(|e| SwapError::TransactionError(format!("Transaction confirmation failed: {}", e)))?;

    if !confirmed {
        return Err(SwapError::TransactionError("Transaction confirmation timeout".to_string()));
    }

    let execution_time = start_time.elapsed().as_secs_f64();

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_CPMM_SUCCESS",
            &format!("✅ Raydium CPMM swap completed in {:.2}s: {}", execution_time, signature)
        );
    }

    Ok(RaydiumCpmmSwapResult {
        success: true,
        transaction_signature: Some(signature),
        input_amount: params.amount_in.to_string(),
        output_amount: "0".to_string(), // Will be calculated by post-transaction analysis
        price_impact: "0.0".to_string(), // Will be calculated by post-transaction analysis
        fee_lamports: 0, // Will be calculated by post-transaction analysis
        execution_time,
        effective_price: None, // Will be calculated by post-transaction analysis
        error: None,
    })
}

/// Build Raydium CPMM swap instruction manually
/// This implements the swap_base_input instruction format for Raydium CPMM
fn build_raydium_cpmm_swap_instruction(params: &RaydiumCpmmSwapParams) -> Result<Instruction, SwapError> {
    let program_id = Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid program ID: {}", e)))?;

    // Get payer from wallet
    let wallet_address = get_wallet_address()?;
    let payer = Pubkey::from_str(&wallet_address)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid wallet: {}", e)))?;

    // Build accounts for Raydium CPMM swap_base_input instruction
    // Based on Raydium CPMM documentation and CPI examples
    let accounts = vec![
        AccountMeta::new_readonly(payer, true),                           // payer (signer)
        AccountMeta::new_readonly(params.pool_authority, false),          // authority
        AccountMeta::new_readonly(params.amm_config, false),              // amm_config
        AccountMeta::new(params.pool, false),                             // pool_state
        AccountMeta::new(params.user_source_ata, false),                  // input_token_account
        AccountMeta::new(params.user_dest_ata, false),                    // output_token_account
        AccountMeta::new(params.vault_a, false),                          // input_vault
        AccountMeta::new(params.vault_b, false),                          // output_vault
        AccountMeta::new_readonly(params.mint_a, false),                  // input_token_mint
        AccountMeta::new_readonly(params.mint_b, false),                  // output_token_mint
        AccountMeta::new_readonly(spl_token::id(), false),                // token_program (SPL Token)
        AccountMeta::new(params.observation_state, false),                // observation_state
    ];

    // Create instruction data for swap_base_input
    // Instruction discriminator + amount_in + minimum_amount_out
    // Based on Raydium CPMM instruction layout
    let mut instruction_data = Vec::new();
    
    // Add instruction discriminator for swap_base_input (8 bytes)
    // Calculated as sha256("global:swap_base_input")[:8] = [143, 190, 90, 218, 196, 30, 51, 222]
    instruction_data.extend_from_slice(&[143, 190, 90, 218, 196, 30, 51, 222]);
    
    // Add amount_in (8 bytes, little endian)
    instruction_data.extend_from_slice(&params.amount_in.to_le_bytes());
    
    // Add minimum_amount_out (8 bytes, little endian)
    instruction_data.extend_from_slice(&params.min_amount_out.to_le_bytes());

    Ok(Instruction {
        program_id,
        accounts,
        data: instruction_data,
    })
}

/// Get test pool configuration for SOL/5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t
pub fn get_test_pool_config(
    user_wallet: &Pubkey,
    amount_in: u64,
    min_amount_out: u64,
) -> Result<RaydiumCpmmSwapParams, SwapError> {
    let pool = Pubkey::from_str(TEST_POOL_ADDRESS)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid pool address: {}", e)))?;
    
    let mint_a = Pubkey::from_str(SOL_MINT)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid SOL mint: {}", e)))?;
    
    let mint_b = Pubkey::from_str(TEST_TOKEN_MINT)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid token mint: {}", e)))?;

    // Derive pool authority PDA
    let (pool_authority, _bump) = Pubkey::find_program_address(
        &[RAYDIUM_CPMM_AUTHORITY_SEED.as_bytes(), pool.as_ref()],
        &Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).unwrap(),
    );

    // Use hardcoded test pool configuration
    let amm_config = Pubkey::from_str(TEST_POOL_CONFIG)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid config address: {}", e)))?;
    
    let vault_a = Pubkey::from_str(TEST_POOL_VAULT_A)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid vault A address: {}", e)))?;
    
    let vault_b = Pubkey::from_str(TEST_POOL_VAULT_B)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid vault B address: {}", e)))?;

    let observation_state = Pubkey::from_str(TEST_OBSERVATION_STATE)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid observation state address: {}", e)))?;

    // Get user's ATAs
    let user_source_ata = spl_associated_token_account::get_associated_token_address(
        user_wallet,
        &mint_a,
    );
    
    let user_dest_ata = spl_associated_token_account::get_associated_token_address(
        user_wallet,
        &mint_b,
    );

    Ok(RaydiumCpmmSwapParams {
        pool,
        amm_config,
        pool_authority,
        vault_a,
        vault_b,
        mint_a,
        mint_b,
        user_source_ata,
        user_dest_ata,
        observation_state,
        amount_in,
        min_amount_out,
        wrap_sol: true, // SOL to token swap requires wrapping
        cu_price_micro_lamports: Some(1_000), // 1000 micro-lamports
        cu_limit: Some(1_000_000), // 1M compute units
    })
}

/// Execute test swap for SOL to 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t
pub async fn execute_test_raydium_cpmm_swap(
    sol_amount: f64,
    slippage_percent: f64,
) -> Result<RaydiumCpmmSwapResult, SwapError> {
    let wallet_address = get_wallet_address()?;
    let user_wallet = Pubkey::from_str(&wallet_address)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid wallet: {}", e)))?;

    let amount_in = sol_to_lamports(sol_amount);
    let min_amount_out = 0; // For now, accept any amount (you'd calculate this based on slippage)

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_CPMM_TEST",
            &format!(
                "🧪 Testing Raydium CPMM swap:\n  • SOL Amount: {:.6}\n  • Slippage: {:.1}%\n  • Pool: {}",
                sol_amount,
                slippage_percent,
                TEST_POOL_ADDRESS
            )
        );
    }

    let params = get_test_pool_config(&user_wallet, amount_in, min_amount_out)?;
    execute_raydium_cpmm_swap(params).await
}
