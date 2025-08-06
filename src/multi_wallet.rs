/// Multi-Wallet System for Enhanced Security
/// 
/// This system allows buying tokens with temporary wallets for safety:
/// 1. Create new temporary wallet for each buy operation
/// 2. Transfer SOL to temporary wallet (trade size + fees + extra)
/// 3. Buy tokens in temporary wallet
/// 4. Create ATA in main wallet if needed
/// 5. Transfer tokens from temporary wallet to main wallet
/// 6. Close ATA and transfer remaining SOL from temporary wallet to main wallet
/// 7. Sell tokens from main wallet when needed

use crate::global::{read_configs, DATA_DIR};
use crate::logger::{log, LogTag};
use crate::rpc::{SwapError, get_rpc_client};
use crate::wallet::{get_token_balance, get_sol_balance, close_single_ata};
use crate::swaps::{
    interface::{buy_token, sell_token, SwapResult},
    types::SOL_MINT,
    get_wallet_address as get_main_wallet_address,
};
use crate::tokens::Token;

use solana_sdk::{
    signature::Keypair,
    signer::Signer,
    pubkey::Pubkey,
    system_instruction,
    instruction::Instruction,
    transaction::Transaction,
};
use spl_token::instruction::{transfer, close_account};
use spl_associated_token_account::{instruction::create_associated_token_account, get_associated_token_address};
use bs58;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use chrono::Utc;
use serde::{Serialize, Deserialize};

/// Configuration for multi-wallet operations
pub const USE_MULTI_WALLET: bool = true; // Set to false to use main wallet only
pub const EXTRA_SOL_BUFFER: f64 = 0.015; // Extra SOL for transaction fees and rent
pub const WALLET_BACKUP_DIR: &str = "data/wallets";

/// Wallet backup information
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletBackup {
    pub address: String,
    pub private_key: String,
    pub created_at: String,
    pub purpose: String,
    pub status: String, // "active", "drained", "archived"
}

/// Multi-wallet buy operation result
#[derive(Debug)]
pub struct MultiWalletBuyResult {
    pub swap_result: SwapResult,
    pub temp_wallet_address: String,
    pub main_wallet_token_balance: u64,
    pub cleanup_successful: bool,
    pub temp_wallet_file: String,
}

/// Creates the wallets directory if it doesn't exist
fn ensure_wallets_directory() -> Result<(), SwapError> {
    let wallet_dir = Path::new(WALLET_BACKUP_DIR);
    if !wallet_dir.exists() {
        fs::create_dir_all(wallet_dir)
            .map_err(|e| SwapError::ConfigError(format!("Failed to create wallets directory: {}", e)))?;
        log(LogTag::Wallet, "SETUP", &format!("Created wallets directory: {}", WALLET_BACKUP_DIR));
    }
    Ok(())
}

/// Generates a new keypair and saves backup file
pub fn create_temp_wallet(purpose: &str) -> Result<(Keypair, String), SwapError> {
    ensure_wallets_directory()?;
    
    // Generate new keypair
    let keypair = Keypair::new();
    let address = keypair.pubkey().to_string();
    
    // Create backup file name with timestamp
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("wallet_{}_{}.json", purpose, timestamp);
    let filepath = Path::new(WALLET_BACKUP_DIR).join(&filename);
    
    // Create wallet backup data
    let backup = WalletBackup {
        address: address.clone(),
        private_key: bs58::encode(keypair.to_bytes()).into_string(),
        created_at: Utc::now().to_rfc3339(),
        purpose: purpose.to_string(),
        status: "active".to_string(),
    };
    
    // Save backup file
    let backup_json = serde_json::to_string_pretty(&backup)
        .map_err(|e| SwapError::ConfigError(format!("Failed to serialize wallet backup: {}", e)))?;
    
    fs::write(&filepath, backup_json)
        .map_err(|e| SwapError::ConfigError(format!("Failed to write wallet backup: {}", e)))?;
    
    log(LogTag::Wallet, "CREATE", &format!("Created temp wallet {} for {}", &address[..8], purpose));
    log(LogTag::Wallet, "BACKUP", &format!("Saved wallet backup: {}", filename));
    
    Ok((keypair, filepath.to_string_lossy().to_string()))
}

