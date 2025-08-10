/// Comprehensive Swap Transaction Discovery Tool
/// 
/// This tool analyzes all wallet transactions to find swap operations:
/// - Scans entire transaction history for swap patterns
/// - Detects token purchases and sales from instruction analysis
/// - Extracts swap amounts, prices, and fees
/// - Identifies swap routers (Jupiter, Raydium, etc.)
/// - Provides detailed swap analytics and statistics
/// - Exports swap data to JSON for further analysis
///
/// Usage:
///   cargo run --bin tool_find_all_swaps [--wallet WALLET] [--limit LIMIT] [--export] [--detailed]

use crate::{
    rpc::{get_rpc_client, init_rpc_client, TransactionDetails, TokenBalance, TransactionMeta, TransactionData, UiTokenAmount},
    logger::{init_file_logging, log, LogTag},
    global::is_debug_transactions_enabled,
    global::read_configs,
    tokens::{get_token_decimals, TokenDatabase},
    tokens::decimals::{SOL_DECIMALS, LAMPORTS_PER_SOL, lamports_to_sol},
    wallets_manager::WalletsManager,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signer,
};
// Remove unused transaction imports
// Remove unused client import
use std::str::FromStr;
use bs58;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};

// Global storage for raw transaction JSON data
lazy_static::lazy_static! {
    static ref TRANSACTION_JSON_CACHE: Arc<Mutex<HashMap<String, serde_json::Value>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Parser)]
#[command(about = "Find and analyze all swap transactions in wallet history")]
pub struct Args {
    /// Wallet address to analyze (if not provided, uses all configured wallets)
    #[arg(short, long)]
    pub wallet: Option<String>,
    
    /// Maximum number of transactions to analyze per wallet (default: 1000)
    #[arg(short, long, default_value = "1000")]
    pub limit: usize,
    
    /// Export detailed swap data to JSON file
    #[arg(short, long)]
    pub export: bool,
    
    /// Show detailed analysis for each swap
    #[arg(short, long)]
    pub detailed: bool,
    
    /// Only analyze recent transactions from the last N days
    #[arg(long)]
    pub days: Option<u64>,
    
    /// Filter by token mint address
    #[arg(long)]
    pub token: Option<String>,
    
    /// Filter by swap type (buy/sell)
    #[arg(long)]
    pub swap_type: Option<String>,
    
    /// Display last 40 transactions in table format with types
    #[arg(short, long)]
    pub table: bool,
    
