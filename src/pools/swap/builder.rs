use super::programs::raydium_clmm::RaydiumClmmSwap;
use super::programs::raydium_cpmm::RaydiumCpmmSwap;
use super::programs::ProgramSwap;
/// Swap builder - High-level interface for creating swaps
///
/// This module provides a builder pattern interface for creating swap operations.
/// It handles validation, parameter calculation, and delegates to appropriate
/// program-specific implementations.
use super::types::{constants::*, SwapDirection, SwapError, SwapRequest, SwapResult};
use crate::constants::{RAYDIUM_CLMM_PROGRAM_ID, RAYDIUM_CPMM_PROGRAM_ID};
use crate::logger::{self, LogTag};
use crate::pools::decoders::PoolDecoder;
use crate::pools::types::ProgramKind;
use crate::pools::AccountData;
use crate::rpc::get_rpc_client;

use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
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
                "🔄 Executing {:?} swap for {} {}",
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
                "Amount must be greater than 0".to_string(),
            ));
        }

        match request.direction {
            SwapDirection::Buy => {
                if request.amount < 0.001 {
                    return Err(SwapError::InvalidInput(
                        "SOL amount too small (minimum 0.001 SOL)".to_string(),
                    ));
                }
            }
            SwapDirection::Sell => {
                if request.amount < 1.0 {
                    return Err(SwapError::InvalidInput(
                        "Token amount too small (minimum 1 token)".to_string(),
                    ));
                }
            }
        }

        if request.slippage_bps > 5000 {
            return Err(SwapError::InvalidInput(
                "Slippage too high (maximum 50%)".to_string(),
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
            .map_err(|e| SwapError::RpcError(format!("Failed to fetch pool: {}", e)))?;

        // Create AccountData
        let account_data = AccountData::from_account(*pool_address, pool_account, 0);

        // Determine program type from owner
        let program_kind = match account_data.owner.to_string().as_str() {
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
            &format!("📊 Detected pool program: {:?}", program_kind),
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
    dry_run: bool,
}

impl SwapRequestBuilder {
    pub fn new() -> Self {
        Self {
            pool_address: None,
            token_mint: None,
            amount: None,
            direction: None,
            slippage_bps: DEFAULT_SLIPPAGE_BPS,
            dry_run: false,
        }
    }

    pub fn pool_address(mut self, address: &str) -> Result<Self, SwapError> {
        self.pool_address = Some(
            Pubkey::from_str(address)
                .map_err(|e| SwapError::InvalidInput(format!("Invalid pool address: {}", e)))?,
        );
        Ok(self)
    }

    pub fn token_mint(mut self, mint: &str) -> Result<Self, SwapError> {
        self.token_mint = Some(
            Pubkey::from_str(mint)
                .map_err(|e| SwapError::InvalidInput(format!("Invalid token mint: {}", e)))?,
        );
        Ok(self)
    }

    pub fn amount(mut self, amount: f64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn amount_sol(self, amount: f64) -> Self {
        self.amount(amount)
    }

    pub fn amount_tokens(self, amount: f64) -> Self {
        self.amount(amount)
    }

    pub fn direction(mut self, dir: SwapDirection) -> Self {
        self.direction = Some(dir);
        self
    }

    pub fn buy(self) -> Self {
        self.direction(SwapDirection::Buy)
    }

    pub fn sell(self) -> Self {
        self.direction(SwapDirection::Sell)
    }

    pub fn slippage_bps(mut self, bps: u16) -> Self {
        self.slippage_bps = bps;
        self
    }

    pub fn slippage_percent(self, percent: f64) -> Self {
        self.slippage_bps((percent * 100.0) as u16)
    }

    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }

    pub fn build(self) -> Result<SwapRequest, SwapError> {
        Ok(SwapRequest {
            pool_address: self
                .pool_address
                .ok_or_else(|| SwapError::InvalidInput("Pool address is required".to_string()))?,
            token_mint: self
                .token_mint
                .ok_or_else(|| SwapError::InvalidInput("Token mint is required".to_string()))?,
            amount: self
                .amount
                .ok_or_else(|| SwapError::InvalidInput("Amount is required".to_string()))?,
            direction: self
                .direction
                .ok_or_else(|| SwapError::InvalidInput("Direction is required".to_string()))?,
            slippage_bps: self.slippage_bps,
            dry_run: self.dry_run,
        })
    }

    /// Build and execute the swap request
    pub async fn execute(self) -> Result<SwapResult, SwapError> {
        let request = self.build()?;
        SwapBuilder::execute(request).await
    }
}
