//! Volume Aggregator Types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

use crate::tools::types::{DelayConfig, DistributionStrategy, SizingConfig, WalletMode};

/// Configuration for volume aggregation session
#[derive(Debug, Clone)]
pub struct VolumeConfig {
    /// Target token mint address
    pub token_mint: Pubkey,
    /// Total SOL volume to generate (buy + sell combined)
    pub total_volume_sol: f64,
    /// Number of wallets to use for distribution
    pub num_wallets: usize,
    /// Delay configuration between transactions
    pub delay_config: DelayConfig,
    /// Sizing configuration for transaction amounts
    pub sizing_config: SizingConfig,
    /// Wallet selection mode
    pub wallet_mode: WalletMode,
    /// Distribution strategy across wallets
    pub strategy: DistributionStrategy,
    /// Specific wallet addresses to use (when wallet_mode is Selected)
    pub wallet_addresses: Option<Vec<String>>,
}

impl VolumeConfig {
    /// Create a new config with sensible defaults
    pub fn new(token_mint: Pubkey, total_volume_sol: f64) -> Self {
        Self {
            token_mint,
            total_volume_sol,
            num_wallets: 5,
            delay_config: DelayConfig::default(),
            sizing_config: SizingConfig::default(),
            wallet_mode: WalletMode::AutoSelect,
            strategy: DistributionStrategy::RoundRobin,
            wallet_addresses: None,
        }
    }

    /// Builder: set delay config
    pub fn with_delay(mut self, delay_config: DelayConfig) -> Self {
        self.delay_config = delay_config;
        self
    }

    /// Builder: set sizing config
    pub fn with_sizing(mut self, sizing_config: SizingConfig) -> Self {
        self.sizing_config = sizing_config;
        self
    }

    /// Builder: set wallet mode
    pub fn with_wallet_mode(mut self, mode: WalletMode) -> Self {
        self.wallet_mode = mode;
        self
    }

    /// Builder: set distribution strategy
    pub fn with_strategy(mut self, strategy: DistributionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Builder: set specific wallet addresses
    pub fn with_wallets(mut self, addresses: Vec<String>) -> Self {
        self.wallet_addresses = Some(addresses);
        self.wallet_mode = WalletMode::Selected;
        self
    }

    /// Builder: set number of wallets
    pub fn with_num_wallets(mut self, count: usize) -> Self {
        self.num_wallets = count;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.total_volume_sol <= 0.0 {
            return Err("Total volume must be positive".to_string());
        }
        if self.num_wallets == 0 {
            return Err("At least one wallet is required".to_string());
        }
        self.sizing_config.validate()?;
        Ok(())
    }

    /// Calculate estimated number of transactions
    pub fn estimate_transaction_count(&self) -> usize {
        let avg_amount = match &self.sizing_config {
            SizingConfig::Fixed { amount_sol } => *amount_sol,
            SizingConfig::Random { min_sol, max_sol } => (*min_sol + *max_sol) / 2.0,
        };
        // Each round trip is buy + sell = 2 transactions
        let round_trips = (self.total_volume_sol / (avg_amount * 2.0)).ceil() as usize;
        round_trips * 2
    }

    /// Calculate estimated duration in seconds
    pub fn estimate_duration_secs(&self) -> u64 {
        let tx_count = self.estimate_transaction_count();
        let avg_delay = match &self.delay_config {
            DelayConfig::Fixed { delay_ms } => *delay_ms,
            DelayConfig::Random { min_ms, max_ms } => (*min_ms + *max_ms) / 2,
        };
        (tx_count as u64 * avg_delay) / 1000
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
    /// Target volume in SOL
    pub target_volume_sol: f64,
    /// All transactions in this session
    pub transactions: Vec<VolumeTransaction>,
    /// Total SOL volume generated (actual)
    pub actual_volume_sol: f64,
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
    /// Current status
    pub status: SessionStatus,
    /// Error message if session failed
    pub error: Option<String>,
    /// Database row ID (if persisted)
    pub db_id: Option<i64>,
}

/// Session status enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is ready to start
    Ready,
    /// Session is currently running
    Running,
    /// Session completed successfully
    Completed,
    /// Session failed with error
    Failed,
    /// Session was aborted by user
    Aborted,
}

impl Default for SessionStatus {
    fn default() -> Self {
        SessionStatus::Ready
    }
}

impl VolumeSession {
    /// Create a new session
    pub fn new(token_mint: &Pubkey, target_volume_sol: f64) -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            token_mint: token_mint.to_string(),
            target_volume_sol,
            transactions: Vec::new(),
            actual_volume_sol: 0.0,
            successful_buys: 0,
            successful_sells: 0,
            failed_count: 0,
            started_at: Utc::now(),
            ended_at: None,
            status: SessionStatus::Ready,
            error: None,
            db_id: None,
        }
    }

    /// Create session with existing ID (for resume)
    pub fn with_id(session_id: String, token_mint: String, target_volume_sol: f64) -> Self {
        Self {
            session_id,
            token_mint,
            target_volume_sol,
            transactions: Vec::new(),
            actual_volume_sol: 0.0,
            successful_buys: 0,
            successful_sells: 0,
            failed_count: 0,
            started_at: Utc::now(),
            ended_at: None,
            status: SessionStatus::Ready,
            error: None,
            db_id: None,
        }
    }

    /// Set database ID after persistence
    pub fn set_db_id(&mut self, id: i64) {
        self.db_id = Some(id);
    }

    /// Mark session as running
    pub fn start(&mut self) {
        self.status = SessionStatus::Running;
        self.started_at = Utc::now();
    }

    /// Add a transaction to the session
    pub fn add_transaction(&mut self, tx: VolumeTransaction) {
        match tx.status {
            TransactionStatus::Confirmed => {
                self.actual_volume_sol += tx.amount_sol;
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
        self.status = SessionStatus::Completed;
    }

    /// Mark session as failed
    pub fn fail(&mut self, error: String) {
        self.ended_at = Some(Utc::now());
        self.status = SessionStatus::Failed;
        self.error = Some(error);
    }

    /// Mark session as aborted
    pub fn abort(&mut self) {
        self.ended_at = Some(Utc::now());
        self.status = SessionStatus::Aborted;
        self.error = Some("Aborted by user".to_string());
    }

    /// Check if session is completed (success or failure)
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Aborted
        )
    }

    /// Check if session can be resumed
    pub fn can_resume(&self) -> bool {
        self.status == SessionStatus::Running && self.actual_volume_sol < self.target_volume_sol
    }

    /// Get remaining volume to generate
    pub fn remaining_volume(&self) -> f64 {
        (self.target_volume_sol - self.actual_volume_sol).max(0.0)
    }

    /// Get progress percentage
    pub fn progress_pct(&self) -> f64 {
        if self.target_volume_sol == 0.0 {
            return 100.0;
        }
        (self.actual_volume_sol / self.target_volume_sol * 100.0).min(100.0)
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

    /// Get last completed transaction index (for resume)
    pub fn last_completed_index(&self) -> usize {
        self.transactions
            .iter()
            .filter(|tx| tx.status != TransactionStatus::Pending)
            .count()
    }
}