    /// Display ONLY transaction table without any analysis (clean output)
    #[arg(long)]
    pub table_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedSwapTransaction {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub date_time: Option<String>,
    pub wallet_address: String,
    pub swap_type: String, // "buy" or "sell"
    pub token_mint: String,
    pub token_symbol: String,
    pub token_name: Option<String>,
    pub sol_amount: f64,
    pub token_amount: u64,
    pub token_decimals: u8,
    pub formatted_token_amount: f64,
    pub effective_price: f64,
    pub price_per_token: f64,
    pub fees_paid: f64,
    pub success: bool,
    pub router_program: Option<String>,
    pub router_name: Option<String>,
    pub instructions_count: usize,
    pub ata_created: bool,
    pub ata_closed: bool,
    pub priority_fee: Option<f64>,
    pub compute_units: Option<u32>,
    pub pre_token_balance: Option<f64>,
    pub post_token_balance: Option<f64>,
    pub token_balance_change: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapAnalytics {
    pub total_swaps: usize,
    pub buy_swaps: usize,
    pub sell_swaps: usize,
    pub unique_tokens: usize,
    pub total_sol_spent: f64,
    pub total_sol_received: f64,
    pub total_fees_paid: f64,
    pub net_sol_change: f64,
    pub average_swap_size: f64,
    pub most_traded_tokens: Vec<(String, usize)>,
    pub router_usage: HashMap<String, usize>,
    pub success_rate: f64,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub swaps_by_month: HashMap<String, usize>,
    pub fee_efficiency: f64, // fees as percentage of volume
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSwapReport {
    pub wallet_address: String,
    pub analytics: SwapAnalytics,
    pub detailed_swaps: Vec<DetailedSwapTransaction>,
}

/// Extract actual swap amounts from inner instructions, excluding ATA creation costs and fees
fn extract_actual_swap_amounts_from_json(transaction_json: &serde_json::Value, target_token_mint: &str) -> Option<(f64, f64)> {
    // Access the meta field to get inner instructions
    let meta = transaction_json.get("meta")?;
    let inner_instructions = meta.get("innerInstructions")?;
    
    if let Some(inner_array) = inner_instructions.as_array() {
        let mut sol_amount = 0.0;
        let mut token_amount = 0.0;
        
        // Look through all inner instruction groups
        for instruction_group in inner_array {
            if let Some(instructions) = instruction_group.get("instructions").and_then(|i| i.as_array()) {
                for instruction in instructions {
                    // Look for transferChecked instructions (actual token transfers)
                    if let Some(program) = instruction.get("program").and_then(|p| p.as_str()) {
                        if program == "spl-token" {
                            if let Some(parsed) = instruction.get("parsed") {
                                if let Some(type_str) = parsed.get("type").and_then(|t| t.as_str()) {
                                    if type_str == "transferChecked" {
                                        if let Some(info) = parsed.get("info") {
                                            if let Some(mint) = info.get("mint").and_then(|m| m.as_str()) {
                                                if let Some(token_amount_obj) = info.get("tokenAmount") {
                                                    if let Some(ui_amount) = token_amount_obj.get("uiAmount").and_then(|a| a.as_f64()) {
                                                        
                                                        // Check if this is SOL (wrapped SOL mint)
                                                        if mint == "So11111111111111111111111111111111111111112" {
                                                            // This is the SOL amount being spent for the swap
                                                        if is_debug_transactions_enabled() {
                                                            log(LogTag::Transactions, "DEBUG", &format!("📊 Found SOL transfer in swap: {} SOL", ui_amount));
                                                        }
                                                            sol_amount = ui_amount;
                                                        } 
                                                        // Check if this is our target token
                                                        else if mint == target_token_mint {
                                                            // This is the token amount being received/sent
                                                        if is_debug_transactions_enabled() {
                                                            log(LogTag::Transactions, "DEBUG", &format!("📊 Found token transfer in swap: {} tokens (mint: {})", ui_amount, mint));
                                                        }
                                                            token_amount = ui_amount;
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
        
        // Return the extracted amounts if we found both
        if sol_amount > 0.0 && token_amount > 0.0 {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Extracted actual swap amounts: {} SOL ↔ {} tokens", sol_amount, token_amount));
            }
            Some((sol_amount, token_amount))
        } else if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "📊 Could not extract both SOL and token amounts from inner instructions");
            None
        } else {
            None
        }
    } else if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "📊 No inner instructions found in transaction");
        None
    } else {
        None
    }
}

async fn analyze_jupiter_swap(
    transaction: &TransactionDetails,
    meta: &TransactionMeta,
    wallet_address: &str,
    transaction_json: Option<&serde_json::Value>,
) -> Option<(bool, BasicSwapInfo)> {
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "🔍 Analyzing Jupiter swap transaction");
    }
    
    // For Jupiter swaps, look at SOL balance changes and token balance changes
    // Parse the message from JSON to get account keys
    let account_keys_value = transaction.transaction.message.get("accountKeys")?;
    let account_keys_array = account_keys_value.as_array()?;
    
    // Convert to pubkeys and find wallet index
    let mut account_keys = Vec::new();
    for (i, key_value) in account_keys_array.iter().enumerate() {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - account key {}: {:?}", i, key_value));
        }
        
        // Handle both string format and object format
        let key_str = if let Some(key_str) = key_value.as_str() {
            // Direct string format
            key_str
        } else if let Some(key_str) = key_value.get("pubkey").and_then(|k| k.as_str()) {
            // Object format with pubkey field
            key_str
        } else {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - unable to extract pubkey from key_value"));
            }
            continue;
        };
        
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - found pubkey string: {}", key_str));
        }
        
        if let Ok(pubkey) = Pubkey::from_str(key_str) {
            account_keys.push(pubkey);
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - converted pubkey {}: {}", account_keys.len() - 1, pubkey));
            }
        } else if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - failed to parse pubkey: {}", key_str));
        }
    }
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - converted {} total pubkeys", account_keys.len()));
    }
    
    let wallet_pubkey = Pubkey::from_str(wallet_address).ok()?;
    let wallet_index = account_keys.iter().position(|key| *key == wallet_pubkey)?;
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - wallet index: {}", wallet_index));
    }
    
    // Analyze SOL balance changes
    let sol_change = if wallet_index < meta.pre_balances.len() && wallet_index < meta.post_balances.len() {
        let pre_balance = meta.pre_balances[wallet_index] as i64;
        let post_balance = meta.post_balances[wallet_index] as i64;
        lamports_to_sol((post_balance - pre_balance).abs() as u64) * if post_balance > pre_balance { 1.0 } else { -1.0 }
    } else {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "📊 Jupiter swap - cannot find wallet SOL balances");
        }
        return None;
    };
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - SOL change: {} SOL", sol_change));
    }
    
    // Look for token balance changes involving wallet
    if let (Some(pre_balances), Some(post_balances)) = (&meta.pre_token_balances, &meta.post_token_balances) {
        // Look for wallet's token accounts that had changes
        let mut wallet_token_changes = HashMap::new();
        
        // Get wallet pubkey string for owner comparison
        let wallet_pubkey_str = wallet_address;
        
        // Collect pre-token balances for wallet-owned accounts only
        for balance in pre_balances {
            // Only include token accounts owned by the wallet
            if let Some(ref owner) = balance.owner {
                if owner == wallet_pubkey_str {
                    let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0) as f64;
                    let decimals = balance.ui_token_amount.decimals;
                    let formatted_amount = amount / 10f64.powi(decimals as i32);
                    *wallet_token_changes.entry(balance.mint.clone()).or_insert(0.0) -= formatted_amount;
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - pre token (wallet-owned): mint={}, amount={}", balance.mint, formatted_amount));
                }
                }
            }
        }
        
        // Collect post-token balances for wallet-owned accounts only
        for balance in post_balances {
            // Only include token accounts owned by the wallet
            if let Some(ref owner) = balance.owner {
                if owner == wallet_pubkey_str {
                    let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0) as f64;
                    let decimals = balance.ui_token_amount.decimals;
                    let formatted_amount = amount / 10f64.powi(decimals as i32);
                    *wallet_token_changes.entry(balance.mint.clone()).or_insert(0.0) += formatted_amount;
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - post token (wallet-owned): mint={}, amount={}", balance.mint, formatted_amount));
                }
                }
            }
        }
        
        // Look for meaningful token changes (positive = gained, negative = lost)
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - analyzing {} total token changes", wallet_token_changes.len()));
            for (mint, change) in &wallet_token_changes {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - token change candidate: mint={}, change={}, significant={}", 
                    mint, change, change.abs() > 0.0001 && *mint != "So11111111111111111111111111111111111111112"));
            }
        }
        
        let significant_changes: Vec<_> = wallet_token_changes.iter()
            .filter(|(mint, &change)| change.abs() > 0.0001 && *mint != "So11111111111111111111111111111111111111112") // Ignore SOL token and dust
            .collect();
        
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - found {} significant token changes", significant_changes.len()));
            
            for (mint, change) in &significant_changes {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap - token change: mint={}, change={}", mint, change));
            }
        }
        
        // If we have token changes and SOL changes, this is likely a swap
        if !significant_changes.is_empty() && sol_change.abs() > 0.001 {
            // Determine if this is a buy or sell based on SOL change
            let is_buy = sol_change < 0.0; // SOL decreased = buying tokens
            
            // Find the main token being traded (largest absolute change)
            let main_token = significant_changes.iter()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(mint, _)| mint.clone())?;
            
            let token_change = *significant_changes.iter()
                .find(|(mint, _)| mint.as_str() == main_token)?
                .1;
            
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Jupiter swap detected: {} {} tokens, SOL change: {}", 
                    if is_buy { "BUY" } else { "SELL" }, main_token, sol_change));
            }
            
            // Try to extract actual swap amounts from inner instructions
            let (actual_sol_amount, actual_token_amount, actual_price) = 
                if let Some(json_data) = transaction_json {
                    if let Some((inner_sol, inner_token)) = extract_actual_swap_amounts_from_json(json_data, &main_token) {
                        let price = if inner_token > 0.0 { inner_sol / inner_token } else { 0.0 };
                        if is_debug_transactions_enabled() {
                            log(LogTag::Transactions, "DEBUG", &format!("📊 Using actual swap amounts: {} SOL ↔ {} tokens, price: {} SOL/token", 
                                inner_sol, inner_token, price));
                        }
                        (inner_sol, inner_token, price)
                    } else {
                        // Fallback to balance change method
                        let fallback_price = if token_change != 0.0 { sol_change.abs() / token_change.abs() } else { 0.0 };
                        if is_debug_transactions_enabled() {
                            log(LogTag::Transactions, "DEBUG", &format!("📊 Using fallback balance change method: {} SOL ↔ {} tokens, price: {} SOL/token", 
                                sol_change.abs(), token_change.abs(), fallback_price));
                        }
                        (sol_change.abs(), token_change.abs(), fallback_price)
                    }
                } else {
                    // Fallback to balance change method when no JSON data available
                    let fallback_price = if token_change != 0.0 { sol_change.abs() / token_change.abs() } else { 0.0 };
                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "DEBUG", &format!("📊 Using fallback balance change method (no JSON): {} SOL ↔ {} tokens, price: {} SOL/token", 
                            sol_change.abs(), token_change.abs(), fallback_price));
                    }
                    (sol_change.abs(), token_change.abs(), fallback_price)
                };
            
            let swap_info = BasicSwapInfo {
                swap_type: if is_buy { "BUY".to_string() } else { "SELL".to_string() },
                token_mint: main_token.to_string(),
                sol_amount: actual_sol_amount,
                token_amount: (actual_token_amount * 1_000_000.0) as u64, // Convert to token base units
                effective_price: actual_price,
                fees_paid: lamports_to_sol(meta.fee), // Convert lamports to SOL
            };
            
            return Some((true, swap_info)); // Always return true since we detected a swap (whether buy or sell)
        }
    }
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "📊 Jupiter swap - no significant changes detected");
    }
    None
}

