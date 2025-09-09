use solana_sdk::{
    instruction::{ AccountMeta, Instruction },
    pubkey::Pubkey,
    program_error::ProgramError,
    transaction::Transaction,
    signature::{ Keypair, Signer },
    system_instruction,
};
use spl_token;
use std::str::FromStr;
use serde::{ Deserialize, Serialize };
use std::collections::{ VecDeque, HashMap };

use crate::pools::swap::types::{ SwapRequest, SwapResult, SwapDirection, SwapError, SwapParams };
use crate::pools::swap::programs::ProgramSwap;
use crate::pools::decoders::raydium_clmm::{ RaydiumClmmDecoder, ClmmPoolInfo, ClmmBasicInfo };
use crate::pools::AccountData;
use crate::pools::swap::executor::SwapExecutor;
use crate::pools::types::RAYDIUM_CLMM_PROGRAM_ID;
use crate::rpc::{ get_rpc_client, sol_to_lamports };
use crate::configs::{ read_configs, load_wallet_from_config };
use crate::logger::{ log, LogTag };

/// SwapV2 instruction discriminator for Raydium CLMM
/// Based on Anchor discriminator pattern
const SWAP_V2_DISCRIMINATOR: [u8; 8] = [0x3f, 0x2a, 0xd9, 0xe2, 0xd1, 0x5d, 0xf7, 0x8b];

/// Raydium CLMM SwapV2 instruction data structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SwapV2Data {
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit_x64: u128,
    pub is_base_input: bool,
}

/// Raydium CLMM swap calculation result
#[derive(Debug, Clone)]
pub struct ClmmSwapResult {
    pub pool_id: Pubkey,
    pub pool_amm_config: Pubkey,
    pub pool_observation: Pubkey,
    pub input_vault: Pubkey,
    pub output_vault: Pubkey,
    pub input_vault_mint: Pubkey,
    pub output_vault_mint: Pubkey,
    pub input_token_program: Pubkey,
    pub output_token_program: Pubkey,
    pub user_input_token: Pubkey,
    pub remaining_tick_array_keys: VecDeque<Pubkey>,
    pub amount: u64,
    pub other_amount_threshold: u64,
    pub sqrt_price_limit_x64: Option<u128>,
    pub is_base_input: bool,
}

/// Raydium CLMM pool information for swaps
#[derive(Debug, Clone)]
pub struct RaydiumClmmPoolInfo {
    pub pool_id: String,
    pub token_mint_0: String,
    pub token_mint_1: String,
    pub token_vault_0: String,
    pub token_vault_1: String,
    pub sqrt_price_x64: u128,
    pub amm_config: String,
    pub observation_key: String,
}

/// Raydium CLMM swap implementation
pub struct RaydiumClmmSwap;

impl ProgramSwap for RaydiumClmmSwap {
    async fn execute_swap(
        request: SwapRequest,
        pool_data: AccountData
    ) -> Result<SwapResult, SwapError> {
        log(
            LogTag::System,
            "INFO",
            &format!("🔀 Executing Raydium CLMM {:?} swap", request.direction)
        );

        // Decode pool state using centralized decoder
        let pool_info = Self::decode_pool_state(&pool_data).await?;

        // Load wallet
        let wallet = Self::load_wallet().await?;

        // Calculate swap parameters
        let swap_params = Self::calculate_swap_params(&request, &pool_info).await?;

        // Build transaction
        let transaction = Self::build_swap_transaction(
            &wallet,
            &request,
            &pool_info,
            &swap_params
        ).await?;

        // Execute transaction
        SwapExecutor::execute_transaction(transaction, swap_params, request.dry_run).await
    }
}

