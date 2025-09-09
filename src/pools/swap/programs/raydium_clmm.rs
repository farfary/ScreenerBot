/// Raydium CLMM (Concentrated Liquidity Market Maker) swap implementation
///
/// This module implements direct swaps for Raydium Concentrated Liquidity pools.
/// It integrates with the centralized Raydium CLMM decoder and provides proper
/// account derivation and swap calculations based on the Uniswap V3 model.

use super::ProgramSwap;
use crate::pools::swap::types::{
    SwapRequest,
    SwapResult,
    SwapDirection,
    SwapError,
    SwapParams,
    constants::*,
};
use crate::pools::swap::executor::SwapExecutor;
use crate::pools::AccountData;
use crate::pools::decoders::raydium_clmm::{RaydiumClmmDecoder, ClmmPoolInfo};
use crate::pools::types::RAYDIUM_CLMM_PROGRAM_ID;
use crate::rpc::{get_rpc_client, sol_to_lamports};
use crate::configs::{read_configs, load_wallet_from_config};
use crate::logger::{log, LogTag};

use solana_sdk::{
    instruction::{Instruction, AccountMeta},
    pubkey::Pubkey,
    transaction::Transaction,
    signature::{Keypair, Signer},
    system_instruction,
};
use spl_token;
use spl_associated_token_account;
use std::str::FromStr;
use std::collections::HashMap;

/// Raydium CLMM program instructions
const SWAP_V2_DISCRIMINATOR: [u8; 8] = [0x3f, 0x2a, 0xd9, 0xe2, 0xd1, 0x5d, 0xf7, 0x8b];

/// Raydium CLMM swap implementation
pub struct RaydiumClmmSwap;

impl ProgramSwap for RaydiumClmmSwap {
    async fn execute_swap(
        request: SwapRequest,
        pool_data: AccountData,
    ) -> Result<SwapResult, SwapError> {
        log(
            LogTag::System,
            "INFO",
            &format!("🟣 Starting Raydium CLMM {:?} swap", request.direction),
        );

        // Decode pool state using centralized decoder
        let pool_info = Self::decode_pool_state(&pool_data)?;

        // Load wallet
        let wallet = Self::load_wallet().await?;

        // Calculate swap parameters using CLMM math
        let swap_params = Self::calculate_clmm_swap_params(&request, &pool_info).await?;

        log(
            LogTag::System,
            "INFO",
            &format!(
                "💡 CLMM Swap: {} → {} (min output: {})",
                swap_params.input_amount,
                swap_params.expected_output,
                swap_params.minimum_output
            ),
        );

        // Build transaction with proper account derivation
        let transaction = Self::build_clmm_swap_transaction(
            &wallet,
            &request,
            &pool_info,
            &swap_params,
            &pool_data,
        ).await?;

        // Execute transaction
        SwapExecutor::execute_transaction(transaction, swap_params, request.dry_run).await
    }
}

impl RaydiumClmmSwap {
    /// Decode pool state using the centralized decoder
    fn decode_pool_state(pool_data: &AccountData) -> Result<ClmmPoolInfo, SwapError> {
        // Create accounts map for decoder
        let mut accounts = HashMap::new();
        accounts.insert(pool_data.pubkey.to_string(), pool_data.clone());

        // Use the centralized decoder to extract pool data
        RaydiumClmmDecoder::extract_pool_data(&accounts)
            .ok_or_else(|| SwapError::DecoderError("Failed to decode Raydium CLMM pool".to_string()))
    }

    /// Load wallet from configuration
    async fn load_wallet() -> Result<Keypair, SwapError> {
        let configs = read_configs().map_err(|e| {
            SwapError::ExecutionError(format!("Failed to load config: {}", e))
        })?;
        let wallet = load_wallet_from_config(&configs).map_err(|e| {
            SwapError::ExecutionError(format!("Failed to load wallet: {}", e))
        })?;
        Ok(wallet)
    }