async fn analyze_pumpfun_swap(
    transaction: &TransactionDetails,
    meta: &TransactionMeta,
    wallet_address: &str,
) -> Option<(bool, BasicSwapInfo)> {
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "🔍 Analyzing Pump.fun swap transaction");
    }
    
    // Parse the message to get account keys
    let account_keys_value = transaction.transaction.message.get("accountKeys")?;
    let account_keys_array = account_keys_value.as_array()?;
    
    // Convert to pubkeys and find wallet index
    let mut account_keys = Vec::new();
    for key_value in account_keys_array {
        // Handle both string format and object format
        let key_str = if let Some(key_str) = key_value.as_str() {
            // Direct string format
            key_str
        } else if let Some(key_str) = key_value.get("pubkey").and_then(|k| k.as_str()) {
            // Object format with pubkey field
            key_str
        } else {
            continue;
        };
        
        if let Ok(pubkey) = Pubkey::from_str(key_str) {
            account_keys.push(pubkey);
        }
    }
    
    let wallet_pubkey = Pubkey::from_str(wallet_address).ok()?;
    let wallet_index = account_keys.iter().position(|key| *key == wallet_pubkey)?;
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap - wallet index: {}", wallet_index));
    }
    
    // Analyze SOL balance changes
    let sol_change = if wallet_index < meta.pre_balances.len() && wallet_index < meta.post_balances.len() {
        let pre_balance = meta.pre_balances[wallet_index] as i64;
        let post_balance = meta.post_balances[wallet_index] as i64;
        lamports_to_sol((post_balance - pre_balance).abs() as u64) * if post_balance > pre_balance { 1.0 } else { -1.0 }
    } else {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "📊 Pump.fun swap - cannot find wallet SOL balances");
        }
        return None;
    };
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap - SOL change: {} SOL", sol_change));
    }
    
    // Look for token balance changes involving wallet
    if let (Some(pre_balances), Some(post_balances)) = (&meta.pre_token_balances, &meta.post_token_balances) {
        let mut wallet_token_changes = HashMap::new();
        let wallet_pubkey_str = wallet_address;
        
        // Collect pre-token balances for wallet-owned accounts only
        for balance in pre_balances {
            if let Some(ref owner) = balance.owner {
                if owner == wallet_pubkey_str {
                    let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0) as f64;
                    let decimals = balance.ui_token_amount.decimals;
                    let formatted_amount = amount / 10f64.powi(decimals as i32);
                    *wallet_token_changes.entry(balance.mint.clone()).or_insert(0.0) -= formatted_amount;
                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap - pre token (wallet-owned): mint={}, amount={}", balance.mint, formatted_amount));
                    }
                }
            }
        }
        
        // Collect post-token balances for wallet-owned accounts only
        for balance in post_balances {
            if let Some(ref owner) = balance.owner {
                if owner == wallet_pubkey_str {
                    let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0) as f64;
                    let decimals = balance.ui_token_amount.decimals;
                    let formatted_amount = amount / 10f64.powi(decimals as i32);
                    *wallet_token_changes.entry(balance.mint.clone()).or_insert(0.0) += formatted_amount;
                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap - post token (wallet-owned): mint={}, amount={}", balance.mint, formatted_amount));
                    }
                }
            }
        }
        
        // Look for meaningful token changes (positive = gained, negative = lost)
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap - analyzing {} total token changes", wallet_token_changes.len()));
            for (mint, change) in &wallet_token_changes {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap - token change candidate: mint={}, change={}, significant={}", 
                    mint, change, change.abs() > 0.0001 && *mint != "So11111111111111111111111111111111111111112"));
            }
        }
        
        let significant_changes: Vec<_> = wallet_token_changes.iter()
            .filter(|(mint, &change)| change.abs() > 0.0001 && *mint != "So11111111111111111111111111111111111111112") // Ignore SOL token and dust
            .collect();
        
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap - found {} significant token changes", significant_changes.len()));
            
            for (mint, change) in &significant_changes {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap - token change: mint={}, change={}", mint, change));
            }
        }
        
        // If we have token changes and SOL changes, this is likely a swap
        if !significant_changes.is_empty() && sol_change.abs() > 0.001 {
            // Determine if this is a buy or sell based on SOL change
            let is_buy = sol_change < 0.0; // SOL decreased = buying tokens
            
            // Find the main token being traded (largest absolute change)
            let main_token = significant_changes.iter()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(mint, _)| mint.clone())?;
            
            let token_change = *significant_changes.iter()
                .find(|(mint, _)| mint.as_str() == main_token)?
                .1;
            
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap detected: {} {} tokens, SOL change: {}", 
                    if is_buy { "BUY" } else { "SELL" }, main_token, sol_change));
            }
            
            // For Pump.fun, use balance change method as it's simpler than Jupiter's inner instructions
            let actual_sol_amount = sol_change.abs();
            let actual_token_amount = token_change.abs();
            let actual_price = if actual_token_amount > 0.0 { actual_sol_amount / actual_token_amount } else { 0.0 };
            
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Pump.fun swap amounts: {} SOL ↔ {} tokens, price: {} SOL/token", 
                    actual_sol_amount, actual_token_amount, actual_price));
            }
            
            let swap_info = BasicSwapInfo {
                swap_type: if is_buy { "BUY".to_string() } else { "SELL".to_string() },
                token_mint: main_token.to_string(),
                sol_amount: actual_sol_amount,
                token_amount: (actual_token_amount * 1_000_000.0) as u64, // Convert to token base units
                effective_price: actual_price,
                fees_paid: lamports_to_sol(meta.fee), // Convert lamports to SOL
            };
            
            return Some((true, swap_info)); // Always return true since we detected a swap
        }
    }
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "📊 Pump.fun swap - no significant changes detected");
    }
    None
}



pub async fn get_all_configured_wallets() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut wallets = Vec::new();
    
    // Try to get wallets from wallets manager
    if let Ok(wallets_manager) = WalletsManager::new() {
        let wallet_infos = wallets_manager.get_all_wallets().await?;
        for wallet_info in wallet_infos {
            wallets.push(wallet_info.public_key);
        }
    }
    
    // If no wallets found, try to get from configs
    if wallets.is_empty() {
        let configs = read_configs()?;
        if !configs.main_wallet_private.is_empty() {
            // Derive wallet address from private key
            if let Ok(private_key_bytes) = bs58::decode(&configs.main_wallet_private).into_vec() {
                if let Ok(keypair) = solana_sdk::signature::Keypair::from_bytes(&private_key_bytes) {
                    wallets.push(keypair.pubkey().to_string());
                }
            }
        }
    }
    
    Ok(wallets)
}

pub async fn analyze_wallet_swaps(
    wallet_address: &str,
    args: &Args,
) -> Result<WalletSwapReport, Box<dyn std::error::Error>> {
    log(LogTag::Transactions, "INFO", &format!("🔄 Fetching transactions for wallet {}", &wallet_address[..8]));
    
    // Get all transactions for this wallet
    let transactions = fetch_wallet_transactions(wallet_address, args.limit).await?;
    
    log(LogTag::Transactions, "INFO", &format!("📊 Analyzing {} transactions for swaps", transactions.len()));
    
    let mut detailed_swaps = Vec::new();
    let mut processed = 0;
    
    // Apply time filter if specified
    let _cutoff_time = if let Some(days) = args.days {
        Some(Utc::now() - chrono::Duration::days(days as i64))
    } else {
        None
    };
    
    for transaction in &transactions {
        processed += 1;
        
        if processed % 100 == 0 {
            log(LogTag::Transactions, "INFO", &format!("📊 Processed {}/{} transactions...", processed, transactions.len()));
        }
        
        // Apply time filter - skip since TransactionDetails doesn't have block_time
        // if let Some(cutoff) = cutoff_time {
        //     // Would need to get block_time from another source
        // }
        
        // Analyze transaction for swap patterns
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("🔍 Analyzing transaction with {} signatures", transaction.transaction.signatures.len()));
        }
        if let Some(swap) = analyze_transaction_for_detailed_swap(transaction, wallet_address).await {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("✅ Swap detected: {} {}", swap.swap_type, swap.token_symbol));
            }
            // Apply filters
            if let Some(ref token_filter) = args.token {
                if swap.token_mint != *token_filter {
                    continue;
                }
            }
            
            if let Some(ref type_filter) = args.swap_type {
                if swap.swap_type != *type_filter {
                    continue;
                }
            }
            
            if args.detailed {
                println!("\n🔍 Swap Found:");
                display_detailed_swap(&swap);
            }
            
            detailed_swaps.push(swap);
        }
    }
    
    // Generate analytics
    let analytics = generate_swap_analytics(&detailed_swaps);
    
    log(LogTag::Transactions, "SUCCESS", &format!(
        "✅ Wallet analysis complete: {} swaps found from {} transactions", 
        detailed_swaps.len(), 
        transactions.len()
    ));
    
    Ok(WalletSwapReport {
        wallet_address: wallet_address.to_string(),
        analytics,
        detailed_swaps,
    })
}