impl RaydiumClmmSwap {
    /// Decode pool state using the centralized CLMM decoder
    async fn decode_pool_state(pool_data: &AccountData) -> Result<RaydiumClmmPoolInfo, SwapError> {
        log(
            LogTag::System,
            "INFO",
            "📊 Extracting complete Raydium CLMM pool data using centralized decoder"
        );

        // Create accounts map for the decoder
        let mut accounts = std::collections::HashMap::new();
        accounts.insert(pool_data.pubkey.to_string(), pool_data.clone());

        // Extract complete pool data using the new raw data extraction approach
        let full_pool_info = match RaydiumClmmDecoder::extract_pool_data(&accounts) {
            Some(data) => data,
            None => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("No pool data found for {}", pool_data.pubkey)
                );
                return Err(SwapError::InvalidPool("No pool data found".to_string()));
            }
        };

        // Get basic info for trading
        let basic_info = RaydiumClmmDecoder::get_basic_pool_info(&full_pool_info);

        log(
            LogTag::System,
            "INFO",
            &format!(
                "✅ Complete CLMM pool data extracted - token_0: {}, token_1: {}, sqrt_price_x64: {}, liquidity: {}, tick_current: {}, tick_spacing: {}",
                basic_info.token_mint_0,
                basic_info.token_mint_1,
                basic_info.sqrt_price_x64,
                basic_info.liquidity,
                basic_info.tick_current,
                basic_info.tick_spacing
            )
        );

        // Convert to our internal format with basic trading info
        Ok(RaydiumClmmPoolInfo {
            pool_id: pool_data.pubkey.to_string(),
            token_mint_0: basic_info.token_mint_0,
            token_mint_1: basic_info.token_mint_1,
            token_vault_0: basic_info.token_vault_0,
            token_vault_1: basic_info.token_vault_1,
            sqrt_price_x64: basic_info.sqrt_price_x64,
            amm_config: full_pool_info.amm_config,
            observation_key: full_pool_info.observation_key,
        })
    }

    /// Load wallet from configuration
    async fn load_wallet() -> Result<Keypair, SwapError> {
        let configs = read_configs().map_err(|e|
            SwapError::ExecutionError(format!("Failed to load config: {}", e))
        )?;
        let wallet = load_wallet_from_config(&configs).map_err(|e|
            SwapError::ExecutionError(format!("Failed to load wallet: {}", e))
        )?;
        Ok(wallet)
    }

    /// Calculate swap parameters using CLMM math (simplified)
    async fn calculate_swap_params(
        request: &SwapRequest,
        pool_info: &RaydiumClmmPoolInfo
    ) -> Result<SwapParams, SwapError> {
        log(LogTag::System, "INFO", "🧮 Calculating CLMM swap parameters");

        // Convert input amount to raw units - use standard decimals for now
        let input_amount_raw = match request.direction {
            SwapDirection::Buy => sol_to_lamports(request.amount),
            SwapDirection::Sell => (request.amount * (10f64).powi(6)) as u64, // Assume 6 decimals for token
        };

        // Use simplified constant product formula for CLMM estimation
        // Real CLMM would use concentrated liquidity math with sqrt pricing
        let estimated_output_raw = {
            // Mock reserves based on liquidity - we need to implement real vault balance fetching
            let reserve_0 = 1000000000u64; // 1000 SOL equivalent
            let reserve_1 = 50000000000u64; // 50000 tokens equivalent

            let fee_rate = 2500u64; // 0.25% default for CLMM
            let fee_denominator = 1000000u64;

            Self::estimate_clmm_output(
                input_amount_raw,
                reserve_0,
                reserve_1,
                fee_rate,
                fee_denominator,
                matches!(request.direction, SwapDirection::Buy)
            )?
        };

        // Apply slippage to get minimum output
        let slippage_multiplier = (10000u64).saturating_sub(request.slippage_bps as u64) as u128;
        let minimum_output_raw = (((estimated_output_raw as u128) * slippage_multiplier) /
            10000u128) as u64;

        // Convert to UI amounts for display - use standard decimals
        let output_decimals = match request.direction {
            SwapDirection::Buy => 6u8, // Token decimals
            SwapDirection::Sell => 9u8, // SOL decimals
        };

        let expected_output = (estimated_output_raw as f64) / (10f64).powi(output_decimals as i32);
        let minimum_output = (minimum_output_raw as f64) / (10f64).powi(output_decimals as i32);

        log(
            LogTag::System,
            "INFO",
            &format!(
                "💹 CLMM Swap calculation: {} → {} (min: {})",
                request.amount,
                expected_output,
                minimum_output
            )
        );

        Ok(SwapParams {
            input_amount: request.amount,
            expected_output,
            minimum_output,
            input_amount_raw,
            minimum_output_raw,
        })
    }

    /// Simplified CLMM output estimation
    fn estimate_clmm_output(
        input_amount: u64,
        reserve_in: u64,
        reserve_out: u64,
        fee_numerator: u64,
        fee_denominator: u64,
        _is_zero_for_one: bool
    ) -> Result<u64, SwapError> {
        if input_amount == 0 || reserve_in == 0 || reserve_out == 0 {
            return Ok(0);
        }

        // Use u128 for all calculations to prevent overflow
        let input_amount_u128 = input_amount as u128;
        let reserve_in_u128 = reserve_in as u128;
        let reserve_out_u128 = reserve_out as u128;
        let fee_numerator_u128 = fee_numerator as u128;
        let fee_denominator_u128 = fee_denominator as u128;

        // Calculate fee
        let fee_amount = input_amount_u128
            .checked_mul(fee_numerator_u128)
            .and_then(|x| x.checked_div(fee_denominator_u128))
            .ok_or_else(|| SwapError::CalculationError("Fee calculation overflow".to_string()))?;

        let input_amount_after_fee = input_amount_u128
            .checked_sub(fee_amount)
            .ok_or_else(|| SwapError::CalculationError("Input after fee underflow".to_string()))?;

        // Simplified constant product formula (real CLMM uses concentrated liquidity)
        let numerator = input_amount_after_fee
            .checked_mul(reserve_out_u128)
            .ok_or_else(|| SwapError::CalculationError("Numerator overflow".to_string()))?;

        let denominator = reserve_in_u128
            .checked_add(input_amount_after_fee)
            .ok_or_else(|| SwapError::CalculationError("Denominator overflow".to_string()))?;

        let output_amount = numerator
            .checked_div(denominator)
            .ok_or_else(|| SwapError::CalculationError("Division by zero".to_string()))?;

        // Convert back to u64, ensuring it fits
        if output_amount > (u64::MAX as u128) {
            return Err(SwapError::CalculationError("Output amount overflow".to_string()));
        }

        Ok(output_amount as u64)
    }

    /// Build the complete swap transaction
    async fn build_swap_transaction(
        wallet: &Keypair,
        request: &SwapRequest,
        pool_info: &RaydiumClmmPoolInfo,
        swap_params: &SwapParams
    ) -> Result<Transaction, SwapError> {
        log(LogTag::System, "INFO", "🔨 Building Raydium CLMM swap transaction");

        let mut instructions = Vec::new();

        // Create a simplified CLMM swap instruction
        let swap_instruction = Self::build_clmm_swap_instruction(
            &wallet.pubkey(),
            pool_info,
            request,
            swap_params
        )?;

        instructions.push(swap_instruction);

        // Create transaction
        let recent_blockhash = get_rpc_client()
            .get_latest_blockhash().await
            .map_err(|e| SwapError::RpcError(format!("Failed to get blockhash: {}", e)))?;

        let transaction = Transaction::new_unsigned(
            solana_sdk::message::Message::new(&instructions, Some(&wallet.pubkey()))
        );

        Ok(transaction)
    }

    /// Build simplified CLMM swap instruction
    fn build_clmm_swap_instruction(
        user: &Pubkey,
        pool_info: &RaydiumClmmPoolInfo,
        request: &SwapRequest,
        swap_params: &SwapParams
    ) -> Result<Instruction, SwapError> {
        let program_id = Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).map_err(|e|
            SwapError::InvalidPool(format!("Invalid CLMM program ID: {}", e))
        )?;

        // Create SwapV2 instruction data
        let swap_data = SwapV2Data {
            amount: swap_params.input_amount_raw,
            other_amount_threshold: swap_params.minimum_output_raw,
            sqrt_price_limit_x64: 0u128, // No price limit
            is_base_input: true,
        };

        let mut instruction_data = SWAP_V2_DISCRIMINATOR.to_vec();
        let serialized_data = bincode
            ::serialize(&swap_data)
            .map_err(|e| SwapError::InvalidPool(format!("Failed to serialize data: {}", e)))?;
        instruction_data.extend_from_slice(&serialized_data);

        // Mock account addresses - in production these would be derived properly
        let pool_id = Pubkey::from_str(&pool_info.pool_id).map_err(|e|
            SwapError::InvalidPool(format!("Invalid pool ID: {}", e))
        )?;
        let amm_config = Pubkey::from_str(&pool_info.amm_config).map_err(|e|
            SwapError::InvalidPool(format!("Invalid AMM config: {}", e))
        )?;
        let observation_state = Pubkey::from_str(&pool_info.observation_key).map_err(|e|
            SwapError::InvalidPool(format!("Invalid observation key: {}", e))
        )?;

        let mock_input_vault = Pubkey::from_str(&pool_info.token_vault_0).map_err(|e|
            SwapError::InvalidPool(format!("Invalid input vault: {}", e))
        )?;
        let mock_output_vault = Pubkey::from_str(&pool_info.token_vault_1).map_err(|e|
            SwapError::InvalidPool(format!("Invalid output vault: {}", e))
        )?;

        // Mock user token accounts
        let mock_input_token = Pubkey::new_unique();
        let mock_output_token = Pubkey::new_unique();

        // Build accounts for SwapSingleV2
        let mut accounts = vec![
            // Payer (signer)
            AccountMeta::new(*user, true),
            // AMM config (readonly)
            AccountMeta::new_readonly(amm_config, false),
            // Pool state (writable)
            AccountMeta::new(pool_id, false),
            // Input token account (writable)
            AccountMeta::new(mock_input_token, false),
            // Output token account (writable)
            AccountMeta::new(mock_output_token, false),
            // Input vault (writable)
            AccountMeta::new(mock_input_vault, false),
            // Output vault (writable)
            AccountMeta::new(mock_output_vault, false),
            // Observation state (writable)
            AccountMeta::new(observation_state, false),
            // Token program (readonly)
            AccountMeta::new_readonly(spl_token::id(), false),
            // Token program 2022 (readonly)
            AccountMeta::new_readonly(spl_token::id(), false),
            // Memo program (readonly)
            AccountMeta::new_readonly(spl_token::id(), false),
            // Input vault mint (readonly)
            AccountMeta::new_readonly(Pubkey::from_str(&pool_info.token_mint_0).unwrap(), false),
            // Output vault mint (readonly)
            AccountMeta::new_readonly(Pubkey::from_str(&pool_info.token_mint_1).unwrap(), false)
        ];

        // Add mock tick arrays (CLMM requires multiple tick arrays)
        for _ in 0..3 {
            accounts.push(AccountMeta::new(Pubkey::new_unique(), false));
        }

        Ok(Instruction {
            program_id,
            accounts,
            data: instruction_data,
        })
    }
}
