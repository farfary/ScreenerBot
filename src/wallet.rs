use crate::global::{ read_configs, is_debug_wallet_enabled };
use crate::tokens::Token;
use crate::logger::{ log, LogTag };
use crate::trader::{ SWAP_FEE_PERCENT, SLIPPAGE_TOLERANCE_PERCENT };
use crate::rpc::get_premium_transaction_rpc;

use reqwest;
use serde::{ Deserialize, Serialize, Deserializer };
use std::error::Error;
use std::fmt;
use base64::{ Engine as _, engine::general_purpose };
use solana_sdk::{
    signature::Keypair,
    transaction::VersionedTransaction,
    signer::Signer,
    pubkey::Pubkey,
    instruction::Instruction,
    transaction::Transaction,
};
use spl_token::instruction::close_account;
use bs58;
use std::str::FromStr;

/// Configuration constants for swap operations
pub const ANTI_MEV: bool = false; // Enable anti-MEV by default
pub const PARTNER: &str = "screenerbot"; // Partner identifier

/// SOL token mint address (native Solana)
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Custom deserializer for fields that can be either string or number
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
    where D: Deserializer<'de>
{
    use serde::de::{ self, Visitor };
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
fn deserialize_optional_string_or_number<'de, D>(
    deserializer: D
) -> Result<Option<String>, D::Error>
    where D: Deserializer<'de>
{
    use serde::de::{ self, Visitor };
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

/// Custom error types for swap operations
#[derive(Debug)]
pub enum SwapError {
    ApiError(String),
    NetworkError(reqwest::Error),
    InvalidResponse(String),
    InsufficientBalance(String),
    SlippageExceeded(String),
    InvalidAmount(String),
    ConfigError(String),
    TransactionError(String),
    SigningError(String),
    ParseError(String),
}

impl fmt::Display for SwapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SwapError::ApiError(msg) => write!(f, "API Error: {}", msg),
            SwapError::NetworkError(err) => write!(f, "Network Error: {}", err),
            SwapError::InvalidResponse(msg) => write!(f, "Invalid Response: {}", msg),
            SwapError::InsufficientBalance(msg) => write!(f, "Insufficient Balance: {}", msg),
            SwapError::SlippageExceeded(msg) => write!(f, "Slippage Exceeded: {}", msg),
            SwapError::InvalidAmount(msg) => write!(f, "Invalid Amount: {}", msg),
            SwapError::ConfigError(msg) => write!(f, "Config Error: {}", msg),
            SwapError::TransactionError(msg) => write!(f, "Transaction Error: {}", msg),
            SwapError::SigningError(msg) => write!(f, "Signing Error: {}", msg),
            SwapError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
        }
    }
}

impl Error for SwapError {}

impl From<reqwest::Error> for SwapError {
    fn from(err: reqwest::Error) -> Self {
        SwapError::NetworkError(err)
    }
}

impl From<serde_json::Error> for SwapError {
    fn from(err: serde_json::Error) -> Self {
        SwapError::ParseError(format!("JSON parsing error: {}", err))
    }
}

/// Quote information from the swap router
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapData {
    pub quote: SwapQuote,
    pub raw_tx: RawTransaction,
    pub amount_in_usd: Option<String>,
    pub amount_out_usd: Option<String>,
    pub jito_order_id: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_string_or_number")]
    pub sol_cost: Option<String>,
}

/// API response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct SwapApiResponse {
    pub code: i32,
    pub msg: String,
    pub tid: Option<String>,
    pub data: Option<SwapData>,
}

/// Swap request parameters
#[derive(Debug, Clone)]
pub struct SwapRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount_sol: f64,
    pub from_address: String,
    pub slippage: f64,
    pub fee: f64,
    pub is_anti_mev: bool,
    pub expected_price: Option<f64>,
}

impl Default for SwapRequest {
    fn default() -> Self {
        Self {
            input_mint: SOL_MINT.to_string(),
            output_mint: String::new(),
            amount_sol: 0.0,
            from_address: String::new(),
            slippage: SLIPPAGE_TOLERANCE_PERCENT,
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
    pub error: Option<String>,
    // New fields for effective price calculation
    pub effective_price: Option<f64>,
    pub actual_input_change: Option<u64>,
    pub actual_output_change: Option<u64>,
    pub quote_vs_actual_difference: Option<f64>,
    // ATA rent separation fields
    pub ata_close_detected: bool,
    pub ata_rent_reclaimed: Option<u64>, // ATA rent amount in lamports
    pub sol_from_trade_only: Option<u64>, // SOL from trade excluding ATA rent
}

/// Transaction details from RPC
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionDetails {
    pub slot: u64,
    pub transaction: TransactionData,
    pub meta: Option<TransactionMeta>,
}

/// Transaction data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionData {
    pub message: serde_json::Value,
    pub signatures: Vec<String>,
}

/// Transaction metadata with balance changes
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionMeta {
    pub err: Option<serde_json::Value>,
    #[serde(rename = "preBalances")]
    pub pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    pub post_balances: Vec<u64>,
    #[serde(rename = "preTokenBalances")]
    pub pre_token_balances: Option<Vec<TokenBalance>>,
    #[serde(rename = "postTokenBalances")]
    pub post_token_balances: Option<Vec<TokenBalance>>,
    pub fee: u64,
    #[serde(rename = "logMessages")]
    pub log_messages: Option<Vec<String>>,
}

/// Token balance information
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalance {
    #[serde(rename = "accountIndex")]
    pub account_index: u32,
    pub mint: String,
    pub owner: Option<String>,
    #[serde(rename = "programId")]
    pub program_id: Option<String>,
    #[serde(rename = "uiTokenAmount")]
    pub ui_token_amount: TokenAmount,
}

/// Token amount details
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenAmount {
    pub amount: String,
    pub decimals: u8,
    #[serde(rename = "uiAmount")]
    pub ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    pub ui_amount_string: Option<String>,
}

/// Converts SOL amount to lamports (1 SOL = 1,000,000,000 lamports)
pub fn sol_to_lamports(sol_amount: f64) -> u64 {
    (sol_amount * 1_000_000_000.0) as u64
}

/// Converts lamports to SOL amount
pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / 1_000_000_000.0
}

/// Gets wallet address from configs by deriving from private key
pub fn get_wallet_address() -> Result<String, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Decode the private key from base58
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key format: {}", e)))?;

    // Create keypair from private key
    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    // Return the public key as base58 string
    Ok(keypair.pubkey().to_string())
}

/// Signs and sends a transaction
pub async fn sign_and_send_transaction(
    swap_transaction_base64: &str,
    rpc_url: &str
) -> Result<String, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "Starting transaction signing: tx_length={} bytes, rpc_url={}",
                swap_transaction_base64.len(),
                rpc_url
            )
        );
    }

    log(
        LogTag::Wallet,
        "SIGN",
        &format!(
            "Signing transaction with wallet (length: {} bytes)",
            swap_transaction_base64.len()
        )
    );

    // Decode the base64 transaction
    let transaction_bytes = general_purpose::STANDARD
        .decode(swap_transaction_base64)
        .map_err(|e| SwapError::SigningError(format!("Failed to decode transaction: {}", e)))?;

    // Deserialize the VersionedTransaction
    let mut transaction: VersionedTransaction = bincode
        ::deserialize(&transaction_bytes)
        .map_err(|e| SwapError::SigningError(format!("Failed to deserialize transaction: {}", e)))?;

    // Create keypair from private key
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key format: {}", e)))?;

    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    // Get the recent blockhash from the transaction message (for reference)
    let _recent_blockhash = match &transaction.message {
        solana_sdk::message::VersionedMessage::Legacy(message) => message.recent_blockhash,
        solana_sdk::message::VersionedMessage::V0(message) => message.recent_blockhash,
    };

    // Sign the transaction
    let signature = keypair.sign_message(&transaction.message.serialize());

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "Transaction signed successfully: wallet_pubkey={}, signature={}",
                keypair.pubkey(),
                signature
            )
        );
    }

    // Add the signature to the transaction
    if transaction.signatures.is_empty() {
        transaction.signatures.push(signature);
    } else {
        transaction.signatures[0] = signature;
    }

    // Serialize the signed transaction back to base64
    let signed_transaction_bytes = bincode
        ::serialize(&transaction)
        .map_err(|e|
            SwapError::SigningError(format!("Failed to serialize signed transaction: {}", e))
        )?;
    let signed_transaction_base64 = general_purpose::STANDARD.encode(&signed_transaction_bytes);

    log(LogTag::Wallet, "SEND", &format!("Sending signed transaction to RPC: {}", rpc_url));

    // Send the signed transaction
    let client = reqwest::Client::new();
    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": [
            signed_transaction_base64,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "processed"
            }
        ]
    });

    // Try fallback RPCs first, main RPC as last resort
    let mut _last_error: Option<SwapError> = None;

    // Try the randomly selected RPC first if it's provided and not the main RPC
    if !rpc_url.eq(&configs.rpc_url) {
        match send_rpc_request(&client, rpc_url, &rpc_payload).await {
            Ok(tx_sig) => {
                log(
                    LogTag::Wallet,
                    "SUCCESS",
                    &format!("Transaction sent successfully via selected RPC: {}", tx_sig)
                );
                return Ok(tx_sig);
            }
            Err(e) => {
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!("Selected RPC {} failed: {}, trying other fallbacks...", rpc_url, e)
                );
                _last_error = Some(e);
            }
        }
    }

    // Try fallback RPCs (skip the one we already tried)
    for fallback_rpc in &configs.rpc_fallbacks {
        if fallback_rpc == rpc_url {
            continue; // Skip if we already tried this RPC
        }
        match send_rpc_request(&client, fallback_rpc, &rpc_payload).await {
            Ok(tx_sig) => {
                log(
                    LogTag::Wallet,
                    "SUCCESS",
                    &format!("Transaction sent via fallback RPC: {}", tx_sig)
                );
                return Ok(tx_sig);
            }
            Err(e) => {
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!("Fallback RPC {} failed: {}", fallback_rpc, e)
                );
                _last_error = Some(e);
            }
        }
    }

    // Try main RPC as a last resort, only if all fallbacks failed and it's not the same as our rpc_url
    if rpc_url != &configs.rpc_url {
        log(LogTag::Wallet, "WARN", "All fallbacks failed, trying main RPC as last resort");
        match send_rpc_request(&client, &configs.rpc_url, &rpc_payload).await {
            Ok(tx_sig) => {
                log(
                    LogTag::Wallet,
                    "SUCCESS",
                    &format!("Transaction sent via main RPC: {}", tx_sig)
                );
                return Ok(tx_sig);
            }
            Err(e) => {
                log(LogTag::Wallet, "ERROR", &format!("Main RPC failed: {}", e));
                _last_error = Some(e);
            }
        }
    }

    // If all RPCs failed, return the last error
    Err(
        _last_error.unwrap_or_else(||
            SwapError::TransactionError("All RPC endpoints failed".to_string())
        )
    )
}

