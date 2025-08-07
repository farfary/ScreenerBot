/// Common swap structures and types used across different swap modules
/// This module contains shared data structures for swap operations

use serde::{Deserialize, Serialize, Deserializer};

/// Configuration constants for swap operations (re-exported from config)
pub use super::config::{
    SOL_MINT, 
    GMGN_ANTI_MEV as ANTI_MEV, 
    GMGN_PARTNER as PARTNER,
    GMGN_DEFAULT_SWAP_MODE,
    JUPITER_DEFAULT_SWAP_MODE,
    SWAP_FEE_PERCENT,
    QUOTE_SLIPPAGE_PERCENT,
    INTERNAL_SLIPPAGE_PERCENT,
    // Legacy alias for backward compatibility in types
    QUOTE_SLIPPAGE_PERCENT as SLIPPAGE_TOLERANCE_PERCENT
};

/// Custom deserializer for fields that can be either string or number
pub fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
    where D: Deserializer<'de>
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrNumber;

    impl<'de> Visitor<'de> for StringOrNumber {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E> where E: de::Error {
            Ok(value.to_owned())
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E> where E: de::Error {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E> where E: de::Error {
            Ok(value.to_string())
        }

        fn visit_f64<E>(self, value: f64) -> Result<String, E> where E: de::Error {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

/// Custom deserializer for optional fields that can be either string or number
pub fn deserialize_optional_string_or_number<'de, D>(
    deserializer: D
) -> Result<Option<String>, D::Error>
    where D: Deserializer<'de>
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct OptionalStringOrNumber;

    impl<'de> Visitor<'de> for OptionalStringOrNumber {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an optional string or number")
        }

        fn visit_none<E>(self) -> Result<Option<String>, E> where E: de::Error {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Option<String>, D::Error>
            where D: Deserializer<'de>
        {
            deserialize_string_or_number(deserializer).map(Some)
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<String>, E> where E: de::Error {
            Ok(Some(value.to_owned()))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<String>, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Option<String>, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Option<String>, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_unit<E>(self) -> Result<Option<String>, E> where E: de::Error {
            Ok(None)
        }
    }

    deserializer.deserialize_option(OptionalStringOrNumber)
}

/// Quote information from the swap router
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapQuote {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "inDecimals")]
    pub in_decimals: u8,
    #[serde(rename = "outDecimals")]
    pub out_decimals: u8,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps", deserialize_with = "deserialize_string_or_number")]
    pub slippage_bps: String,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<String>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: serde_json::Value,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: f64,
}

/// Raw transaction data from the swap router
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawTransaction {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: u64,
    #[serde(rename = "recentBlockhash")]
    pub recent_blockhash: String,
    pub version: Option<String>,
}

/// Complete swap response data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapData {
    pub quote: SwapQuote,
    pub raw_tx: RawTransaction,
    pub amount_in_usd: Option<String>,
    pub amount_out_usd: Option<String>,
    pub jito_order_id: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_string_or_number")]
    pub sol_cost: Option<String>,
}

/// GMGN API response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct GMGNApiResponse {
    pub code: i32,
    pub msg: String,
    pub tid: Option<String>,
    pub data: Option<SwapData>,
}

/// Jupiter API response structure for quotes
#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<serde_json::Value>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<serde_json::Value>,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

/// Jupiter API response structure for swap transactions
#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: u64,
}

/// Swap request parameters
#[derive(Debug, Clone)]
pub struct SwapRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64, // Amount in smallest unit (lamports for SOL, raw amount for tokens)
    pub from_address: String,
    pub slippage: f64,
    pub swap_mode: String, // "ExactIn" or "ExactOut", default is "ExactIn"
    pub fee: f64,
    pub is_anti_mev: bool,
    pub expected_price: Option<f64>,
}

impl Default for SwapRequest {
    fn default() -> Self {
        Self {
            input_mint: SOL_MINT.to_string(),
            output_mint: String::new(),
            input_amount: 0,
            from_address: String::new(),
            slippage: SLIPPAGE_TOLERANCE_PERCENT,
            swap_mode: GMGN_DEFAULT_SWAP_MODE.to_string(), // Use config default
            fee: SWAP_FEE_PERCENT,
            is_anti_mev: ANTI_MEV,
            expected_price: None,
        }
    }
}

/// Result of a swap operation
#[derive(Debug)]
pub struct SwapResult {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub input_amount: String,
    pub output_amount: String,
    pub price_impact: String,
    pub fee_lamports: u64,
    pub execution_time: f64,
    pub effective_price: Option<f64>, // Price per token in SOL
    pub swap_data: Option<SwapData>, // Complete swap data for reference
    pub error: Option<String>,
}
