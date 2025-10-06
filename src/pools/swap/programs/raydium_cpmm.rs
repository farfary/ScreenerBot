/// Raydium CPMM swap implementation
///
/// This module implements direct swaps for Raydium Constant Product Market Maker pools.
/// It integrates with the centralized Raydium CPMM decoder and provides both buy and sell operations.
use super::ProgramSwap;
use crate::logger::{log, LogTag};
use crate::pools::decoders::raydium_cpmm::{RaydiumCpmmDecoder, RaydiumCpmmPoolInfo};
use crate::pools::swap::executor::SwapExecutor;
use crate::pools::swap::types::{
    constants::*, SwapDirection, SwapError, SwapParams, SwapRequest, SwapResult,
};
use crate::pools::types::RAYDIUM_CPMM_PROGRAM_ID;
use crate::pools::AccountData;
use crate::rpc::{get_rpc_client, sol_to_lamports};

use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account;
use spl_token;
use std::collections::HashMap;
use std::str::FromStr;

/// Raydium CPMM swap implementation
pub struct RaydiumCpmmSwap;

impl ProgramSwap for RaydiumCpmmSwap {
    async fn execute_swap(
        request: SwapRequest,
        pool_data: AccountData,
    ) -> Result<SwapResult, SwapError> {
        log(
            LogTag::System,
            "INFO",
            &format!("🔄 Executing Raydium CPMM {:?} swap", request.direction),
        );

        // Decode pool state using centralized decoder
        let pool_info = Self::decode_pool_state(&pool_data)?;

        // Load wallet
        let wallet = Self::load_wallet().await?;

        // Calculate swap parameters
        let swap_params = Self::calculate_swap_params(&request, &pool_info).await?;

        log(
            LogTag::System,
            "INFO",
            &format!(
                "💰 Swap calculation: {:.6} → {:.6} (min: {:.6})",
                swap_params.input_amount, swap_params.expected_output, swap_params.minimum_output
            ),
        );

        // Build transaction
        let transaction =
            Self::build_swap_transaction(&wallet, &request, &pool_info, &swap_params).await?;

        // Execute transaction
        SwapExecutor::execute_transaction(transaction, swap_params, request.dry_run).await
    }
}

impl RaydiumCpmmSwap {
    /// Decode pool state using the centralized decoder
    fn decode_pool_state(pool_data: &AccountData) -> Result<RaydiumCpmmPoolInfo, SwapError> {
        // Create accounts map for decoder
        let mut accounts = HashMap::new();
        accounts.insert(pool_data.pubkey.to_string(), pool_data.clone());

        // Use the centralized decoder to decode pool info
        // We need to call the internal decode method directly since we already have the pool account
        RaydiumCpmmDecoder::decode_raydium_cpmm_pool(&pool_data.data, &pool_data.pubkey.to_string())
            .ok_or_else(|| {
                SwapError::DecoderError("Failed to decode Raydium CPMM pool".to_string())
            })
    }

    /// Load wallet from configuration
    async fn load_wallet() -> Result<Keypair, SwapError> {
        crate::config::get_wallet_keypair()
            .map_err(|e| SwapError::ExecutionError(format!("Failed to load wallet: {}", e)))
    }

    /// Calculate swap parameters using constant product formula
    async fn calculate_swap_params(
        request: &SwapRequest,
        pool_info: &RaydiumCpmmPoolInfo,
    ) -> Result<SwapParams, SwapError> {
        let rpc_client = get_rpc_client();

        // Get vault balances
        let vault_0_balance = Self::get_token_account_balance(&pool_info.token_0_vault).await?;
        let vault_1_balance = Self::get_token_account_balance(&pool_info.token_1_vault).await?;

        log(
            LogTag::System,
            "DEBUG",
            &format!(
                "📊 Vault balances: {} = {}, {} = {}",
                pool_info.token_0_vault, vault_0_balance, pool_info.token_1_vault, vault_1_balance
            ),
        );

        // Determine which vault is SOL and which is the token
        let (sol_reserve, token_reserve, token_decimals) = if pool_info.token_0_mint == WSOL_MINT {
            (vault_0_balance, vault_1_balance, pool_info.token_1_decimals)
        } else {
            (vault_1_balance, vault_0_balance, pool_info.token_0_decimals)
        };

        // Calculate using constant product formula: x * y = k
        let (input_amount, expected_output, input_amount_raw, minimum_output_raw) = match request
            .direction
        {
            SwapDirection::Sell => {
                // For selling: user provides token amount, get SOL output
                let token_amount_raw =
                    (request.amount * (10_f64).powi(token_decimals as i32)) as u64;

                // Calculate expected SOL output using constant product formula
                // Use u128 to prevent overflow with large numbers
                let sol_output_raw = ((sol_reserve as u128) * (token_amount_raw as u128))
                    / ((token_reserve as u128) + (token_amount_raw as u128));
                let sol_output = (sol_output_raw as f64) / (10_f64).powi(9); // SOL always has 9 decimals

                // Apply slippage to the calculated output
                let min_sol_output_raw =
                    (sol_output_raw * (10000 - (request.slippage_bps as u128))) / 10000;

                (
                    request.amount,
                    sol_output,
                    token_amount_raw,
                    min_sol_output_raw as u64,
                )
            }
            SwapDirection::Buy => {
                // Buying tokens with SOL: user provides SOL amount, get token output
                let sol_amount_raw = sol_to_lamports(request.amount); // Keep as lamports

                // Use u128 to prevent overflow with large numbers
                let token_output_raw = ((token_reserve as u128) * (sol_amount_raw as u128))
                    / ((sol_reserve as u128) + (sol_amount_raw as u128));
                let token_output = (token_output_raw as f64) / (10_f64).powi(token_decimals as i32);
                let min_token_output_raw =
                    (token_output_raw * (10000 - (request.slippage_bps as u128))) / 10000;

                (
                    request.amount,
                    token_output,
                    sol_amount_raw,
                    min_token_output_raw as u64,
                )
            }
        };

        Ok(SwapParams {
            input_amount,
            expected_output,
            minimum_output: expected_output * (1.0 - (request.slippage_bps as f64) / 10000.0),
            input_amount_raw,
            minimum_output_raw,
        })
    }

