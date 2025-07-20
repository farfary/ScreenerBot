use serde::{ Deserialize, Serialize };
use reqwest;
use std::error::Error;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use chrono::{ Utc, TimeZone };
use crate::logger::{ log, LogTag };
use crate::global::Configs;

/// Maximum number of transactions to fetch in one request
pub const MAX_TRANSACTIONS_PER_REQUEST: usize = 100;

/// Path to transaction cache file
const TRANSACTION_CACHE_FILE: &str = "transactions.json";

/// Known DEX Program IDs for identifying swap transactions
pub mod dex_program_ids {
    /// Serum DEX V2
    pub const SERUM_DEX_V2: &str = "EUqojwWA2rd19FZrzeBncJsm38Jm1hEhE3zsmX3bRc2o";

    /// Serum DEX V3
    pub const SERUM_DEX_V3: &str = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";

    /// Raydium Liquidity Pool V2
    pub const RAYDIUM_V2: &str = "RVKd61ztZW9GUwhRbbLoYVRE5Xf1B2tVscKqwZqXgEr";

    /// Raydium Liquidity Pool V3
    pub const RAYDIUM_V3: &str = "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv";

    /// Raydium Liquidity Pool V4
    pub const RAYDIUM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

    /// Raydium AMM Routing
    pub const RAYDIUM_ROUTING: &str = "routeUGWgWzqBWFcrCfv8tritsqukccJPu3q5GPP3xS";

    /// Raydium Concentrated Liquidity
    pub const RAYDIUM_CONCENTRATED: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

    /// OpenOcean
    pub const OPENOCEAN: &str = "DF6c7dTBdZ9cb59pywKAVwy5NMSXiSfmXzYNwYFPNz9F";

    /// Jupiter
    pub const JUPITER: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

    /// Phoenix
    pub const PHOENIX: &str = "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY";
}

/// Get DEX name from program ID
pub fn get_dex_name(program_id: &str) -> Option<&'static str> {
    match program_id {
        dex_program_ids::SERUM_DEX_V2 => Some("Serum DEX V2"),
        dex_program_ids::SERUM_DEX_V3 => Some("Serum DEX V3"),
        dex_program_ids::RAYDIUM_V2 => Some("Raydium V2"),
        dex_program_ids::RAYDIUM_V3 => Some("Raydium V3"),
        dex_program_ids::RAYDIUM_V4 => Some("Raydium V4"),
        dex_program_ids::RAYDIUM_ROUTING => Some("Raydium Routing"),
        dex_program_ids::RAYDIUM_CONCENTRATED => Some("Raydium CLMM"),
        dex_program_ids::OPENOCEAN => Some("OpenOcean"),
        dex_program_ids::JUPITER => Some("Jupiter"),
        dex_program_ids::PHOENIX => Some("Phoenix"),
        _ => None,
    }
}

/// Check if a program ID is a known DEX
pub fn is_known_dex(program_id: &str) -> bool {
    get_dex_name(program_id).is_some()
}

/// Get all known DEX program IDs as a vector
pub fn get_all_dex_program_ids() -> Vec<&'static str> {
    vec![
        dex_program_ids::SERUM_DEX_V2,
        dex_program_ids::SERUM_DEX_V3,
        dex_program_ids::RAYDIUM_V2,
        dex_program_ids::RAYDIUM_V3,
        dex_program_ids::RAYDIUM_V4,
        dex_program_ids::RAYDIUM_ROUTING,
        dex_program_ids::RAYDIUM_CONCENTRATED,
        dex_program_ids::OPENOCEAN,
        dex_program_ids::JUPITER,
        dex_program_ids::PHOENIX
    ]
}

/// Transaction cache structure
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TransactionCache {
    pub transactions: HashMap<String, TransactionResult>,
    pub last_updated: u64,
}

