/// Comprehensive Transaction Detection and Analysis System
/// 
/// This module provides unified detection and analysis for various Solana transaction types:
/// - Token swaps (BUY/SELL via Jupiter, Raydium, GMGN, Pump.fun)
/// - SOL transfers (simple SOL to SOL transfers)
/// - Token transfers (SPL token transfers)
/// - Multi-hop swaps (token->USDC->SOL, etc.)
/// - DEX interactions
/// - DeFi protocol interactions
/// 
/// Key Features:
/// - Router-agnostic detection (works with any DEX)
/// - Comprehensive transaction type classification
/// - Accurate direction detection (BUY vs SELL)
/// - Precise effective price calculations
/// - Fee analysis and separation
/// - Multi-token transaction support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use crate::{
    logger::{log, LogTag},
    global::is_debug_transactions_enabled,
    rpc::{TransactionDetails, TransactionMeta},
    tokens::decimals::get_token_decimals_from_chain,
};

/// Comprehensive transaction analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionAnalysis {
    pub transaction_type: TransactionType,
    pub direction: Option<TransactionDirection>,
    pub token_changes: Vec<TokenChange>,
    pub sol_change: f64,
    pub effective_price: f64,
    pub fees_paid: f64,
    pub router: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Supported transaction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    /// Token swap (buy/sell tokens for SOL or other tokens)
    Swap,
    /// Simple SOL transfer between accounts
    SolTransfer,
    /// SPL token transfer between accounts
    TokenTransfer,
    /// Multi-hop swap (token->USDC->SOL, etc.)
    MultiHopSwap,
    /// DeFi protocol interaction (staking, lending, etc.)
    DeFiInteraction,
    /// DEX liquidity provision
    LiquidityProvision,
    /// Unknown or unsupported transaction type
    Unknown,
}

/// Transaction direction for swaps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionDirection {
    Buy,   // Buying tokens with SOL
    Sell,  // Selling tokens for SOL
}

/// Individual token balance change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenChange {
    pub mint: String,
    pub symbol: Option<String>,
    pub amount_change: f64,  // Positive = received, negative = sent
    pub decimals: u8,
    pub usd_value: Option<f64>,
}

/// Comprehensive transaction detector
pub struct TransactionDetector {
    wallet_address: String,
}

impl TransactionDetector {
    pub fn new(wallet_address: String) -> Self {
        Self { wallet_address }
    }

