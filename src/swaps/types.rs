//! Common swap structures and types used across different swap modules
//! This module contains shared data structures for swap operations

use serde::{Deserialize, Deserializer, Serialize};

// ============================================================================
// CORE SWAP TYPES
// ============================================================================

/// Quote request parameters (immutable, passed to all routers)
#[derive(Debug, Clone)]
pub struct QuoteRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub wallet_address: String,
    pub slippage_pct: f64,
    pub swap_mode: SwapMode,
}

/// Swap mode enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SwapMode {
    ExactIn,
    ExactOut,
}

impl SwapMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SwapMode::ExactIn => "ExactIn",
            SwapMode::ExactOut => "ExactOut",
        }
    }
}

/// Unified quote response (router-agnostic)
#[derive(Debug, Clone)]
pub struct Quote {
    pub router_id: String,
    pub router_name: String,
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub slippage_bps: u16,
    pub route_plan: String,
    pub swap_mode: SwapMode,
    pub wallet_address: String,
    pub execution_data: Vec<u8>,
}

/// Swap execution result (router-agnostic)
#[derive(Debug)]
pub struct SwapResult {
    pub success: bool,
    pub router_id: String,
    pub router_name: String,
    pub transaction_signature: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub execution_time_ms: u64,
    pub effective_price_sol: Option<f64>,
}

impl SwapResult {
    /// Create a failed swap result
    pub fn failed(router_id: String, router_name: String, _error: String) -> Self {
        Self {
            success: false,
            router_id,
            router_name,
            transaction_signature: String::new(),
            input_amount: 0,
            output_amount: 0,
            price_impact_pct: 0.0,
            fee_lamports: 0,
            execution_time_ms: 0,
            effective_price_sol: None,
        }
    }
}

// ============================================================================
// SWAP ENUMS
// ============================================================================

/// Router types for swap operations
#[derive(Debug, Clone, PartialEq)]
pub enum RouterType {
    GMGN,
    Jupiter,
}

/// Exit type for position closing
#[derive(Debug, Clone, PartialEq)]
pub enum ExitType {
    Full,                        // Sell all remaining tokens
    Partial { percentage: f64 }, // Sell a percentage (0.0-100.0)
}

// ============================================================================
// CUSTOM DESERIALIZERS
// ============================================================================

/// Custom deserializer for fields that can be either string or number
pub fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrNumber;

    impl<'de> Visitor<'de> for StringOrNumber {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_owned())
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_f64<E>(self, value: f64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

/// Custom deserializer for optional fields that can be either string or number
pub fn deserialize_optional_string_or_number<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct OptionalStringOrNumber;

    impl<'de> Visitor<'de> for OptionalStringOrNumber {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an optional string or number")
        }

        fn visit_none<E>(self) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Option<String>, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserialize_string_or_number(deserializer).map(Some)
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_owned()))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_unit<E>(self) -> Result<Option<String>, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_option(OptionalStringOrNumber)
}