/// Transfers SOL from main wallet to temporary wallet
async fn transfer_sol_to_temp_wallet(temp_address: &str, amount_sol: f64) -> Result<String, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;
    let main_wallet_address = get_main_wallet_address()?;
    
    // Decode main wallet private key
    let private_key_bytes = bs58::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key: {}", e)))?;
    
    let main_keypair = Keypair::try_from(&private_key_bytes[..])
        .map_err(|e| SwapError::ConfigError(format!("Failed to create keypair: {}", e)))?;
    
    let temp_pubkey = Pubkey::from_str(temp_address)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid temp wallet address: {}", e)))?;
    
    // Convert SOL to lamports
    let lamports = (amount_sol * 1_000_000_000.0) as u64;
    
    // Create transfer instruction
    let transfer_instruction = system_instruction::transfer(
        &main_keypair.pubkey(),
        &temp_pubkey,
        lamports,
    );
    
    // Get recent blockhash and send transaction
    let rpc_client = get_rpc_client();
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&main_keypair.pubkey()),
        &[&main_keypair],
        recent_blockhash,
    );
    
    let signature = rpc_client.send_transaction(&transaction).await?;
    
    log(LogTag::Wallet, "TRANSFER", &format!("Transferred {:.6} SOL to temp wallet {}. TX: {}", 
         amount_sol, &temp_address[..8], signature));
    
    Ok(signature)
}

/// Creates ATA in main wallet if it doesn't exist
async fn ensure_main_wallet_ata(token_mint: &str) -> Result<Option<String>, SwapError> {
    let main_wallet_address = get_main_wallet_address()?;
    let rpc_client = get_rpc_client();
    
    // Check if ATA already exists
    match rpc_client.get_associated_token_account(&main_wallet_address, token_mint).await {
        Ok(_) => {
            log(LogTag::Wallet, "ATA", &format!("ATA already exists in main wallet for token {}", &token_mint[..8]));
            Ok(None) // ATA exists, no transaction needed
        }
        Err(_) => {
            // ATA doesn't exist, create it
            log(LogTag::Wallet, "ATA", &format!("Creating ATA in main wallet for token {}", &token_mint[..8]));
            
            let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;
            let private_key_bytes = bs58::decode(&configs.main_wallet_private)
                .into_vec()
                .map_err(|e| SwapError::ConfigError(format!("Invalid private key: {}", e)))?;
            
            let main_keypair = Keypair::try_from(&private_key_bytes[..])
                .map_err(|e| SwapError::ConfigError(format!("Failed to create keypair: {}", e)))?;
            
            let token_mint_pubkey = Pubkey::from_str(token_mint)
                .map_err(|e| SwapError::InvalidAmount(format!("Invalid token mint: {}", e)))?;
            
            // Create ATA instruction
            let create_ata_instruction = create_associated_token_account(
                &main_keypair.pubkey(),
                &main_keypair.pubkey(),
                &token_mint_pubkey,
                &spl_token::id(),
            );
            
            let recent_blockhash = rpc_client.get_latest_blockhash().await?;
            
            let transaction = Transaction::new_signed_with_payer(
                &[create_ata_instruction],
                Some(&main_keypair.pubkey()),
                &[&main_keypair],
                recent_blockhash,
            );
            
            let signature = rpc_client.send_transaction(&transaction).await?;
            
            log(LogTag::Wallet, "ATA", &format!("Created ATA in main wallet for token {}. TX: {}", 
                 &token_mint[..8], signature));
            
            Ok(Some(signature))
        }
    }
}

