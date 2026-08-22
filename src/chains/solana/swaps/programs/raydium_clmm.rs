//! Raydium CLMM (Concentrated Liquidity Market Maker) swap implementation
//!
//! This module implements direct swaps for Raydium Concentrated Liquidity pools.
//! It integrates with the centralized Raydium CLMM decoder and provides proper
//! account derivation and swap calculations based on the Uniswap V3 model.

use super::ProgramSwap;
use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::constants::{
    MEMO_PROGRAM_ID, RAYDIUM_CLMM_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
};
use crate::chains::solana::pools::decoders::raydium_clmm::{ClmmPoolInfo, RaydiumClmmDecoder};
use crate::chains::solana::pools::fetcher::AccountData;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::swaps::executor::SwapExecutor;
use crate::chains::solana::swaps::types::{
    SwapDirection, SwapError, SwapParams, SwapRequest, SwapResult,
};
use crate::logger::{self, LogTag};
use crate::utils::sol_to_lamports;

use crate::chains::solana::solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use crate::chains::solana::spl_associated_token_account;
use crate::chains::solana::spl_token;
use crate::chains::solana::spl_token_2022;
use std::collections::HashMap;
use std::str::FromStr;

/// Raydium CLMM swap implementation
pub struct RaydiumClmmSwap;

impl ProgramSwap for RaydiumClmmSwap {
    async fn execute_swap(
        request: SwapRequest,
        pool_data: AccountData,
    ) -> Result<SwapResult, SwapError> {
        logger::info(
            LogTag::System,
            &format!("Starting Raydium CLMM {:?} swap", request.direction),
        );

        // Decode pool state using centralized decoder
        let pool_info = Self::decode_pool_state(&pool_data)?;

        // Load wallet
        let wallet = Self::load_wallet().await?;

        // Calculate swap parameters using CLMM math
        let swap_params = Self::calculate_clmm_swap_params(&request, &pool_info).await?;

        logger::info(
            LogTag::System,
            &format!(
                "CLMM Swap: {} → {} (min output: {})",
                swap_params.input_amount, swap_params.expected_output, swap_params.minimum_output
            ),
        );

        // Build transaction with proper account derivation
        let transaction = Self::build_clmm_swap_transaction(
            &wallet,
            &request,
            &pool_info,
            &swap_params,
            &pool_data,
        )
        .await?;

        // Execute transaction
        SwapExecutor::execute_transaction(transaction, swap_params).await
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
            .ok_or_else(|| SwapError::DecoderError("Failed to decode Raydium CLMM pool".to_owned()))
    }

    /// Load wallet from configuration
    async fn load_wallet() -> Result<Keypair, SwapError> {
        crate::chains::solana::accounts::configured_keypair()
            .map_err(|e| SwapError::ExecutionError(format!("Failed to load wallet: {e}")))
    }

    /// Calculate swap parameters using CLMM concentrated liquidity math
    async fn calculate_clmm_swap_params(
        request: &SwapRequest,
        pool_info: &ClmmPoolInfo,
    ) -> Result<SwapParams, SwapError> {
        // Get vault balances
        let vault_0_balance = Self::get_token_account_balance(&pool_info.token_vault_0).await?;
        let vault_1_balance = Self::get_token_account_balance(&pool_info.token_vault_1).await?;

        logger::info(
            LogTag::System,
            &format!(
                "CLMM Vault balances - Vault0: {}, Vault1: {}, Current tick: {}, Price: {:.12}",
                vault_0_balance,
                vault_1_balance,
                pool_info.tick_current,
                Self::sqrt_price_x64_to_price(pool_info.sqrt_price_x64)
            ),
        );

        // Determine which token is SOL and get current price
        let (sol_mint, token_mint, sol_decimals, token_decimals, is_token_0_sol) =
            if pool_info.token_mint_0 == SOL_MINT {
                (
                    SOL_MINT,
                    &pool_info.token_mint_1,
                    9,
                    pool_info.mint_decimals_1,
                    true,
                )
            } else if pool_info.token_mint_1 == SOL_MINT {
                (
                    SOL_MINT,
                    &pool_info.token_mint_0,
                    9,
                    pool_info.mint_decimals_0,
                    false,
                )
            } else {
                return Err(SwapError::InvalidPool(
                    "Pool does not contain SOL".to_owned(),
                ));
            };

        // Convert sqrt_price_x64 to actual price
        let sqrt_price = Self::sqrt_price_x64_to_price(pool_info.sqrt_price_x64);
        let current_price = sqrt_price * sqrt_price;

        // Calculate swap amounts based on CLMM pricing
        let (input_amount, expected_output, input_amount_raw, minimum_output_raw) = match request
            .direction
        {
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

                let token_amount_raw = (token_amount * (10_f64).powi(token_decimals as i32)) as u64;
                let minimum_token_raw = ((token_amount_raw as f64)
                    * (1.0 - (request.slippage_bps as f64) / 10000.0))
                    as u64;

                (sol_amount, token_amount, sol_amount_raw, minimum_token_raw)
            }
            SwapDirection::Sell => {
                // Selling tokens for SOL
                let token_amount = request.amount;
                let token_amount_raw = (token_amount * (10_f64).powi(token_decimals as i32)) as u64;

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
                let minimum_sol_raw = ((sol_amount_raw as f64)
                    * (1.0 - (request.slippage_bps as f64) / 10000.0))
                    as u64;

                (token_amount, sol_amount, token_amount_raw, minimum_sol_raw)
            }
        };