async fn fetch_wallet_transactions(
    wallet_address: &str,
    limit: usize,
) -> Result<Vec<TransactionDetails>, Box<dyn std::error::Error>> {
    let _rpc_client = get_rpc_client();
    
    log(LogTag::Transactions, "INFO", "📥 Loading transaction files from data/transactions/...");
    
    // Read all JSON files from data/transactions directory
    let transactions_dir = Path::new("data/transactions");
    if !transactions_dir.exists() {
        log(LogTag::Transactions, "WARN", "No transactions directory found at data/transactions/");
        return Ok(Vec::new());
    }
    
    let mut transactions = Vec::new();
    let mut files_processed = 0;
    let mut processed_signatures = std::collections::HashSet::new();
    
    for entry in fs::read_dir(transactions_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            files_processed += 1;
            
            if files_processed % 10 == 0 {
                log(LogTag::Transactions, "INFO", &format!("📊 Processed {} transaction files...", files_processed));
            }
            
            match load_transaction_from_file(&path).await {
                Ok(Some(transaction)) => {
                    // Check for duplicate signatures first
                    let signature = transaction.transaction.signatures.get(0)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("no_signature_{}", files_processed));
                    
                    if processed_signatures.contains(&signature) {
                        if is_debug_transactions_enabled() {
                            log(LogTag::Transactions, "DEBUG", &format!("📊 Skipping duplicate transaction: {}", signature));
                        }
                        continue;
                    }
                    
                    // Check if this transaction involves the target wallet
                    let involves_wallet = transaction_involves_wallet(&transaction, wallet_address);
                    if is_debug_transactions_enabled() {
                        log(LogTag::Transactions, "DEBUG", &format!("📊 Transaction in {:?} involves wallet {}: {}", 
                            path.file_name().unwrap_or_default(), wallet_address, involves_wallet));
                    }
                    if involves_wallet {
                        processed_signatures.insert(signature.clone());
                        transactions.push(transaction);
                        
                        if transactions.len() >= limit {
                            break;
                        }
                    }
                },
                Ok(None) => {
                    // File was loaded but doesn't contain valid transaction data
                    continue;
                },
                Err(e) => {
                    log(LogTag::Transactions, "WARN", &format!(
                        "Failed to load transaction from {}: {}", 
                        path.display(), 
                        e
                    ));
                }
            }
        }
    }
    
    log(LogTag::Transactions, "INFO", &format!("� Simulating transaction fetch for wallet {}", &wallet_address[..8]));
    
    // For demonstration, we'll return an empty list
    // In a real implementation, you would use the Solana RPC to get signatures first
    
    log(LogTag::Transactions, "SUCCESS", &format!("✅ Fetched {} transactions", transactions.len()));
    
    Ok(transactions)
}

async fn analyze_transaction_for_detailed_swap(
    transaction: &TransactionDetails,
    wallet_address: &str,
) -> Option<DetailedSwapTransaction> {
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Analyzing transaction for wallet {}", &wallet_address[..8]));
    }
    
    let meta = transaction.meta.as_ref()?;
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Transaction has meta: success={}", meta.err.is_none()));
    }
    
    // Check if transaction was successful
    if meta.err.is_some() {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "❌ Transaction failed, skipping");
        }
        return None;
    }
    
    // Get basic transaction info
    let signature = transaction.transaction.signatures.get(0)?.clone();
    let slot = transaction.slot;
    let block_time = None; // TransactionDetails doesn't have block_time in our current implementation
    let date_time = block_time.and_then(|t| {
        DateTime::from_timestamp(t, 0).map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
    });
    
    // Analyze for swap patterns
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "🔍 Calling detect_swap_from_transaction");
    }
    let (swap_detected, swap_info) = detect_swap_from_transaction(transaction, meta, wallet_address).await?;
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("🔍 Swap detection result: {}", swap_detected));
    }
    
    if !swap_detected {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "❌ No swap detected in transaction");
        }
        return None;
    }
    
    // Extract additional details
    let router_info = (Some("Unknown".to_string()), Some("Unknown".to_string())); // Removed identify_swap_router - use transactions_detector.rs instead
    let (instructions_count, ata_created, ata_closed) = analyze_instruction_patterns(transaction, meta);
    let priority_fee = extract_priority_fee(transaction);
    let compute_units = extract_compute_units(transaction);
    
    // Get token balance changes
    let (pre_balance, post_balance, balance_change) = get_token_balance_changes(
        meta, 
        &swap_info.token_mint, 
        wallet_address
    );
    
    // Get token information from database
    let (token_symbol, token_name, token_decimals) = get_token_info_safe(&swap_info.token_mint).await;
    
    let formatted_token_amount = swap_info.token_amount as f64 / 10f64.powi(token_decimals as i32);
    let price_per_token = if formatted_token_amount > 0.0 {
        swap_info.sol_amount / formatted_token_amount
    } else {
        0.0
    };
    
    Some(DetailedSwapTransaction {
        signature,
        slot,
        block_time,
        date_time,
        wallet_address: wallet_address.to_string(),
        swap_type: swap_info.swap_type,
        token_mint: swap_info.token_mint,
        token_symbol,
        token_name,
        sol_amount: swap_info.sol_amount,
        token_amount: swap_info.token_amount,
        token_decimals,
        formatted_token_amount,
        effective_price: swap_info.effective_price,
        price_per_token,
        fees_paid: swap_info.fees_paid,
        success: true,
        router_program: router_info.0,
        router_name: router_info.1,
        instructions_count,
        ata_created,
        ata_closed,
        priority_fee,
        compute_units,
        pre_token_balance: pre_balance,
        post_token_balance: post_balance,
        token_balance_change: balance_change,
    })
}

#[derive(Debug)]
struct BasicSwapInfo {
    swap_type: String,
    token_mint: String,
    sol_amount: f64,
    token_amount: u64,
    effective_price: f64,
    fees_paid: f64,
}

async fn detect_swap_from_transaction(
    transaction: &TransactionDetails,
    meta: &TransactionMeta,
    wallet_address: &str,
) -> Option<(bool, BasicSwapInfo)> {
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "🔍 Starting swap detection");
    }
    
    // Router detection is now handled by transactions_detector.rs - this is legacy code
    let router_name = Some("Unknown".to_string());
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "📊 Router detection moved to transactions_detector.rs");
    }
    
    // For now, test Jupiter analysis on transactions with token balances
    if meta.pre_token_balances.is_some() && meta.post_token_balances.is_some() {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "📊 Transaction has token balances - testing Jupiter analysis");
        }
        if let Some(result) = {
            let json_data = if let Some(signature) = transaction.transaction.signatures.get(0) {
                TRANSACTION_JSON_CACHE.lock().ok().and_then(|cache| cache.get(signature).cloned())
            } else {
                None
            };
            analyze_jupiter_swap(transaction, meta, wallet_address, json_data.as_ref()).await
        } {
            return Some(result);
        }
    }
    
    // If we detect a Jupiter transaction, analyze it differently
    if let Some(ref router) = router_name {
        if router.contains("Jupiter") {
            return {
                let json_data = if let Some(signature) = transaction.transaction.signatures.get(0) {
                    TRANSACTION_JSON_CACHE.lock().ok().and_then(|cache| cache.get(signature).cloned())
                } else {
                    None
                };
                analyze_jupiter_swap(transaction, meta, wallet_address, json_data.as_ref()).await
            };
        } else if router.contains("Pump.fun") {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", "📊 Detected Pump.fun transaction - analyzing with balance changes");
            }
            return analyze_pumpfun_swap(transaction, meta, wallet_address).await;
        }
    }
    
    // Parse the message from JSON to get account keys
    let account_keys_value = transaction.transaction.message.get("accountKeys")?;
    let account_keys_array = account_keys_value.as_array()?;
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Found {} account keys", account_keys_array.len()));
    }
    
    // Convert to pubkeys
    let mut account_keys = Vec::new();
    for key_value in account_keys_array {
        if let Some(key_str) = key_value.get("pubkey").and_then(|k| k.as_str()) {
            if let Ok(pubkey) = Pubkey::from_str(key_str) {
                account_keys.push(pubkey);
            }
        }
    }
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Converted {} pubkeys", account_keys.len()));
    }
    
    // Find wallet account index
    let wallet_pubkey = Pubkey::from_str(wallet_address).ok()?;
    let wallet_index = account_keys.iter().position(|key| *key == wallet_pubkey)?;
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Wallet index: {}", wallet_index));
    }
    
    // Analyze SOL balance changes
    let sol_change = if wallet_index < meta.pre_balances.len() && wallet_index < meta.post_balances.len() {
        let pre_balance = meta.pre_balances[wallet_index] as i64;
        let post_balance = meta.post_balances[wallet_index] as i64;
        lamports_to_sol((post_balance - pre_balance).abs() as u64) * if post_balance > pre_balance { 1.0 } else { -1.0 }
    } else {
        return None;
    };
    
    // Analyze token balance changes
    let token_changes = analyze_token_balance_changes(meta, wallet_index);
    
    // Look for swap patterns: SOL decrease + token increase = buy, SOL increase + token decrease = sell
    for (mint, token_change) in token_changes {
        if token_change.abs() < 1.0 { // Ignore dust changes
            continue;
        }
        
        let (swap_type, sol_amount, token_amount, effective_price) = if sol_change < -0.001 && token_change > 0.0 {
            // Buy: SOL decreased, tokens increased
            let sol_spent = sol_change.abs();
            let price = if token_change > 0.0 { sol_spent / token_change } else { 0.0 };
            ("BUY".to_string(), sol_spent, token_change as u64, price)
        } else if sol_change > 0.001 && token_change < 0.0 {
            // Sell: SOL increased, tokens decreased
            let sol_received = sol_change;
            let tokens_sold = token_change.abs();
            let price = if tokens_sold > 0.0 { sol_received / tokens_sold } else { 0.0 };
            ("SELL".to_string(), sol_received, tokens_sold as u64, price)
        } else {
            continue;
        };
        
        let fees_paid = lamports_to_sol(meta.fee);
        
        return Some((true, BasicSwapInfo {
            swap_type,
            token_mint: mint,
            sol_amount,
            token_amount,
            effective_price,
            fees_paid,
        }));
    }
    
    None
}