/// Gets transaction details from RPC to analyze balance changes
async fn get_transaction_details(
    client: &reqwest::Client,
    transaction_signature: &str,
    rpc_url: &str
) -> Result<TransactionDetails, SwapError> {
    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            transaction_signature,
            {
                "encoding": "json",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send().await
        .map_err(|e| SwapError::NetworkError(e))?;

    if !response.status().is_success() {
        return Err(
            SwapError::TransactionError(
                format!("Failed to get transaction details: {}", response.status())
            )
        );
    }

    let rpc_response: serde_json::Value = response
        .json().await
        .map_err(|e| SwapError::NetworkError(e))?;

    if let Some(error) = rpc_response.get("error") {
        return Err(
            SwapError::TransactionError(format!("RPC error getting transaction: {:?}", error))
        );
    }

    if let Some(result) = rpc_response.get("result") {
        if result.is_null() {
            return Err(
                SwapError::TransactionError(
                    "Transaction not found or not confirmed yet".to_string()
                )
            );
        }

        let transaction_details: TransactionDetails = serde_json
            ::from_value(result.clone())
            .map_err(|e|
                SwapError::InvalidResponse(format!("Failed to parse transaction details: {}", e))
            )?;

        return Ok(transaction_details);
    }

    Err(SwapError::TransactionError("Invalid transaction response format".to_string()))
}

/// Sends RPC request to a specific endpoint
async fn send_rpc_request(
    client: &reqwest::Client,
    rpc_url: &str,
    payload: &serde_json::Value
) -> Result<String, SwapError> {
    if is_debug_wallet_enabled() {
        log(LogTag::Wallet, "DEBUG", &format!("Sending RPC request to: {}", rpc_url));
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "RPC payload: {}",
                serde_json::to_string(payload).unwrap_or_else(|_| "Invalid JSON".to_string())
            )
        );
    }

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(payload)
        .send().await
        .map_err(|e| SwapError::NetworkError(e))?;

    if is_debug_wallet_enabled() {
        log(LogTag::Wallet, "DEBUG", &format!("RPC response status: {}", response.status()));
    }

    if !response.status().is_success() {
        return Err(
            SwapError::TransactionError(
                format!("RPC request failed with status: {}", response.status())
            )
        );
    }

    let rpc_response: serde_json::Value = response
        .json().await
        .map_err(|e| SwapError::NetworkError(e))?;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "RPC response: {}",
                serde_json
                    ::to_string(&rpc_response)
                    .unwrap_or_else(|_| "Invalid JSON".to_string())
                    [..(500).min(rpc_response.to_string().len())].to_string()
            )
        );
    }

    // Check for RPC errors
    if let Some(error) = rpc_response.get("error") {
        return Err(SwapError::TransactionError(format!("RPC error: {:?}", error)));
    }

    // Extract the transaction signature from the response
    if let Some(result) = rpc_response.get("result") {
        if let Some(signature) = result.as_str() {
            if is_debug_wallet_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!("Transaction signature received: {}", signature)
                );
            }
            return Ok(signature.to_string());
        }
    }

    Err(SwapError::TransactionError("Invalid RPC response format".to_string()))
}