        Ok(SwapParams {
            input_amount,
            expected_output,
            minimum_output: (minimum_output_raw as f64)
                / (10_f64).powi(match request.direction {
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

        // Determine token mint and programs - need to properly detect Token-2022
        let (token_mint, token_program_id, is_token_0_sol) = if pool_info.token_mint_0 == SOL_MINT {
            let token_mint = &pool_info.token_mint_1;
            // Check if this is a Token-2022 token by attempting to get account info
            let token_program_id = Self::get_token_program_for_mint(token_mint).await?;
            (token_mint, token_program_id, false)
        } else if pool_info.token_mint_1 == SOL_MINT {
            let token_mint = &pool_info.token_mint_0;
            let token_program_id = Self::get_token_program_for_mint(token_mint).await?;
            (token_mint, token_program_id, true)
        } else {
            return Err(SwapError::InvalidPool(
                "Pool does not contain SOL".to_owned(),
            ));
        };

        // Get associated token accounts with correct program
        let wsol_ata =
            crate::chains::solana::spl_associated_token_account::get_associated_token_address(
                &wallet_pubkey,
                &Pubkey::from_str(SOL_MINT).expect("invalid SOL_MINT constant"),
            );

        let token_ata = if token_program_id
            == Pubkey::from_str(TOKEN_2022_PROGRAM_ID)
                .expect("invalid TOKEN_2022_PROGRAM_ID constant")
        {
            // Token-2022 ATA
            crate::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id(
                &wallet_pubkey,
                &Pubkey::from_str(token_mint).expect("invalid pubkey string"),
                &token_program_id,
            )
        } else {
            // Legacy SPL token ATA
            crate::chains::solana::spl_associated_token_account::get_associated_token_address(
                &wallet_pubkey,
                &Pubkey::from_str(token_mint).expect("invalid pubkey string"),
            )
        };

        // Create token accounts if needed
        if !Self::account_exists(&wsol_ata).await? {
            let create_wsol_ix =
                crate::chains::solana::spl_associated_token_account::instruction::create_associated_token_account(
                    &wallet_pubkey,
                    &wallet_pubkey,
                    &Pubkey::from_str(SOL_MINT).expect("invalid SOL_MINT constant"),
                    &crate::chains::solana::spl_token::id(),
                );
            instructions.push(create_wsol_ix);
        }

        if !Self::account_exists(&token_ata).await? {
            let create_token_ix =
                crate::chains::solana::spl_associated_token_account::instruction::create_associated_token_account(
                    &wallet_pubkey,
                    &wallet_pubkey,
                    &Pubkey::from_str(token_mint).expect("invalid pubkey string"),
                    &token_program_id,
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

            let sync_native_ix = crate::chains::solana::spl_token::instruction::sync_native(
                &crate::chains::solana::spl_token::id(),
                &wsol_ata,
            )?;
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
        )
        .await?;
        instructions.push(swap_ix);

        // Handle WSOL unwrapping
        let close_wsol_ix = crate::chains::solana::spl_token::instruction::close_account(
            &crate::chains::solana::spl_token::id(),
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
            .map_err(|e| SwapError::RpcError(format!("Failed to get blockhash: {e}")))?;

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
            .map_err(|e| SwapError::TransactionError(format!("Invalid amm_config: {e}")))?;
        let observation_key = Pubkey::from_str(&pool_info.observation_key)
            .map_err(|e| SwapError::TransactionError(format!("Invalid observation_key: {e}")))?;

        // Get mint addresses
        let wsol_mint = Pubkey::from_str(SOL_MINT).expect("invalid SOL_MINT constant");
        let token_mint = if is_token_0_sol {
            Pubkey::from_str(&pool_info.token_mint_1).expect("invalid pubkey string")
        } else {
            Pubkey::from_str(&pool_info.token_mint_0).expect("invalid pubkey string")
        };

        // Token vaults
        let token_vault_0 = Pubkey::from_str(&pool_info.token_vault_0)
            .map_err(|e| SwapError::TransactionError(format!("Invalid token_vault_0: {e}")))?;
        let token_vault_1 = Pubkey::from_str(&pool_info.token_vault_1)
            .map_err(|e| SwapError::TransactionError(format!("Invalid token_vault_1: {e}")))?;

        // Determine input/output accounts based on direction and token orientation
        let (input_token_account, output_token_account, input_vault, output_vault) =
            match (direction, is_token_0_sol) {
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

        // Build instruction data with correct SwapV2 discriminator
        let mut instruction_data = vec![0x96, 0x43, 0x18, 0xcd, 0xc5, 0x65, 0x95, 0x7b]; // SwapV2 discriminator
        instruction_data.extend_from_slice(&swap_params.input_amount_raw.to_le_bytes());
        instruction_data.extend_from_slice(&swap_params.minimum_output_raw.to_le_bytes());

        // sqrt_price_limit_x64 - set to 0 for no limit
        instruction_data.extend_from_slice(&(0u128).to_le_bytes());

        // is_base_input - true for exact input swaps
        instruction_data.push(1u8);

        // Determine input/output mints based on direction
        let (input_mint, output_mint) = match (direction, is_token_0_sol) {
            (SwapDirection::Buy, true) => (wsol_mint, token_mint), // SOL → Token
            (SwapDirection::Buy, false) => (wsol_mint, token_mint), // SOL → Token
            (SwapDirection::Sell, true) => (token_mint, wsol_mint), // Token → SOL
            (SwapDirection::Sell, false) => (token_mint, wsol_mint), // Token → SOL
        };

        // Build accounts in correct SwapSingleV2 order
        let accounts = vec![
            AccountMeta::new_readonly(*user, true),         // payer
            AccountMeta::new_readonly(amm_config, false),   // amm_config
            AccountMeta::new(*pool_address, false),         // pool_state
            AccountMeta::new(*input_token_account, false),  // input_token_account
            AccountMeta::new(*output_token_account, false), // output_token_account
            AccountMeta::new(*input_vault, false),          // input_vault
            AccountMeta::new(*output_vault, false),         // output_vault
            AccountMeta::new(observation_key, false),       // observation_state
            AccountMeta::new_readonly(crate::chains::solana::spl_token::id(), false), // token_program
            AccountMeta::new_readonly(crate::chains::solana::spl_token_2022::id(), false), // token_program_2022
            AccountMeta::new_readonly(
                Pubkey::from_str(MEMO_PROGRAM_ID).expect("invalid pubkey string"),
                false,
            ), // memo_program
            AccountMeta::new_readonly(input_mint, false), // input_vault_mint
            AccountMeta::new_readonly(output_mint, false), // output_vault_mint
        ];

        Ok(Instruction {
            program_id: Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).expect("invalid pubkey string"),
            accounts,
            data: instruction_data,
        })
    }

    /// Helper functions
    async fn account_exists(pubkey: &Pubkey) -> Result<bool, SwapError> {
        let rpc_client = get_rpc_client();
        match rpc_client.get_account(pubkey).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Ok(false),
        }
    }

    /// Determine the correct token program for a mint
    async fn get_token_program_for_mint(mint_address: &str) -> Result<Pubkey, SwapError> {
        let rpc_client = get_rpc_client();
        let mint_pubkey = Pubkey::from_str(mint_address)
            .map_err(|e| SwapError::RpcError(format!("Invalid mint address: {e}")))?;

        // Get the mint account to check its owner
        let mint_account = rpc_client
            .get_account(&mint_pubkey)
            .await
            .map_err(|e| SwapError::RpcError(format!("Failed to fetch mint account: {e}")))?
            .ok_or_else(|| {
                SwapError::RpcError(format!("Mint account not found: {mint_address}"))
            })?;

        // Check the owner to determine if it's Token-2022 or legacy SPL Token
        if mint_account.owner
            == Pubkey::from_str(TOKEN_2022_PROGRAM_ID)
                .expect("invalid TOKEN_2022_PROGRAM_ID constant")
        {
            Ok(Pubkey::from_str(TOKEN_2022_PROGRAM_ID)
                .expect("invalid TOKEN_2022_PROGRAM_ID constant"))
        } else {
            Ok(crate::chains::solana::spl_token::id()) // Default to legacy SPL Token
        }
    }

    async fn get_token_account_balance(account_address: &str) -> Result<u64, SwapError> {
        let rpc_client = get_rpc_client();
        let pubkey = Pubkey::from_str(account_address)
            .map_err(|e| SwapError::RpcError(format!("Invalid account address: {e}")))?;

        let account = rpc_client
            .get_account(&pubkey)
            .await
            .map_err(|e| SwapError::RpcError(format!("Failed to fetch account: {e}")))?
            .ok_or_else(|| {
                SwapError::RpcError(format!("Token account not found: {account_address}"))
            })?;

        // Parse token account data to get amount
        if account.data.len() >= 72 {
            let amount_bytes: [u8; 8] = account.data[64..72]
                .try_into()
                .map_err(|_| SwapError::RpcError("Invalid token account data".to_owned()))?;
            Ok(u64::from_le_bytes(amount_bytes))
        } else {
            Err(SwapError::RpcError("Account data too short".to_owned()))
        }
    }

    /// Convert sqrt_price_x64 to normal price
    fn sqrt_price_x64_to_price(sqrt_price_x64: u128) -> f64 {
        let sqrt_price = (sqrt_price_x64 as f64) / (2_f64).powi(64);
        sqrt_price
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_price_x64_to_price_divides_by_two_to_the_64() {
        assert_eq!(RaydiumClmmSwap::sqrt_price_x64_to_price(0), 0.0);
        let one_x64 = 1u128 << 64;
        assert_eq!(RaydiumClmmSwap::sqrt_price_x64_to_price(one_x64), 1.0);
        assert_eq!(RaydiumClmmSwap::sqrt_price_x64_to_price(one_x64 * 2), 2.0);
    }

    // ========================================================================
    // Money-path instruction structure — the SwapV2 account order, signer/
    // writable flags and discriminator are what a wrong-account send would
    // silently corrupt. `build_clmm_swap_instruction` is private and pure (no
    // `.await` in its body despite the `async fn` signature), so it is driven
    // here co-located, matching this file's own `sqrt_price_x64_to_price` test
    // and the documented convention in `tests/common/mod.rs` ("Internal /
    // private pure logic is tested with co-located `#[cfg(test)] mod tests`").
    // ========================================================================

    fn pool_info() -> ClmmPoolInfo {
        ClmmPoolInfo {
            bump: 255,
            amm_config: Pubkey::new_unique().to_string(),
            owner: Pubkey::new_unique().to_string(),
            token_mint_0: SOL_MINT.to_owned(),
            token_mint_1: Pubkey::new_unique().to_string(),
            token_vault_0: Pubkey::new_unique().to_string(),
            token_vault_1: Pubkey::new_unique().to_string(),
            observation_key: Pubkey::new_unique().to_string(),
            mint_decimals_0: 9,
            mint_decimals_1: 6,
            tick_spacing: 60,
            liquidity: 0,
            sqrt_price_x64: 1u128 << 64,
            tick_current: 0,
            padding3: 0,
            padding4: 0,
            fee_growth_global_0_x64: 0,
            fee_growth_global_1_x64: 0,
            protocol_fees_token_0: 0,
            protocol_fees_token_1: 0,
            swap_in_amount_token_0: 0,
            swap_out_amount_token_1: 0,
            swap_in_amount_token_1: 0,
            swap_out_amount_token_0: 0,
            status: 0,
            padding: [0; 7],
            reward_infos: Vec::new(),
            tick_array_bitmap: [0; 16],
            total_fees_token_0: 0,
            total_fees_claimed_token_0: 0,
            total_fees_token_1: 0,
            total_fees_claimed_token_1: 0,
            fund_fees_token_0: 0,
            fund_fees_token_1: 0,
            open_time: 0,
            recent_epoch: 0,
            padding1: [0; 24],
            padding2: [0; 32],
        }
    }

    fn swap_params() -> SwapParams {
        SwapParams {
            input_amount: 1.0,
            expected_output: 100.0,
            minimum_output: 95.0,
            input_amount_raw: 1_000_000_000,
            minimum_output_raw: 95_000_000,
        }
    }

    #[test]
    fn buy_swap_v2_instruction_has_the_documented_account_order_and_discriminator() {
        let user = Pubkey::new_unique();
        let wsol_ata = Pubkey::new_unique();
        let token_ata = Pubkey::new_unique();
        let pool_address = Pubkey::new_unique();
        let info = pool_info();
        let params = swap_params();

        // token_mint_0 == SOL_MINT in this fixture, so is_token_0_sol = false
        // (mirrors the `else if pool_info.token_mint_1 == SOL_MINT` branch is
        // NOT taken; token_mint_0 is SOL here so the caller passes `true`).
        let ix = futures::executor::block_on(RaydiumClmmSwap::build_clmm_swap_instruction(
            &user,
            &info,
            &wsol_ata,
            &token_ata,
            SwapDirection::Buy,
            &params,
            true, // is_token_0_sol
            &pool_address,
        ))
        .expect("instruction must build from well-formed pool info");

        assert_eq!(
            ix.program_id,
            Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).unwrap()
        );
        assert_eq!(ix.accounts.len(), 13);

        // Documented SwapSingleV2 account order.
        assert_eq!(ix.accounts[0].pubkey, user, "payer is the signer account");
        assert!(ix.accounts[0].is_signer);
        assert!(!ix.accounts[0].is_writable, "payer is readonly here");
        assert_eq!(
            ix.accounts[1].pubkey,
            Pubkey::from_str(&info.amm_config).unwrap()
        );
        assert_eq!(ix.accounts[2].pubkey, pool_address, "pool_state");
        assert!(ix.accounts[2].is_writable);
        assert_eq!(
            ix.accounts[3].pubkey, wsol_ata,
            "buy, token_0=SOL: input_token_account is the WSOL ATA"
        );
        assert_eq!(
            ix.accounts[4].pubkey, token_ata,
            "buy: output_token_account is the target token ATA"
        );
        assert_eq!(
            ix.accounts[7].pubkey,
            Pubkey::from_str(&info.observation_key).unwrap()
        );
        // Both token programs are ALWAYS present (accounts 8 and 9), regardless
        // of whether the traded mint is legacy SPL or Token-2022 — the ATA
        // resolution picks the right program, but the instruction always lists
        // both so Token-2022 transfer-fee mints validate correctly.
        assert_eq!(
            ix.accounts[8].pubkey,
            crate::chains::solana::spl_token::id(),
            "token_program (legacy) always present"
        );
        assert_eq!(
            ix.accounts[9].pubkey,
            crate::chains::solana::spl_token_2022::id(),
            "token_program_2022 always present"
        );
        assert_eq!(
            ix.accounts[10].pubkey,
            Pubkey::from_str(MEMO_PROGRAM_ID).unwrap()
        );
        assert!(!ix.accounts[10].is_signer);

        // SwapV2 discriminator, then raw in/min-out amounts, sqrt price limit,
        // and the is_base_input flag.
        assert_eq!(
            &ix.data[0..8],
            &[0x96, 0x43, 0x18, 0xcd, 0xc5, 0x65, 0x95, 0x7b]
        );
        assert_eq!(
            u64::from_le_bytes(ix.data[8..16].try_into().unwrap()),
            params.input_amount_raw
        );
        assert_eq!(
            u64::from_le_bytes(ix.data[16..24].try_into().unwrap()),
            params.minimum_output_raw
        );
        assert_eq!(
            u128::from_le_bytes(ix.data[24..40].try_into().unwrap()),
            0,
            "sqrt_price_limit_x64 is 0 (no limit)"
        );
        assert_eq!(ix.data[40], 1u8, "is_base_input is always true (ExactIn)");
    }

    #[test]
    fn sell_reverses_input_and_output_token_accounts_relative_to_buy() {
        let user = Pubkey::new_unique();
        let wsol_ata = Pubkey::new_unique();
        let token_ata = Pubkey::new_unique();
        let pool_address = Pubkey::new_unique();
        let info = pool_info();
        let params = swap_params();

        let ix = futures::executor::block_on(RaydiumClmmSwap::build_clmm_swap_instruction(
            &user,
            &info,
            &wsol_ata,
            &token_ata,
            SwapDirection::Sell,
            &params,
            true, // is_token_0_sol
            &pool_address,
        ))
        .expect("instruction must build from well-formed pool info");

        assert_eq!(
            ix.accounts[3].pubkey, token_ata,
            "sell: input_token_account is the target token ATA"
        );
        assert_eq!(
            ix.accounts[4].pubkey, wsol_ata,
            "sell: output_token_account is the WSOL ATA"
        );
    }
}