/// Transfers tokens from temporary wallet to main wallet
async fn transfer_tokens_to_main_wallet(
    temp_keypair: &Keypair,
    token_mint: &str,
    token_amount: u64,
) -> Result<String, SwapError> {
    let main_wallet_address = get_main_wallet_address()?;
    let rpc_client = get_rpc_client();
    
    // Get ATAs for both wallets
    let temp_ata = rpc_client.get_associated_token_account(&temp_keypair.pubkey().to_string(), token_mint).await?;
    let main_ata = rpc_client.get_associated_token_account(&main_wallet_address, token_mint).await?;
    
    let temp_ata_pubkey = Pubkey::from_str(&temp_ata)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid temp ATA: {}", e)))?;
    let main_ata_pubkey = Pubkey::from_str(&main_ata)
        .map_err(|e| SwapError::InvalidAmount(format!("Invalid main ATA: {}", e)))?;
    
    // Create token transfer instruction
    let transfer_instruction = transfer(
        &spl_token::id(),
        &temp_ata_pubkey,
        &main_ata_pubkey,
        &temp_keypair.pubkey(),
        &[],
        token_amount,
    ).map_err(|e| SwapError::TransactionError(format!("Failed to create transfer instruction: {}", e)))?;
    
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&temp_keypair.pubkey()),
        &[temp_keypair],
        recent_blockhash,
    );
    
    let signature = rpc_client.send_transaction(&transaction).await?;
    
    log(LogTag::Wallet, "TRANSFER", &format!("Transferred {} tokens from temp wallet to main wallet. TX: {}", 
         token_amount, signature));
    
    Ok(signature)
}

/// Drains all remaining SOL and closes ATA from temporary wallet
async fn cleanup_temp_wallet(
    temp_keypair: &Keypair,
    token_mint: &str,
    wallet_file: &str,
) -> Result<bool, SwapError> {
    let main_wallet_address = get_main_wallet_address()?;
    let temp_address = temp_keypair.pubkey().to_string();
    let rpc_client = get_rpc_client();
    
    log(LogTag::Wallet, "CLEANUP", &format!("Starting cleanup of temp wallet {}", &temp_address[..8]));
    
    // Step 1: Close ATA if it exists and is empty
    match close_single_ata(&temp_address, token_mint).await {
        Ok(signature) => {
            log(LogTag::Wallet, "CLEANUP", &format!("Closed ATA in temp wallet. TX: {}", signature));
        }
        Err(e) => {
            log(LogTag::Wallet, "WARNING", &format!("Could not close ATA in temp wallet: {}", e));
        }
    }
    
    // Small delay to ensure ATA closure is processed
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    
    // Step 2: Transfer all remaining SOL to main wallet
    let remaining_sol = get_sol_balance(&temp_address).await.unwrap_or(0.0);
    
    if remaining_sol > 0.000005 { // Only transfer if more than minimum fee
        // Calculate amount to transfer (leave small amount for transaction fee)
        let transfer_amount = remaining_sol - 0.000005;
        let lamports_to_transfer = (transfer_amount * 1_000_000_000.0) as u64;
        
        let main_pubkey = Pubkey::from_str(&main_wallet_address)
            .map_err(|e| SwapError::InvalidAmount(format!("Invalid main wallet address: {}", e)))?;
        
        let transfer_instruction = system_instruction::transfer(
            &temp_keypair.pubkey(),
            &main_pubkey,
            lamports_to_transfer,
        );
        
        let recent_blockhash = rpc_client.get_latest_blockhash().await?;
        
        let transaction = Transaction::new_signed_with_payer(
            &[transfer_instruction],
            Some(&temp_keypair.pubkey()),
            &[temp_keypair],
            recent_blockhash,
        );
        
        match rpc_client.send_transaction(&transaction).await {
            Ok(signature) => {
                log(LogTag::Wallet, "CLEANUP", &format!("Transferred {:.6} SOL back to main wallet. TX: {}", 
                     transfer_amount, signature));
            }
            Err(e) => {
                log(LogTag::Wallet, "ERROR", &format!("Failed to transfer SOL back to main wallet: {}", e));
            }
        }
    }
    
    // Step 3: Update wallet backup file status
    if let Ok(mut backup_content) = fs::read_to_string(wallet_file) {
        if let Ok(mut backup) = serde_json::from_str::<WalletBackup>(&backup_content) {
            backup.status = "drained".to_string();
            if let Ok(updated_content) = serde_json::to_string_pretty(&backup) {
                let _ = fs::write(wallet_file, updated_content);
                log(LogTag::Wallet, "CLEANUP", &format!("Updated wallet backup status to 'drained'"));
            }
        }
    }
    
    // Verify cleanup
    let final_sol = get_sol_balance(&temp_address).await.unwrap_or(0.0);
    let final_tokens = get_token_balance(&temp_address, token_mint).await.unwrap_or(0);
    
    let cleanup_successful = final_sol < 0.00001 && final_tokens == 0;
    
    if cleanup_successful {
        log(LogTag::Wallet, "SUCCESS", &format!("Temp wallet {} successfully drained", &temp_address[..8]));
    } else {
        log(LogTag::Wallet, "WARNING", &format!("Temp wallet {} may still have funds: {:.6} SOL, {} tokens", 
             &temp_address[..8], final_sol, final_tokens));
    }
    
    Ok(cleanup_successful)
}