/// Calculates effective price using intended trade amount instead of total balance changes
/// This provides more accurate slippage calculation by excluding fees and ATA costs
pub async fn calculate_effective_price_with_intended_amount(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    intended_trade_amount_sol: f64, // The actual amount intended for trading
    _rpc_url: &str, // Unused parameter - kept for backwards compatibility
    configs: &crate::global::Configs
) -> Result<(f64, u64, u64, f64), SwapError> {
    log(
        LogTag::Wallet,
        "ANALYZE",
        &format!(
            "Calculating effective price for transaction: {} with intended amount: {} SOL",
            transaction_signature,
            intended_trade_amount_sol
        )
    );

    // Wait a moment for transaction to be confirmed
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Try all available RPC endpoints with retries - prioritize fallbacks
    let mut rpc_endpoints = configs.rpc_fallbacks.iter().collect::<Vec<_>>();
    if rpc_endpoints.is_empty() {
        rpc_endpoints.push(&configs.rpc_url);
    }

    let mut transaction_details = None;
    for (rpc_idx, rpc_endpoint) in rpc_endpoints.iter().enumerate() {
        for attempt in 1..=3 {
            match get_transaction_details(client, transaction_signature, rpc_endpoint).await {
                Ok(details) => {
                    transaction_details = Some(details);
                    log(
                        LogTag::Wallet,
                        "SUCCESS",
                        &format!(
                            "Got transaction details from RPC {} on attempt {}",
                            rpc_idx + 1,
                            attempt
                        )
                    );
                    break;
                }
                Err(e) => {
                    log(
                        LogTag::Wallet,
                        "RETRY",
                        &format!("RPC {} attempt {} failed: {}", rpc_idx + 1, attempt, e)
                    );
                    if attempt < 3 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    }
                }
            }
        }
        if transaction_details.is_some() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    let details = transaction_details.ok_or_else(||
        SwapError::TransactionError("Failed to get transaction details after retries".to_string())
    )?;
    let meta = details.meta
        .as_ref()
        .ok_or_else(||
            SwapError::TransactionError("Transaction metadata not available".to_string())
        )?;
    if meta.err.is_some() {
        return Err(SwapError::TransactionError("Transaction failed on-chain".to_string()));
    }

    // Parse the transaction as JSON to extract inner instructions (like debug tool)
    let transaction_json = serde_json
        ::to_value(&details)
        .map_err(|e|
            SwapError::InvalidResponse(format!("Failed to serialize transaction: {}", e))
        )?;

    // Extract token transfer amounts from inner instructions
    let mut input_amount = 0.0;
    let mut output_amount = 0.0;
    let mut input_decimals = 9u8;
    let mut output_decimals = 9u8;

    // Analyze inner instruction transfers to determine exact swap amounts
    if
        let Some(inner_instructions) = transaction_json
            .get("meta")
            .and_then(|m| m.get("innerInstructions"))
            .and_then(|inner| inner.as_array())
    {
        for inner_group in inner_instructions {
            if let Some(instructions) = inner_group.get("instructions").and_then(|i| i.as_array()) {
                for instruction in instructions {
                    if let Some(parsed) = instruction.get("parsed") {
                        if let Some(info) = parsed.get("info") {
                            if
                                let Some(instruction_type) = parsed
                                    .get("type")
                                    .and_then(|t| t.as_str())
                            {
                                if instruction_type == "transferChecked" {
                                    if
                                        let (Some(mint), Some(token_amount)) = (
                                            info.get("mint").and_then(|m| m.as_str()),
                                            info.get("tokenAmount"),
                                        )
                                    {
                                        if
                                            let (Some(ui_amount), Some(decimals)) = (
                                                token_amount
                                                    .get("uiAmount")
                                                    .and_then(|ua| ua.as_f64()),
                                                token_amount
                                                    .get("decimals")
                                                    .and_then(|d| d.as_u64()),
                                            )
                                        {
                                            if mint == input_mint {
                                                input_amount = ui_amount;
                                                input_decimals = decimals as u8;
                                            } else if mint == output_mint {
                                                output_amount = ui_amount;
                                                output_decimals = decimals as u8;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If we couldn't extract from inner instructions, fall back to token balance changes only
    // For SOL amounts, use the INTENDED trade amount instead of total balance change
    if input_amount == 0.0 && output_amount == 0.0 {
        log(
            LogTag::Wallet,
            "DEBUG",
            "No transfers found in inner instructions, falling back to token balance changes"
        );

        // Fall back to balance change analysis for tokens only
        if
            let (Some(pre_tokens), Some(post_tokens)) = (
                &meta.pre_token_balances,
                &meta.post_token_balances,
            )
        {
            // Find input token balance change (non-SOL only)
            if input_mint != SOL_MINT {
                let mut pre_amount = 0.0;
                let mut post_amount = 0.0;

                // Get pre-transaction balance
                for balance in pre_tokens {
                    if
                        balance.mint == input_mint &&
                        balance.owner.as_ref().map(|o| o.as_str()) == Some(wallet_address)
                    {
                        pre_amount = balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                        input_decimals = balance.ui_token_amount.decimals;
                        break;
                    }
                }

                // Get post-transaction balance
                for balance in post_tokens {
                    if
                        balance.mint == input_mint &&
                        balance.owner.as_ref().map(|o| o.as_str()) == Some(wallet_address)
                    {
                        post_amount = balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                        break;
                    }
                }

                // Calculate the change (should be negative for input)
                input_amount = if pre_amount > post_amount {
                    pre_amount - post_amount // Tokens spent
                } else {
                    0.0
                };
            }

            // Find output token balance change (non-SOL only)
            if output_mint != SOL_MINT {
                let mut pre_amount = 0.0;
                let mut post_amount = 0.0;

                // Get pre-transaction balance
                for balance in pre_tokens {
                    if
                        balance.mint == output_mint &&
                        balance.owner.as_ref().map(|o| o.as_str()) == Some(wallet_address)
                    {
                        pre_amount = balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                        output_decimals = balance.ui_token_amount.decimals;
                        break;
                    }
                }

                // Get post-transaction balance
                for balance in post_tokens {
                    if
                        balance.mint == output_mint &&
                        balance.owner.as_ref().map(|o| o.as_str()) == Some(wallet_address)
                    {
                        post_amount = balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                        break;
                    }
                }

                // Calculate the change (should be positive for output)
                output_amount = if post_amount > pre_amount {
                    post_amount - pre_amount // Tokens received
                } else {
                    0.0
                };
            }
        }

        // For SOL amounts, use the intended trade amount to avoid wallet-wide balance corruption
        // This excludes fees, ATA creation costs, and concurrent operations
        if input_mint == SOL_MINT {
            input_amount = intended_trade_amount_sol;
            input_decimals = 9;
            log(
                LogTag::Wallet,
                "DEBUG",
                &format!("Using intended SOL trade amount: {} SOL instead of wallet balance change", intended_trade_amount_sol)
            );
        } else if output_mint == SOL_MINT {
            // For sells (Token -> SOL), use account-specific balance changes
            // Find the wallet's account index to get precise SOL changes
            if
                let Some(account_keys) = transaction_json
                    .get("transaction")
                    .and_then(|t| t.get("message"))
                    .and_then(|m| m.get("accountKeys"))
                    .and_then(|ak| ak.as_array())
            {
                // Find the wallet's SOL account balance change
                let wallet_index = account_keys
                    .iter()
                    .position(|key| key.as_str() == Some(wallet_address));

                if let Some(index) = wallet_index {
                    if
                        let (Some(pre), Some(post)) = (
                            meta.pre_balances.get(index),
                            meta.post_balances.get(index),
                        )
                    {
                        let sol_change = if post > pre {
                            lamports_to_sol(post - pre)
                        } else {
                            lamports_to_sol(pre - post)
                        };

                        // Validate the change makes sense (not too large compared to input tokens)
                        let max_expected_sol = input_amount * 0.001; // Max 0.001 SOL per token (very generous)
                        if sol_change <= max_expected_sol {
                            output_amount = sol_change;
                        } else {
                            // Fallback: calculate based on token amount and conservative price
                            output_amount = input_amount * 0.00001; // Very conservative fallback
                            log(
                                LogTag::Wallet,
                                "DEBUG",
                                &format!(
                                    "SOL change too large ({}), using conservative fallback: {}",
                                    sol_change,
                                    output_amount
                                )
                            );
                        }

                        output_decimals = 9;
                        log(
                            LogTag::Wallet,
                            "DEBUG",
                            &format!(
                                "Using account-specific SOL change: {} SOL (index: {})",
                                output_amount,
                                index
                            )
                        );
                    }
                } else {
                    // Fallback if we can't find the wallet index
                    output_amount = input_amount * 0.00001; // Very conservative fallback
                    output_decimals = 9;
                    log(
                        LogTag::Wallet,
                        "DEBUG",
                        &format!("Could not find wallet index, using conservative fallback: {} SOL", output_amount)
                    );
                }
            }
        }
    }

    // Calculate effective price using intended trade amount
    let effective_price = if input_amount > 0.0 && output_amount > 0.0 {
        if input_mint == SOL_MINT {
            // SOL -> Token: price = SOL / tokens (using intended amount)
            input_amount / output_amount
        } else {
            // Token -> SOL: price = SOL / tokens
            output_amount / input_amount
        }
    } else {
        0.0
    };

    // Convert amounts to raw amounts for return values
    let input_raw = (input_amount * (10_f64).powi(input_decimals as i32)) as u64;
    let output_raw = (output_amount * (10_f64).powi(output_decimals as i32)) as u64;

    log(
        LogTag::Wallet,
        "EFFECTIVE",
        &format!(
            "EffPrice: {:.15} SOL/token (intended_input={:.12} {}, output={:.12} {})",
            effective_price,
            input_amount,
            if input_mint == SOL_MINT {
                "SOL"
            } else {
                "tokens"
            },
            output_amount,
            if output_mint == SOL_MINT {
                "SOL"
            } else {
                "tokens"
            }
        )
    );

    Ok((effective_price, input_raw, output_raw, 0.0))
}

/// Calculates effective price from actual balance changes using the same approach as the debug tool
/// Analyzes inner instruction transfers to determine exact token amounts
/// NOTE: This version includes fees and ATA costs in SOL calculations - use calculate_effective_price_with_intended_amount for accurate slippage
pub async fn calculate_effective_price(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    _rpc_url: &str, // Unused parameter - kept for backwards compatibility
    configs: &crate::global::Configs
) -> Result<(f64, u64, u64, f64), SwapError> {
    log(
        LogTag::Wallet,
        "ANALYZE",
        &format!("Calculating effective price for transaction: {}", transaction_signature)
    );

    // Wait a moment for transaction to be confirmed
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Try all available RPC endpoints with retries - prioritize fallbacks
    let mut rpc_endpoints = configs.rpc_fallbacks.iter().collect::<Vec<_>>();
    if rpc_endpoints.is_empty() {
        rpc_endpoints.push(&configs.rpc_url);
    }

    let mut transaction_details = None;
    for (rpc_idx, rpc_endpoint) in rpc_endpoints.iter().enumerate() {
        for attempt in 1..=3 {
            match get_transaction_details(client, transaction_signature, rpc_endpoint).await {
                Ok(details) => {
                    transaction_details = Some(details);
                    log(
                        LogTag::Wallet,
                        "SUCCESS",
                        &format!(
                            "Got transaction details from RPC {} on attempt {}",
                            rpc_idx + 1,
                            attempt
                        )
                    );
                    break;
                }
                Err(e) => {
                    log(
                        LogTag::Wallet,
                        "RETRY",
                        &format!("RPC {} attempt {} failed: {}", rpc_idx + 1, attempt, e)
                    );
                    if attempt < 3 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
                    }
                }
            }
        }
        if transaction_details.is_some() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    let details = transaction_details.ok_or_else(||
        SwapError::TransactionError("Failed to get transaction details after retries".to_string())
    )?;
    let meta = details.meta
        .as_ref()
        .ok_or_else(||
            SwapError::TransactionError("Transaction metadata not available".to_string())
        )?;
    if meta.err.is_some() {
        return Err(SwapError::TransactionError("Transaction failed on-chain".to_string()));
    }

    // Parse the transaction as JSON to extract inner instructions (like debug tool)
    let transaction_json = serde_json
        ::to_value(&details)
        .map_err(|e|
            SwapError::InvalidResponse(format!("Failed to serialize transaction: {}", e))
        )?;

    // Extract token transfer amounts from inner instructions
    let mut input_amount = 0.0;
    let mut output_amount = 0.0;
    let mut input_decimals = 9u8;
    let mut output_decimals = 9u8;

    // Analyze inner instruction transfers to determine exact swap amounts
    if
        let Some(inner_instructions) = transaction_json
            .get("meta")
            .and_then(|m| m.get("innerInstructions"))
            .and_then(|inner| inner.as_array())
    {
        for inner_group in inner_instructions {
            if let Some(instructions) = inner_group.get("instructions").and_then(|i| i.as_array()) {
                for instruction in instructions {
                    if let Some(parsed) = instruction.get("parsed") {
                        if let Some(info) = parsed.get("info") {
                            if
                                let Some(instruction_type) = parsed
                                    .get("type")
                                    .and_then(|t| t.as_str())
                            {
                                if instruction_type == "transferChecked" {
                                    if
                                        let (Some(mint), Some(token_amount)) = (
                                            info.get("mint").and_then(|m| m.as_str()),
                                            info.get("tokenAmount"),
                                        )
                                    {
                                        if
                                            let (Some(ui_amount), Some(decimals)) = (
                                                token_amount
                                                    .get("uiAmount")
                                                    .and_then(|ua| ua.as_f64()),
                                                token_amount
                                                    .get("decimals")
                                                    .and_then(|d| d.as_u64()),
                                            )
                                        {
                                            if mint == input_mint {
                                                input_amount = ui_amount;
                                                input_decimals = decimals as u8;
                                            } else if mint == output_mint {
                                                output_amount = ui_amount;
                                                output_decimals = decimals as u8;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If we couldn't extract from inner instructions, fall back to balance changes
    if input_amount == 0.0 && output_amount == 0.0 {
        log(
            LogTag::Wallet,
            "DEBUG",
            "No transfers found in inner instructions, falling back to balance changes"
        );

        // Fall back to balance change analysis
        if
            let (Some(pre_tokens), Some(post_tokens)) = (
                &meta.pre_token_balances,
                &meta.post_token_balances,
            )
        {
            // Find input token balance change
            if input_mint != SOL_MINT {
                for balance in pre_tokens {
                    if
                        balance.mint == input_mint &&
                        balance.owner.as_ref().map(|o| o.as_str()) == Some(wallet_address)
                    {
                        input_amount = balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                        input_decimals = balance.ui_token_amount.decimals;
                        break;
                    }
                }
            }

            // Find output token balance change
            if output_mint != SOL_MINT {
                for balance in post_tokens {
                    if
                        balance.mint == output_mint &&
                        balance.owner.as_ref().map(|o| o.as_str()) == Some(wallet_address)
                    {
                        output_amount = balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                        output_decimals = balance.ui_token_amount.decimals;
                        break;
                    }
                }
            }
        }

        // Handle SOL amounts from SOL balance changes
        if input_mint == SOL_MINT || output_mint == SOL_MINT {
            if let (Some(pre), Some(post)) = (meta.pre_balances.get(0), meta.post_balances.get(0)) {
                let sol_change = if post > pre {
                    lamports_to_sol(post - pre)
                } else {
                    lamports_to_sol(pre - post)
                };

                if input_mint == SOL_MINT {
                    input_amount = sol_change;
                    input_decimals = 9;
                } else {
                    output_amount = sol_change;
                    output_decimals = 9;
                }
            }
        }
    }

    // Calculate effective price
    let effective_price = if input_amount > 0.0 && output_amount > 0.0 {
        if input_mint == SOL_MINT {
            // SOL -> Token: price = SOL / tokens
            input_amount / output_amount
        } else {
            // Token -> SOL: price = SOL / tokens
            output_amount / input_amount
        }
    } else {
        0.0
    };

    // Convert amounts to raw amounts for return values
    let input_raw = (input_amount * (10_f64).powi(input_decimals as i32)) as u64;
    let output_raw = (output_amount * (10_f64).powi(output_decimals as i32)) as u64;

    log(
        LogTag::Wallet,
        "EFFECTIVE",
        &format!(
            "EffPrice: {:.15} SOL/token (input={:.12} {}, output={:.12} {})",
            effective_price,
            input_amount,
            if input_mint == SOL_MINT {
                "SOL"
            } else {
                "tokens"
            },
            output_amount,
            if output_mint == SOL_MINT {
                "SOL"
            } else {
                "tokens"
            }
        )
    );

    Ok((effective_price, input_raw, output_raw, 0.0))
}

/// Validates swap parameters before execution
fn validate_swap_request(request: &SwapRequest) -> Result<(), SwapError> {
    if request.input_mint.is_empty() {
        return Err(SwapError::InvalidAmount("Input mint cannot be empty".to_string()));
    }

    if request.output_mint.is_empty() {
        return Err(SwapError::InvalidAmount("Output mint cannot be empty".to_string()));
    }

    if request.from_address.is_empty() {
        return Err(SwapError::InvalidAmount("From address cannot be empty".to_string()));
    }

    if request.amount_sol <= 0.0 {
        return Err(SwapError::InvalidAmount("Amount must be greater than 0".to_string()));
    }

    if request.slippage < 0.0 || request.slippage > 100.0 {
        return Err(
            SwapError::InvalidAmount("Slippage must be between 0 and 100 percent".to_string())
        );
    }

    if request.fee < 0.0 {
        return Err(SwapError::InvalidAmount("Fee cannot be negative".to_string()));
    }

    Ok(())
}

/// Gets a swap quote from the GMGN router API with retry logic
pub async fn get_swap_quote(request: &SwapRequest) -> Result<SwapData, SwapError> {
    validate_swap_request(request)?;

    let amount_lamports = sol_to_lamports(request.amount_sol);

    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&fee={}&is_anti_mev={}&partner={}",
        request.input_mint,
        request.output_mint,
        amount_lamports,
        request.from_address,
        request.slippage,
        request.fee,
        request.is_anti_mev,
        PARTNER
    );

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "Swap request details: amount_sol={:.6}, slippage={}, fee={}, anti_mev={}, from_address={}",
                request.amount_sol,
                request.slippage,
                request.fee,
                request.is_anti_mev,
                &request.from_address[..8]
            )
        );
        log(LogTag::Wallet, "DEBUG", &format!("API URL: {}", url));
    }

    log(
        LogTag::Wallet,
        "QUOTE",
        &format!(
            "Requesting swap quote: {} SOL {} -> {}",
            request.amount_sol,
            if request.input_mint == SOL_MINT {
                "SOL"
            } else {
                &request.input_mint[..8]
            },
            if request.output_mint == SOL_MINT {
                "SOL"
            } else {
                &request.output_mint[..8]
            }
        )
    );

    let client = reqwest::Client::new();
    let mut last_error = None;

    // Retry up to 3 times with increasing delays
    for attempt in 1..=3 {
        match client.get(&url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    let status_code = response.status().as_u16();
                    let error_text = response
                        .text().await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    let error = SwapError::ApiError(
                        format!("HTTP error {}: {}", status_code, error_text)
                    );

                    if attempt < 3 && status_code >= 500 {
                        log(
                            LogTag::Wallet,
                            "WARNING",
                            &format!("API error on attempt {}: {}, retrying...", attempt, error)
                        );
                        last_error = Some(error);
                        tokio::time::sleep(
                            tokio::time::Duration::from_millis(1000 * attempt)
                        ).await;
                        continue;
                    } else {
                        return Err(error);
                    }
                }

                // Get the raw response text first to handle parsing errors better
                let response_text = match response.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        let error = SwapError::NetworkError(e);
                        if attempt < 3 {
                            log(
                                LogTag::Wallet,
                                "WARNING",
                                &format!(
                                    "Network error on attempt {}: {}, retrying...",
                                    attempt,
                                    error
                                )
                            );
                            last_error = Some(error);
                            tokio::time::sleep(
                                tokio::time::Duration::from_millis(1000 * attempt)
                            ).await;
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                };

                // Log the raw response for debugging
                if is_debug_wallet_enabled() {
                    log(
                        LogTag::Wallet,
                        "DEBUG",
                        &format!(
                            "Raw API response: {}",
                            &response_text[..response_text.len().min(500)]
                        )
                    );
                }

                // Try to parse the JSON response with better error handling
                let api_response: SwapApiResponse = match serde_json::from_str(&response_text) {
                    Ok(response) => response,
                    Err(e) => {
                        let error = SwapError::InvalidResponse(
                            format!("JSON parsing error: {} - Response: {}", e, response_text)
                        );
                        if attempt < 3 {
                            log(
                                LogTag::Wallet,
                                "WARNING",
                                &format!(
                                    "Parse error on attempt {}: {}, retrying...",
                                    attempt,
                                    error
                                )
                            );
                            last_error = Some(error);
                            tokio::time::sleep(
                                tokio::time::Duration::from_millis(1000 * attempt)
                            ).await;
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                };

                // Add delay to prevent rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                if api_response.code != 0 {
                    return Err(
                        SwapError::ApiError(
                            format!("API error: {} - {}", api_response.code, api_response.msg)
                        )
                    );
                }

                match api_response.data {
                    Some(data) => {
                        log(
                            LogTag::Wallet,
                            "QUOTE",
                            &format!(
                                "Quote received: {} -> {} (Impact: {}%, Time: {:.3}s)",
                                lamports_to_sol(data.quote.in_amount.parse().unwrap_or(0)),
                                lamports_to_sol(data.quote.out_amount.parse().unwrap_or(0)),
                                data.quote.price_impact_pct,
                                data.quote.time_taken
                            )
                        );
                        return Ok(data);
                    }
                    None => {
                        let error = SwapError::InvalidResponse("No data in response".to_string());
                        if attempt < 3 {
                            log(
                                LogTag::Wallet,
                                "WARNING",
                                &format!("No data on attempt {}, retrying...", attempt)
                            );
                            last_error = Some(error);
                            tokio::time::sleep(
                                tokio::time::Duration::from_millis(1000 * attempt)
                            ).await;
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
            }
            Err(e) => {
                let error = SwapError::NetworkError(e);
                if attempt < 3 {
                    log(
                        LogTag::Wallet,
                        "WARNING",
                        &format!("Network error on attempt {}: {}, retrying...", attempt, error)
                    );
                    last_error = Some(error);
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempt)).await;
                    continue;
                } else {
                    return Err(error);
                }
            }
        }
    }

    // If we get here, all retries failed
    Err(last_error.unwrap_or_else(|| SwapError::ApiError("All retry attempts failed".to_string())))
}

/// Executes a swap operation with a pre-fetched quote to avoid duplicate API calls
pub async fn execute_swap_with_quote(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    amount_sol: f64,
    expected_price: Option<f64>,
    swap_data: SwapData
) -> Result<SwapResult, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;
    let wallet_address = get_wallet_address()?;

    log(
        LogTag::Wallet,
        "SWAP",
        &format!(
            "Executing swap for {} ({}) - {} SOL {} -> {} (using cached quote)",
            token.symbol,
            token.name,
            amount_sol,
            if input_mint == SOL_MINT {
                "SOL"
            } else {
                &input_mint[..8]
            },
            if output_mint == SOL_MINT {
                "SOL"
            } else {
                &output_mint[..8]
            }
        )
    );

    // Validate expected price if provided (using cached quote)
    if let Some(expected) = expected_price {
        let input_sol = amount_sol;
        let output_amount_str = &swap_data.quote.out_amount;
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("Final - Raw out_amount string: '{}'", output_amount_str)
        );

        let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Final - Failed to parse out_amount '{}': {}", output_amount_str, e)
            );
            0.0
        });

        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("Final - Parsed output_amount_raw: {}", output_amount_raw)
        );

        // CRITICAL FIX: Use actual token decimals from quote response, not token struct
        let token_decimals = swap_data.quote.out_decimals as u32;
        let output_tokens = output_amount_raw / (10_f64).powi(token_decimals as i32);

        let actual_price_per_token = if output_tokens > 0.0 {
            input_sol / output_tokens
        } else {
            0.0
        };

        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "Final price calc debug: raw_amount={}, decimals={} (from quote), output_tokens={:.12}, actual_price={:.12}",
                output_amount_raw,
                token_decimals,
                output_tokens,
                actual_price_per_token
            )
        );

        let price_difference = (((actual_price_per_token - expected) / expected) * 100.0).abs();

        log(
            LogTag::Wallet,
            "PRICE",
            &format!(
                "Final price validation: Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
                expected,
                actual_price_per_token,
                price_difference
            )
        );

        if price_difference > SLIPPAGE_TOLERANCE_PERCENT {
            return Err(
                SwapError::SlippageExceeded(
                    format!(
                        "Price difference {:.2}% exceeds slippage tolerance {:.2}%",
                        price_difference,
                        SLIPPAGE_TOLERANCE_PERCENT
                    )
                )
            );
        }
    }

    // Sign and send the transaction using premium RPC
    let selected_rpc = get_premium_transaction_rpc(&configs);
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &selected_rpc
    ).await?;

    log(
        LogTag::Wallet,
        "SIGN",
        &format!("Transaction submitted successfully! TX: {}", transaction_signature)
    );

    // Calculate effective price using intended trade amount for accurate slippage calculation
    let (effective_price, actual_input_change, actual_output_change, quote_vs_actual_diff) = match
        calculate_effective_price_with_intended_amount(
            &reqwest::Client::new(),
            &transaction_signature,
            input_mint,
            output_mint,
            &wallet_address,
            amount_sol, // Use the intended trade amount instead of total balance change
            &selected_rpc, // Use the same randomly selected RPC endpoint
            &configs
        ).await
    {
        Ok((effective_price, input_change, output_change, diff)) => {
            log(
                LogTag::Wallet,
                "SUCCESS",
                &format!(
                    "Transaction verified successful on-chain. Effective price: {:.12} SOL",
                    effective_price
                )
            );
            (effective_price, input_change, output_change, diff)
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Transaction failed on-chain or verification failed: {}", e)
            );

            // Return failed swap result for failed transactions
            return Ok(SwapResult {
                success: false,
                transaction_signature: Some(transaction_signature),
                input_amount: swap_data.quote.in_amount,
                output_amount: swap_data.quote.out_amount,
                price_impact: swap_data.quote.price_impact_pct,
                fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
                execution_time: swap_data.quote.time_taken,
                error: Some(format!("Transaction failed on-chain: {}", e)),
                effective_price: None,
                actual_input_change: None,
                actual_output_change: None,
                quote_vs_actual_difference: None,
                ata_close_detected: false,
                ata_rent_reclaimed: None,
                sol_from_trade_only: None,
            });
        }
    };

    Ok(SwapResult {
        success: true,
        transaction_signature: Some(transaction_signature),
        input_amount: swap_data.quote.in_amount,
        output_amount: swap_data.quote.out_amount,
        price_impact: swap_data.quote.price_impact_pct,
        fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
        execution_time: swap_data.quote.time_taken,
        error: None,
        effective_price: Some(effective_price),
        actual_input_change: Some(actual_input_change),
        actual_output_change: Some(actual_output_change),
        quote_vs_actual_difference: Some(quote_vs_actual_diff),
        ata_close_detected: false, // Buy transactions don't close ATAs
        ata_rent_reclaimed: None,
        sol_from_trade_only: None,
    })
}

/// Executes a swap operation with real transaction signing and sending
pub async fn execute_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    amount_sol: f64,
    expected_price: Option<f64>
) -> Result<SwapResult, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;
    let wallet_address = get_wallet_address()?;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "Starting swap execution: token={} ({}), amount_sol={}, input_mint={}, output_mint={}, wallet_address={}",
                token.symbol,
                token.name,
                amount_sol,
                input_mint,
                output_mint,
                &wallet_address[..8]
            )
        );
        if let Some(price) = expected_price {
            log(LogTag::Wallet, "DEBUG", &format!("Expected price: {:.10} SOL per token", price));
        }
    }

    let request = SwapRequest {
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        amount_sol,
        from_address: wallet_address.clone(),
        expected_price,
        ..Default::default()
    };

    log(
        LogTag::Wallet,
        "SWAP",
        &format!(
            "Executing swap for {} ({}) - {} SOL {} -> {}",
            token.symbol,
            token.name,
            amount_sol,
            if input_mint == SOL_MINT {
                "SOL"
            } else {
                &input_mint[..8]
            },
            if output_mint == SOL_MINT {
                "SOL"
            } else {
                &output_mint[..8]
            }
        )
    );

    // Get quote first
    if is_debug_wallet_enabled() {
        log(LogTag::Wallet, "DEBUG", "Requesting swap quote...");
    }
    let swap_data = get_swap_quote(&request).await?;

    // Validate expected price if provided
    if let Some(expected) = expected_price {
        // Calculate the actual price per token from the quote
        let input_sol = request.amount_sol;
        let output_amount_raw = swap_data.quote.out_amount.parse().unwrap_or(0) as f64;
        // CRITICAL FIX: Use actual token decimals from quote response, not token struct
        let token_decimals = swap_data.quote.out_decimals as u32;
        let output_tokens = output_amount_raw / (10_f64).powi(token_decimals as i32);

        let actual_price_per_token = if output_tokens > 0.0 {
            input_sol / output_tokens
        } else {
            0.0
        };

        let price_difference = (((actual_price_per_token - expected) / expected) * 100.0).abs();

        log(
            LogTag::Wallet,
            "PRICE",
            &format!(
                "Price validation: Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
                expected,
                actual_price_per_token,
                price_difference
            )
        );

        if price_difference > request.slippage {
            return Err(
                SwapError::SlippageExceeded(
                    format!(
                        "Price difference {:.2}% exceeds slippage tolerance {:.2}%",
                        price_difference,
                        request.slippage
                    )
                )
            );
        }
    }

    // Sign and send the transaction using premium RPC (buy_token)
    let selected_rpc = get_premium_transaction_rpc(&configs);
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &selected_rpc
    ).await?;

    log(
        LogTag::Wallet,
        "SUCCESS",
        &format!("Swap executed successfully! TX: {}", transaction_signature)
    );

    // Calculate effective price from actual balance changes
    let (effective_price, actual_input_change, actual_output_change, quote_vs_actual_diff) =
        calculate_effective_price(
            &reqwest::Client::new(),
            &transaction_signature,
            input_mint,
            output_mint,
            &wallet_address,
            &selected_rpc, // Use the same randomly selected RPC endpoint
            &configs
        ).await.unwrap_or_else(|e| {
            log(LogTag::Wallet, "WARNING", &format!("Failed to calculate effective price: {}", e));
            (0.0, 0, 0, 0.0)
        });

    Ok(SwapResult {
        success: true,
        transaction_signature: Some(transaction_signature),
        input_amount: swap_data.quote.in_amount,
        output_amount: swap_data.quote.out_amount,
        price_impact: swap_data.quote.price_impact_pct,
        fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
        execution_time: swap_data.quote.time_taken,
        error: None,
        effective_price: Some(effective_price),
        actual_input_change: Some(actual_input_change),
        actual_output_change: Some(actual_output_change),
        quote_vs_actual_difference: Some(quote_vs_actual_diff),
        ata_close_detected: false, // This is from execute_swap (buy operation)
        ata_rent_reclaimed: None,
        sol_from_trade_only: None,
    })
}

/// Helper function to buy a token with SOL
pub async fn buy_token(
    token: &Token,
    amount_sol: f64,
    expected_price: Option<f64>
) -> Result<SwapResult, SwapError> {
    // CRITICAL SAFETY CHECK: Validate expected price if provided
    if let Some(price) = expected_price {
        if price <= 0.0 || !price.is_finite() {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!(
                    "❌ REFUSING TO BUY: Invalid expected_price for {} ({}). Price = {:.10}",
                    token.symbol,
                    token.mint,
                    price
                )
            );
            return Err(SwapError::InvalidAmount(format!("Invalid expected price: {:.10}", price)));
        }
    }

    let wallet_address = get_wallet_address()?;

    log(
        LogTag::Wallet,
        "BUY",
        &format!(
            "🎯 Starting token purchase: {} ({}) | Amount: {:.6} SOL | Expected price: {}",
            token.symbol,
            token.name,
            amount_sol,
            expected_price.map(|p| format!("{:.8} SOL", p)).unwrap_or_else(|| "Any".to_string())
        )
    );

    // Check SOL balance before swap
    log(LogTag::Wallet, "BALANCE", "💰 Checking SOL balance...");
    let sol_balance = get_sol_balance(&wallet_address).await?;
    log(LogTag::Wallet, "BALANCE", &format!("💰 Current SOL balance: {:.6} SOL", sol_balance));

    if sol_balance < amount_sol {
        log(
            LogTag::Wallet,
            "ERROR",
            &format!(
                "❌ Insufficient SOL balance! Have: {:.6} SOL, Need: {:.6} SOL (Deficit: {:.6} SOL)",
                sol_balance,
                amount_sol,
                amount_sol - sol_balance
            )
        );
        return Err(
            SwapError::InsufficientBalance(
                format!(
                    "Insufficient SOL balance. Have: {:.6} SOL, Need: {:.6} SOL",
                    sol_balance,
                    amount_sol
                )
            )
        );
    }

    log(
        LogTag::Wallet,
        "SUCCESS",
        &format!(
            "✅ SOL balance sufficient! Available: {:.6} SOL, Required: {:.6} SOL",
            sol_balance,
            amount_sol
        )
    );

    // Get quote once and use it for both price validation and execution
    let request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: token.mint.clone(),
        amount_sol,
        from_address: wallet_address.clone(),
        expected_price,
        ..Default::default()
    };

    log(
        LogTag::Wallet,
        "QUOTE",
        &format!(
            "📊 Requesting swap quote: {} SOL → {} | Mint: {}...{}",
            amount_sol,
            token.symbol,
            &token.mint[..8],
            &token.mint[token.mint.len() - 8..]
        )
    );

    // Get quote once
    let swap_data = get_swap_quote(&request).await?;

    log(
        LogTag::Wallet,
        "QUOTE",
        &format!(
            "📋 Quote received: Input: {} | Output: {} | Price Impact: {:.4}% | Fee: {} lamports",
            swap_data.quote.in_amount,
            swap_data.quote.out_amount,
            swap_data.quote.price_impact_pct,
            swap_data.raw_tx.prioritization_fee_lamports
        )
    );

    // Check current price if expected price is provided
    if let Some(expected) = expected_price {
        log(LogTag::Wallet, "PRICE", "🔍 Validating current token price...");

        // Calculate current price from quote, accounting for token decimals
        let output_amount_str = &swap_data.quote.out_amount;
        log(LogTag::Wallet, "DEBUG", &format!("📋 Raw out_amount string: '{}'", output_amount_str));

        let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("❌ Failed to parse out_amount '{}': {}", output_amount_str, e)
            );
            0.0
        });

        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("🔢 Parsed output_amount_raw: {}", output_amount_raw)
        );

        // CRITICAL FIX: Use the actual token decimals from the quote response, not from token struct
        let token_decimals = swap_data.quote.out_decimals as u32;
        let output_tokens = output_amount_raw / (10_f64).powi(token_decimals as i32);
        let current_price = if output_tokens > 0.0 { amount_sol / output_tokens } else { 0.0 };

        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "🧮 Price calculation: raw_amount={}, decimals={} (from quote), output_tokens={:.12}, current_price={:.12}",
                output_amount_raw,
                token_decimals,
                output_tokens,
                current_price
            )
        );

        log(
            LogTag::Wallet,
            "PRICE",
            &format!("💲 Current price: {:.12} SOL, Expected: {:.12} SOL", current_price, expected)
        );

        // Use 5% tolerance for price validation
        if current_price > 0.0 && !validate_price_near_expected(current_price, expected, 5.0) {
            let price_diff = ((current_price - expected) / expected) * 100.0;
            log(
                LogTag::Wallet,
                "ERROR",
                &format!(
                    "❌ Price validation failed! Current: {:.12} SOL, Expected: {:.12} SOL, Diff: {:.2}% (Max: {:.1}%)",
                    current_price,
                    expected,
                    price_diff,
                    SLIPPAGE_TOLERANCE_PERCENT
                )
            );
            return Err(
                SwapError::SlippageExceeded(
                    format!(
                        "Current price {:.12} SOL differs from expected {:.12} SOL by {:.2}% (tolerance: {:.1}%)",
                        current_price,
                        expected,
                        price_diff,
                        SLIPPAGE_TOLERANCE_PERCENT
                    )
                )
            );
        } else if current_price <= 0.0 {
            log(
                LogTag::Wallet,
                "WARNING",
                "⚠️ Could not calculate current price from quote, proceeding without validation"
            );
        } else {
            let price_diff = ((current_price - expected) / expected) * 100.0;
            log(
                LogTag::Wallet,
                "SUCCESS",
                &format!(
                    "✅ Price validation passed! Diff: {:.2}% (within {:.1}% tolerance)",
                    price_diff,
                    SLIPPAGE_TOLERANCE_PERCENT
                )
            );
        }
    }

    log(LogTag::Wallet, "SWAP", &format!("🚀 Executing swap with validated quote..."));

    execute_swap_with_quote(
        token,
        SOL_MINT,
        &token.mint,
        amount_sol,
        expected_price,
        swap_data
    ).await
}