fn analyze_token_balance_changes(
    meta: &TransactionMeta,
    wallet_index: usize,
) -> HashMap<String, f64> {
    let mut changes = HashMap::new();
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Analyzing token balance changes for wallet_index: {}", wallet_index));
        
        log(LogTag::Transactions, "DEBUG", &format!("📊 Has pre_token_balances: {}", meta.pre_token_balances.is_some()));
        log(LogTag::Transactions, "DEBUG", &format!("📊 Has post_token_balances: {}", meta.post_token_balances.is_some()));
    }
    
    if let (Some(pre_balances), Some(post_balances)) = (&meta.pre_token_balances, &meta.post_token_balances) {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Pre token balances: {}, Post token balances: {}", pre_balances.len(), post_balances.len()));
        }
        
        // Create maps for easier lookup
        let mut pre_map = HashMap::new();
        let mut post_map = HashMap::new();
        
        for balance in pre_balances {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Pre balance: account_index={}, mint={}, amount={}", 
                    balance.account_index, balance.mint, balance.ui_token_amount.amount));
            }
            // Remove the wallet_index filter - analyze all token changes in the transaction
            let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0) as f64;
            let decimals = balance.ui_token_amount.decimals;
            let formatted_amount = amount / 10f64.powi(decimals as i32);
            // Accumulate amounts for the same mint
            *pre_map.entry(balance.mint.clone()).or_insert(0.0) += formatted_amount;
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Pre balance accumulated: mint={}, amount={}", balance.mint, formatted_amount));
            }
        }
        
        for balance in post_balances {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Post balance: account_index={}, mint={}, amount={}", 
                    balance.account_index, balance.mint, balance.ui_token_amount.amount));
            }
            // Remove the wallet_index filter - analyze all token changes in the transaction
            let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0) as f64;
            let decimals = balance.ui_token_amount.decimals;
            let formatted_amount = amount / 10f64.powi(decimals as i32);
            // Accumulate amounts for the same mint
            *post_map.entry(balance.mint.clone()).or_insert(0.0) += formatted_amount;
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Post balance accumulated: mint={}, amount={}", balance.mint, formatted_amount));
            }
        }
        
        // Calculate changes for all tokens
        let all_mints: HashSet<_> = pre_map.keys().chain(post_map.keys()).collect();
        
        for mint in all_mints {
            let pre_amount = pre_map.get(mint).copied().unwrap_or(0.0);
            let post_amount = post_map.get(mint).copied().unwrap_or(0.0);
            let change = post_amount - pre_amount;
            
            if change.abs() > 0.000001 { // Filter out dust
                changes.insert(mint.clone(), change);
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "DEBUG", &format!("📊 Token change: mint={}, change={}", mint, change));
                }
            }
        }
    }
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Total token changes found: {}", changes.len()));
    }
    changes
}

fn analyze_instruction_patterns(
    transaction: &TransactionDetails,
    meta: &TransactionMeta,
) -> (usize, bool, bool) {
    // Count instructions from JSON message
    let instructions_count = transaction.transaction.message
        .get("instructions")
        .and_then(|instr| instr.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);
    
    // Check logs for ATA operations
    let mut ata_created = false;
    let mut ata_closed = false;
    
    if let Some(log_messages) = &meta.log_messages {
        for log in log_messages {
            if log.contains("InitializeAccount") || log.contains("CreateIdempotent") {
                ata_created = true;
            }
            if log.contains("CloseAccount") {
                ata_closed = true;
            }
        }
    }
    
    (instructions_count, ata_created, ata_closed)
}