impl TransactionCache {
    /// Load cache from file
    pub fn load() -> Self {
        if !Path::new(TRANSACTION_CACHE_FILE).exists() {
            log(LogTag::System, "INFO", "Transaction cache file not found, creating new cache");
            return Self::default();
        }

        match fs::read_to_string(TRANSACTION_CACHE_FILE) {
            Ok(content) => {
                match serde_json::from_str::<TransactionCache>(&content) {
                    Ok(cache) => {
                        log(
                            LogTag::System,
                            "SUCCESS",
                            &format!(
                                "Loaded {} cached transactions from {}",
                                cache.transactions.len(),
                                TRANSACTION_CACHE_FILE
                            )
                        );
                        cache
                    }
                    Err(e) => {
                        log(
                            LogTag::System,
                            "WARNING",
                            &format!("Failed to parse transaction cache: {}, creating new cache", e)
                        );
                        Self::default()
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Failed to read transaction cache: {}, creating new cache", e)
                );
                Self::default()
            }
        }
    }

    /// Save cache to file
    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let json_content = serde_json::to_string_pretty(self)?;
        fs::write(TRANSACTION_CACHE_FILE, json_content)?;
        log(
            LogTag::System,
            "SUCCESS",
            &format!(
                "Saved {} transactions to cache file {}",
                self.transactions.len(),
                TRANSACTION_CACHE_FILE
            )
        );
        Ok(())
    }

    /// Get transaction from cache
    pub fn get_transaction(&self, signature: &str) -> Option<&TransactionResult> {
        self.transactions.get(signature)
    }

    /// Add transaction to cache
    pub fn add_transaction(&mut self, signature: String, transaction: TransactionResult) {
        self.transactions.insert(signature, transaction);
        self.last_updated = Utc::now().timestamp() as u64;
    }

    /// Check if transaction exists in cache
    pub fn contains(&self, signature: &str) -> bool {
        self.transactions.contains_key(signature)
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, u64) {
        (self.transactions.len(), self.last_updated)
    }
}

/// Transaction signature response from Solana RPC
#[derive(Debug, Deserialize)]
pub struct SignatureResponse {
    pub result: Option<Vec<SignatureInfo>>,
    pub error: Option<serde_json::Value>,
}

/// Individual signature information
#[derive(Debug, Deserialize, Clone)]
pub struct SignatureInfo {
    pub signature: String,
    pub slot: u64,
    #[serde(rename = "blockTime")]
    pub block_time: Option<u64>,
    pub err: Option<serde_json::Value>,
    pub memo: Option<String>,
    #[serde(rename = "confirmationStatus")]
    pub confirmation_status: Option<String>,
}

