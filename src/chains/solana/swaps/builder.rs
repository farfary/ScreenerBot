//! Swap builder - High-level interface for creating swaps
//!
//! This module provides a builder pattern interface for creating swap operations.
//! It handles validation, parameter calculation, and delegates to appropriate
//! program-specific implementations.

use super::programs::raydium_clmm::RaydiumClmmSwap;
use super::programs::raydium_cpmm::RaydiumCpmmSwap;
use super::programs::ProgramSwap;
use super::types::{SwapDirection, SwapError, SwapRequest, SwapResult};

use crate::chains::solana::constants::{RAYDIUM_CLMM_PROGRAM_ID, RAYDIUM_CPMM_PROGRAM_ID};
use crate::chains::solana::pools::fetcher::AccountData;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::config::with_config;
use crate::logger::{self, LogTag};

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Main swap builder for creating and executing swaps
pub struct SwapBuilder;

impl SwapBuilder {
    /// Create a new swap request
    pub fn new() -> SwapRequestBuilder {
        SwapRequestBuilder::new()
    }

    /// Execute a swap request
    pub async fn execute(request: SwapRequest) -> Result<SwapResult, SwapError> {
        logger::info(
            LogTag::System,
            &format!(
                "Executing {:?} swap for {} {}",
                request.direction,
                request.amount,
                match request.direction {
                    SwapDirection::Buy => "SOL",
                    SwapDirection::Sell => "tokens",
                }
            ),
        );

        // Validate request
        Self::validate_request(&request)?;

        // Fetch pool state and determine program type
        let (pool_data, program_kind) = Self::fetch_pool_data(&request.pool_address).await?;

        // Delegate to appropriate program implementation
        match program_kind {
            ProgramKind::RaydiumCpmm => RaydiumCpmmSwap::execute_swap(request, pool_data).await,
            ProgramKind::RaydiumClmm => RaydiumClmmSwap::execute_swap(request, pool_data).await,
            _ => Err(SwapError::InvalidPool(format!(
                "Unsupported program: {:?}",
                program_kind
            ))),
        }
    }

    /// Validate swap request parameters
    fn validate_request(request: &SwapRequest) -> Result<(), SwapError> {
        if request.amount <= 0.0 {
            return Err(SwapError::InvalidInput(
                "Amount must be greater than 0".to_owned(),
            ));
        }

        match request.direction {
            SwapDirection::Buy => {
                if request.amount < 0.001 {
                    return Err(SwapError::InvalidInput(
                        "SOL amount too small (minimum 0.001 SOL)".to_owned(),
                    ));
                }
            }
            SwapDirection::Sell => {
                if request.amount < 1.0 {
                    return Err(SwapError::InvalidInput(
                        "Token amount too small (minimum 1 token)".to_owned(),
                    ));
                }
            }
        }

        if request.slippage_bps > 5000 {
            return Err(SwapError::InvalidInput(
                "Slippage too high (maximum 50%)".to_owned(),
            ));
        }

        Ok(())
    }

    /// Fetch pool account data and determine program type
    async fn fetch_pool_data(
        pool_address: &Pubkey,
    ) -> Result<(AccountData, ProgramKind), SwapError> {
        let rpc_client = get_rpc_client();

        // Get pool account
        let pool_account = rpc_client
            .get_account(pool_address)
            .await
            .map_err(|e| SwapError::RpcError(format!("Failed to fetch pool: {e}")))?
            .ok_or_else(|| {
                SwapError::RpcError(format!("Pool account not found: {pool_address}"))
            })?;

        // Create AccountData
        let account_data = AccountData::from_account(*pool_address, pool_account, 0);

        // Determine program type from owner
        let owner_str = account_data.owner.to_string();
        let program_kind = match owner_str.as_str() {
            RAYDIUM_CPMM_PROGRAM_ID => ProgramKind::RaydiumCpmm,
            RAYDIUM_CLMM_PROGRAM_ID => ProgramKind::RaydiumClmm,
            _ => {
                return Err(SwapError::InvalidPool(format!(
                    "Unsupported pool program: {}",
                    account_data.owner
                )));
            }
        };

        logger::info(
            LogTag::System,
            &format!("Detected pool program: {:?}", program_kind),
        );

        Ok((account_data, program_kind))
    }
}

