// transactions/types.rs - Core data structures and types
use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };

/// Maximum number of transactions to fetch in one request
pub const MAX_TRANSACTIONS_PER_REQUEST: usize = 100;

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
pub enum WebSocketMessage {
    Subscribe {
        address: String,
    },
    Unsubscribe {
        address: String,
    },
    TransactionUpdate {
        signature: String,
        slot: u64,
    },
    Error {
        message: String,
    },
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
