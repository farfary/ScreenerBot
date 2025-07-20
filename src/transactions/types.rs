use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };
use std::collections::HashMap;

/// Transaction type classification
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum TransactionType {
    Unknown,
    Swap,
    Transfer,
    Airdrop,
    StakeUnstake,
    ProgramDeploy,
    AccountCreation,
}

/// Program interaction information
#[derive(Debug, Clone, Serialize)]
pub struct ProgramInteraction {
    pub instruction_index: usize,
    pub program_id: String,
    pub dex_name: Option<String>,
    pub is_known_dex: bool,
    pub data_length: usize,
}

/// Token transfer information
#[derive(Debug, Clone, Serialize)]
pub struct TokenTransfer {
    pub mint: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub amount: String,
    pub decimals: u8,
    pub ui_amount: Option<f64>,
    pub account_index: u8,
    pub amount_change: f64,
    pub is_incoming: bool,
}

/// Detailed swap information
#[derive(Debug, Clone, Serialize)]
pub struct SwapInfo {
    pub dex_name: String,
    pub program_id: String,
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: String,
    pub output_amount: String,
    pub input_decimals: u8,
    pub output_decimals: u8,
    pub swap_type: SwapType,
    pub input_token: String,
    pub output_token: String,
    pub effective_price: f64,
}

/// Transaction categorization result
#[derive(Debug, Clone, Serialize)]
pub struct TransactionCategorization {
    pub total_transactions: usize,
    pub swaps: Vec<String>,
    pub transfers: Vec<String>,
    pub airdrops: Vec<String>,
    pub unknown: Vec<String>,
    pub success_rate: f64,
    pub dex_usage: std::collections::HashMap<String, usize>,
}

/// Transaction processing statistics
#[derive(Debug, Clone, Serialize)]
pub struct TransactionStats {
    pub total: usize,
    pub swaps: usize,
    pub airdrops: usize,
    pub transfers: usize,
    pub unknown: usize,
    pub swap_percentage: f64,
    pub most_used_dex: Option<String>,
    pub total_processed: usize,
    pub successful: usize,
    pub failed: usize,
    pub swaps_detected: usize,
    pub average_processing_time_ms: f64,
}

/// Maximum number of transactions to fetch in one request
/// Solana RPC limits (conservative values for rate limiting)
pub const MAX_SIGNATURES_PER_REQUEST: usize = 1000;
pub const MAX_TRANSACTIONS_PER_REQUEST: usize = 1000;

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
    /// Whirlpool (Orca)
    pub const WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
}

/// DEX Program IDs as tuples for iteration
pub const DEX_PROGRAM_IDS: &[(&str, &str)] = &[
    (dex_program_ids::SERUM_DEX_V2, "Serum DEX V2"),
    (dex_program_ids::SERUM_DEX_V3, "Serum DEX V3"),
    (dex_program_ids::RAYDIUM_V2, "Raydium V2"),
    (dex_program_ids::RAYDIUM_V3, "Raydium V3"),
    (dex_program_ids::RAYDIUM_V4, "Raydium V4"),
    (dex_program_ids::RAYDIUM_ROUTING, "Raydium Routing"),
    (dex_program_ids::RAYDIUM_CONCENTRATED, "Raydium CLMM"),
    (dex_program_ids::OPENOCEAN, "OpenOcean"),
    (dex_program_ids::JUPITER, "Jupiter"),
    (dex_program_ids::PHOENIX, "Phoenix"),
    (dex_program_ids::WHIRLPOOL, "Whirlpool"),
];

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
        dex_program_ids::WHIRLPOOL => Some("Whirlpool"),
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

#[derive(Debug, Deserialize, Clone, Serialize, Default)]
pub struct TransactionResult {
    pub transaction: Transaction,
    pub meta: Option<TransactionMeta>,
    #[serde(rename = "blockTime")]
    pub block_time: Option<u64>,
    pub slot: u64,
}

#[derive(Debug, Deserialize, Clone, Serialize, Default)]
pub struct Transaction {
    pub message: TransactionMessage,
    pub signatures: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize, Default)]
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
    pub dex_name: Option<String>,
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

impl std::fmt::Display for SwapType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapType::Buy => write!(f, "Buy"),
            SwapType::Sell => write!(f, "Sell"),
            SwapType::SwapAtoB => write!(f, "A→B"),
            SwapType::SwapBtoA => write!(f, "B→A"),
            SwapType::Unknown => write!(f, "Unknown"),
        }
    }
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
    pub transaction_type: TransactionType,
    pub is_swap: bool,
    pub is_airdrop: bool,
    pub is_transfer: bool,
    pub swap_info: Option<SwapInfo>,
    pub token_transfers: Vec<TokenTransfer>,
    pub sol_balance_change: i64,
    pub contains_swaps: bool,
    pub swaps: Vec<SwapTransaction>,
    pub token_changes: Vec<TokenBalanceChange>,
    pub involves_target_token: bool,
    pub program_interactions: Vec<ProgramInteraction>,
}