fn extract_priority_fee(transaction: &TransactionDetails) -> Option<f64> {
    // Parse instructions from JSON to look for compute budget instructions
    let instructions_value = transaction.transaction.message.get("instructions")?;
    let instructions_array = instructions_value.as_array()?;
    
    let account_keys_value = transaction.transaction.message.get("accountKeys")?;
    let account_keys_array = account_keys_value.as_array()?;
    
    for instruction in instructions_array {
        if let Some(program_id_index) = instruction.get("programIdIndex").and_then(|p| p.as_u64()) {
            if let Some(program_key_value) = account_keys_array.get(program_id_index as usize) {
                if let Some(program_id) = program_key_value.as_str() {
                    if program_id == "ComputeBudget111111111111111111111111111111" {
                        // Try to decode compute budget instruction data
                        if let Some(data_str) = instruction.get("data").and_then(|d| d.as_str()) {
                            if let Ok(decoded_data) = bs58::decode(data_str).into_vec() {
                                if decoded_data.len() >= 12 {
                                    let instruction_type = u32::from_le_bytes([
                                        decoded_data[0], decoded_data[1], 
                                        decoded_data[2], decoded_data[3]
                                    ]);
                                    
                                    if instruction_type == 2 { // SetComputeUnitPrice
                                        let price = u64::from_le_bytes([
                                            decoded_data[4], decoded_data[5], decoded_data[6], decoded_data[7],
                                            decoded_data[8], decoded_data[9], decoded_data[10], decoded_data[11]
                                        ]);
                                        return Some(price as f64 / 1_000_000.0); // Convert micro-lamports to lamports
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}

fn extract_compute_units(transaction: &TransactionDetails) -> Option<u32> {
    // Parse instructions from JSON to look for compute budget instructions  
    let instructions_value = transaction.transaction.message.get("instructions")?;
    let instructions_array = instructions_value.as_array()?;
    
    let account_keys_value = transaction.transaction.message.get("accountKeys")?;
    let account_keys_array = account_keys_value.as_array()?;
    
    for instruction in instructions_array {
        if let Some(program_id_index) = instruction.get("programIdIndex").and_then(|p| p.as_u64()) {
            if let Some(program_key_value) = account_keys_array.get(program_id_index as usize) {
                if let Some(program_id) = program_key_value.as_str() {
                    if program_id == "ComputeBudget111111111111111111111111111111" {
                        if let Some(data_str) = instruction.get("data").and_then(|d| d.as_str()) {
                            if let Ok(decoded_data) = bs58::decode(data_str).into_vec() {
                                if decoded_data.len() >= 8 {
                                    let instruction_type = u32::from_le_bytes([
                                        decoded_data[0], decoded_data[1], 
                                        decoded_data[2], decoded_data[3]
                                    ]);
                                    
                                    if instruction_type == 3 { // SetComputeUnitLimit
                                        let limit = u32::from_le_bytes([
                                            decoded_data[4], decoded_data[5], 
                                            decoded_data[6], decoded_data[7]
                                        ]);
                                        return Some(limit);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Helper function to extract priority fee as u64
fn extract_priority_fee_u64(transaction: &TransactionDetails) -> Option<u64> {
    if let Some(fee) = extract_priority_fee(transaction) {
        Some(fee as u64)
    } else {
        None
    }
}

/// Helper function to extract block time from transaction
fn extract_block_time(_transaction: &TransactionDetails) -> Option<i64> {
    // TransactionDetails might not have block_time directly
    // This would need to be extracted from the actual transaction data
    // For now, return None as a placeholder
    None
}

fn get_token_balance_changes(
    meta: &TransactionMeta,
    token_mint: &str,
    wallet_address: &str,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    let wallet_pubkey = match Pubkey::from_str(wallet_address) {
        Ok(pk) => pk,
        Err(_) => return (None, None, None),
    };
    
    // Find balances for this token and wallet
    let pre_balance = find_token_balance(&meta.pre_token_balances, token_mint, &wallet_pubkey);
    let post_balance = find_token_balance(&meta.post_token_balances, token_mint, &wallet_pubkey);
    
    let change = match (pre_balance, post_balance) {
        (Some(pre), Some(post)) => Some(post - pre),
        (None, Some(post)) => Some(post),
        (Some(pre), None) => Some(-pre),
        (None, None) => None,
    };
    
    (pre_balance, post_balance, change)
}

fn find_token_balance(
    balances: &Option<Vec<TokenBalance>>,
    token_mint: &str,
    _wallet_pubkey: &Pubkey,
) -> Option<f64> {
    if let Some(balances) = balances {
        for balance in balances {
            if balance.mint == token_mint {
                // Check if this is the wallet's token account (simplified check)
                let amount = balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0) as f64;
                let decimals = balance.ui_token_amount.decimals;
                return Some(amount / 10f64.powi(decimals as i32));
            }
        }
    }
    None
}

async fn get_token_info_safe(mint: &str) -> (String, Option<String>, u8) {
    // Try to get token information from database
    let symbol = if let Ok(db) = TokenDatabase::new() {
        if let Ok(Some(token)) = db.get_token_by_mint(mint) {
            token.symbol
        } else {
            format!("TOKEN_{}", &mint[..8])
        }
    } else {
        format!("TOKEN_{}", &mint[..8])
    };
    
    let name = if let Ok(db) = TokenDatabase::new() {
        if let Ok(Some(token)) = db.get_token_by_mint(mint) {
            Some(token.name)
        } else {
            None
        }
    } else {
        None
    };
    
    let decimals = if let Some(decimals) = get_token_decimals(mint).await {
        decimals
    } else {
        9 // Default to 9 decimals
    };
    
    (symbol, name, decimals)
}

fn generate_swap_analytics(swaps: &[DetailedSwapTransaction]) -> SwapAnalytics {
    if swaps.is_empty() {
        return SwapAnalytics {
            total_swaps: 0,
            buy_swaps: 0,
            sell_swaps: 0,
            unique_tokens: 0,
            total_sol_spent: 0.0,
            total_sol_received: 0.0,
            total_fees_paid: 0.0,
            net_sol_change: 0.0,
            average_swap_size: 0.0,
            most_traded_tokens: Vec::new(),
            router_usage: HashMap::new(),
            success_rate: 0.0,
            time_range: None,
            swaps_by_month: HashMap::new(),
            fee_efficiency: 0.0,
        };
    }
    
    let total_swaps = swaps.len();
    let buy_swaps = swaps.iter().filter(|s| s.swap_type == "BUY").count();
    let sell_swaps = swaps.iter().filter(|s| s.swap_type == "SELL").count();
    
    let unique_tokens = swaps.iter()
        .map(|s| &s.token_mint)
        .collect::<HashSet<_>>()
        .len();
    
    let mut total_sol_spent = 0.0;
    let mut total_sol_received = 0.0;
    let mut total_fees_paid = 0.0;
    let mut token_counts = HashMap::new();
    let mut router_usage = HashMap::new();
    let mut swaps_by_month = HashMap::new();
    
    let mut earliest_time = None;
    let mut latest_time = None;
    
    for swap in swaps {
        // SOL flow
        if swap.swap_type == "BUY" {
            total_sol_spent += swap.sol_amount;
        } else {
            total_sol_received += swap.sol_amount;
        }
        
        total_fees_paid += swap.fees_paid;
        
        // Token counts
        *token_counts.entry(swap.token_symbol.clone()).or_insert(0) += 1;
        
        // Router usage
        if let Some(ref router) = swap.router_name {
            *router_usage.entry(router.clone()).or_insert(0) += 1;
        }
        
        // Time analysis
        if let Some(block_time) = swap.block_time {
            if let Some(dt) = DateTime::from_timestamp(block_time, 0) {
                let month_key = dt.format("%Y-%m").to_string();
                *swaps_by_month.entry(month_key).or_insert(0) += 1;
                
                if earliest_time.is_none() || Some(dt) < earliest_time {
                    earliest_time = Some(dt);
                }
                if latest_time.is_none() || Some(dt) > latest_time {
                    latest_time = Some(dt);
                }
            }
        }
    }
    
    let net_sol_change = total_sol_received - total_sol_spent;
    let total_volume = total_sol_spent + total_sol_received;
    let average_swap_size = if total_swaps > 0 { total_volume / total_swaps as f64 } else { 0.0 };
    let fee_efficiency = if total_volume > 0.0 { (total_fees_paid / total_volume) * 100.0 } else { 0.0 };
    
    // Most traded tokens (top 10)
    let mut token_pairs: Vec<_> = token_counts.into_iter().collect();
    token_pairs.sort_by(|a, b| b.1.cmp(&a.1));
    let most_traded_tokens = token_pairs.into_iter().take(10).collect();
    
    let success_rate = 100.0; // All included swaps are successful
    
    let time_range = match (earliest_time, latest_time) {
        (Some(start), Some(end)) => Some((start, end)),
        _ => None,
    };
    
    SwapAnalytics {
        total_swaps,
        buy_swaps,
        sell_swaps,
        unique_tokens,
        total_sol_spent,
        total_sol_received,
        total_fees_paid,
        net_sol_change,
        average_swap_size,
        most_traded_tokens,
        router_usage,
        success_rate,
        time_range,
        swaps_by_month,
        fee_efficiency,
    }
}

pub fn display_comprehensive_results(
    reports: &[WalletSwapReport],
    _args: &Args,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 COMPREHENSIVE SWAP ANALYSIS RESULTS");
    println!("═══════════════════════════════════════════════════════════════");
    
    // Combined analytics
    let total_swaps: usize = reports.iter().map(|r| r.analytics.total_swaps).sum();
    let total_buy_swaps: usize = reports.iter().map(|r| r.analytics.buy_swaps).sum();
    let total_sell_swaps: usize = reports.iter().map(|r| r.analytics.sell_swaps).sum();
    let total_sol_spent: f64 = reports.iter().map(|r| r.analytics.total_sol_spent).sum();
    let total_sol_received: f64 = reports.iter().map(|r| r.analytics.total_sol_received).sum();
    let total_fees: f64 = reports.iter().map(|r| r.analytics.total_fees_paid).sum();
    let net_sol_change = total_sol_received - total_sol_spent;
    
    println!("📊 OVERALL STATISTICS:");
    println!("  • Total Wallets Analyzed: {}", reports.len());
    println!("  • Total Swaps Found: {}", total_swaps);
    println!("  • Buy Swaps: {} ({:.1}%)", total_buy_swaps, (total_buy_swaps as f64 / total_swaps as f64) * 100.0);
    println!("  • Sell Swaps: {} ({:.1}%)", total_sell_swaps, (total_sell_swaps as f64 / total_swaps as f64) * 100.0);
    println!("  • Total SOL Spent: {:.6} SOL", total_sol_spent);
    println!("  • Total SOL Received: {:.6} SOL", total_sol_received);
    println!("  • Net SOL Change: {:.6} SOL", net_sol_change);
    println!("  • Total Fees Paid: {:.6} SOL", total_fees);
    
    if total_swaps > 0 {
        let avg_swap_size = (total_sol_spent + total_sol_received) / total_swaps as f64;
        println!("  • Average Swap Size: {:.6} SOL", avg_swap_size);
    }
    
    // Per-wallet breakdown
    if reports.len() > 1 {
        println!("\n💼 PER-WALLET BREAKDOWN:");
        for (i, report) in reports.iter().enumerate() {
            println!("  {}. Wallet {} ({}):", 
                i + 1, 
                &report.wallet_address[..8], 
                &report.wallet_address[report.wallet_address.len()-8..]
            );
            println!("     • Swaps: {} (Buy: {}, Sell: {})", 
                report.analytics.total_swaps,
                report.analytics.buy_swaps,
                report.analytics.sell_swaps
            );
            println!("     • SOL Spent: {:.6}, Received: {:.6}, Net: {:.6}", 
                report.analytics.total_sol_spent,
                report.analytics.total_sol_received,
                report.analytics.net_sol_change
            );
            println!("     • Unique Tokens: {}", report.analytics.unique_tokens);
            println!("     • Total Fees: {:.6} SOL", report.analytics.total_fees_paid);
        }
    }
    
    // Combined router usage
    let mut all_routers = HashMap::new();
    for report in reports {
        for (router, count) in &report.analytics.router_usage {
            *all_routers.entry(router.clone()).or_insert(0) += count;
        }
    }
    
    if !all_routers.is_empty() {
        println!("\n🔀 ROUTER USAGE:");
        let mut router_pairs: Vec<_> = all_routers.into_iter().collect();
        router_pairs.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (router, count) in router_pairs {
            let percentage = (count as f64 / total_swaps as f64) * 100.0;
            println!("  • {}: {} swaps ({:.1}%)", router, count, percentage);
        }
    }
    
    // Combined token usage
    let mut all_tokens = HashMap::new();
    for report in reports {
        for (token, count) in &report.analytics.most_traded_tokens {
            *all_tokens.entry(token.clone()).or_insert(0) += count;
        }
    }
    
    if !all_tokens.is_empty() {
        println!("\n🏷️  MOST TRADED TOKENS:");
        let mut token_pairs: Vec<_> = all_tokens.into_iter().collect();
        token_pairs.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (i, (token, count)) in token_pairs.iter().take(10).enumerate() {
            let percentage = (*count as f64 / total_swaps as f64) * 100.0;
            println!("  {}. {}: {} swaps ({:.1}%)", i + 1, token, count, percentage);
        }
    }
    
    println!("\n═══════════════════════════════════════════════════════════════");
    
    Ok(())
}

fn display_detailed_swap(swap: &DetailedSwapTransaction) {
    println!("  📝 Signature: {}", &swap.signature[..16]);
    println!("  📅 Date: {}", swap.date_time.as_deref().unwrap_or("N/A"));
    println!("  💱 Type: {} {} for {:.6} SOL", 
        swap.swap_type.to_uppercase(), 
        swap.token_symbol, 
        swap.sol_amount
    );
    println!("  🪙 Amount: {:.6} tokens (raw: {})", 
        swap.formatted_token_amount, 
        swap.token_amount
    );
    println!("  💰 Price: {:.9} SOL per token", swap.price_per_token);
    println!("  💸 Fees: {:.6} SOL", swap.fees_paid);
    
    if let Some(ref router) = swap.router_name {
        println!("  🔀 Router: {}", router);
    }
    
    if let Some(priority_fee) = swap.priority_fee {
        println!("  ⚡ Priority Fee: {:.6} SOL", priority_fee);
    }
    
    if swap.ata_created {
        println!("  🆕 ATA Created");
    }
    if swap.ata_closed {
        println!("  🗑️ ATA Closed");
    }
}

pub fn export_results_to_json(
    reports: &[WalletSwapReport],
    _args: &Args,
) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("swap_analysis_{}.json", timestamp);
    let filepath = Path::new("data").join(&filename);
    
    // Ensure data directory exists
    if let Some(parent) = filepath.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Create export data
    let export_data = serde_json::json!({
        "export_timestamp": Utc::now(),
        "total_wallets": reports.len(),
        "total_swaps": reports.iter().map(|r| r.analytics.total_swaps).sum::<usize>(),
        "reports": reports
    });
    
    // Write to file
    fs::write(&filepath, serde_json::to_string_pretty(&export_data)?)?;
    
    println!("\n💾 EXPORT COMPLETE:");
    println!("  📁 File: {}", filepath.display());
    println!("  📊 Data: {} wallet reports with detailed swap information", reports.len());
    
    log(LogTag::Transactions, "SUCCESS", &format!("Exported swap analysis to: {}", filepath.display()));
    
    Ok(())
}

// Helper function to load a transaction from a JSON file
async fn load_transaction_from_file(file_path: &Path) -> Result<Option<TransactionDetails>, Box<dyn std::error::Error>> {
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Loading transaction from file: {:?}", file_path));
    }
    let content = fs::read_to_string(file_path)?;
    let json_data: serde_json::Value = serde_json::from_str(&content)?;
    
    // Extract the transaction_data field from the JSON
    if let Some(transaction_data) = json_data.get("transaction_data") {
        // Convert to TransactionDetails format
        if let Ok(transaction_details) = convert_json_to_transaction_details(transaction_data) {
            // Store the raw JSON data in the cache using the signature as key
            if let Some(signature) = transaction_details.transaction.signatures.get(0) {
                if let Ok(mut cache) = TRANSACTION_JSON_CACHE.lock() {
                    cache.insert(signature.clone(), transaction_data.clone());
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "DEBUG", &format!("📊 Cached raw JSON for transaction: {}", signature));
                }
                }
            }
            return Ok(Some(transaction_details));
        }
    }
    
    Ok(None)
}

// Helper function to convert JSON data to TransactionDetails
fn convert_json_to_transaction_details(data: &serde_json::Value) -> Result<TransactionDetails, Box<dyn std::error::Error>> {
    let slot = data.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
    
    // Extract transaction data
    let transaction_json = data.get("transaction").ok_or("Missing transaction field")?;
    
    // Extract meta data if present
    let meta = if let Some(meta_json) = data.get("meta") {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Found meta field with keys: {:?}", 
                meta_json.as_object().map(|obj| obj.keys().collect::<Vec<_>>()).unwrap_or_default()));
        }
        Some(convert_meta_from_json(meta_json)?)
    } else {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "📊 No meta field found in transaction data");
        }
        None
    };
    
    // Create TransactionData
    let transaction_data = TransactionData {
        message: transaction_json.get("message").unwrap_or(&serde_json::Value::Null).clone(),
        signatures: transaction_json.get("signatures")
            .and_then(|s| s.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
    };
    
    Ok(TransactionDetails {
        slot,
        transaction: transaction_data,
        meta,
    })
}

// Helper function to convert meta JSON to TransactionMeta
fn convert_meta_from_json(meta_json: &serde_json::Value) -> Result<TransactionMeta, Box<dyn std::error::Error>> {
    // Handle err field properly: JSON null should become None
    let err = match meta_json.get("err") {
        Some(serde_json::Value::Null) => None,
        Some(other) => Some(other.clone()),
        None => None,
    };
    
    let pre_balances = meta_json.get("preBalances")
        .and_then(|b| b.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();
        
    let post_balances = meta_json.get("postBalances")
        .and_then(|b| b.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();
    
    let fee = meta_json.get("fee").and_then(|f| f.as_u64()).unwrap_or(0);
    
    let log_messages = meta_json.get("logMessages")
        .and_then(|l| l.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());
    
    // Convert token balances
    let pre_token_balances_result = convert_token_balances(meta_json.get("preTokenBalances"));
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Pre token balances conversion result: {:?}", 
            pre_token_balances_result.as_ref().map(|v| v.len()).unwrap_or(0)));
    }
    
    let post_token_balances_result = convert_token_balances(meta_json.get("postTokenBalances"));
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 Post token balances conversion result: {:?}", 
            post_token_balances_result.as_ref().map(|v| v.len()).unwrap_or(0)));
    }
    
    Ok(TransactionMeta {
        err,
        pre_balances,
        post_balances,
        pre_token_balances: pre_token_balances_result,
        post_token_balances: post_token_balances_result,
        fee,
        log_messages,
    })
}

// Helper function to convert token balance arrays
fn convert_token_balances(balances_json: Option<&serde_json::Value>) -> Option<Vec<TokenBalance>> {
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!("📊 convert_token_balances called with: {:?}", 
            balances_json.map(|v| format!("is_array={}", v.is_array()))));
    }
    
    if let Some(balances_array) = balances_json.and_then(|b| b.as_array()) {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!("📊 Converting {} token balances", balances_array.len()));
        }
        let mut balances = Vec::new();
        
        for balance_json in balances_array {
            if let Ok(balance) = convert_single_token_balance(balance_json) {
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "DEBUG", &format!("📊 Converted token balance for mint: {}", balance.mint));
                }
                balances.push(balance);
            } else if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", "📊 Failed to convert a token balance");
            }
        }
        
        if !balances.is_empty() {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!("📊 Successfully converted {} token balances", balances.len()));
            }
            return Some(balances);
        } else if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", "📊 No token balances converted successfully");
        }
    } else if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", "📊 No token balances array found in JSON");
    }
    None
}

