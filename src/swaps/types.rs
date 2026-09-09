//! Common swap structures and types used across different swap modules
//! This module contains shared data structures for swap operations

use crate::chains::ChainId;
use serde::{Deserialize, Deserializer, Serialize};

// ============================================================================
// CORE SWAP TYPES
// ============================================================================

/// Quote request parameters (immutable, passed to all routers)
#[derive(Debug, Clone)]
pub struct QuoteRequest {
    /// The chain this request is for. Routers must refuse a request whose
    /// chain does not match their own (see `SwapRouter::accept_own_chain`).
    pub chain: ChainId,
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub wallet_address: String,
    pub slippage_pct: f64,
    pub swap_mode: SwapMode,
    /// DEX labels to exclude from routing (e.g. "Pump.fun Amm" for graduated tokens)
    pub exclude_dexes: Option<Vec<String>>,
}

/// Swap mode enum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SwapMode {
    ExactIn,
    ExactOut,
}

impl SwapMode {
    /// String representation of the swap mode for API requests
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
    /// The chain this quote was produced for, carried over from the request.
    pub chain: ChainId,
    pub router_id: String,
    pub router_name: String,
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
    /// The minimum amount the receiving wallet is guaranteed to keep.  This is
    /// router-authored: it is not derivable from `output_amount` when a fee is
    /// collected on the output leg.
    pub minimum_output_amount: u64,
    pub price_impact_pct: f64,
    /// A platform fee expressed in WSOL lamports, only when the router can
    /// honestly establish both the fee amount and mint.
    pub platform_fee_lamports: Option<u64>,
    /// The router's estimate of the network fee for this quote, in lamports.
    pub estimated_network_fee_lamports: Option<u64>,
    pub slippage_bps: u16,
    pub route_plan: String,
    pub swap_mode: SwapMode,
    pub wallet_address: String,
    /// Constraints from the request which must survive a fallback/re-quote.
    pub exclude_dexes: Option<Vec<String>>,
    pub execution_data: Vec<u8>,
}

/// Which router a caller intends to use.
///
/// `Auto` compares every enabled router and takes the best net output;
/// `Specific` asks exactly one registered router and fails if it cannot serve
/// the trade. This is a routing decision, not a wire type: an API surface
/// carries the router id as a plain string and parses it here, so an unknown
/// id is refused by the registry rather than by a deserializer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterChoice {
    Auto,
    Specific(String),
}

impl RouterChoice {
    /// Read a caller's selection. Absent, blank and `"auto"` all mean `Auto`;
    /// anything else names a router id the registry must resolve.
    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None => Self::Auto,
            Some(id) if id.eq_ignore_ascii_case("auto") => Self::Auto,
            Some(id) => Self::Specific(id.to_owned()),
        }
    }
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
