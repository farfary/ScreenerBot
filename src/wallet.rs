use crate::global::{ Token, read_configs };
use crate::logger::{ log, LogTag };
use reqwest;
use serde::{ Deserialize, Serialize };
use std::error::Error;
use std::fmt;
use base64::{ Engine as _, engine::general_purpose };
use solana_sdk::{ signature::Keypair, transaction::VersionedTransaction, signer::Signer };
use bs58;

/// Configuration constants for swap operations
pub const DEFAULT_SLIPPAGE: f64 = 5.0; // 5% slippage
pub const DEFAULT_FEE: f64 = 0.000001; // 0.006 SOL fee
pub const ANTI_MEV: bool = false; // Enable anti-MEV by default
pub const PARTNER: &str = "screenerbot"; // Partner identifier

/// SOL token mint address (native Solana)
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

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
        }
    }
}

impl Error for SwapError {}

impl From<reqwest::Error> for SwapError {
    fn from(err: reqwest::Error) -> Self {
        SwapError::NetworkError(err)
    }
}

/// Select a random RPC endpoint from the fallbacks (avoiding main RPC for transactions)
fn get_random_transaction_rpc(configs: &crate::global::Configs) -> String {
    use rand::seq::SliceRandom;

    if configs.rpc_fallbacks.is_empty() {
        // If no fallbacks, use main RPC as last resort
        log(LogTag::Trader, "WARN", "No RPC fallbacks configured, using main RPC for transaction");
        return configs.rpc_url.clone();
    }

    // Randomly select from fallbacks only
    let mut rng = rand::thread_rng();
    match configs.rpc_fallbacks.choose(&mut rng) {
        Some(rpc) => {
            log(LogTag::Trader, "RPC", &format!("Selected random transaction RPC: {}", rpc));
            rpc.clone()
        }
        None => {
            log(LogTag::Trader, "WARN", "Failed to select random RPC, using main RPC");
            configs.rpc_url.clone()
        }
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
    #[serde(rename = "slippageBps")]
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
            slippage: DEFAULT_SLIPPAGE,
            fee: DEFAULT_FEE,
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

    log(
        LogTag::Trader,
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

    log(LogTag::Trader, "SEND", &format!("Sending signed transaction to RPC: {}", rpc_url));

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

    // Try main RPC first, then fallbacks
    let mut _last_error: Option<SwapError> = None;

    // Try main RPC
    match send_rpc_request(&client, &configs.rpc_url, &rpc_payload).await {
        Ok(tx_sig) => {
            log(LogTag::Trader, "SUCCESS", &format!("Transaction sent successfully: {}", tx_sig));
            return Ok(tx_sig);
        }
        Err(e) => {
            log(LogTag::Trader, "ERROR", &format!("Main RPC failed: {}, trying fallbacks...", e));
            _last_error = Some(e);
        }
    }

    // Try fallback RPCs
    for fallback_rpc in &configs.rpc_fallbacks {
        match send_rpc_request(&client, fallback_rpc, &rpc_payload).await {
            Ok(tx_sig) => {
                log(
                    LogTag::Trader,
                    "SUCCESS",
                    &format!("Transaction sent via fallback RPC: {}", tx_sig)
                );
                return Ok(tx_sig);
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Fallback RPC {} failed: {}", fallback_rpc, e)
                );
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
    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(payload)
        .send().await
        .map_err(|e| SwapError::NetworkError(e))?;

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

    // Check for RPC errors
    if let Some(error) = rpc_response.get("error") {
        return Err(SwapError::TransactionError(format!("RPC error: {:?}", error)));
    }

    // Extract the transaction signature from the response
    if let Some(result) = rpc_response.get("result") {
        if let Some(signature) = result.as_str() {
            return Ok(signature.to_string());
        }
    }

    Err(SwapError::TransactionError("Invalid RPC response format".to_string()))
}

/// Calculates effective price from actual balance changes
pub async fn calculate_effective_price(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    _rpc_url: &str,
    configs: &crate::global::Configs
) -> Result<(f64, u64, u64, f64), SwapError> {
    log(
        LogTag::Trader,
        "ANALYZE",
        &format!("Calculating effective price for transaction: {}", transaction_signature)
    );

    // Wait a moment for transaction to be confirmed
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Try all available RPC endpoints with retries
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    let mut transaction_details = None;

    for (rpc_idx, rpc_endpoint) in rpc_endpoints.iter().enumerate() {
        for attempt in 1..=3 {
            // Reduced attempts per endpoint
            match get_transaction_details(client, transaction_signature, rpc_endpoint).await {
                Ok(details) => {
                    transaction_details = Some(details);
                    log(
                        LogTag::Trader,
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
                        LogTag::Trader,
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

        // Wait between RPC endpoints to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    let details = transaction_details.ok_or_else(|| {
        SwapError::TransactionError("Failed to get transaction details after retries".to_string())
    })?;

    let meta = details.meta.ok_or_else(|| {
        SwapError::TransactionError("Transaction metadata not available".to_string())
    })?;

    if meta.err.is_some() {
        return Err(SwapError::TransactionError("Transaction failed on-chain".to_string()));
    }

    // Calculate balance changes
    let (actual_input_change, actual_output_change) = calculate_balance_changes(
        &meta,
        input_mint,
        output_mint,
        wallet_address
    )?;

    // Calculate effective price
    let effective_price = if actual_input_change > 0 && actual_output_change > 0 {
        if input_mint == SOL_MINT {
            // SOL -> Token: price = SOL spent / tokens received
            lamports_to_sol(actual_input_change) / (actual_output_change as f64)
        } else if output_mint == SOL_MINT {
            // Token -> SOL: price = SOL received / tokens spent
            lamports_to_sol(actual_output_change) / (actual_input_change as f64)
        } else {
            // Token -> Token: ratio
            (actual_output_change as f64) / (actual_input_change as f64)
        }
    } else {
        0.0
    };

    log(
        LogTag::Trader,
        "EFFECTIVE",
        &format!(
            "Effective price calculated: {:.15} (Input: {} lamports, Output: {} lamports)",
            effective_price,
            actual_input_change,
            actual_output_change
        )
    );

    Ok((effective_price, actual_input_change, actual_output_change, 0.0))
}

/// Calculates balance changes from transaction metadata
fn calculate_balance_changes(
    meta: &TransactionMeta,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> Result<(u64, u64), SwapError> {
    let mut input_change = 0u64;
    let mut output_change = 0u64;

    // Handle SOL balance changes
    if input_mint == SOL_MINT || output_mint == SOL_MINT {
        // Find wallet's account index by checking all accounts
        // This is a simplified approach - in reality you'd need to parse the transaction message
        if let (Some(pre), Some(post)) = (meta.pre_balances.get(0), meta.post_balances.get(0)) {
            let sol_change = if post > pre { post - pre } else { pre - post };

            if input_mint == SOL_MINT {
                input_change = sol_change;
            } else {
                output_change = sol_change;
            }
        }
    }

    // Handle token balance changes
    if
        let (Some(pre_tokens), Some(post_tokens)) = (
            &meta.pre_token_balances,
            &meta.post_token_balances,
        )
    {
        // Find changes for input token
        if input_mint != SOL_MINT {
            if
                let Some(change) = find_token_balance_change(
                    pre_tokens,
                    post_tokens,
                    input_mint,
                    wallet_address
                )
            {
                input_change = change;
            }
        }

        // Find changes for output token
        if output_mint != SOL_MINT {
            if
                let Some(change) = find_token_balance_change(
                    pre_tokens,
                    post_tokens,
                    output_mint,
                    wallet_address
                )
            {
                output_change = change;
            }
        }
    }

    Ok((input_change, output_change))
}

/// Finds token balance change for a specific mint and wallet
fn find_token_balance_change(
    pre_balances: &[TokenBalance],
    post_balances: &[TokenBalance],
    mint: &str,
    wallet_address: &str
) -> Option<u64> {
    // Find pre-balance for this mint and wallet
    let pre_balance = pre_balances
        .iter()
        .find(|tb| tb.mint == mint && tb.owner.as_ref() == Some(&wallet_address.to_string()))
        .and_then(|tb| tb.ui_token_amount.amount.parse::<u64>().ok())
        .unwrap_or(0);

    // Find post-balance for this mint and wallet
    let post_balance = post_balances
        .iter()
        .find(|tb| tb.mint == mint && tb.owner.as_ref() == Some(&wallet_address.to_string()))
        .and_then(|tb| tb.ui_token_amount.amount.parse::<u64>().ok())
        .unwrap_or(0);

    // Return the absolute change
    if post_balance > pre_balance {
        Some(post_balance - pre_balance)
    } else {
        Some(pre_balance - post_balance)
    }
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

    log(
        LogTag::Trader,
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
                            LogTag::Trader,
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
                                LogTag::Trader,
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
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!("Raw API response: {}", &response_text[..response_text.len().min(500)])
                );

                // Try to parse the JSON response with better error handling
                let api_response: SwapApiResponse = match serde_json::from_str(&response_text) {
                    Ok(response) => response,
                    Err(e) => {
                        let error = SwapError::InvalidResponse(
                            format!("JSON parsing error: {} - Response: {}", e, response_text)
                        );
                        if attempt < 3 {
                            log(
                                LogTag::Trader,
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
                            LogTag::Trader,
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
                                LogTag::Trader,
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
                        LogTag::Trader,
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

/// Gets a swap quote using wallet address from configs
pub async fn get_swap_quote_with_config(
    input_mint: &str,
    output_mint: &str,
    amount_sol: f64
) -> Result<SwapData, SwapError> {
    let wallet_address = get_wallet_address()?;

    let request = SwapRequest {
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        amount_sol,
        from_address: wallet_address,
        ..Default::default()
    };

    get_swap_quote(&request).await
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
        LogTag::Trader,
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
            LogTag::Trader,
            "DEBUG",
            &format!("Final - Raw out_amount string: '{}'", output_amount_str)
        );

        let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Final - Failed to parse out_amount '{}': {}", output_amount_str, e)
            );
            0.0
        });

        log(
            LogTag::Trader,
            "DEBUG",
            &format!("Final - Parsed output_amount_raw: {}", output_amount_raw)
        );

        let token_decimals = token.decimals as u32;
        let output_tokens = output_amount_raw / (10_f64).powi(token_decimals as i32);

        let actual_price_per_token = if output_tokens > 0.0 {
            input_sol / output_tokens
        } else {
            0.0
        };

        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "Final price calc debug: raw_amount={}, decimals={}, output_tokens={:.12}, actual_price={:.12}",
                output_amount_raw,
                token_decimals,
                output_tokens,
                actual_price_per_token
            )
        );

        let price_difference = (((actual_price_per_token - expected) / expected) * 100.0).abs();

        log(
            LogTag::Trader,
            "PRICE",
            &format!(
                "Final price validation: Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
                expected,
                actual_price_per_token,
                price_difference
            )
        );

        if price_difference > DEFAULT_SLIPPAGE {
            return Err(
                SwapError::SlippageExceeded(
                    format!(
                        "Price difference {:.2}% exceeds slippage tolerance {:.2}%",
                        price_difference,
                        DEFAULT_SLIPPAGE
                    )
                )
            );
        }
    }

    // Sign and send the transaction using random fallback RPC
    let selected_rpc = get_random_transaction_rpc(&configs);
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &selected_rpc
    ).await?;

    log(
        LogTag::Trader,
        "SIGN",
        &format!("Transaction submitted successfully! TX: {}", transaction_signature)
    );

    // Calculate effective price from actual balance changes and verify transaction success
    let (effective_price, actual_input_change, actual_output_change, quote_vs_actual_diff) = match
        calculate_effective_price(
            &reqwest::Client::new(),
            &transaction_signature,
            input_mint,
            output_mint,
            &wallet_address,
            &configs.rpc_url,
            &configs
        ).await
    {
        Ok((effective_price, input_change, output_change, diff)) => {
            log(
                LogTag::Trader,
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
                LogTag::Trader,
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

    let request = SwapRequest {
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        amount_sol,
        from_address: wallet_address.clone(),
        expected_price,
        ..Default::default()
    };

    log(
        LogTag::Trader,
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
    let swap_data = get_swap_quote(&request).await?;

    // Validate expected price if provided
    if let Some(expected) = expected_price {
        // Calculate the actual price per token from the quote
        let input_sol = request.amount_sol;
        let output_amount_raw = swap_data.quote.out_amount.parse().unwrap_or(0) as f64;
        let token_decimals = token.decimals as u32;
        let output_tokens = output_amount_raw / (10_f64).powi(token_decimals as i32);

        let actual_price_per_token = if output_tokens > 0.0 {
            input_sol / output_tokens
        } else {
            0.0
        };

        let price_difference = (((actual_price_per_token - expected) / expected) * 100.0).abs();

        log(
            LogTag::Trader,
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

    // Sign and send the transaction using random fallback RPC (buy_token)
    let selected_rpc = get_random_transaction_rpc(&configs);
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &selected_rpc
    ).await?;

    log(
        LogTag::Trader,
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
            &configs.rpc_url,
            &configs
        ).await.unwrap_or_else(|e| {
            log(LogTag::Trader, "WARNING", &format!("Failed to calculate effective price: {}", e));
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
    })
}

/// Helper function to buy a token with SOL
pub async fn buy_token(
    token: &Token,
    amount_sol: f64,
    expected_price: Option<f64>
) -> Result<SwapResult, SwapError> {
    let wallet_address = get_wallet_address()?;

    // Check SOL balance before swap
    log(LogTag::Trader, "BALANCE", "Checking SOL balance...");
    let sol_balance = get_sol_balance(&wallet_address).await?;
    log(LogTag::Trader, "BALANCE", &format!("Current SOL balance: {:.6} SOL", sol_balance));

    if sol_balance < amount_sol {
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
        LogTag::Trader,
        "SWAP",
        &format!(
            "Getting quote for {} ({}) - {} SOL -> {}",
            token.symbol,
            token.name,
            amount_sol,
            &token.mint[..8]
        )
    );

    // Get quote once
    let swap_data = get_swap_quote(&request).await?;

    // Check current price if expected price is provided
    if let Some(expected) = expected_price {
        log(LogTag::Trader, "PRICE", "Validating current token price...");

        // Calculate current price from quote, accounting for token decimals
        let output_amount_str = &swap_data.quote.out_amount;
        log(LogTag::Trader, "DEBUG", &format!("Raw out_amount string: '{}'", output_amount_str));

        let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Failed to parse out_amount '{}': {}", output_amount_str, e)
            );
            0.0
        });

        log(LogTag::Trader, "DEBUG", &format!("Parsed output_amount_raw: {}", output_amount_raw));

        let token_decimals = token.decimals as u32;
        let output_tokens = output_amount_raw / (10_f64).powi(token_decimals as i32);
        let current_price = if output_tokens > 0.0 { amount_sol / output_tokens } else { 0.0 };

        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "Price calc debug: raw_amount={}, decimals={}, output_tokens={:.12}, current_price={:.12}",
                output_amount_raw,
                token_decimals,
                output_tokens,
                current_price
            )
        );

        log(
            LogTag::Trader,
            "PRICE",
            &format!("Current price: {:.12} SOL, Expected: {:.12} SOL", current_price, expected)
        );

        // Use 5% tolerance for price validation
        if current_price > 0.0 && !validate_price_near_expected(current_price, expected, 5.0) {
            let price_diff = ((current_price - expected) / expected) * 100.0;
            return Err(
                SwapError::SlippageExceeded(
                    format!(
                        "Current price {:.12} SOL differs from expected {:.12} SOL by {:.2}% (tolerance: 5%)",
                        current_price,
                        expected,
                        price_diff
                    )
                )
            );
        } else if current_price <= 0.0 {
            log(
                LogTag::Trader,
                "WARNING",
                "Could not calculate current price from quote, proceeding without validation"
            );
        } else {
            log(LogTag::Trader, "PRICE", "✅ Price validation passed");
        }
    }

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
    log(LogTag::Trader, "BALANCE", &format!("Checking {} balance...", token.symbol));
    let token_balance = get_token_balance(&wallet_address, &token.mint).await?;
    log(
        LogTag::Trader,
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
        log(LogTag::Trader, "PRICE", "Validating expected SOL output...");
        match get_token_price_sol(&token.mint).await {
            Ok(current_price) => {
                let estimated_sol_output = current_price * (token_amount as f64);
                log(
                    LogTag::Trader,
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
                log(LogTag::Trader, "PRICE", "✅ Price validation passed");
            }
            Err(e) => {
                log(LogTag::Trader, "WARNING", &format!("Could not validate price: {}", e));
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
        LogTag::Trader,
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
        LogTag::Trader,
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
        LogTag::Trader,
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

    // Sign and send the transaction using random fallback RPC (sell_token)
    let selected_rpc = get_random_transaction_rpc(&configs);
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &selected_rpc
    ).await?;

    log(
        LogTag::Trader,
        "SUCCESS",
        &format!("Sell executed successfully! TX: {}", transaction_signature)
    );

    // Calculate effective price from actual balance changes
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
            log(LogTag::Trader, "WARNING", &format!("Failed to calculate effective price: {}", e));
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
    })
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
                                return Ok(lamports_to_sol(balance_lamports));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Trader,
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
                    LogTag::Trader,
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