/// Transaction details response from Solana RPC
#[derive(Debug, Deserialize)]
pub struct TransactionResponse {
    pub result: Option<TransactionResult>,
    pub error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TransactionResult {
    pub transaction: Transaction,
    pub meta: Option<TransactionMeta>,
    #[serde(rename = "blockTime")]
    pub block_time: Option<u64>,
    pub slot: u64,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Transaction {
    pub message: TransactionMessage,
    pub signatures: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TransactionMessage {
    #[serde(rename = "accountKeys")]
    pub account_keys: Vec<String>,
    pub instructions: Vec<TransactionInstruction>,
    #[serde(default)]
    pub header: Option<TransactionHeader>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TransactionHeader {
    #[serde(rename = "numRequiredSignatures")]
    pub num_required_signatures: u8,
    #[serde(rename = "numReadonlySignedAccounts")]
    pub num_readonly_signed_accounts: u8,
    #[serde(rename = "numReadonlyUnsignedAccounts")]
    pub num_readonly_unsigned_accounts: u8,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TransactionInstruction {
    #[serde(rename = "programId")]
    pub program_id: Option<String>,
    #[serde(rename = "programIdIndex")]
    pub program_id_index: Option<u8>,
    pub accounts: Vec<u8>,
    pub data: String,
    #[serde(rename = "stackHeight")]
    pub stack_height: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TransactionMeta {
    pub err: Option<serde_json::Value>,
    pub fee: u64,
    #[serde(rename = "preBalances")]
    pub pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    pub post_balances: Vec<u64>,
    #[serde(rename = "preTokenBalances")]
    pub pre_token_balances: Option<Vec<TokenBalance>>,
    #[serde(rename = "postTokenBalances")]
    pub post_token_balances: Option<Vec<TokenBalance>>,
    #[serde(rename = "innerInstructions")]
    pub inner_instructions: Option<Vec<InnerInstruction>>,
    #[serde(rename = "logMessages")]
    pub log_messages: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct InnerInstruction {
    pub index: u8,
    pub instructions: Vec<TransactionInstruction>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TokenBalance {
    #[serde(rename = "accountIndex")]
    pub account_index: u8,
    pub mint: String,
    #[serde(rename = "uiTokenAmount")]
    pub ui_token_amount: TokenAmount,
    pub owner: Option<String>,
    #[serde(rename = "programId")]
    pub program_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TokenAmount {
    pub amount: String,
    pub decimals: u8,
    #[serde(rename = "uiAmount")]
    pub ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    pub ui_amount_string: Option<String>,
}

/// Information about a detected swap transaction
#[derive(Debug, Clone, Serialize)]
pub struct SwapTransaction {
    pub signature: String,
    pub block_time: Option<u64>,
    pub slot: u64,
    pub is_success: bool,
    pub fee_sol: f64,
    pub swap_type: SwapType,
    pub input_token: SwapTokenInfo,
    pub output_token: SwapTokenInfo,
    pub program_id: String,
    pub dex_name: Option<String>, // Human-readable DEX name
    pub log_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum SwapType {
    Buy,
    Sell,
    SwapAtoB,
    SwapBtoA,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapTokenInfo {
    pub mint: String,
    pub symbol: Option<String>,
    pub amount_raw: String,
    pub amount_ui: f64,
    pub decimals: u8,
}

/// Information about token balance changes
#[derive(Debug, Clone, Serialize)]
pub struct TokenBalanceChange {
    pub mint: String,
    pub pre_amount: f64,
    pub post_amount: f64,
    pub change: f64,
    pub decimals: u8,
    pub change_type: TokenChangeType,
}

#[derive(Debug, Clone, Serialize)]
pub enum TokenChangeType {
    Increase,
    Decrease,
    NoChange,
}

/// Comprehensive transaction analysis result
#[derive(Debug, Clone, Serialize)]
pub struct TransactionAnalysis {
    pub signature: String,
    pub block_time: Option<u64>,
    pub slot: u64,
    pub is_success: bool,
    pub fee_sol: f64,
    pub contains_swaps: bool,
    pub swaps: Vec<SwapTransaction>,
    pub token_changes: Vec<TokenBalanceChange>,
    pub involves_target_token: bool,
    pub program_interactions: Vec<String>,
}

/// Get recent transaction signatures for a wallet address with RPC fallback support
pub async fn get_recent_signatures(
    client: &reqwest::Client,
    wallet_address: &str,
    rpc_url: &str,
    limit: usize
) -> Result<Vec<SignatureInfo>, Box<dyn Error>> {
    let actual_limit = limit.min(MAX_TRANSACTIONS_PER_REQUEST);

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getSignaturesForAddress",
        "params": [
            wallet_address,
            {
                "limit": actual_limit,
                "commitment": "confirmed"
            }
        ]
    });

    log(
        LogTag::System,
        "INFO",
        &format!("Fetching recent {} signatures for wallet: {}", actual_limit, wallet_address)
    );

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send().await?;

    if !response.status().is_success() {
        return Err(format!("Failed to get signatures: {}", response.status()).into());
    }

    let response_text = response.text().await?;

    let rpc_response: SignatureResponse = serde_json
        ::from_str(&response_text)
        .map_err(|e|
            format!("Failed to parse signature response: {} - Response text: {}", e, response_text)
        )?;

    if let Some(error) = rpc_response.error {
        return Err(format!("RPC error: {:?}", error).into());
    }

    match rpc_response.result {
        Some(signatures) => Ok(signatures),
        None => Err("No result in response".into()),
    }
}

/// Get recent transaction signatures with RPC fallback support for rate limiting
pub async fn get_recent_signatures_with_fallback(
    client: &reqwest::Client,
    wallet_address: &str,
    configs: &Configs,
    limit: usize
) -> Result<Vec<SignatureInfo>, Box<dyn Error>> {
    // Try main RPC first
    match get_recent_signatures(client, wallet_address, &configs.rpc_url, limit).await {
        Ok(signatures) => {
            return Ok(signatures);
        }
        Err(e) => {
            if e.to_string().contains("429") {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Main RPC rate limited, trying fallbacks...")
                );
            } else {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Main RPC failed: {}, trying fallbacks...", e)
                );
            }
        }
    }

    // Try fallback RPCs
    for (i, fallback_rpc) in configs.rpc_fallbacks.iter().enumerate() {
        match get_recent_signatures(client, wallet_address, fallback_rpc, limit).await {
            Ok(signatures) => {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Got signatures from fallback RPC {} ({})", i + 1, fallback_rpc)
                );
                return Ok(signatures);
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Fallback RPC {} failed: {}", fallback_rpc, e)
                );
            }
        }
    }

    Err("All RPC endpoints failed for signature fetching".into())
}

/// Get detailed transaction information with rate limiting protection
pub async fn get_transaction_details(
    client: &reqwest::Client,
    signature: &str,
    rpc_url: &str
) -> Result<Option<TransactionResult>, Box<dyn Error>> {
    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            signature,
            {
                "encoding": "json",
                "maxSupportedTransactionVersion": 0,
                "commitment": "confirmed"
            }
        ]
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send().await?;

    if !response.status().is_success() {
        let status = response.status();
        if status.as_u16() == 429 {
            // Rate limited - let caller handle the retry logic
            return Err(format!("Rate limited (429)").into());
        }
        return Err(format!("Failed to get transaction details: {}", status).into());
    }

    let response_text = response.text().await?;

    let rpc_response: TransactionResponse = serde_json
        ::from_str(&response_text)
        .map_err(|e|
            format!(
                "Failed to parse transaction response: {} - Response text: {}",
                e,
                response_text
            )
        )?;

    if let Some(error) = rpc_response.error {
        return Err(format!("RPC error getting transaction: {:?}", error).into());
    }

    Ok(rpc_response.result)
}

/// Get detailed transaction information with RPC fallback support
pub async fn get_transaction_details_with_fallback(
    client: &reqwest::Client,
    signature: &str,
    configs: &Configs
) -> Result<Option<TransactionResult>, Box<dyn Error>> {
    // Try main RPC first
    match get_transaction_details(client, signature, &configs.rpc_url).await {
        Ok(transaction) => {
            return Ok(transaction);
        }
        Err(e) => {
            if e.to_string().contains("429") {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Main RPC rate limited for transaction {}, trying fallbacks...", signature)
                );
            } else {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!(
                        "Main RPC failed for transaction {}: {}, trying fallbacks...",
                        signature,
                        e
                    )
                );
            }
        }
    }

    // Try fallback RPCs
    for (i, fallback_rpc) in configs.rpc_fallbacks.iter().enumerate() {
        match get_transaction_details(client, signature, fallback_rpc).await {
            Ok(transaction) => {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!(
                        "Got transaction details from fallback RPC {} ({})",
                        i + 1,
                        fallback_rpc
                    )
                );
                return Ok(transaction);
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!(
                        "Fallback RPC {} failed for transaction {}: {}",
                        fallback_rpc,
                        signature,
                        e
                    )
                );
            }
        }
    }