/// Database record for a cached transaction
#[derive(Debug, Clone)]
pub struct TransactionRecord {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<u64>,
    pub data: String, // Serialized TransactionResult
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

/// Batch processing configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub batch_size: usize,
    pub max_concurrent: usize,
    pub delay_between_batches_ms: u64,
    pub delay_between_requests_ms: u64,
    pub max_retries: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            max_concurrent: 3,
            delay_between_batches_ms: 500,
            delay_between_requests_ms: 100,
            max_retries: 3,
        }
    }
}

/// WebSocket message types for real-time transaction updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub jsonrpc: Option<String>,
    pub id: Option<serde_json::Value>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}

/// Transaction sync status
#[derive(Debug, Clone)]
pub struct SyncStatus {
    pub last_sync_slot: u64,
    pub last_sync_time: DateTime<Utc>,
    pub total_transactions: u64,
    pub pending_transactions: u64,
}

/// Utility function to format timestamp
pub fn format_timestamp(timestamp: Option<u64>) -> String {
    match timestamp {
        Some(ts) => {
            let dt = DateTime::from_timestamp(ts as i64, 0);
            match dt {
                Some(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                None => "Invalid timestamp".to_string(),
            }
        }
        None => "Unknown time".to_string(),
    }
}

/// Enhanced swap detection functions for finding specific token pairs
pub mod swap_detection {
    use super::*;

    /// Find swaps between two specific tokens
    pub fn find_swaps_between_tokens(
        transactions: &[(SignatureInfo, TransactionResult)],
        token_a: &str,
        token_b: &str
    ) -> Vec<SwapTransaction> {
        let analyzer = crate::transactions::analyzer::TransactionAnalyzer::new();
        let mut swaps = Vec::new();

        for (_, transaction) in transactions {
            let detected_swaps = analyzer.detect_swaps_advanced(transaction);

            for swap in detected_swaps {
                // Check if this swap involves the target token pair
                let involves_pair =
                    (swap.input_token.mint == token_a && swap.output_token.mint == token_b) ||
                    (swap.input_token.mint == token_b && swap.output_token.mint == token_a);

                if involves_pair {
                    swaps.push(swap);
                }
            }
        }

        swaps
    }

    /// Find all swaps involving a specific token
    pub fn find_swaps_with_token(
        transactions: &[(SignatureInfo, TransactionResult)],
        token_mint: &str
    ) -> Vec<SwapTransaction> {
        let analyzer = crate::transactions::analyzer::TransactionAnalyzer::new();
        let mut swaps = Vec::new();

        for (_, transaction) in transactions {
            let detected_swaps = analyzer.detect_swaps_advanced(transaction);

            for swap in detected_swaps {
                if swap.input_token.mint == token_mint || swap.output_token.mint == token_mint {
                    swaps.push(swap);
                }
            }
        }

        swaps
    }

    /// Get swap statistics for a specific DEX
    pub fn get_dex_swap_stats(
        transactions: &[(SignatureInfo, TransactionResult)],
        dex_name: &str
    ) -> DexSwapStats {
        let analyzer = crate::transactions::analyzer::TransactionAnalyzer::new();
        let mut stats = DexSwapStats {
            dex_name: dex_name.to_string(),
            total_swaps: 0,
            successful_swaps: 0,
            failed_swaps: 0,
            total_volume_usd: 0.0,
            unique_tokens: std::collections::HashSet::new(),
            swap_types: std::collections::HashMap::new(),
        };

        for (_, transaction) in transactions {
            let detected_swaps = analyzer.detect_swaps_advanced(transaction);

            for swap in detected_swaps {
                if let Some(swap_dex) = &swap.dex_name {
                    if swap_dex == dex_name {
                        stats.total_swaps += 1;

                        if swap.is_success {
                            stats.successful_swaps += 1;
                        } else {
                            stats.failed_swaps += 1;
                        }

                        stats.unique_tokens.insert(swap.input_token.mint.clone());
                        stats.unique_tokens.insert(swap.output_token.mint.clone());

                        *stats.swap_types.entry(format!("{}", swap.swap_type)).or_insert(0) += 1;
                    }
                }
            }
        }

        stats
    }
}

/// DEX-specific swap statistics
#[derive(Debug, Clone)]
pub struct DexSwapStats {
    pub dex_name: String,
    pub total_swaps: usize,
    pub successful_swaps: usize,
    pub failed_swaps: usize,
    pub total_volume_usd: f64,
    pub unique_tokens: std::collections::HashSet<String>,
    pub swap_types: std::collections::HashMap<String, usize>,
}