// Helper function to convert a single token balance
fn convert_single_token_balance(balance_json: &serde_json::Value) -> Result<TokenBalance, Box<dyn std::error::Error>> {
    let account_index = balance_json.get("accountIndex").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
    let mint = balance_json.get("mint").and_then(|m| m.as_str()).unwrap_or("").to_string();
    let owner = balance_json.get("owner").and_then(|o| o.as_str()).map(|s| s.to_string());
    let program_id = balance_json.get("programId").and_then(|p| p.as_str()).map(|s| s.to_string());
    
    // Convert UI token amount
    let ui_token_amount = if let Some(ui_amount_json) = balance_json.get("uiTokenAmount") {
        UiTokenAmount {
            amount: ui_amount_json.get("amount").and_then(|a| a.as_str()).unwrap_or("0").to_string(),
            decimals: ui_amount_json.get("decimals").and_then(|d| d.as_u64()).unwrap_or(0) as u8,
            ui_amount: ui_amount_json.get("uiAmount").and_then(|u| u.as_f64()),
            ui_amount_string: Some(ui_amount_json.get("uiAmountString").and_then(|s| s.as_str()).unwrap_or("0").to_string()),
        }
    } else {
        UiTokenAmount {
            amount: "0".to_string(),
            decimals: 0,
            ui_amount: None,
            ui_amount_string: Some("0".to_string()),
        }
    };
    
    Ok(TokenBalance {
        account_index,
        mint,
        owner,
        program_id,
        ui_token_amount,
    })
}

