// TransactionsManager - Core manager struct for transaction monitoring and coordination
//
// This module contains the main TransactionsManager struct that coordinates
// all transaction-related operations for the ScreenerBot trading system.

use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use tokio::sync::{ Mutex, Notify };
use chrono::{ DateTime, Utc };
use solana_sdk::pubkey::Pubkey;

use crate::logger::{ log, LogTag };
use crate::global::is_debug_transactions_enabled;
use crate::tokens::TokenDatabase;
use crate::transactions::{ types::*, database::TransactionDatabase, utils::* };

// =============================================================================
// TRANSACTIONS MANAGER STRUCT
// =============================================================================

/// TransactionsManager - Main service for real-time transaction monitoring
///
/// This struct coordinates all transaction-related functionality including:
/// - Real-time transaction monitoring via WebSocket integration
/// - High-performance batch RPC operations
/// - Transaction analysis and classification
/// - Position integration for entry/exit verification
/// - Database caching and persistence
/// - Retry logic for network resilience
pub struct TransactionsManager {
    // Core identification
    pub wallet_pubkey: Pubkey,
    pub debug_enabled: bool,

    // Transaction tracking state
    pub known_signatures: HashSet<String>,
    pub last_signature_check: Option<String>,
    pub total_transactions: u64,
    pub new_transactions_count: u64,

    // Database integrations
    pub token_database: Option<Arc<Mutex<TokenDatabase>>>,
    pub transaction_database: Option<Arc<TransactionDatabase>>,

    // Retry management for network resilience
    pub deferred_retries: HashMap<String, DeferredRetry>,

    // WebSocket integration for real-time monitoring
    pub websocket_receiver: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    pub websocket_shutdown: Option<Arc<Notify>>,

    // Pending transaction tracking
    pub pending_transactions: HashMap<String, DateTime<Utc>>,

    // Service control
    pub is_running: bool,
    pub shutdown_notify: Arc<Notify>,
}

// =============================================================================
// IMPLEMENTATION - CREATION AND LIFECYCLE
// =============================================================================

impl TransactionsManager {
    /// Create new TransactionsManager instance with token database integration
    pub async fn new(wallet_pubkey: Pubkey) -> Result<Self, String> {
        let debug_enabled = is_debug_transactions_enabled();

        if debug_enabled {
            log(
                LogTag::Transactions,
                "INFO",
                &format!(
                    "Creating TransactionsManager for wallet: {}",
                    format_address_full(&wallet_pubkey.to_string())
                )
            );
        }

        // Initialize transaction database
        let transaction_database = match TransactionDatabase::new().await {
            Ok(db) => {
                if debug_enabled {
                    log(
                        LogTag::Transactions,
                        "INFO",
                        "Transaction database initialized successfully"
                    );
                }
                Some(Arc::new(db))
            }
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to initialize transaction database: {}", e)
                );
                None
            }
        };

        // Initialize token database integration
        let token_database = match TokenDatabase::new() {
            Ok(db) => {
                if debug_enabled {
                    log(LogTag::Transactions, "INFO", "Token database integration initialized");
                }
                Some(Arc::new(Mutex::new(db)))
            }
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to initialize token database integration: {}", e)
                );
                None
            }
        };

        Ok(Self {
            wallet_pubkey,
            debug_enabled,
            known_signatures: HashSet::new(),
            last_signature_check: None,
            total_transactions: 0,
            new_transactions_count: 0,
            token_database,
            transaction_database,
            deferred_retries: HashMap::new(),
            websocket_receiver: None,
            websocket_shutdown: None,
            pending_transactions: HashMap::new(),
            is_running: false,
            shutdown_notify: Arc::new(Notify::new()),
        })
    }

    /// Initialize the manager with existing state from database
    pub async fn initialize(&mut self) -> Result<(), String> {
        let duration = DurationMeasure::start("TransactionsManager::initialize");

        // Load known signatures from database if available
        if let Some(ref db) = self.transaction_database {
            match db.get_known_signatures_count().await {
                Ok(count) => {
                    self.total_transactions = count;
                    if self.debug_enabled {
                        log(
                            LogTag::Transactions,
                            "INFO",
                            &format!("Loaded {} known signatures from database", count)
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("Failed to load known signatures count: {}", e)
                    );
                }
            }
        }

        // Load pending transactions from database
        if let Some(ref db) = self.transaction_database {
            match db.get_pending_transactions().await {
                Ok(pending) => {
                    self.pending_transactions = pending;
                    if self.debug_enabled && !self.pending_transactions.is_empty() {
                        log(
                            LogTag::Transactions,
                            "INFO",
                            &format!(
                                "Loaded {} pending transactions from database",
                                self.pending_transactions.len()
                            )
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("Failed to load pending transactions: {}", e)
                    );
                }
            }
        }

        // Initialize WebSocket connection if configured
        self.initialize_websocket().await?;

        duration.finish_and_log();

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "INFO",
                &format!(
                    "TransactionsManager initialized for wallet: {} (known transactions: {})",
                    format_address_full(&self.wallet_pubkey.to_string()),
                    self.total_transactions
                )
            );
        }

        Ok(())
    }

    /// Initialize WebSocket connection for real-time transaction monitoring
    async fn initialize_websocket(&mut self) -> Result<(), String> {
        // This will be implemented when integrating with the existing websocket module
        // For now, we'll skip WebSocket initialization to avoid breaking changes

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "DEBUG",
                "WebSocket initialization skipped (will be integrated in service module)"
            );
        }

        Ok(())
    }

    /// Shutdown the manager and cleanup resources
    pub async fn shutdown(&mut self) -> Result<(), String> {
        log(LogTag::Transactions, "INFO", "TransactionsManager shutting down...");

        self.is_running = false;

        // Signal shutdown to any running services
        self.shutdown_notify.notify_waiters();

        // Close WebSocket connection if active
        if let Some(shutdown) = self.websocket_shutdown.take() {
            shutdown.notify_waiters();
            log(LogTag::Transactions, "DEBUG", "WebSocket shutdown signal sent");
        }

        // Cleanup deferred retries
        self.deferred_retries.clear();

        // Save pending transactions to database
        if let Some(ref db) = self.transaction_database {
            if let Err(e) = db.save_pending_transactions(&self.pending_transactions).await {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to save pending transactions during shutdown: {}", e)
                );
            }
        }

        log(LogTag::Transactions, "INFO", "TransactionsManager shutdown complete");
        Ok(())
    }
}