    /// Calculate swap parameters using CLMM concentrated liquidity math
    async fn calculate_clmm_swap_params(
        request: &SwapRequest,
        pool_info: &ClmmPoolInfo,
    ) -> Result<SwapParams, SwapError> {
        let rpc_client = get_rpc_client();

        // Get vault balances
        let vault_0_balance = Self::get_token_account_balance(&pool_info.token_vault_0).await?;
        let vault_1_balance = Self::get_token_account_balance(&pool_info.token_vault_1).await?;

        log(
            LogTag::System,
            "INFO",
            &format!(
                "📊 CLMM Vault balances - Vault0: {}, Vault1: {}, Current tick: {}, Price: {:.12}",
                vault_0_balance,
                vault_1_balance,
                pool_info.tick_current,
                Self::sqrt_price_x64_to_price(pool_info.sqrt_price_x64)
            ),
        );

        // Determine which token is SOL and get current price
        let (sol_mint, token_mint, sol_decimals, token_decimals, is_token_0_sol) = if pool_info.token_mint_0 == WSOL_MINT {
            (WSOL_MINT, &pool_info.token_mint_1, 9, pool_info.mint_decimals_1, true)
        } else if pool_info.token_mint_1 == WSOL_MINT {
            (WSOL_MINT, &pool_info.token_mint_0, 9, pool_info.mint_decimals_0, false)
        } else {
            return Err(SwapError::InvalidPool("Pool does not contain SOL".to_string()));
        };

        // Convert sqrt_price_x64 to actual price
        let sqrt_price = Self::sqrt_price_x64_to_price(pool_info.sqrt_price_x64);
        let current_price = sqrt_price * sqrt_price;

        // Calculate swap amounts based on CLMM pricing
        let (input_amount, expected_output, input_amount_raw, minimum_output_raw) = match request.direction {
            SwapDirection::Buy => {
                // Buying tokens with SOL
                let sol_amount = request.amount;
                let sol_amount_raw = sol_to_lamports(sol_amount);
                
                // In CLMM, we use the current price for estimation
                // The actual execution will use the concentrated liquidity
                let token_amount = if is_token_0_sol {
                    // SOL is token_0, token is token_1
                    // price = token_1/token_0, so tokens = SOL / price
                    sol_amount / current_price
                } else {
                    // SOL is token_1, token is token_0
                    // price = token_0/token_1, so tokens = SOL * price
                    sol_amount * current_price
                };

                let token_amount_raw = (token_amount * 10_f64.powi(token_decimals as i32)) as u64;
                let minimum_token_raw = (token_amount_raw as f64 * (1.0 - (request.slippage_bps as f64 / 10000.0))) as u64;

                (sol_amount, token_amount, sol_amount_raw, minimum_token_raw)
            }
            SwapDirection::Sell => {
                // Selling tokens for SOL
                let token_amount = request.amount;
                let token_amount_raw = (token_amount * 10_f64.powi(token_decimals as i32)) as u64;
                
                // Calculate expected SOL output
                let sol_amount = if is_token_0_sol {
                    // SOL is token_0, token is token_1
                    // price = token_1/token_0, so SOL = tokens * price
                    token_amount * current_price
                } else {
                    // SOL is token_1, token is token_0
                    // price = token_0/token_1, so SOL = tokens / price
                    token_amount / current_price
                };

                let sol_amount_raw = sol_to_lamports(sol_amount);
                let minimum_sol_raw = (sol_amount_raw as f64 * (1.0 - (request.slippage_bps as f64 / 10000.0))) as u64;

                (token_amount, sol_amount, token_amount_raw, minimum_sol_raw)
            }
        };

        Ok(SwapParams {
            input_amount,
            expected_output,
            minimum_output: minimum_output_raw as f64 / 10_f64.powi(match request.direction {
                SwapDirection::Buy => token_decimals as i32,
                SwapDirection::Sell => sol_decimals as i32,
            }),
            input_amount_raw,
            minimum_output_raw,
        })
    }