    /// Build the complete swap transaction
    async fn build_swap_transaction(
        wallet: &Keypair,
        request: &SwapRequest,
        pool_info: &RaydiumCpmmPoolInfo,
        swap_params: &SwapParams,
    ) -> Result<Transaction, SwapError> {
        let mut instructions = Vec::new();
        let wallet_pubkey = wallet.pubkey();

        // Determine token mint (non-SOL token) and its program
        let (token_mint, token_program) = if pool_info.token_0_mint == WSOL_MINT {
            (&pool_info.token_1_mint, &pool_info.token_1_program)
        } else {
            (&pool_info.token_0_mint, &pool_info.token_0_program)
        };

        // Get associated token accounts
        let wsol_ata = spl_associated_token_account::get_associated_token_address(
            &wallet_pubkey,
            &Pubkey::from_str(WSOL_MINT).unwrap(),
        );

        let token_ata = if token_program == TOKEN_2022_PROGRAM_ID {
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &wallet_pubkey,
                &Pubkey::from_str(token_mint).unwrap(),
                &Pubkey::from_str(token_program).unwrap(),
            )
        } else {
            spl_associated_token_account::get_associated_token_address(
                &wallet_pubkey,
                &Pubkey::from_str(token_mint).unwrap(),
            )
        };

        // Create token accounts if needed
        let rpc_client = get_rpc_client();

