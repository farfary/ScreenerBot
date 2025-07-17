use std::collections::HashMap;
use serde::{ Deserialize, Serialize };
use crate::market_data::PoolType;

/// Common interface for all pool decoders
#[async_trait::async_trait]
pub trait PoolDecoder {
    async fn decode(&self, data: &[u8]) -> Result<DecodedPoolData, PoolDecoderError>;
    fn supported_pool_type(&self) -> PoolType;
}

/// Decoded pool data structure containing standardized information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedPoolData {
    pub pool_type: PoolType,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_vault: String,
    pub token_b_vault: String,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub fee_rate: f64,
    pub sqrt_price: Option<u128>,
    pub tick_current: Option<i32>,
    pub liquidity: Option<u128>,
    pub additional_data: HashMap<String, serde_json::Value>,
}

/// Custom error type for pool decoding operations
#[derive(Debug, thiserror::Error)]
pub enum PoolDecoderError {
    #[error("Invalid data length: expected at least {expected}, got {actual}")] InvalidDataLength {
        expected: usize,
        actual: usize,
    },

    #[error("Failed to parse {field}: {source}")] ParseError {
        field: String,
        #[source] source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Unsupported pool type: {pool_type}")] UnsupportedPoolType {
        pool_type: String,
    },

    #[error("Unable to decode pool data with any known decoder")]
    AutoDetectionFailed,

    #[error("Invalid pool structure for {decoder}")] InvalidPoolStructure {
        decoder: String,
    },
}

impl DecodedPoolData {
    /// Create a new DecodedPoolData with minimal required fields
    pub fn new(
        pool_type: PoolType,
        token_a_mint: String,
        token_b_mint: String,
        token_a_amount: u64,
        token_b_amount: u64,
        fee_rate: f64
    ) -> Self {
        Self {
            pool_type,
            token_a_mint,
            token_b_mint,
            token_a_vault: String::new(),
            token_b_vault: String::new(),
            token_a_amount,
            token_b_amount,
            fee_rate,
            sqrt_price: None,
            tick_current: None,
            liquidity: None,
            additional_data: HashMap::new(),
        }
    }

    /// Add additional metadata to the pool data
    pub fn with_additional_data(mut self, key: String, value: serde_json::Value) -> Self {
        self.additional_data.insert(key, value);
        self
    }

    /// Set vault addresses
    pub fn with_vaults(mut self, token_a_vault: String, token_b_vault: String) -> Self {
        self.token_a_vault = token_a_vault;
        self.token_b_vault = token_b_vault;
        self
    }

    /// Set concentrated liquidity parameters
    pub fn with_cl_params(
        mut self,
        sqrt_price: Option<u128>,
        tick_current: Option<i32>,
        liquidity: Option<u128>
    ) -> Self {
        self.sqrt_price = sqrt_price;
        self.tick_current = tick_current;
        self.liquidity = liquidity;
        self
    }
}