/// Helper function to sell a token for SOL
pub async fn sell_token(
    token: &Token,
    token_amount: u64, // Amount in token's smallest unit
    expected_sol_output: Option<f64>
) -> Result<SwapResult, SwapError> {
    // CRITICAL SAFETY CHECK: Validate expected SOL output if provided
    if let Some(expected_sol) = expected_sol_output {
        if expected_sol <= 0.0 || !expected_sol.is_finite() {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!(
                    "❌ REFUSING TO SELL: Invalid expected_sol_output for {} ({}). Expected SOL = {:.10}",
                    token.symbol,
                    token.mint,
                    expected_sol
                )
            );
            return Err(
                SwapError::InvalidAmount(
                    format!("Invalid expected SOL output: {:.10}", expected_sol)
                )
            );
        }
    }

    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;
    let wallet_address = get_wallet_address()?;

    // Check if trying to sell 0 tokens
    if token_amount == 0 {
        return Err(
            SwapError::InvalidAmount(
                "Cannot sell 0 tokens. Token amount must be greater than 0.".to_string()
            )
        );
    }

    // Check token balance before swap
    log(LogTag::Wallet, "BALANCE", &format!("Checking {} balance...", token.symbol));
    let token_balance = get_token_balance(&wallet_address, &token.mint).await?;
    log(
        LogTag::Wallet,
        "BALANCE",
        &format!("Current {} balance: {} tokens", token.symbol, token_balance)
    );

    if token_balance < token_amount {
        return Err(
            SwapError::InsufficientBalance(
                format!(
                    "Insufficient {} balance. Have: {} tokens, Need: {} tokens",
                    token.symbol,
                    token_balance,
                    token_amount
                )
            )
        );
    }

    // Check current price if expected SOL output is provided
    if let Some(expected_sol) = expected_sol_output {
        log(LogTag::Wallet, "PRICE", "Validating expected SOL output...");
        match get_token_price_sol(&token.mint).await {
            Ok(current_price) => {
                let estimated_sol_output = current_price * (token_amount as f64);
                log(
                    LogTag::Wallet,
                    "PRICE",
                    &format!(
                        "Estimated SOL output: {:.6} SOL, Expected: {:.6} SOL",
                        estimated_sol_output,
                        expected_sol
                    )
                );

                // Use 5% tolerance for price validation
                if !validate_price_near_expected(estimated_sol_output, expected_sol, 5.0) {
                    let price_diff = ((estimated_sol_output - expected_sol) / expected_sol) * 100.0;
                    return Err(
                        SwapError::SlippageExceeded(
                            format!(
                                "Estimated SOL output {:.6} differs from expected {:.6} by {:.2}% (tolerance: 5%)",
                                estimated_sol_output,
                                expected_sol,
                                price_diff
                            )
                        )
                    );
                }
                log(LogTag::Wallet, "PRICE", "✅ Price validation passed");
            }
            Err(e) => {
                log(LogTag::Wallet, "WARNING", &format!("Could not validate price: {}", e));
            }
        }
    }

    let request = SwapRequest {
        input_mint: token.mint.clone(),
        output_mint: SOL_MINT.to_string(),
        amount_sol: 0.0, // Not used for token-to-SOL swaps
        from_address: wallet_address.clone(),
        expected_price: expected_sol_output,
        ..Default::default()
    };

    log(
        LogTag::Wallet,
        "SWAP",
        &format!(
            "Executing sell for {} ({}) - {} tokens -> SOL",
            token.symbol,
            token.name,
            token_amount
        )
    );

    // Build URL for token-to-SOL swap
    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&fee={}&is_anti_mev={}&partner={}",
        request.input_mint,
        request.output_mint,
        token_amount,
        request.from_address,
        request.slippage,
        request.fee,
        request.is_anti_mev,
        PARTNER
    );

    log(
        LogTag::Wallet,
        "QUOTE",
        &format!("Requesting sell quote: {} tokens {} -> SOL", token_amount, &token.symbol)
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(SwapError::ApiError(format!("HTTP error: {}", response.status())));
    }

    // Get response text first for better error reporting
    let response_text = response.text().await?;

    // Try to parse the JSON response with better error handling
    let api_response: SwapApiResponse = match serde_json::from_str(&response_text) {
        Ok(response) => response,
        Err(e) => {
            return Err(
                SwapError::InvalidResponse(
                    format!("JSON parsing error: {} - Response: {}", e, response_text)
                )
            );
        }
    };

    if api_response.code != 0 {
        return Err(
            SwapError::ApiError(format!("API error: {} - {}", api_response.code, api_response.msg))
        );
    }

    let swap_data = match api_response.data {
        Some(data) => data,
        None => {
            return Err(SwapError::InvalidResponse("No data in response".to_string()));
        }
    };

    log(
        LogTag::Wallet,
        "QUOTE",
        &format!(
            "Sell quote received: {} tokens -> {} SOL (Impact: {}%, Time: {:.3}s)",
            token_amount,
            lamports_to_sol(swap_data.quote.out_amount.parse().unwrap_or(0)),
            swap_data.quote.price_impact_pct,
            swap_data.quote.time_taken
        )
    );

    // Validate expected output if provided
    if let Some(expected) = expected_sol_output {
        let actual_output = lamports_to_sol(swap_data.quote.out_amount.parse().unwrap_or(0));
        let price_difference = (((actual_output - expected) / expected) * 100.0).abs();

        if price_difference > request.slippage {
            return Err(
                SwapError::SlippageExceeded(
                    format!(
                        "Price difference {:.2}% exceeds slippage tolerance {:.2}%",
                        price_difference,
                        request.slippage
                    )
                )
            );
        }
    }

    // Sign and send the transaction using premium RPC (sell_token)
    let selected_rpc = get_premium_transaction_rpc(&configs);
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &selected_rpc
    ).await?;

    log(
        LogTag::Wallet,
        "SUCCESS",
        &format!("Sell executed successfully! TX: {}", transaction_signature)
    );

    // Calculate effective price from actual transaction data
    let (effective_price, actual_input_change, actual_output_change, quote_vs_actual_diff) =
        calculate_effective_price(
            &reqwest::Client::new(),
            &transaction_signature,
            &request.input_mint,
            &request.output_mint,
            &wallet_address,
            &configs.rpc_url,
            &configs
        ).await.unwrap_or_else(|e| {
            log(LogTag::Wallet, "WARNING", &format!("Failed to calculate effective price: {}", e));
            (0.0, 0, 0, 0.0)
        });

    // ============== DEBUG WALLET LOGGING - FINAL CALCULATIONS ==============
    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG_PRICE",
            &format!(
                "💰 EFFECTIVE PRICE CALCULATION - Token: {} | Effective Price: {:.12} SOL | Input Change: {} lamports | Output: {:.9} SOL",
                token.symbol,
                effective_price,
                actual_input_change,
                lamports_to_sol(actual_output_change)
            )
        );

        let entry_sol = lamports_to_sol(actual_input_change);
        let exit_sol = lamports_to_sol(actual_output_change);
        let profit_sol = exit_sol - entry_sol;
        let profit_percent = if entry_sol > 0.0 { (profit_sol / entry_sol) * 100.0 } else { 0.0 };

        log(
            LogTag::Wallet,
            "DEBUG_PROFIT",
            &format!(
                "📊 PROFIT ANALYSIS - Token: {} | Entry: {:.9} SOL | Exit: {:.9} SOL | P&L: {:.9} SOL ({:.2}%)",
                token.symbol,
                entry_sol,
                exit_sol,
                profit_sol,
                profit_percent
            )
        );

        // Check if this looks like a big loss that might be incorrect
        if profit_percent < -50.0 {
            log(
                LogTag::Wallet,
                "DEBUG_ALERT",
                &format!(
                    "⚠️ LARGE LOSS DETECTED - Token: {} | Loss: {:.2}% | Please verify calculation accuracy",
                    token.symbol,
                    profit_percent
                )
            );
        }
    }

    // Note: ATA cleanup is now handled by background service - no blocking here
    log(LogTag::Wallet, "ATA", "💡 ATA cleanup handled by background service");

    Ok(SwapResult {
        success: true,
        transaction_signature: Some(transaction_signature),
        input_amount: swap_data.quote.in_amount,
        output_amount: swap_data.quote.out_amount,
        price_impact: swap_data.quote.price_impact_pct,
        fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
        execution_time: swap_data.quote.time_taken,
        error: None,
        effective_price: Some(effective_price),
        actual_input_change: Some(actual_input_change),
        actual_output_change: Some(actual_output_change),
        quote_vs_actual_difference: Some(quote_vs_actual_diff),
        ata_close_detected: false, // Simplified - no longer detecting ATA
        ata_rent_reclaimed: None,
        sol_from_trade_only: None,
    })
}