        if !Self::account_exists(&wsol_ata).await? {
            instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &wallet_pubkey,
                    &wallet_pubkey,
                    &Pubkey::from_str(WSOL_MINT).unwrap(),
                    &spl_token::id(),
                ),
            );
        }

        if !Self::account_exists(&token_ata).await? {
            let token_program_id = Pubkey::from_str(token_program).unwrap();
            instructions.push(
                spl_associated_token_account::instruction::create_associated_token_account(
                    &wallet_pubkey,
                    &wallet_pubkey,
                    &Pubkey::from_str(token_mint).unwrap(),
                    &token_program_id,
                ),
            );
        }

        // Handle WSOL wrapping for buy operations
        if request.direction == SwapDirection::Buy {
            let wsol_amount = sol_to_lamports(request.amount);

            instructions.push(system_instruction::transfer(
                &wallet_pubkey,
                &wsol_ata,
                wsol_amount,
            ));

            instructions
                .push(spl_token::instruction::sync_native(&spl_token::id(), &wsol_ata).unwrap());
        }

        // Build swap instruction
        let swap_instruction = Self::build_swap_instruction(
            &wallet_pubkey,
            pool_info,
            &wsol_ata,
            &token_ata,
            request.direction,
            swap_params,
        )?;

        instructions.push(swap_instruction);

        // Handle WSOL unwrapping for sell operations or remaining WSOL after buy
        if request.direction == SwapDirection::Sell || request.direction == SwapDirection::Buy {
            instructions.push(
                spl_token::instruction::close_account(
                    &spl_token::id(),
                    &wsol_ata,
                    &wallet_pubkey,
                    &wallet_pubkey,
                    &[],
                )
                .unwrap(),
            );
        }

        // Get recent blockhash and create transaction
        let recent_blockhash = rpc_client
            .get_latest_blockhash()
            .await
            .map_err(|e| SwapError::RpcError(format!("Failed to get recent blockhash: {}", e)))?;

        let transaction = Transaction::new_with_payer(&instructions, Some(&wallet_pubkey));
        let mut transaction_with_blockhash = transaction;
        transaction_with_blockhash.message.recent_blockhash = recent_blockhash;

        Ok(transaction_with_blockhash)
    }

    /// Build the Raydium CPMM swap instruction
    fn build_swap_instruction(
        user: &Pubkey,
        pool_info: &RaydiumCpmmPoolInfo,
        wsol_ata: &Pubkey,
        token_ata: &Pubkey,
        direction: SwapDirection,
        swap_params: &SwapParams,
    ) -> Result<Instruction, SwapError> {
        // SwapBaseInput instruction discriminator calculated from SHA256("global:swap_base_input")
        // 8fbe5adac41e33de7fd664ed224c46a877892e28fe513659f68296f7079c123d -> first 8 bytes
        let mut instruction_data = vec![0x8f, 0xbe, 0x5a, 0xda, 0xc4, 0x1e, 0x33, 0xde];
        instruction_data.extend_from_slice(&swap_params.input_amount_raw.to_le_bytes());
        instruction_data.extend_from_slice(&swap_params.minimum_output_raw.to_le_bytes());

        // Determine input/output accounts based on direction
        let (
            input_token_account,
            output_token_account,
            input_vault,
            output_vault,
            input_mint,
            output_mint,
            input_program,
            output_program,
        ) = match direction {
            SwapDirection::Buy => {
                // Buying: SOL → Token
                (
                    *wsol_ata,                                             // SOL account
                    *token_ata,                                            // Token account
                    Pubkey::from_str(&pool_info.token_0_vault).unwrap(),   // SOL vault
                    Pubkey::from_str(&pool_info.token_1_vault).unwrap(),   // Token vault
                    Pubkey::from_str(&pool_info.token_0_mint).unwrap(),    // SOL mint
                    Pubkey::from_str(&pool_info.token_1_mint).unwrap(),    // Token mint
                    Pubkey::from_str(&pool_info.token_0_program).unwrap(), // SOL program
                    Pubkey::from_str(&pool_info.token_1_program).unwrap(), // Token program
                )
            }
            SwapDirection::Sell => {
                // Selling: Token → SOL
                (
                    *token_ata,                                            // Token account
                    *wsol_ata,                                             // SOL account
                    Pubkey::from_str(&pool_info.token_1_vault).unwrap(),   // Token vault
                    Pubkey::from_str(&pool_info.token_0_vault).unwrap(),   // SOL vault
                    Pubkey::from_str(&pool_info.token_1_mint).unwrap(),    // Token mint
                    Pubkey::from_str(&pool_info.token_0_mint).unwrap(),    // SOL mint
                    Pubkey::from_str(&pool_info.token_1_program).unwrap(), // Token program
                    Pubkey::from_str(&pool_info.token_0_program).unwrap(), // SOL program
                )
            }
        };

        // Authority PDA (derived from "vault_and_lp_mint_auth_seed" seed)
        let authority = Pubkey::find_program_address(
            &[b"vault_and_lp_mint_auth_seed"],
            &Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).unwrap(),
        )
        .0;

        let pool_pubkey = Pubkey::from_str(&pool_info.pool_id).unwrap();
        let amm_config = Pubkey::from_str(&pool_info.amm_config).unwrap();
        let observation_key = Pubkey::from_str(&pool_info.observation_key).unwrap();

        // Build accounts according to Raydium CPMM swap instruction format
        let accounts = vec![
            AccountMeta::new_readonly(*user, true),       // payer (signer)
            AccountMeta::new_readonly(authority, false),  // authority
            AccountMeta::new_readonly(amm_config, false), // amm_config
            AccountMeta::new(pool_pubkey, false),         // pool_state
            AccountMeta::new(input_token_account, false), // input_token_account
            AccountMeta::new(output_token_account, false), // output_token_account
            AccountMeta::new(input_vault, false),         // input_vault
            AccountMeta::new(output_vault, false),        // output_vault
            AccountMeta::new_readonly(input_program, false), // input_token_program
            AccountMeta::new_readonly(output_program, false), // output_token_program
            AccountMeta::new_readonly(input_mint, false), // input_token_mint
            AccountMeta::new_readonly(output_mint, false), // output_token_mint
            AccountMeta::new(observation_key, false),     // observation_state
        ];

        Ok(Instruction {
            program_id: Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).unwrap(),
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
        let account_pubkey = Pubkey::from_str(account_address)
            .map_err(|e| SwapError::InvalidInput(format!("Invalid account address: {}", e)))?;

        let account = rpc_client
            .get_account(&account_pubkey)
            .await
            .map_err(|e| SwapError::RpcError(format!("Failed to get token account: {}", e)))?;

        // Decode token account amount (at offset 64)
        if account.data.len() < 72 {
            return Err(SwapError::DecoderError(
                "Invalid token account data".to_string(),
            ));
        }

        let amount_bytes = &account.data[64..72];
        let amount =
            u64::from_le_bytes(amount_bytes.try_into().map_err(|e| {
                SwapError::DecoderError(format!("Failed to decode amount: {:?}", e))
            })?);

        Ok(amount)
    }
}