// =============================================================================
// IMPLEMENTATION - STATISTICS AND STATE
// =============================================================================

impl TransactionsManager {
    /// Get transaction statistics
    pub fn get_stats(&self) -> TransactionStats {
        TransactionStats {
            total_transactions: self.total_transactions,
            new_transactions_count: self.new_transactions_count,
            known_signatures_count: self.known_signatures.len() as u64,
            pending_transactions_count: self.pending_transactions.len() as u64,
            failed_transactions_count: 0, // Will be calculated from database
            successful_transactions_count: 0, // Will be calculated from database
        }
    }

    /// Get enhanced statistics with database queries
    pub async fn get_enhanced_stats(&self) -> TransactionStats {
        let mut stats = self.get_stats();

        if let Some(ref db) = self.transaction_database {
            // Get success/failure counts from database
            if let Ok(success_count) = db.get_successful_transactions_count().await {
                stats.successful_transactions_count = success_count;
            }

            if let Ok(failed_count) = db.get_failed_transactions_count().await {
                stats.failed_transactions_count = failed_count;
            }
        }

        stats
    }

    /// Check if signature is known using database (if available) or fallback to HashSet
    pub async fn is_signature_known(&self, signature: &str) -> bool {
        // First check global cache
        if is_signature_known_globally(signature).await {
            return true;
        }

        // Then check database if available
        if let Some(ref db) = self.transaction_database {
            if let Ok(known) = db.is_signature_known(signature).await {
                return known;
            }
        }

        // Fallback to local HashSet
        self.known_signatures.contains(signature)
    }

    /// Add signature to known cache using database (if available) or fallback to HashSet
    pub async fn add_signature_to_known(&mut self, signature: String) {
        // Add to global cache
        add_signature_to_known_globally(signature.clone()).await;

        // Add to database if available
        if let Some(ref db) = self.transaction_database {
            if let Err(e) = db.add_known_signature(&signature).await {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("Failed to add signature to database: {}", e)
                    );
                }
            }
        }

        // Add to local HashSet as fallback
        self.known_signatures.insert(signature);
    }

    /// Check if manager is currently running
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Get shutdown notification handle
    pub fn get_shutdown_notify(&self) -> Arc<Notify> {
        Arc::clone(&self.shutdown_notify)
    }

    /// Get wallet pubkey
    pub fn get_wallet_pubkey(&self) -> Pubkey {
        self.wallet_pubkey
    }

    /// Get debug status
    pub fn is_debug_enabled(&self) -> bool {
        self.debug_enabled
    }
}

// =============================================================================
// IMPLEMENTATION - PENDING TRANSACTIONS MANAGEMENT
// =============================================================================