/// Public function to manually close all empty ATAs for the configured wallet
/// Note: ATA cleanup is now handled automatically by background service (see ata_cleanup.rs)
/// This function is kept for manual cleanup or emergency situations
pub async fn cleanup_all_empty_atas() -> Result<(u32, Vec<String>), SwapError> {
    log(
        LogTag::Wallet,
        "ATA",
        "⚠️ Manual ATA cleanup triggered (normally handled by background service)"
    );
    let wallet_address = get_wallet_address()?;
    close_all_empty_atas(&wallet_address).await
}

/// Gets current token price by requesting a small quote
pub async fn get_token_price_sol(token_mint: &str) -> Result<f64, SwapError> {
    let wallet_address = get_wallet_address()?;
    let small_amount = 0.001; // 0.001 SOL

    let request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: token_mint.to_string(),
        amount_sol: small_amount,
        from_address: wallet_address,
        ..Default::default()
    };

    let quote = get_swap_quote(&request).await?;
    let output_lamports: u64 = quote.quote.out_amount
        .parse()
        .map_err(|_| SwapError::InvalidResponse("Invalid output amount".to_string()))?;

    let output_tokens = output_lamports as f64;
    let price_per_token = (small_amount * 1_000_000_000.0) / output_tokens; // Price in lamports per token

    Ok(price_per_token / 1_000_000_000.0) // Convert back to SOL
}