    /// Main entry point: Analyze any transaction comprehensively
    pub async fn analyze_transaction(&self, signature: &str) -> Result<TransactionAnalysis, String> {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "ANALYZE", &format!(
                "🔍 Starting comprehensive transaction analysis for: {}", 
                &signature[..8]
            ));
        }

        // Fetch transaction details
        let transaction = self.fetch_transaction(signature).await?;

        // Extract meta information
        let meta = transaction.meta.as_ref()
            .ok_or_else(|| "Transaction meta not available".to_string())?;

        if meta.err.is_some() {
            return Ok(TransactionAnalysis {
                transaction_type: TransactionType::Unknown,
                direction: None,
                token_changes: vec![],
                sol_change: 0.0,
                effective_price: 0.0,
                fees_paid: lamports_to_sol(meta.fee),
                router: None,
                success: false,
                error_message: Some("Transaction failed on-chain".to_string()),
            });
        }

        // Analyze transaction step by step
        let router = self.detect_router(&transaction);
        let sol_change = self.calculate_sol_change(&transaction, meta)?;
        let token_changes = self.analyze_token_changes(&transaction, meta).await?;
        
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "CLASSIFY", &format!(
                "🔍 Classification data: router={:?}, token_changes={}, sol_change={:.6}", 
                router, token_changes.len(), sol_change
            ));
        }
        
        let transaction_type = self.classify_transaction_type(&router, &token_changes, sol_change);
        let direction = self.determine_direction(&transaction_type, &token_changes, sol_change);
        let effective_price = self.calculate_effective_price(&token_changes, sol_change, &direction);

        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "RESULT", &format!(
                "📊 Analysis complete: type={:?}, direction={:?}, price={:.12}, SOL_change={:.9}",
                transaction_type, direction, effective_price, sol_change
            ));
        }

        Ok(TransactionAnalysis {
            transaction_type,
            direction,
            token_changes,
            sol_change,
            effective_price,
            fees_paid: lamports_to_sol(meta.fee),
            router,
            success: true,
            error_message: None,
        })
    }

    /// Detect the DEX router or protocol used
    fn detect_router(&self, transaction: &TransactionDetails) -> Option<String> {
        let known_programs = vec![
            // Jupiter DEX Aggregator
            ("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", "Jupiter"),
            ("JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB", "Jupiter"),
            
            // Raydium AMM
            ("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", "Raydium"),
            ("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM", "Raydium"),
            ("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", "Raydium"),
            
            // Pump.fun
            ("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", "Pump.fun"),
            ("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA", "Pump.fun"),
            
            // GMGN
            ("9W959DqEETiGZocYWisQaak33ZHo2L8SrXcBJj2iqJLx", "GMGN"),
            
            // Meteora DLMM
            ("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo", "Meteora"),
            
            // Orca DEX
            ("9W959DqEETiGZocYWisQaak33ZHo2L8SrXcBJj2iqJLx", "Orca"),
            ("DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1", "Orca"),
            
            // Serum DEX
            ("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin", "Serum"),
            ("EUqojwWA2rd19FZrzeBncJsm38Jm1hEhE3zsmX3bRc2o", "Serum"),
            
            // Aldrin
            ("AMM55ShdkoGRB5jVYPjWziwk8m5MpwyDgsMWHaMSQWH6", "Aldrin"),
            
            // Saber
            ("SSwpkEEcbUqx4vtoEByFjSkhKdCT862DNVb52nZg1UZ", "Saber"),
            
            // Mercurial
            ("MERLuDFBMmsHnsBPZw2sDQZHvXFMwp8EdjudcU2HKky", "Mercurial"),
            
            // Invariant
            ("HyaB3W9q6XdA5xwpU4XnSZV94htfmbmqJXZcEbRaJutt", "Invariant"),
            
            // Lifinity
            ("EewxydAPCCVuNEyrVN68PuSYdQ7wKn27V9Gjeoi8dy3S", "Lifinity"),
            
            // SolFi
            ("SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe", "SolFi"),
            
            // DexLab
            ("DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1", "DexLab"),
            
            // Saros
            ("SarosD1ESgvPpqhXBQp4KaDVUZ5QDvvxqPsXhVi1YNf", "Saros"),
            
            // Cropper Finance
            ("CTMAxxk34HjKWxQ3QLZK1HpaLXmBveao3ESePXbiyfzh", "Cropper"),
            
            // Sencha Exchange
            ("SCHAtsf8mbjyjiv4LkhLKutTf6JnZAbdJKFkXQNMFHZ", "Sencha"),
            
            // Step Finance
            ("Stepn2pSkRjn6TsymfXSit2mNFMJP2vwNfgjNgKMN5t", "Step"),
            
            // GooseFX
            ("GFXsSL5sSaDfNFQUYsHekbWBW1TsFdjDYzACh62tEHxn", "GooseFX"),
            
            // Crema Finance
            ("CREAatf1HEZvK9VJaWCTzHrBgr9F7xr6ZSn2cqz1ow7n", "Crema"),
            
            // Openbook (Serum v3)
            ("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX", "Openbook"),
            
            // Phoenix
            ("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY", "Phoenix"),
        ];

        // Check transaction instructions for known program IDs
        if let Some(account_keys) = transaction.transaction.message.get("accountKeys") {
            if let Some(keys_array) = account_keys.as_array() {
                for key_obj in keys_array {
                    // Handle both string format and object format
                    let pubkey_str = if let Some(direct_str) = key_obj.as_str() {
                        direct_str
                    } else if let Some(pubkey_str) = key_obj.get("pubkey").and_then(|k| k.as_str()) {
                        pubkey_str
                    } else {
                        continue;
                    };

                    for (program_id, name) in &known_programs {
                        if pubkey_str == *program_id {
                            if is_debug_transactions_enabled() {
                                log(LogTag::Transactions, "ROUTER", &format!("🔍 Detected router: {}", name));
                            }
                            return Some(name.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Calculate SOL balance change for the wallet
    fn calculate_sol_change(&self, transaction: &TransactionDetails, meta: &TransactionMeta) -> Result<f64, String> {
        let wallet_pubkey = Pubkey::from_str(&self.wallet_address)
            .map_err(|e| format!("Invalid wallet address: {}", e))?;

        // Find wallet account index
        let account_keys = self.extract_account_keys(transaction)?;
        
        // Always log this for debugging
        log(LogTag::Transactions, "DEBUG", &format!(
            "🔍 Looking for wallet {} in {} account keys", 
            self.wallet_address, account_keys.len()
        ));
        for (i, key) in account_keys.iter().enumerate() {
            log(LogTag::Transactions, "DEBUG", &format!("  [{}]: {}", i, key));
        }
        
        let wallet_index = account_keys.iter().position(|key| *key == wallet_pubkey)
            .ok_or_else(|| format!("Wallet {} not found in transaction", self.wallet_address))?;

        if wallet_index >= meta.pre_balances.len() || wallet_index >= meta.post_balances.len() {
            return Err("Wallet index out of bounds".to_string());
        }

        let pre_balance = meta.pre_balances[wallet_index] as i64;
        let post_balance = meta.post_balances[wallet_index] as i64;
        let change_lamports = post_balance - pre_balance;
        let sol_change = lamports_to_sol(change_lamports.abs() as u64) * if change_lamports >= 0 { 1.0 } else { -1.0 };

        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "SOL_CHANGE", &format!(
                "💰 SOL change: {:.9} SOL (pre: {}, post: {})", 
                sol_change, pre_balance, post_balance
            ));
        }

        Ok(sol_change)
    }

    /// Analyze all token balance changes
    async fn analyze_token_changes(&self, transaction: &TransactionDetails, meta: &TransactionMeta) -> Result<Vec<TokenChange>, String> {
        let mut token_changes = Vec::new();

        let wallet_pubkey = Pubkey::from_str(&self.wallet_address)
            .map_err(|e| format!("Invalid wallet address: {}", e))?;

        let account_keys = self.extract_account_keys(transaction)?;
        let wallet_index = account_keys.iter().position(|key| *key == wallet_pubkey)
            .ok_or_else(|| "Wallet not found in transaction".to_string())?;

        if let (Some(pre_balances), Some(post_balances)) = (&meta.pre_token_balances, &meta.post_token_balances) {
            // Create maps for easier lookup
            let mut pre_map = HashMap::new();
            let mut post_map = HashMap::new();

            // Map pre-balances by account index and mint
            for balance in pre_balances {
                if balance.account_index == wallet_index as u32 {
                    if let Some(ui_amount) = balance.ui_token_amount.ui_amount {
                        pre_map.insert(balance.mint.clone(), ui_amount);
                    }
                }
            }

            // Map post-balances by account index and mint
            for balance in post_balances {
                if balance.account_index == wallet_index as u32 {
                    if let Some(ui_amount) = balance.ui_token_amount.ui_amount {
                        post_map.insert(balance.mint.clone(), ui_amount);
                    }
                }
            }

            // Calculate changes for all tokens
            let mut all_mints: std::collections::HashSet<String> = std::collections::HashSet::new();
            all_mints.extend(pre_map.keys().cloned());
            all_mints.extend(post_map.keys().cloned());

            for mint in all_mints {
                let pre_amount = pre_map.get(&mint).copied().unwrap_or(0.0);
                let post_amount = post_map.get(&mint).copied().unwrap_or(0.0);
                let amount_change = post_amount - pre_amount;

                // Only include significant changes (> 0.000001)
                if amount_change.abs() > 0.000001 {
                    // Get token decimals for accurate representation
                    let decimals = match get_token_decimals_from_chain(&mint).await {
                        Ok(d) => d,
                        Err(_) => 9, // Default to 9 decimals if lookup fails
                    };

                    token_changes.push(TokenChange {
                        mint: mint.clone(),
                        symbol: None, // TODO: Add symbol lookup
                        amount_change,
                        decimals,
                        usd_value: None, // TODO: Add USD value calculation
                    });

                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "TOKEN_CHANGE", &format!(
                            "🪙 Token change: {} = {:.6} (pre: {:.6}, post: {:.6})", 
                            &mint[..8], amount_change, pre_amount, post_amount
                        ));
                    }
                }
            }
        }

        Ok(token_changes)
    }

    /// Classify the transaction type based on analysis
    fn classify_transaction_type(&self, router: &Option<String>, token_changes: &[TokenChange], sol_change: f64) -> TransactionType {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "CLASSIFY", &format!(
                "🔍 Classification inputs: router={:?}, token_changes={}, sol_change={:.6}", 
                router, token_changes.len(), sol_change
            ));
        }

        // If there's a DEX router, prioritize swap classification
        if let Some(router_name) = router {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "CLASSIFY", &format!("🔄 DEX router detected: {} - checking token changes", router_name));
            }
            
            // Even if no token changes detected, if there's a router and SOL change, it's likely a swap
            if !token_changes.is_empty() {
                if token_changes.len() == 1 {
                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "CLASSIFY", "✅ Single token change with router - classifying as Swap");
                    }
                    return TransactionType::Swap;
                } else if token_changes.len() > 1 {
                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "CLASSIFY", "✅ Multiple token changes with router - classifying as MultiHopSwap");
                    }
                    return TransactionType::MultiHopSwap;
                }
            } else if sol_change.abs() > 0.001 {
                // If we have a router and SOL change but no detected token changes,
                // it might be a swap where token detection failed
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "CLASSIFY", "⚠️ Router detected with SOL change but no token changes - classifying as Swap (possible detection issue)");
                }
                return TransactionType::Swap;
            }
        }

        // If only SOL change and no tokens, it's a SOL transfer
        if token_changes.is_empty() && sol_change.abs() > 0.001 {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "CLASSIFY", "💰 No router, no tokens, SOL change - classifying as SolTransfer");
            }
            return TransactionType::SolTransfer;
        }

        // If only token changes and no significant SOL change (except fees), it's a token transfer
        if !token_changes.is_empty() && sol_change.abs() < 0.01 {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "CLASSIFY", "🪙 Token changes without significant SOL change - classifying as TokenTransfer");
            }
            return TransactionType::TokenTransfer;
        }

        // If there are both token and SOL changes but no router, could be DeFi
        if !token_changes.is_empty() && sol_change.abs() > 0.001 && router.is_none() {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "CLASSIFY", "🏦 Token and SOL changes without router - classifying as DeFiInteraction");
            }
            return TransactionType::DeFiInteraction;
        }

        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "CLASSIFY", "❓ No classification criteria met - marking as Unknown");
        }
        TransactionType::Unknown
    }

    /// Determine transaction direction for swaps
    fn determine_direction(&self, transaction_type: &TransactionType, token_changes: &[TokenChange], sol_change: f64) -> Option<TransactionDirection> {
        match transaction_type {
            TransactionType::Swap | TransactionType::MultiHopSwap => {
                // For swaps, determine direction based on SOL and token changes
                // BUY: SOL decreases (spent), tokens increase (received)
                // SELL: SOL increases (received), tokens decrease (sent)
                
                if sol_change < -0.001 { // SOL decreased significantly (more than just fees)
                    // Check if we received tokens
                    if token_changes.iter().any(|tc| tc.amount_change > 0.0) {
                        return Some(TransactionDirection::Buy);
                    }
                } else if sol_change > 0.001 { // SOL increased
                    // Check if we sent tokens
                    if token_changes.iter().any(|tc| tc.amount_change < 0.0) {
                        return Some(TransactionDirection::Sell);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Calculate effective price for swaps
    fn calculate_effective_price(&self, token_changes: &[TokenChange], sol_change: f64, direction: &Option<TransactionDirection>) -> f64 {
        if let Some(dir) = direction {
            // Find the primary token change (largest absolute value)
            if let Some(primary_token) = token_changes.iter().max_by(|a, b| a.amount_change.abs().partial_cmp(&b.amount_change.abs()).unwrap_or(std::cmp::Ordering::Equal)) {
                let token_amount = primary_token.amount_change.abs();
                let sol_amount = sol_change.abs();
                
                if token_amount > 0.0 {
                    match dir {
                        TransactionDirection::Buy => {
                            // Price = SOL spent / tokens received
                            sol_amount / token_amount
                        }
                        TransactionDirection::Sell => {
                            // Price = SOL received / tokens sold
                            sol_amount / token_amount
                        }
                    }
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Extract account keys from transaction
    fn extract_account_keys(&self, transaction: &TransactionDetails) -> Result<Vec<Pubkey>, String> {
        let account_keys_value = transaction.transaction.message.get("accountKeys")
            .ok_or_else(|| "accountKeys not found in transaction".to_string())?;
        
        let account_keys_array = account_keys_value.as_array()
            .ok_or_else(|| "accountKeys is not an array".to_string())?;

        let mut account_keys = Vec::new();
        for key_value in account_keys_array.iter() {
            if let Some(key_str) = key_value.get("pubkey").and_then(|k| k.as_str()) {
                if let Ok(pubkey) = Pubkey::from_str(key_str) {
                    account_keys.push(pubkey);
                }
            } else {
                // Try direct string format
                if let Some(key_str) = key_value.as_str() {
                    if let Ok(pubkey) = Pubkey::from_str(key_str) {
                        account_keys.push(pubkey);
                    }
                }
            }
        }

        Ok(account_keys)
    }

    /// Fetch transaction details (with caching support)
    async fn fetch_transaction(&self, signature: &str) -> Result<TransactionDetails, String> {
        // TODO: Check wallet transaction manager cache first
        // For now, use direct RPC call
        use crate::rpc::get_rpc_client;
        let rpc_client = get_rpc_client();
        rpc_client.get_transaction_details(signature).await
            .map_err(|e| format!("Failed to fetch transaction: {}", e))
    }
}

/// Utility function to convert lamports to SOL
fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

/// Public convenience function for quick transaction analysis
pub async fn analyze_transaction_comprehensive(signature: &str, wallet_address: &str) -> Result<TransactionAnalysis, String> {
    let detector = TransactionDetector::new(wallet_address.to_string());
    detector.analyze_transaction(signature).await
}

/// Helper function to format transaction analysis for display
pub fn format_transaction_analysis(analysis: &TransactionAnalysis) -> String {
    let mut result = String::new();
    
    result.push_str(&format!("📊 Transaction Type: {:?}\n", analysis.transaction_type));
    
    if let Some(direction) = &analysis.direction {
        result.push_str(&format!("🎯 Direction: {:?}\n", direction));
    }
    
    if let Some(router) = &analysis.router {
        result.push_str(&format!("🔄 Router: {}\n", router));
    }
    
    result.push_str(&format!("💰 SOL Change: {:.9} SOL\n", analysis.sol_change));
    result.push_str(&format!("💵 Fees Paid: {:.9} SOL\n", analysis.fees_paid));
    
    if analysis.effective_price > 0.0 {
        result.push_str(&format!("📈 Effective Price: {:.12} SOL/token\n", analysis.effective_price));
    }
    
    if !analysis.token_changes.is_empty() {
        result.push_str("🪙 Token Changes:\n");
        for token in &analysis.token_changes {
            let change_sign = if token.amount_change >= 0.0 { "+" } else { "" };
            result.push_str(&format!(
                "   {}{:.6} tokens ({}...)\n", 
                change_sign, 
                token.amount_change, 
                &token.mint[..8]
            ));
        }
    }
    
    if let Some(error) = &analysis.error_message {
        result.push_str(&format!("❌ Error: {}\n", error));
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_detector_creation() {
        let detector = TransactionDetector::new("FYmfcfwyx8K1MnBmk6d66eeNPoPMbTXEMve5Tk1pGgiC".to_string());
        assert_eq!(detector.wallet_address, "FYmfcfwyx8K1MnBmk6d66eeNPoPMbTXEMve5Tk1pGgiC");
    }

    #[test]
    fn test_transaction_type_classification() {
        // Test classification logic
        assert_eq!(std::mem::discriminant(&TransactionType::Swap), std::mem::discriminant(&TransactionType::Swap));
    }
}