impl TransactionsManager {
    /// Add a pending transaction
    pub async fn add_pending_transaction(&mut self, signature: String) {
        let now = Utc::now();
        self.pending_transactions.insert(signature.clone(), now);

        // Also add to global pending cache
        add_pending_transaction_globally(signature.clone(), now).await;

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "DEBUG",
                &format!("Added pending transaction: {}", format_signature_short(&signature))
            );
        }
    }

    /// Remove a pending transaction
    pub async fn remove_pending_transaction(&mut self, signature: &str) {
        if self.pending_transactions.remove(signature).is_some() {
            // Also remove from global pending cache
            remove_pending_transaction_globally(signature).await;

            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "DEBUG",
                    &format!("Removed pending transaction: {}", format_signature_short(signature))
                );
            }
        }
    }

    /// Get pending transactions list
    pub fn get_pending_transactions(&self) -> Vec<String> {
        self.pending_transactions.keys().cloned().collect()
    }

    /// Cleanup expired pending transactions
    pub async fn cleanup_expired_pending(&mut self) -> usize {
        let now = Utc::now();
        let mut expired_count = 0;

        self.pending_transactions.retain(|signature, timestamp| {
            let age_secs = (now - *timestamp).num_seconds();
            if age_secs > PENDING_MAX_AGE_SECS {
                if self.debug_enabled {
                    log(
                        LogTag::Transactions,
                        "DEBUG",
                        &format!(
                            "Expired pending transaction: {} (age: {}s)",
                            format_signature_short(signature),
                            age_secs
                        )
                    );
                }
                expired_count += 1;
                false
            } else {
                true
            }
        });

        if expired_count > 0 {
            log(
                LogTag::Transactions,
                "INFO",
                &format!("Cleaned up {} expired pending transactions", expired_count)
            );

            // Also cleanup global pending cache
            cleanup_expired_pending_transactions().await;
        }

        expired_count
    }
}

// =============================================================================
// IMPLEMENTATION - DEFERRED RETRIES MANAGEMENT
// =============================================================================

impl TransactionsManager {
    /// Add a deferred retry for a failed signature
    pub fn add_deferred_retry(&mut self, signature: String, error: Option<String>) {
        let retry = DeferredRetry {
            signature: signature.clone(),
            next_retry_at: Utc::now() + chrono::Duration::seconds(30), // Start with 30 second delay
            remaining_attempts: 3,
            current_delay_secs: 30,
            last_error: error,
        };

        self.deferred_retries.insert(signature.clone(), retry);

        if self.debug_enabled {
            log(
                LogTag::Transactions,
                "DEBUG",
                &format!(
                    "Added deferred retry for signature: {}",
                    format_signature_short(&signature)
                )
            );
        }
    }

    /// Get retries that are ready to be processed
    pub fn get_ready_retries(&mut self) -> Vec<DeferredRetry> {
        let now = Utc::now();
        let mut ready_retries = Vec::new();

        self.deferred_retries.retain(|signature, retry| {
            if now >= retry.next_retry_at {
                if retry.remaining_attempts > 0 {
                    ready_retries.push(retry.clone());

                    // Update retry for next attempt
                    let mut updated_retry = retry.clone();
                    updated_retry.remaining_attempts -= 1;
                    updated_retry.current_delay_secs *= 2; // Exponential backoff
                    updated_retry.next_retry_at =
                        now + chrono::Duration::seconds(updated_retry.current_delay_secs);

                    if updated_retry.remaining_attempts > 0 {
                        self.deferred_retries.insert(signature.clone(), updated_retry);
                        false // Keep in map for future retries
                    } else {
                        if self.debug_enabled {
                            log(
                                LogTag::Transactions,
                                "WARN",
                                &format!(
                                    "Exhausted retries for signature: {}",
                                    format_signature_short(signature)
                                )
                            );
                        }
                        true // Remove from map
                    }
                } else {
                    true // Remove expired retry
                }
            } else {
                false // Keep for future processing
            }
        });

        ready_retries
    }

    /// Remove a deferred retry (usually after successful processing)
    pub fn remove_deferred_retry(&mut self, signature: &str) {
        if self.deferred_retries.remove(signature).is_some() {
            if self.debug_enabled {
                log(
                    LogTag::Transactions,
                    "DEBUG",
                    &format!(
                        "Removed deferred retry for signature: {}",
                        format_signature_short(signature)
                    )
                );
            }
        }
    }

    /// Get count of pending deferred retries
    pub fn get_deferred_retries_count(&self) -> usize {
        self.deferred_retries.len()
    }
}

// =============================================================================
// IMPLEMENTATION - DATABASE INTEGRATION
// =============================================================================

impl TransactionsManager {
    /// Get database connection if available
    pub fn get_transaction_database(&self) -> Option<Arc<TransactionDatabase>> {
        self.transaction_database.as_ref().map(Arc::clone)
    }

    /// Get token database connection if available
    pub fn get_token_database(&self) -> Option<Arc<Mutex<TokenDatabase>>> {
        self.token_database.as_ref().map(Arc::clone)
    }

    /// Check if database is available and connected
    pub async fn is_database_connected(&self) -> bool {
        if let Some(ref db) = self.transaction_database {
            db.health_check().await.is_ok()
        } else {
            false
        }
    }
}

// =============================================================================
// STATIC GLOBAL TRANSACTION STATISTICS
// =============================================================================

impl TransactionsManager {
    /// Get global transaction statistics (static method for compatibility)
    pub async fn get_transaction_stats() -> TransactionStats {
        TransactionStats {
            total_transactions: 0, // Will be populated by global service
            new_transactions_count: 0,
            known_signatures_count: get_known_signatures_count().await as u64,
            pending_transactions_count: get_pending_transactions_count().await as u64,
            failed_transactions_count: 0,
            successful_transactions_count: 0,
        }
    }
}