    Err(format!("All RPC endpoints failed for transaction {}", signature).into())
}

/// Get detailed transaction information with caching and RPC fallback support
pub async fn get_transaction_details_cached(
    client: &reqwest::Client,
    signature: &str,
    configs: &Configs,
    cache: &mut TransactionCache
) -> Result<Option<TransactionResult>, Box<dyn Error>> {
    // Check cache first
    if let Some(cached_transaction) = cache.get_transaction(signature) {
        log(LogTag::System, "SUCCESS", &format!("Retrieved transaction {} from cache", signature));
        return Ok(Some(cached_transaction.clone()));
    }

    // Not in cache, fetch from RPC
    log(
        LogTag::System,
        "INFO",
        &format!("Transaction {} not in cache, fetching from RPC...", signature)
    );

    match get_transaction_details_with_fallback(client, signature, configs).await {
        Ok(Some(transaction)) => {
            // Add to cache
            cache.add_transaction(signature.to_string(), transaction.clone());

            // Save cache to disk (we can optimize this to batch saves later)
            if let Err(e) = cache.save() {
                log(LogTag::System, "WARNING", &format!("Failed to save transaction cache: {}", e));
            }

            log(
                LogTag::System,
                "SUCCESS",
                &format!("Fetched and cached transaction {}", signature)
            );

            Ok(Some(transaction))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Check if transaction involves the specific token
pub fn transaction_involves_token(transaction: &TransactionResult, token_mint: &str) -> bool {
    // Check if token mint appears in account keys
    if transaction.transaction.message.account_keys.contains(&token_mint.to_string()) {
        return true;
    }

    // Check token balances
    if let Some(ref meta) = transaction.meta {
        // Check pre-token balances
        if let Some(ref pre_balances) = meta.pre_token_balances {
            for balance in pre_balances {
                if balance.mint == token_mint {
                    return true;
                }
            }
        }

        // Check post-token balances
        if let Some(ref post_balances) = meta.post_token_balances {
            for balance in post_balances {
                if balance.mint == token_mint {
                    return true;
                }
            }
        }
    }

    false
}

/// Analyze token balance changes in a transaction
pub fn analyze_token_changes(
    transaction: &TransactionResult,
    wallet_address: &str
) -> Vec<TokenBalanceChange> {
    let mut changes = Vec::new();

    if let Some(ref meta) = transaction.meta {
        let mut token_pre_balances = std::collections::HashMap::new();
        let mut token_post_balances = std::collections::HashMap::new();

        // Collect pre-balances
        if let Some(ref pre_balances) = meta.pre_token_balances {
            for balance in pre_balances {
                if let Some(owner) = &balance.owner {
                    if owner == wallet_address {
                        token_pre_balances.insert(balance.mint.clone(), (
                            balance.ui_token_amount.ui_amount.unwrap_or(0.0),
                            balance.ui_token_amount.decimals,
                        ));
                    }
                }
            }
        }

        // Collect post-balances
        if let Some(ref post_balances) = meta.post_token_balances {
            for balance in post_balances {
                if let Some(owner) = &balance.owner {
                    if owner == wallet_address {
                        token_post_balances.insert(balance.mint.clone(), (
                            balance.ui_token_amount.ui_amount.unwrap_or(0.0),
                            balance.ui_token_amount.decimals,
                        ));
                    }
                }
            }
        }

        // Calculate changes for all tokens
        let mut all_mints = std::collections::HashSet::new();
        all_mints.extend(token_pre_balances.keys());
        all_mints.extend(token_post_balances.keys());

        for mint in all_mints {
            let default_pre = (0.0, 9);
            let (pre_amount, decimals) = token_pre_balances.get(mint).unwrap_or(&default_pre);
            let default_post = (0.0, *decimals);
            let (post_amount, _) = token_post_balances.get(mint).unwrap_or(&default_post);

            let change = post_amount - pre_amount;
            let change_type = if change > 0.0 {
                TokenChangeType::Increase
            } else if change < 0.0 {
                TokenChangeType::Decrease
            } else {
                TokenChangeType::NoChange
            };

            if change != 0.0 {
                changes.push(TokenBalanceChange {
                    mint: mint.clone(),
                    pre_amount: *pre_amount,
                    post_amount: *post_amount,
                    change,
                    decimals: *decimals,
                    change_type,
                });
            }
        }
    }

    changes
}

/// Detect and parse swap transactions from transaction data
pub fn detect_swaps_in_transaction(
    transaction: &TransactionResult,
    wallet_address: &str
) -> Vec<SwapTransaction> {
    let mut swaps = Vec::new();

    if let Some(ref meta) = transaction.meta {
        // Get token balance changes
        let token_changes = analyze_token_changes(transaction, wallet_address);

        // Look for swap patterns (one token decrease, another increase)
        let decreases: Vec<_> = token_changes
            .iter()
            .filter(|tc| matches!(tc.change_type, TokenChangeType::Decrease))
            .collect();
        let increases: Vec<_> = token_changes
            .iter()
            .filter(|tc| matches!(tc.change_type, TokenChangeType::Increase))
            .collect();

        // Detect program interactions from log messages
        let program_interactions = detect_program_interactions(&meta.log_messages);

        // If we have both decreases and increases, it's likely a swap
        if !decreases.is_empty() && !increases.is_empty() {
            // Try to pair decreases with increases
            for decrease in &decreases {
                for increase in &increases {
                    // Determine swap type
                    let swap_type = determine_swap_type(decrease, increase, &program_interactions);

                    // Find the program that executed the swap
                    let program_id = find_swap_program(
                        &transaction.transaction.message,
                        &program_interactions
                    );

                    // Get DEX name from program ID
                    let dex_name = get_dex_name(&program_id).map(|name| name.to_string());

                    let swap = SwapTransaction {
                        signature: transaction.transaction.signatures
                            .first()
                            .unwrap_or(&"unknown".to_string())
                            .clone(),
                        block_time: transaction.block_time,
                        slot: transaction.slot,
                        is_success: meta.err.is_none(),
                        fee_sol: (meta.fee as f64) / 1_000_000_000.0,
                        swap_type,
                        input_token: SwapTokenInfo {
                            mint: decrease.mint.clone(),
                            symbol: None, // Could be enhanced with token registry lookup
                            amount_raw: format!(
                                "{}",
                                (decrease.change.abs() *
                                    (10_f64).powi(decrease.decimals as i32)) as u64
                            ),
                            amount_ui: decrease.change.abs(),
                            decimals: decrease.decimals,
                        },
                        output_token: SwapTokenInfo {
                            mint: increase.mint.clone(),
                            symbol: None,
                            amount_raw: format!(
                                "{}",
                                (increase.change * (10_f64).powi(increase.decimals as i32)) as u64
                            ),
                            amount_ui: increase.change,
                            decimals: increase.decimals,
                        },
                        program_id,
                        dex_name, // Add the DEX name
                        log_messages: meta.log_messages.clone().unwrap_or_default(),
                    };

                    swaps.push(swap);
                }
            }
        }
    }

    swaps
}

/// Detect program interactions from log messages
fn detect_program_interactions(log_messages: &Option<Vec<String>>) -> Vec<String> {
    let mut programs = Vec::new();

    if let Some(logs) = log_messages {
        for log in logs {
            if log.contains("Program ") && log.contains(" invoke") {
                // Extract program ID from log like "Program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc invoke [1]"
                if let Some(start) = log.find("Program ") {
                    if let Some(end) = log[start + 8..].find(" invoke") {
                        let program_id = &log[start + 8..start + 8 + end];
                        if !programs.contains(&program_id.to_string()) {
                            programs.push(program_id.to_string());
                        }
                    }
                }
            }
        }
    }

    programs
}

/// Determine swap type based on token changes and program interactions
fn determine_swap_type(
    decrease: &TokenBalanceChange,
    increase: &TokenBalanceChange,
    _program_interactions: &[String]
) -> SwapType {
    // SOL mint address
    const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

    if decrease.mint == SOL_MINT {
        SwapType::Buy // Spent SOL to get tokens
    } else if increase.mint == SOL_MINT {
        SwapType::Sell // Sold tokens to get SOL
    } else {
        SwapType::SwapAtoB // Token-to-token swap
    }
}

/// Find the main swap program from transaction instructions
fn find_swap_program(_message: &TransactionMessage, program_interactions: &[String]) -> String {
    // Known DEX program IDs
    const KNOWN_DEXES: &[(&str, &str)] = &[
        ("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc", "Whirlpool"),
        ("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM", "Orca"),
        ("DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1", "Orca V2"),
        ("srmqPioktnPG6V2eRJoGnR7X6Qe6hR6R7KrMxRn5dFx", "Serum"),
        ("CAVu6WCU6h4eBZo2jNTHJrmKTUYjfyBvVTBSX3fPpGm", "CavernApp"),
    ];

    // Check if any known DEX program is in the interactions
    for (program_id, _name) in KNOWN_DEXES {
        if program_interactions.contains(&program_id.to_string()) {
            return program_id.to_string();
        }
    }

    // Fallback to first program interaction
    program_interactions.first().unwrap_or(&"Unknown".to_string()).clone()
}

/// Comprehensive analysis of a transaction
pub fn analyze_transaction(
    transaction: &TransactionResult,
    wallet_address: &str,
    target_token: Option<&str>
) -> TransactionAnalysis {
    let token_changes = analyze_token_changes(transaction, wallet_address);
    let swaps = detect_swaps_in_transaction(transaction, wallet_address);

    let involves_target_token = if let Some(token_mint) = target_token {
        transaction_involves_token(transaction, token_mint)
    } else {
        false
    };

    let program_interactions = if let Some(ref meta) = transaction.meta {
        detect_program_interactions(&meta.log_messages)
    } else {
        Vec::new()
    };

    let fee_sol = transaction.meta
        .as_ref()
        .map(|m| (m.fee as f64) / 1_000_000_000.0)
        .unwrap_or(0.0);

    TransactionAnalysis {
        signature: transaction.transaction.signatures
            .first()
            .unwrap_or(&"unknown".to_string())
            .clone(),
        block_time: transaction.block_time,
        slot: transaction.slot,
        is_success: transaction.meta.as_ref().map_or(false, |m| m.err.is_none()),
        fee_sol,
        contains_swaps: !swaps.is_empty(),
        swaps,
        token_changes,
        involves_target_token,
        program_interactions,
    }
}

/// Format timestamp for display
pub fn format_timestamp(timestamp: Option<u64>) -> String {
    match timestamp {
        Some(ts) => {
            let dt = Utc.timestamp_opt(ts as i64, 0).single();
            match dt {
                Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                None => "Invalid timestamp".to_string(),
            }
        }
        None => "Unknown time".to_string(),
    }
}

/// Get transactions with automatic rate limiting, retries, and RPC fallbacks
pub async fn get_transactions_with_retry(
    client: &reqwest::Client,
    signatures: &[SignatureInfo],
    rpc_url: &str,
    max_transactions: Option<usize>
) -> Vec<(SignatureInfo, TransactionResult)> {
    let mut results = Vec::new();
    let limit = max_transactions.unwrap_or(signatures.len()).min(signatures.len());

    for (i, sig_info) in signatures.iter().take(limit).enumerate() {
        // Progress reporting
        if (i + 1) % 5 == 0 || i + 1 == limit {
            log(LogTag::System, "INFO", &format!("Processing transaction {}/{}", i + 1, limit));
        }

        let mut retry_count = 0;
        let max_retries = 3;
        let mut delay_ms = 250;

        loop {
            match get_transaction_details(client, &sig_info.signature, rpc_url).await {
                Ok(Some(transaction)) => {
                    results.push((sig_info.clone(), transaction));
                    break;
                }
                Ok(None) => {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!("Transaction not found: {}", sig_info.signature)
                    );
                    break;
                }
                Err(e) => {
                    if e.to_string().contains("429") && retry_count < max_retries {
                        retry_count += 1;
                        log(
                            LogTag::System,
                            "WARNING",
                            &format!(
                                "Rate limited on transaction {} (attempt {}/{}), waiting {}ms...",
                                sig_info.signature,
                                retry_count,
                                max_retries + 1,
                                delay_ms
                            )
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        delay_ms *= 2; // Exponential backoff
                    } else {
                        log(
                            LogTag::System,
                            "WARNING",
                            &format!("Failed to get transaction {}: {}", sig_info.signature, e)
                        );
                        break;
                    }
                }
            }
        }

        // Base delay between requests
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    results
}

/// Get transactions with automatic rate limiting, retries, and RPC fallbacks (optimized for batch processing)
pub async fn get_transactions_with_retry_and_fallback(
    client: &reqwest::Client,
    signatures: &[SignatureInfo],
    configs: &Configs,
    max_transactions: Option<usize>
) -> Vec<(SignatureInfo, TransactionResult)> {
    let mut results = Vec::new();
    let limit = max_transactions.unwrap_or(signatures.len()).min(signatures.len());

    // Process in smaller batches to reduce RPC pressure
    const BATCH_SIZE: usize = 5;

    for batch_start in (0..limit).step_by(BATCH_SIZE) {
        let batch_end = (batch_start + BATCH_SIZE).min(limit);
        let batch = &signatures[batch_start..batch_end];

        log(
            LogTag::System,
            "INFO",
            &format!("Processing batch {}-{} of {}", batch_start + 1, batch_end, limit)
        );

        for (i, sig_info) in batch.iter().enumerate() {
            let overall_index = batch_start + i + 1;

            // Progress reporting for significant milestones
            if overall_index % 10 == 0 || overall_index == limit {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("Processing transaction {}/{}", overall_index, limit)
                );
            }

            let mut retry_count = 0;
            let max_retries = 1; // Reduced since we have fallback RPCs
            let mut delay_ms = 50; // Reduced initial delay

            loop {
                match
                    get_transaction_details_with_fallback(
                        client,
                        &sig_info.signature,
                        configs
                    ).await
                {
                    Ok(Some(transaction)) => {
                        results.push((sig_info.clone(), transaction));
                        break;
                    }
                    Ok(None) => {
                        log(
                            LogTag::System,
                            "WARNING",
                            &format!("Transaction not found: {}", sig_info.signature)
                        );
                        break;
                    }
                    Err(e) => {
                        if e.to_string().contains("429") && retry_count < max_retries {
                            retry_count += 1;
                            log(
                                LogTag::System,
                                "WARNING",
                                &format!(
                                    "All RPCs rate limited for transaction {} (attempt {}/{}), waiting {}ms...",
                                    sig_info.signature,
                                    retry_count,
                                    max_retries + 1,
                                    delay_ms
                                )
                            );
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            delay_ms *= 2; // Exponential backoff
                        } else {
                            // Skip failed transactions silently to reduce spam
                            break;
                        }
                    }
                }
            }

            // Very short delay between requests since we have fallbacks
            tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
        }

        // Longer delay between batches to be respectful to RPC
        if batch_end < limit {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    }

    results
}

/// Get transactions with caching, automatic rate limiting, retries, and RPC fallbacks (optimized for batch processing)
pub async fn get_transactions_with_cache_and_fallback(
    client: &reqwest::Client,
    signatures: &[SignatureInfo],
    configs: &Configs,
    max_transactions: Option<usize>
) -> Vec<(SignatureInfo, TransactionResult)> {
    let mut results = Vec::new();
    let limit = max_transactions.unwrap_or(signatures.len()).min(signatures.len());

    // Load cache
    let mut cache = TransactionCache::load();
    let (initial_cache_size, _) = cache.stats();

    // Separate cached and uncached signatures
    let mut cached_results = Vec::new();
    let mut uncached_signatures = Vec::new();

    for sig_info in signatures.iter().take(limit) {
        if let Some(cached_transaction) = cache.get_transaction(&sig_info.signature) {
            cached_results.push((sig_info.clone(), cached_transaction.clone()));
        } else {
            uncached_signatures.push(sig_info.clone());
        }
    }

    log(
        LogTag::System,
        "INFO",
        &format!(
            "Cache status: {} cached, {} need fetching from {} total signatures",
            cached_results.len(),
            uncached_signatures.len(),
            limit
        )
    );

    // Add cached results
    results.extend(cached_results);

    if uncached_signatures.is_empty() {
        log(LogTag::System, "SUCCESS", "All transactions found in cache!");
        return results;
    }

    // Process uncached signatures in smaller batches to reduce RPC pressure
    const BATCH_SIZE: usize = 5;
    let uncached_count = uncached_signatures.len();

    for batch_start in (0..uncached_count).step_by(BATCH_SIZE) {
        let batch_end = (batch_start + BATCH_SIZE).min(uncached_count);
        let batch = &uncached_signatures[batch_start..batch_end];

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Fetching batch {}-{} of {} uncached transactions",
                batch_start + 1,
                batch_end,
                uncached_count
            )
        );

        for (i, sig_info) in batch.iter().enumerate() {
            let overall_index = batch_start + i + 1;

            // Progress reporting for significant milestones
            if overall_index % 10 == 0 || overall_index == uncached_count {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("Fetching transaction {}/{}", overall_index, uncached_count)
                );
            }

            match
                get_transaction_details_cached(
                    client,
                    &sig_info.signature,
                    configs,
                    &mut cache
                ).await
            {
                Ok(Some(transaction)) => {
                    results.push((sig_info.clone(), transaction));
                }
                Ok(None) => {
                    log(
                        LogTag::System,
                        "WARNING",
                        &format!("Transaction not found: {}", sig_info.signature)
                    );
                }
                Err(_) => {
                    // Skip failed transactions silently to reduce spam
                }
            }

            // Very short delay between requests since we have fallbacks
            tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
        }

        // Longer delay between batches to be respectful to RPC
        if batch_end < uncached_count {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    }

    // Final cache save (in case there were any save failures during individual fetches)
    let (final_cache_size, _) = cache.stats();
    if final_cache_size > initial_cache_size {
        if let Err(e) = cache.save() {
            log(
                LogTag::System,
                "WARNING",
                &format!("Failed to save final transaction cache: {}", e)
            );
        } else {
            log(
                LogTag::System,
                "SUCCESS",
                &format!(
                    "Added {} new transactions to cache",
                    final_cache_size - initial_cache_size
                )
            );
        }
    }

    results
}