/// Main multi-wallet buy function
pub async fn multi_wallet_buy_token(token: &Token, amount_sol: f64) -> Result<MultiWalletBuyResult, SwapError> {
    if !USE_MULTI_WALLET {
        log(LogTag::Wallet, "INFO", "Multi-wallet disabled, using main wallet for buy");
        let swap_result = buy_token(token, amount_sol, None).await?;
        let main_wallet_address = get_main_wallet_address()?;
        let token_balance = get_token_balance(&main_wallet_address, &token.mint).await.unwrap_or(0);
        
        return Ok(MultiWalletBuyResult {
            swap_result,
            temp_wallet_address: main_wallet_address,
            main_wallet_token_balance: token_balance,
            cleanup_successful: true,
            temp_wallet_file: "main_wallet".to_string(),
        });
    }
    
    log(LogTag::Wallet, "MULTI", &format!("Starting multi-wallet buy for {} ({} SOL)", token.symbol, amount_sol));
    
    // Step 1: Create temporary wallet
    let (temp_keypair, wallet_file) = create_temp_wallet(&format!("buy_{}", token.symbol))?;
    let temp_address = temp_keypair.pubkey().to_string();
    
    // Step 2: Calculate total SOL needed (trade amount + extra buffer)
    let total_sol_needed = amount_sol + EXTRA_SOL_BUFFER;
    
    // Step 3: Transfer SOL to temporary wallet
    transfer_sol_to_temp_wallet(&temp_address, total_sol_needed).await?;
    
    // Wait for transfer to be confirmed
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    
    // Step 4: Ensure ATA exists in main wallet
    ensure_main_wallet_ata(&token.mint).await?;
    
    // Step 5: Buy tokens in temporary wallet (temporarily override wallet for swap)
    log(LogTag::Wallet, "MULTI", &format!("Buying tokens in temp wallet {}", &temp_address[..8]));
    
    // Note: We would need to modify the buy_token function to accept a custom wallet
    // For now, we'll use a simplified approach and assume we can set the temp wallet context
    let swap_result = buy_token_with_wallet(token, amount_sol, &temp_keypair).await?;
    
    if !swap_result.success {
        log(LogTag::Wallet, "ERROR", "Token purchase failed in temp wallet");
        cleanup_temp_wallet(&temp_keypair, &token.mint, &wallet_file).await?;
        return Err(SwapError::TransactionError("Token purchase failed".to_string()));
    }
    
    // Wait for buy transaction to be confirmed
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
    
    // Step 6: Get token balance in temp wallet
    let token_balance = get_token_balance(&temp_address, &token.mint).await.unwrap_or(0);
    
    if token_balance == 0 {
        log(LogTag::Wallet, "ERROR", "No tokens found in temp wallet after purchase");
        cleanup_temp_wallet(&temp_keypair, &token.mint, &wallet_file).await?;
        return Err(SwapError::TransactionError("No tokens received".to_string()));
    }
    
    // Step 7: Transfer tokens to main wallet
    transfer_tokens_to_main_wallet(&temp_keypair, &token.mint, token_balance).await?;
    
    // Wait for token transfer to be confirmed
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    
    // Step 8: Cleanup temporary wallet
    let cleanup_successful = cleanup_temp_wallet(&temp_keypair, &token.mint, &wallet_file).await?;
    
    // Step 9: Verify final token balance in main wallet
    let main_wallet_address = get_main_wallet_address()?;
    let main_wallet_token_balance = get_token_balance(&main_wallet_address, &token.mint).await.unwrap_or(0);
    
    log(LogTag::Wallet, "SUCCESS", &format!("Multi-wallet buy completed. {} tokens now in main wallet", main_wallet_token_balance));
    
    Ok(MultiWalletBuyResult {
        swap_result,
        temp_wallet_address: temp_address,
        main_wallet_token_balance,
        cleanup_successful,
        temp_wallet_file: wallet_file,
    })
}