/// Builder for constructing swap requests
pub struct SwapRequestBuilder {
    pool_address: Option<Pubkey>,
    token_mint: Option<Pubkey>,
    amount: Option<f64>,
    direction: Option<SwapDirection>,
    slippage_bps: u16,
}

impl SwapRequestBuilder {
    /// Create a new swap request builder with default slippage from config
    pub fn new() -> Self {
        Self {
            pool_address: None,
            token_mint: None,
            amount: None,
            direction: None,
            slippage_bps: with_config(|cfg| cfg.swaps.raydium.default_slippage_bps),
        }
    }

    /// Set the target pool address (parses from string to Pubkey)
    pub fn pool_address(mut self, address: &str) -> Result<Self, SwapError> {
        self.pool_address = Some(
            Pubkey::from_str(address)
                .map_err(|e| SwapError::InvalidInput(format!("Invalid pool address: {e}")))?,
        );
        Ok(self)
    }

    /// Set the token mint address (parses from string to Pubkey)
    pub fn token_mint(mut self, mint: &str) -> Result<Self, SwapError> {
        self.token_mint = Some(
            Pubkey::from_str(mint)
                .map_err(|e| SwapError::InvalidInput(format!("Invalid token mint: {e}")))?,
        );
        Ok(self)
    }

    /// Set the swap amount (raw value, interpretation depends on direction)
    pub fn amount(mut self, amount: f64) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Set the swap amount in SOL (alias for amount)
    pub fn amount_sol(self, amount: f64) -> Self {
        self.amount(amount)
    }

    /// Set the swap amount in tokens (alias for amount)
    pub fn amount_tokens(self, amount: f64) -> Self {
        self.amount(amount)
    }

    /// Set the swap direction (Buy or Sell)
    pub fn direction(mut self, dir: SwapDirection) -> Self {
        self.direction = Some(dir);
        self
    }

    /// Set direction to Buy (SOL → Token)
    pub fn buy(self) -> Self {
        self.direction(SwapDirection::Buy)
    }

    /// Set direction to Sell (Token → SOL)
    pub fn sell(self) -> Self {
        self.direction(SwapDirection::Sell)
    }

    /// Set slippage tolerance in basis points (100 bps = 1%)
    pub fn slippage_bps(mut self, bps: u16) -> Self {
        self.slippage_bps = bps;
        self
    }

    /// Set slippage tolerance as a percentage (e.g., 1.0 = 1%)
    pub fn slippage_percent(self, percent: f64) -> Self {
        self.slippage_bps((percent * 100.0) as u16)
    }

    /// Validate and build the final SwapRequest
    pub fn build(self) -> Result<SwapRequest, SwapError> {
        Ok(SwapRequest {
            pool_address: self
                .pool_address
                .ok_or_else(|| SwapError::InvalidInput("Pool address is required".to_owned()))?,
            token_mint: self
                .token_mint
                .ok_or_else(|| SwapError::InvalidInput("Token mint is required".to_owned()))?,
            amount: self
                .amount
                .ok_or_else(|| SwapError::InvalidInput("Amount is required".to_owned()))?,
            direction: self
                .direction
                .ok_or_else(|| SwapError::InvalidInput("Direction is required".to_owned()))?,
            slippage_bps: self.slippage_bps,
        })
    }

    /// Build and execute the swap request
    pub async fn execute(self) -> Result<SwapResult, SwapError> {
        let request = self.build()?;
        SwapBuilder::execute(request).await
    }
}