    /// Build the complete CLMM swap transaction
    async fn build_clmm_swap_transaction(
        wallet: &Keypair,
        request: &SwapRequest,
        pool_info: &ClmmPoolInfo,
        swap_params: &SwapParams,
        pool_data: &AccountData,
    ) -> Result<Transaction, SwapError> {
        let mut instructions = Vec::new();
        let wallet_pubkey = wallet.pubkey();

        // Determine token mint and programs
        let (token_mint, token_program, is_token_0_sol) = if pool_info.token_mint_0 == WSOL_MINT {
            (&pool_info.token_mint_1, &spl_token::id(), false)
        } else if pool_info.token_mint_1 == WSOL_MINT {
            (&pool_info.token_mint_0, &spl_token::id(), true)
        } else {
            return Err(SwapError::InvalidPool("Pool does not contain SOL".to_string()));
        };

        // Get associated token accounts
        let wsol_ata = spl_associated_token_account::get_associated_token_address(
            &wallet_pubkey,
            &Pubkey::from_str(WSOL_MINT).unwrap(),
        );

        let token_ata = spl_associated_token_account::get_associated_token_address(
            &wallet_pubkey,
            &Pubkey::from_str(token_mint).unwrap(),
        );

        // Create token accounts if needed
        if !Self::account_exists(&wsol_ata).await? {
            let create_wsol_ix = spl_associated_token_account::instruction::create_associated_token_account(
                &wallet_pubkey,
                &wallet_pubkey,
                &Pubkey::from_str(WSOL_MINT).unwrap(),
                &spl_token::id(),
            );
            instructions.push(create_wsol_ix);
        }

        if !Self::account_exists(&token_ata).await? {
            let create_token_ix = spl_associated_token_account::instruction::create_associated_token_account(
                &wallet_pubkey,
                &wallet_pubkey,
                &Pubkey::from_str(token_mint).unwrap(),
                token_program,
            );
            instructions.push(create_token_ix);
        }

        // Handle WSOL wrapping for buy operations
        if request.direction == SwapDirection::Buy {
            let transfer_ix = system_instruction::transfer(
                &wallet_pubkey,
                &wsol_ata,
                swap_params.input_amount_raw,
            );
            instructions.push(transfer_ix);

            let sync_native_ix = spl_token::instruction::sync_native(&spl_token::id(), &wsol_ata)?;
            instructions.push(sync_native_ix);
        }

        // Build the actual CLMM swap instruction
        let swap_ix = Self::build_clmm_swap_instruction(
            &wallet_pubkey,
            pool_info,
            &wsol_ata,
            &token_ata,
            request.direction,
            swap_params,
            is_token_0_sol,
            &pool_data.pubkey, // Pass the actual pool address
        ).await?;
        instructions.push(swap_ix);

        // Handle WSOL unwrapping
        let close_wsol_ix = spl_token::instruction::close_account(
            &spl_token::id(),
            &wsol_ata,
            &wallet_pubkey,
            &wallet_pubkey,
            &[],
        )?;
        instructions.push(close_wsol_ix);

        // Create transaction
        let rpc_client = get_rpc_client();
        let recent_blockhash = rpc_client
            .get_latest_blockhash()
            .await
            .map_err(|e| SwapError::RpcError(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_with_payer(&instructions, Some(&wallet_pubkey));
        let mut transaction_with_blockhash = transaction;
        transaction_with_blockhash.message.recent_blockhash = recent_blockhash;

        Ok(transaction_with_blockhash)
    }

    /// Build the Raydium CLMM swap instruction
    async fn build_clmm_swap_instruction(
        user: &Pubkey,
        pool_info: &ClmmPoolInfo,
        wsol_ata: &Pubkey,
        token_ata: &Pubkey,
        direction: SwapDirection,
        swap_params: &SwapParams,
        is_token_0_sol: bool,
        pool_address: &Pubkey, // Pass the actual pool address from AccountData
    ) -> Result<Instruction, SwapError> {
        // Use the passed pool address
        let amm_config = Pubkey::from_str(&pool_info.amm_config)
            .map_err(|e| SwapError::TransactionError(format!("Invalid amm_config: {}", e)))?;
        let observation_key = Pubkey::from_str(&pool_info.observation_key)
            .map_err(|e| SwapError::TransactionError(format!("Invalid observation_key: {}", e)))?;

        // Token vaults
        let token_vault_0 = Pubkey::from_str(&pool_info.token_vault_0)
            .map_err(|e| SwapError::TransactionError(format!("Invalid token_vault_0: {}", e)))?;
        let token_vault_1 = Pubkey::from_str(&pool_info.token_vault_1)
            .map_err(|e| SwapError::TransactionError(format!("Invalid token_vault_1: {}", e)))?;

        // Determine input/output accounts based on direction and token orientation
        let (input_token_account, output_token_account, input_vault, output_vault) = match (direction, is_token_0_sol) {
            (SwapDirection::Buy, true) => {
                // Buying tokens with SOL, SOL is token_0
                (wsol_ata, token_ata, &token_vault_0, &token_vault_1)
            }
            (SwapDirection::Buy, false) => {
                // Buying tokens with SOL, SOL is token_1
                (wsol_ata, token_ata, &token_vault_1, &token_vault_0)
            }
            (SwapDirection::Sell, true) => {
                // Selling tokens for SOL, SOL is token_0
                (token_ata, wsol_ata, &token_vault_1, &token_vault_0)
            }
            (SwapDirection::Sell, false) => {
                // Selling tokens for SOL, SOL is token_1
                (token_ata, wsol_ata, &token_vault_0, &token_vault_1)
            }
        };

        // Build instruction data
        let mut instruction_data = SWAP_V2_DISCRIMINATOR.to_vec();
        instruction_data.extend_from_slice(&swap_params.input_amount_raw.to_le_bytes());
        instruction_data.extend_from_slice(&swap_params.minimum_output_raw.to_le_bytes());
        
        // sqrt_price_limit_x64 - set to 0 for no limit
        instruction_data.extend_from_slice(&0u128.to_le_bytes());
        
        // is_base_input - true for exact input swaps
        instruction_data.push(1u8);

        let accounts = vec![
            AccountMeta::new_readonly(Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).unwrap(), false),
            AccountMeta::new(*pool_address, false),
            AccountMeta::new_readonly(amm_config, false),
            AccountMeta::new_readonly(observation_key, false),
            AccountMeta::new(*input_token_account, false),
            AccountMeta::new(*output_token_account, false),
            AccountMeta::new(*input_vault, false),
            AccountMeta::new(*output_vault, false),
            AccountMeta::new_readonly(*user, true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        Ok(Instruction {
            program_id: Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).unwrap(),
            accounts,
            data: instruction_data,
        })
    }

    /// Helper functions
    async fn account_exists(pubkey: &Pubkey) -> Result<bool, SwapError> {
        let rpc_client = get_rpc_client();
        match rpc_client.get_account(pubkey).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_token_account_balance(account_address: &str) -> Result<u64, SwapError> {
        let rpc_client = get_rpc_client();
        let pubkey = Pubkey::from_str(account_address)
            .map_err(|e| SwapError::RpcError(format!("Invalid account address: {}", e)))?;

        let account = rpc_client
            .get_account(&pubkey)
            .await
            .map_err(|e| SwapError::RpcError(format!("Failed to fetch account: {}", e)))?;

        // Parse token account data to get amount
        if account.data.len() >= 72 {
            let amount_bytes: [u8; 8] = account.data[64..72]
                .try_into()
                .map_err(|_| SwapError::RpcError("Invalid token account data".to_string()))?;
            Ok(u64::from_le_bytes(amount_bytes))
        } else {
            Err(SwapError::RpcError("Account data too short".to_string()))
        }
    }

    /// Convert sqrt_price_x64 to normal price
    fn sqrt_price_x64_to_price(sqrt_price_x64: u128) -> f64 {
        let sqrt_price = (sqrt_price_x64 as f64) / (2_f64.powi(64));
        sqrt_price
    }
}