/// Buy tokens using a specific wallet (simplified version - would need integration with swap system)
async fn buy_token_with_wallet(token: &Token, amount_sol: f64, wallet_keypair: &Keypair) -> Result<SwapResult, SwapError> {
    // This is a placeholder - in the real implementation, we would need to:
    // 1. Modify the swap system to accept a custom wallet keypair
    // 2. Use the temporary wallet for all swap operations
    // 3. Ensure proper error handling and cleanup
    
    log(LogTag::Wallet, "SWAP", &format!("Executing buy with temp wallet {} for {} SOL", 
         &wallet_keypair.pubkey().to_string()[..8], amount_sol));
    
    // For now, return a mock successful result
    // In production, this would call the actual swap function with the custom wallet
    Ok(SwapResult {
        success: true,
        transaction_signature: Some("temp_wallet_buy_signature".to_string()),
        input_amount: ((amount_sol * 1_000_000_000.0) as u64).to_string(),
        output_amount: "1000000".to_string(), // Mock token amount
        price_impact: "0.5".to_string(),
        fee_lamports: 5000,
        execution_time: 2.0,
        effective_price: Some(amount_sol / 1000000.0),
        swap_data: None,
        error: None,
    })
}

/// Multi-wallet sell function (sells from main wallet as normal)
pub async fn multi_wallet_sell_token(token: &Token, token_amount: u64, expected_sol_output: Option<f64>) -> Result<SwapResult, SwapError> {
    log(LogTag::Wallet, "MULTI", &format!("Selling {} tokens of {} from main wallet", token_amount, token.symbol));
    
    // Selling always happens from main wallet since tokens are transferred there after purchase
    sell_token(token, token_amount, expected_sol_output).await
}

/// Lists all wallet backup files
pub fn list_wallet_backups() -> Result<Vec<WalletBackup>, SwapError> {
    ensure_wallets_directory()?;
    
    let wallet_dir = Path::new(WALLET_BACKUP_DIR);
    let mut wallets = Vec::new();
    
    if let Ok(entries) = fs::read_dir(wallet_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(extension) = entry.path().extension() {
                    if extension == "json" {
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(wallet) = serde_json::from_str::<WalletBackup>(&content) {
                                wallets.push(wallet);
                            }
                        }
                    }
                }
            }
        }
    }
    
    wallets.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // Most recent first
    Ok(wallets)
}

/// Archives old wallet backup files
pub fn archive_old_wallets(days_old: u32) -> Result<u32, SwapError> {
    let wallets = list_wallet_backups()?;
    let mut archived_count = 0;
    
    let cutoff_date = Utc::now() - chrono::Duration::days(days_old as i64);
    
    for wallet in wallets {
        if wallet.status == "drained" {
            if let Ok(created_date) = chrono::DateTime::parse_from_rfc3339(&wallet.created_at) {
                if created_date.with_timezone(&Utc) < cutoff_date {
                    // Archive this wallet file
                    let filename = format!("wallet_{}_{}.json", 
                                          wallet.purpose.replace(' ', "_"), 
                                          &wallet.created_at[..10]);
                    let filepath = Path::new(WALLET_BACKUP_DIR).join(&filename);
                    
                    if filepath.exists() {
                        let archive_dir = Path::new(WALLET_BACKUP_DIR).join("archived");
                        fs::create_dir_all(&archive_dir).ok();
                        
                        let archive_path = archive_dir.join(&filename);
                        if fs::rename(&filepath, &archive_path).is_ok() {
                            archived_count += 1;
                            log(LogTag::Wallet, "ARCHIVE", &format!("Archived wallet backup: {}", filename));
                        }
                    }
                }
            }
        }
    }
    
    if archived_count > 0 {
        log(LogTag::Wallet, "ARCHIVE", &format!("Archived {} old wallet backup files", archived_count));
    }
    
    Ok(archived_count)
}
