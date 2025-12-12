//! Volume Aggregator Types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Configuration for volume aggregation session
#[derive(Debug, Clone)]
pub struct VolumeConfig {
    /// Target token mint address
    pub token_mint: Pubkey,
    /// Total SOL volume to generate (buy + sell combined)
    pub total_volume_sol: f64,
    /// Number of wallets to use for distribution
    pub num_wallets: usize,
    /// Minimum SOL amount per transaction
    pub min_amount_sol: f64,
    /// Maximum SOL amount per transaction
    pub max_amount_sol: f64,
    /// Delay between transactions in milliseconds
    pub delay_between_ms: u64,
    /// Whether to randomize transaction amounts
    pub randomize_amounts: bool,
}

impl VolumeConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.total_volume_sol <= 0.0 {
            return Err("Total volume must be positive".to_string());
        }
        if self.num_wallets == 0 {
            return Err("At least one wallet is required".to_string());
        }
        if self.min_amount_sol <= 0.0 {
            return Err("Minimum amount must be positive".to_string());
        }
        if self.max_amount_sol < self.min_amount_sol {
            return Err("Maximum amount must be >= minimum amount".to_string());
        }
        if self.min_amount_sol < 0.001 {
            return Err("Minimum amount must be at least 0.001 SOL".to_string());
        }
        Ok(())
    }

    /// Calculate estimated number of transactions
    pub fn estimate_transaction_count(&self) -> usize {
        let avg_amount = (self.min_amount_sol + self.max_amount_sol) / 2.0;
        // Each round trip is buy + sell = 2 transactions
        let round_trips = (self.total_volume_sol / (avg_amount * 2.0)).ceil() as usize;
        round_trips * 2
    }

    /// Calculate estimated duration in seconds
    pub fn estimate_duration_secs(&self) -> u64 {
        let tx_count = self.estimate_transaction_count();
        (tx_count as u64 * self.delay_between_ms) / 1000
    }
}

/// Status of an individual transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    /// Transaction is pending execution
    Pending,
    /// Transaction was submitted and confirmed
    Confirmed,
    /// Transaction failed
    Failed,
}

/// Individual transaction in a volume session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeTransaction {
    /// Unique ID for this transaction
    pub id: usize,
    /// Wallet address that executed this transaction
    pub wallet_address: String,
    /// Transaction signature (if submitted)
    pub signature: Option<String>,
    /// Whether this is a buy (true) or sell (false)
    pub is_buy: bool,
    /// Amount in SOL
    pub amount_sol: f64,
    /// Token amount received/sold
    pub token_amount: Option<f64>,
    /// Status of the transaction
    pub status: TransactionStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// Timestamp when transaction was executed
    pub executed_at: Option<DateTime<Utc>>,
}

impl VolumeTransaction {
    /// Create a new pending transaction
    pub fn new(id: usize, wallet_address: String, is_buy: bool, amount_sol: f64) -> Self {
        Self {
            id,
            wallet_address,
            signature: None,
            is_buy,
            amount_sol,
            token_amount: None,
            status: TransactionStatus::Pending,
            error: None,
            executed_at: None,
        }
    }

    /// Mark transaction as confirmed
    pub fn confirm(&mut self, signature: String, token_amount: f64) {
        self.signature = Some(signature);
        self.token_amount = Some(token_amount);
        self.status = TransactionStatus::Confirmed;
        self.executed_at = Some(Utc::now());
    }

    /// Mark transaction as failed
    pub fn fail(&mut self, error: String) {
        self.status = TransactionStatus::Failed;
        self.error = Some(error);
        self.executed_at = Some(Utc::now());
    }
}

/// Complete volume generation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSession {
    /// Unique session ID
    pub session_id: String,
    /// Target token mint
    pub token_mint: String,
    /// All transactions in this session
    pub transactions: Vec<VolumeTransaction>,
    /// Total SOL volume generated (actual)
    pub total_volume_sol: f64,
    /// Number of successful buy transactions
    pub successful_buys: usize,
    /// Number of successful sell transactions
    pub successful_sells: usize,
    /// Number of failed transactions
    pub failed_count: usize,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Session end time
    pub ended_at: Option<DateTime<Utc>>,
    /// Whether session completed successfully
    pub completed: bool,
    /// Error message if session failed
    pub error: Option<String>,
}

impl VolumeSession {
    /// Create a new session
    pub fn new(token_mint: &Pubkey) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            token_mint: token_mint.to_string(),
            transactions: Vec::new(),
            total_volume_sol: 0.0,
            successful_buys: 0,
            successful_sells: 0,
            failed_count: 0,
            started_at: Utc::now(),
            ended_at: None,
            completed: false,
            error: None,
        }
    }

    /// Add a transaction to the session
    pub fn add_transaction(&mut self, tx: VolumeTransaction) {
        match tx.status {
            TransactionStatus::Confirmed => {
                self.total_volume_sol += tx.amount_sol;
                if tx.is_buy {
                    self.successful_buys += 1;
                } else {
                    self.successful_sells += 1;
                }
            }
            TransactionStatus::Failed => {
                self.failed_count += 1;
            }
            TransactionStatus::Pending => {}
        }
        self.transactions.push(tx);
    }

    /// Mark session as completed
    pub fn complete(&mut self) {
        self.ended_at = Some(Utc::now());
        self.completed = true;
    }

    /// Mark session as failed
    pub fn fail(&mut self, error: String) {
        self.ended_at = Some(Utc::now());
        self.completed = false;
        self.error = Some(error);
    }

    /// Get session duration in seconds
    pub fn duration_secs(&self) -> i64 {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        (end - self.started_at).num_seconds()
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.successful_buys + self.successful_sells + self.failed_count;
        if total == 0 {
            return 0.0;
        }
        ((self.successful_buys + self.successful_sells) as f64 / total as f64) * 100.0
    }
}