// Helper function to check if a transaction involves a specific wallet
fn transaction_involves_wallet(transaction: &TransactionDetails, wallet_address: &str) -> bool {
    // Check if wallet is in account keys
    if let Some(account_keys) = transaction.transaction.message.get("accountKeys") {
        if let Some(keys_array) = account_keys.as_array() {
            for key_obj in keys_array {
                if let Some(pubkey) = key_obj.get("pubkey").and_then(|k| k.as_str()) {
                    if pubkey == wallet_address {
                        return true;
                    }
                }
            }
        }
    }
    
    // Also check in signatures (though this is less reliable for finding the specific wallet)
    for signature in &transaction.transaction.signatures {
        if signature.len() > 0 {
            // Could add more sophisticated wallet matching here
            // For now, we'll rely on account keys check above
        }
    }
    
    false
}

/// Analyze a swap transaction after execution using only the signature
/// This replaces the complex verify_swap_transaction approach with simple signature-based analysis
/// Returns effective price, fees, and all swap details
/// Simplified post-swap transaction analysis using only signature and wallet address
/// This is useful when we don't know the exact input/output mints beforehand
pub async fn analyze_post_swap_transaction_simple(
    signature: &str,
    wallet_address: &str,
) -> Result<PostSwapAnalysis, String> {
    use crate::rpc::get_rpc_client;
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "DEBUG", &format!(
            "📊 Starting simple post-swap analysis for: {}", 
            &signature[..8]
        ));
    }
    
    // Use wallet transaction manager for cached transaction access
    use crate::transactions_manager::get_transaction_details_global;
    let transaction = get_transaction_details_global(signature).await
        .map_err(|e| format!("Failed to fetch transaction: {}", e))?;
    
    // Analyze the transaction for swap information
    if let Some(meta) = &transaction.meta {
        if let Some((success, swap_info)) = detect_swap_from_transaction(
            &transaction, 
            meta, 
            wallet_address
        ).await {
            // Use the swap_type from detect_swap_from_transaction instead of re-determining
            let direction = swap_info.swap_type.to_lowercase();
            
            let effective_price = if swap_info.token_amount > 0 {
                swap_info.sol_amount.abs() / (swap_info.token_amount as f64)
            } else {
                0.0
            };
            
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "DEBUG", &format!(
                    "✅ Simple post-swap analysis complete: direction={}, price={:.10} SOL/token, fees={:.6} SOL",
                    direction, effective_price, swap_info.fees_paid
                ));
            }
            
            Ok(PostSwapAnalysis {
                signature: signature.to_string(),
                effective_price,
                sol_amount: swap_info.sol_amount.abs(),
                token_amount: swap_info.token_amount as f64,
                transaction_fee: Some(meta.fee),
                priority_fee: None, // Simplified - can be enhanced later
                slot: Some(transaction.slot),
                block_time: None, // TransactionDetails doesn't have block_time field
                success,
                token_mint: Some(swap_info.token_mint.clone()),
                token_decimals: None, // Not available in BasicSwapInfo
                fees_paid: swap_info.fees_paid,
                direction: direction.to_string(),
                ata_rent_reclaimed: None, // Not available in BasicSwapInfo
                ata_created: false, // Default value
                ata_closed: false, // Default value  
                router_name: Some("Unknown".to_string()), // Default value
            })
        } else {
            Err("No swap detected in transaction".to_string())
        }
    } else {
        Err("No transaction metadata found".to_string())
    }
}

pub async fn analyze_post_swap_transaction(
    signature: &str,
    wallet_address: &str,
    input_mint: &str,
    output_mint: &str,
    direction: &str, // "buy" or "sell"
) -> Result<PostSwapAnalysis, String> {
    log(LogTag::Transactions, "POST_SWAP", &format!(
        "📊 Analyzing post-swap transaction: {} ({})", 
        &signature[..8], direction
    ));
    
    // Use wallet transaction manager for cached transaction access
    use crate::transactions_manager::get_transaction_details_global;
    let transaction = get_transaction_details_global(signature).await
        .map_err(|e| format!("Failed to fetch transaction: {}", e))?;
    
    // Analyze the transaction for swap information
    if let Some(meta) = &transaction.meta {
        if let Some((success, swap_info)) = detect_swap_from_transaction(
            &transaction, 
            meta, 
            wallet_address
        ).await {
            let effective_price = if direction == "buy" {
                // Buy: SOL -> Token, calculate SOL per token
                if swap_info.token_amount > 0 {
                    swap_info.sol_amount / (swap_info.token_amount as f64)
                } else {
                    0.0
                }
            } else {
                // Sell: Token -> SOL, calculate SOL per token
                if swap_info.token_amount > 0 {
                    swap_info.sol_amount / (swap_info.token_amount as f64)
                } else {
                    0.0
                }
            };
            
            log(LogTag::Transactions, "POST_SWAP", &format!(
                "✅ Post-swap analysis complete: price={:.10} SOL/token, fees={:.6} SOL",
                effective_price, swap_info.fees_paid
            ));
            
            Ok(PostSwapAnalysis {
                signature: signature.to_string(),
                effective_price,
                sol_amount: swap_info.sol_amount,
                token_amount: swap_info.token_amount as f64,
                fees_paid: swap_info.fees_paid,
                ata_created: false, // BasicSwapInfo doesn't have this field
                ata_closed: false, // BasicSwapInfo doesn't have this field  
                router_name: Some("Unknown".to_string()), // BasicSwapInfo doesn't have this field
                success,
                transaction_fee: Some(meta.fee),
                priority_fee: None, // Simplified - can be enhanced later
                slot: Some(transaction.slot),
                block_time: None, // TransactionDetails doesn't have block_time field
                token_mint: Some(swap_info.token_mint.clone()),
                token_decimals: None, // Not available in BasicSwapInfo
                direction: direction.to_string(),
                ata_rent_reclaimed: None, // Not available in BasicSwapInfo
            })
        } else {
            Err("No swap data found in transaction".to_string())
        }
    } else {
        Err("No transaction metadata available".to_string())
    }
}

/// Result of post-swap transaction analysis
#[derive(Debug, Clone, Serialize)]
pub struct PostSwapAnalysis {
    pub signature: String,
    pub effective_price: f64,
    pub sol_amount: f64,
    pub token_amount: f64,
    pub fees_paid: f64,
    pub ata_created: bool,
    pub ata_closed: bool,
    pub router_name: Option<String>,
    pub success: bool,
    pub transaction_fee: Option<u64>,
    pub priority_fee: Option<u64>,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub token_mint: Option<String>,
    pub token_decimals: Option<u8>,
    pub direction: String,
    pub ata_rent_reclaimed: Option<f64>,
}