/// Checks wallet balance for SOL
pub async fn get_sol_balance(wallet_address: &str) -> Result<f64, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("Checking SOL balance for wallet: {}", &wallet_address[..8])
        );
    }

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [wallet_address]
    });

    let client = reqwest::Client::new();

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for rpc_url in rpc_endpoints {
        match
            client
                .post(rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(balance_lamports) = value.as_u64() {
                                let balance_sol = lamports_to_sol(balance_lamports);
                                if is_debug_wallet_enabled() {
                                    log(
                                        LogTag::Wallet,
                                        "DEBUG",
                                        &format!(
                                            "SOL balance retrieved: {} lamports ({:.6} SOL) from RPC: {}",
                                            balance_lamports,
                                            balance_sol,
                                            rpc_url
                                        )
                                    );
                                }
                                return Ok(balance_sol);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Wallet,
                    "WARNING",
                    &format!("Failed to get balance from {}: {}", rpc_url, e)
                );
                continue;
            }
        }
    }

    Err(SwapError::TransactionError("Failed to get balance from all RPC endpoints".to_string()))
}

/// Checks wallet balance for a specific token
pub async fn get_token_balance(wallet_address: &str, mint: &str) -> Result<u64, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet_address,
            {
                "mint": mint
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let client = reqwest::Client::new();

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for rpc_url in rpc_endpoints {
        match
            client
                .post(rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(accounts) = value.as_array() {
                                if let Some(account) = accounts.first() {
                                    if let Some(account_data) = account.get("account") {
                                        if let Some(data) = account_data.get("data") {
                                            if let Some(parsed) = data.get("parsed") {
                                                if let Some(info) = parsed.get("info") {
                                                    if
                                                        let Some(token_amount) =
                                                            info.get("tokenAmount")
                                                    {
                                                        if
                                                            let Some(amount_str) =
                                                                token_amount.get("amount")
                                                        {
                                                            if
                                                                let Some(amount_str) =
                                                                    amount_str.as_str()
                                                            {
                                                                if
                                                                    let Ok(amount) =
                                                                        amount_str.parse::<u64>()
                                                                {
                                                                    return Ok(amount);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Wallet,
                    "WARNING",
                    &format!("Failed to get token balance from {}: {}", rpc_url, e)
                );
                continue;
            }
        }
    }

    Ok(0) // Return 0 if no token account found or all RPCs failed
}

/// Validates if the current price is near the expected price within tolerance
pub fn validate_price_near_expected(
    current_price: f64,
    expected_price: f64,
    tolerance_percent: f64
) -> bool {
    let price_difference = (((current_price - expected_price) / expected_price) * 100.0).abs();
    price_difference <= tolerance_percent
}

/// Structure to hold token account information
#[derive(Debug)]
pub struct TokenAccountInfo {
    pub account: String,
    pub mint: String,
    pub balance: u64,
    pub is_token_2022: bool,
}

/// Gets all token accounts for a wallet
pub async fn get_all_token_accounts(
    wallet_address: &str
) -> Result<Vec<TokenAccountInfo>, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet_address,
            {
                "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    // Also get Token-2022 accounts
    let rpc_payload_2022 =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet_address,
            {
                "programId": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let client = reqwest::Client::new();
    let mut all_accounts = Vec::new();

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for rpc_url in &rpc_endpoints {
        // Get SPL Token accounts
        if
            let Ok(response) = client
                .post(*rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                if let Some(result) = rpc_response.get("result") {
                    if let Some(value) = result.get("value") {
                        if let Some(accounts) = value.as_array() {
                            for account in accounts {
                                if
                                    let Some(parsed_info) = extract_token_account_info(
                                        account,
                                        false
                                    )
                                {
                                    all_accounts.push(parsed_info);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Get Token-2022 accounts
        if
            let Ok(response) = client
                .post(*rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload_2022)
                .send().await
        {
            if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                if let Some(result) = rpc_response.get("result") {
                    if let Some(value) = result.get("value") {
                        if let Some(accounts) = value.as_array() {
                            for account in accounts {
                                if
                                    let Some(parsed_info) = extract_token_account_info(
                                        account,
                                        true
                                    )
                                {
                                    all_accounts.push(parsed_info);
                                }
                            }
                        }
                    }
                }
            }
        }

        // If we got accounts from this RPC, break
        if !all_accounts.is_empty() {
            break;
        }
    }

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Found {} total token accounts for wallet", all_accounts.len())
    );
    Ok(all_accounts)
}

/// Extracts token account information from RPC response
fn extract_token_account_info(
    account: &serde_json::Value,
    is_token_2022: bool
) -> Option<TokenAccountInfo> {
    let pubkey = account.get("pubkey")?.as_str()?;
    let account_data = account.get("account")?;
    let parsed = account_data.get("data")?.get("parsed")?;
    let info = parsed.get("info")?;

    let mint = info.get("mint")?.as_str()?;
    let token_amount = info.get("tokenAmount")?;
    let amount_str = token_amount.get("amount")?.as_str()?;
    let balance = amount_str.parse::<u64>().ok()?;

    Some(TokenAccountInfo {
        account: pubkey.to_string(),
        mint: mint.to_string(),
        balance,
        is_token_2022,
    })
}

/// Closes a single empty ATA (Associated Token Account) for a specific mint
/// Returns the transaction signature if successful
pub async fn close_single_ata(wallet_address: &str, mint: &str) -> Result<String, SwapError> {
    log(LogTag::Wallet, "ATA", &format!("Attempting to close single ATA for mint {}", &mint[..8]));

    // Get all token accounts to find the specific one
    let token_accounts = get_all_token_accounts(wallet_address).await?;

    // Find the account for this mint
    let target_account = token_accounts
        .iter()
        .find(|account| account.mint == mint && account.balance == 0);

    match target_account {
        Some(account) => {
            log(
                LogTag::Wallet,
                "ATA",
                &format!("Found empty ATA {} for mint {}", account.account, &mint[..8])
            );

            // Close the ATA
            match close_ata(wallet_address, &account.account, mint, account.is_token_2022).await {
                Ok(signature) => {
                    log(
                        LogTag::Wallet,
                        "SUCCESS",
                        &format!(
                            "Closed ATA {} for mint {}. TX: {}",
                            account.account,
                            &mint[..8],
                            signature
                        )
                    );
                    Ok(signature)
                }
                Err(e) => {
                    log(
                        LogTag::Wallet,
                        "ERROR",
                        &format!(
                            "Failed to close ATA {} for mint {}: {}",
                            account.account,
                            &mint[..8],
                            e
                        )
                    );
                    Err(e)
                }
            }
        }
        None => {
            let error_msg = format!("No empty ATA found for mint {}", &mint[..8]);
            log(LogTag::Wallet, "WARNING", &error_msg);
            Err(SwapError::InvalidAmount(error_msg))
        }
    }
}

/// Closes all empty ATAs (Associated Token Accounts) for a wallet
/// This reclaims the rent SOL (~0.002 SOL per account) from all empty token accounts
/// Returns the number of accounts closed and total signatures
pub async fn close_all_empty_atas(wallet_address: &str) -> Result<(u32, Vec<String>), SwapError> {
    log(LogTag::Wallet, "ATA", "🔍 Checking for empty token accounts to close...");

    // Get all token accounts for the wallet
    let all_accounts = get_all_token_accounts(wallet_address).await?;

    if all_accounts.is_empty() {
        log(LogTag::Wallet, "ATA", "No token accounts found in wallet");
        return Ok((0, vec![]));
    }

    // Filter for empty accounts (balance = 0)
    let empty_accounts: Vec<&TokenAccountInfo> = all_accounts
        .iter()
        .filter(|account| account.balance == 0)
        .collect();

    if empty_accounts.is_empty() {
        log(LogTag::Wallet, "ATA", "No empty token accounts found to close");
        return Ok((0, vec![]));
    }

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Found {} empty token accounts to close", empty_accounts.len())
    );

    let mut signatures = Vec::new();
    let mut closed_count = 0u32;

    // Close each empty account
    for account_info in empty_accounts {
        log(
            LogTag::Wallet,
            "ATA",
            &format!(
                "Closing empty {} account {} for mint {}",
                if account_info.is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                account_info.account,
                account_info.mint
            )
        );

        match
            close_ata(
                wallet_address,
                &account_info.account,
                &account_info.mint,
                account_info.is_token_2022
            ).await
        {
            Ok(signature) => {
                log(
                    LogTag::Wallet,
                    "SUCCESS",
                    &format!("✅ Closed empty ATA {}. TX: {}", account_info.account, signature)
                );
                signatures.push(signature);
                closed_count += 1;

                // Small delay between closures to avoid overwhelming the network
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            Err(e) => {
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!("❌ Failed to close ATA {}: {}", account_info.account, e)
                );
                // Continue with other accounts even if one fails
            }
        }
    }

    let rent_reclaimed = (closed_count as f64) * 0.00203928; // Approximate ATA rent in SOL
    log(
        LogTag::Wallet,
        "ATA",
        &format!(
            "🎉 ATA cleanup complete! Closed {} accounts, reclaimed ~{:.6} SOL in rent",
            closed_count,
            rent_reclaimed
        )
    );

    Ok((closed_count, signatures))
}

/// Closes the Associated Token Account (ATA) for a given token mint after selling all tokens
/// This reclaims the rent SOL (~0.002 SOL) from empty token accounts
/// Supports both regular SPL tokens and Token-2022 tokens
pub async fn close_token_account(mint: &str, wallet_address: &str) -> Result<String, SwapError> {
    log(LogTag::Wallet, "ATA", &format!("Attempting to close token account for mint: {}", mint));

    // First verify the token balance is actually zero
    match get_token_balance(wallet_address, mint).await {
        Ok(balance) => {
            if balance > 0 {
                return Err(
                    SwapError::InvalidAmount(
                        format!("Cannot close token account - still has {} tokens", balance)
                    )
                );
            }
            log(
                LogTag::Wallet,
                "ATA",
                &format!("Verified zero balance for {}, proceeding to close ATA", mint)
            );
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "WARN",
                &format!("Could not verify token balance before closing ATA: {}", e)
            );
            // Continue anyway - the close instruction will fail if tokens remain
        }
    }

    // Get the associated token account address
    let token_account = match get_associated_token_account(wallet_address, mint).await {
        Ok(account) => account,
        Err(e) => {
            log(
                LogTag::Wallet,
                "WARN",
                &format!("Could not find associated token account for {}: {}", mint, e)
            );
            return Err(e);
        }
    };

    log(LogTag::Wallet, "ATA", &format!("Found token account to close: {}", token_account));

    // Determine if this is a Token-2022 token by checking the token program
    let is_token_2022 = is_token_2022_mint(mint).await.unwrap_or(false);

    if is_token_2022 {
        log(LogTag::Wallet, "ATA", "Detected Token-2022, using Token Extensions program");
    } else {
        log(LogTag::Wallet, "ATA", "Using standard SPL Token program");
    }

    // Create and send the close account instruction using GMGN API approach
    match close_ata(wallet_address, &token_account, mint, is_token_2022).await {
        Ok(signature) => {
            log(
                LogTag::Wallet,
                "SUCCESS",
                &format!("Successfully closed token account for {}. TX: {}", mint, signature)
            );
            Ok(signature)
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Failed to close token account for {}: {}", mint, e)
            );
            Err(e)
        }
    }
}

/// Gets the associated token account address for a wallet and mint
async fn get_associated_token_account(
    wallet_address: &str,
    mint: &str
) -> Result<String, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet_address,
            {
                "mint": mint
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let client = reqwest::Client::new();

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for rpc_url in rpc_endpoints {
        match
            client
                .post(rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(accounts) = value.as_array() {
                                if !accounts.is_empty() {
                                    if let Some(pubkey) = accounts[0].get("pubkey") {
                                        if let Some(pubkey_str) = pubkey.as_str() {
                                            return Ok(pubkey_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                continue;
            }
        }
    }

    Err(SwapError::TransactionError("No associated token account found".to_string()))
}

/// Checks if a mint is a Token-2022 token by examining its program ID
async fn is_token_2022_mint(mint: &str) -> Result<bool, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [
            mint,
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let client = reqwest::Client::new();

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for rpc_url in rpc_endpoints {
        match
            client
                .post(rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(owner) = value.get("owner") {
                                if let Some(owner_str) = owner.as_str() {
                                    // Token Extensions Program ID (Token-2022)
                                    return Ok(
                                        owner_str == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                continue;
            }
        }
    }

    // Default to false if we can't determine
    Ok(false)
}

/// Closes ATA using proper Solana SDK for real ATA closing
async fn close_ata(
    wallet_address: &str,
    token_account: &str,
    mint: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    log(
        LogTag::Wallet,
        "ATA",
        &format!("Closing ATA {} for mint {} using {} program", token_account, mint, if
            is_token_2022
        {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Use proper Solana SDK to build and send close instruction
    match build_and_send_close_instruction(wallet_address, token_account, is_token_2022).await {
        Ok(signature) => {
            log(LogTag::Wallet, "SUCCESS", &format!("ATA closed successfully. TX: {}", signature));
            Ok(signature)
        }
        Err(e) => {
            log(LogTag::Wallet, "ERROR", &format!("Failed to close ATA: {}", e));
            Err(e)
        }
    }
}

/// Builds and sends close account instruction using Solana SDK
async fn build_and_send_close_instruction(
    wallet_address: &str,
    token_account: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Parse addresses
    let owner_pubkey = Pubkey::from_str(wallet_address).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid wallet address: {}", e))
    )?;

    let token_account_pubkey = Pubkey::from_str(token_account).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid token account: {}", e))
    )?;

    // Decode private key
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key: {}", e)))?;

    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    // Build close account instruction
    let close_instruction = if is_token_2022 {
        // For Token-2022, use the Token Extensions program
        build_token_2022_close_instruction(&token_account_pubkey, &owner_pubkey)?
    } else {
        // For regular SPL tokens, use standard close_account instruction
        close_account(
            &spl_token::id(),
            &token_account_pubkey,
            &owner_pubkey,
            &owner_pubkey,
            &[]
        ).map_err(|e|
            SwapError::TransactionError(format!("Failed to build close instruction: {}", e))
        )?
    };

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Built close instruction for {} account", if is_token_2022 {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Get recent blockhash via RPC
    let recent_blockhash = get_latest_blockhash(&configs.rpc_url).await?;

    // Build transaction
    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[&keypair],
        recent_blockhash
    );

    log(LogTag::Wallet, "ATA", "Built and signed close transaction");

    // Send transaction via RPC
    send_close_transaction_via_rpc(&transaction, &configs).await
}

/// Builds close instruction for Token-2022 accounts
fn build_token_2022_close_instruction(
    token_account: &Pubkey,
    owner: &Pubkey
) -> Result<Instruction, SwapError> {
    // Token-2022 uses the same close account instruction format
    // but with different program ID
    let token_2022_program_id = Pubkey::from_str(
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    ).map_err(|e| SwapError::TransactionError(format!("Invalid Token-2022 program ID: {}", e)))?;

    close_account(&token_2022_program_id, token_account, owner, owner, &[]).map_err(|e|
        SwapError::TransactionError(format!("Failed to build Token-2022 close instruction: {}", e))
    )
}

/// Gets latest blockhash from Solana RPC
pub async fn get_latest_blockhash(rpc_url: &str) -> Result<solana_sdk::hash::Hash, SwapError> {
    let client = reqwest::Client::new();

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": [
            {
                "commitment": "finalized"
            }
        ]
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send().await
        .map_err(|e| SwapError::NetworkError(e))?;

    let rpc_response: serde_json::Value = response
        .json().await
        .map_err(|e| SwapError::NetworkError(e))?;

    if let Some(result) = rpc_response.get("result") {
        if let Some(value) = result.get("value") {
            if let Some(blockhash_str) = value.get("blockhash").and_then(|b| b.as_str()) {
                let blockhash = solana_sdk::hash::Hash
                    ::from_str(blockhash_str)
                    .map_err(|e| SwapError::TransactionError(format!("Invalid blockhash: {}", e)))?;
                return Ok(blockhash);
            }
        }
    }

    Err(SwapError::TransactionError("Failed to get latest blockhash".to_string()))
}

/// Sends close transaction via RPC
pub async fn send_close_transaction_via_rpc(
    transaction: &Transaction,
    configs: &crate::global::Configs
) -> Result<String, SwapError> {
    let client = reqwest::Client::new();

    // Serialize transaction
    let serialized_tx = bincode
        ::serialize(transaction)
        .map_err(|e|
            SwapError::TransactionError(format!("Failed to serialize transaction: {}", e))
        )?;

    let tx_base64 = base64::engine::general_purpose::STANDARD.encode(&serialized_tx);

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": [
            tx_base64,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "processed",
                "maxRetries": 3
            }
        ]
    });

    log(LogTag::Wallet, "ATA", "Sending close transaction to Solana network...");

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for (i, rpc_url) in rpc_endpoints.iter().enumerate() {
        log(
            LogTag::Wallet,
            "ATA",
            &format!("Trying RPC endpoint {} ({}/{})", rpc_url, i + 1, rpc_endpoints.len())
        );

        match
            client
                .post(*rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(signature) = result.as_str() {
                            log(
                                LogTag::Wallet,
                                "SUCCESS",
                                &format!(
                                    "Transaction sent successfully via {}: {}",
                                    rpc_url,
                                    signature
                                )
                            );
                            return Ok(signature.to_string());
                        }
                    }
                    if let Some(error) = rpc_response.get("error") {
                        log(
                            LogTag::Wallet,
                            "ERROR",
                            &format!("RPC error from {}: {:?}", rpc_url, error)
                        );
                        continue;
                    }
                }
            }
            Err(e) => {
                log(LogTag::Wallet, "ERROR", &format!("Network error with {}: {}", rpc_url, e));
                continue;
            }
        }
    }

    Err(SwapError::TransactionError("All RPC endpoints failed to send transaction".to_string()))
}
