//! Transfer Utilities for Multi-Wallet Operations
//!
//! SOL and token transfer functions for funding and consolidation.

use std::str::FromStr;

use futures::stream::{self, StreamExt};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use tokio::time::{sleep, Duration};

use crate::constants::{LAMPORTS_PER_SOL, TOKEN_2022_PROGRAM_ID};
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::wallets::WalletWithKey;

use super::types::WalletOpResult;

/// Minimum rent-exempt balance for accounts (~0.00089 SOL)
pub const RENT_EXEMPT_MINIMUM: u64 = 890_880;

/// Convert SOL to lamports
fn sol_to_lamports(sol: f64) -> u64 {
    (sol * LAMPORTS_PER_SOL as f64) as u64
}

/// Convert lamports to SOL
fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL as f64
}

// =============================================================================
// SOL TRANSFER
// =============================================================================

/// Transfer SOL from one wallet to another
///
/// # Arguments
/// * `from_keypair` - Keypair of the sending wallet
/// * `to_address` - Recipient wallet address (base58)
/// * `amount_sol` - Amount of SOL to transfer
///
/// # Returns
/// Transaction signature on success
pub async fn transfer_sol(
    from_keypair: &Keypair,
    to_address: &str,
    amount_sol: f64,
) -> Result<String, String> {
    let rpc_client = get_rpc_client();

    let from_pubkey = from_keypair.pubkey();
    let to_pubkey = Pubkey::from_str(to_address)
        .map_err(|e| format!("Invalid recipient address: {}", e))?;

    let lamports = sol_to_lamports(amount_sol);

    // Create transfer instruction
    let instruction = system_instruction::transfer(&from_pubkey, &to_pubkey, lamports);

    // Get recent blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| format!("Failed to get blockhash: {}", e))?;

    // Build and sign transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&from_pubkey),
        &[from_keypair],
        recent_blockhash,
    );

    // Send and confirm
    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map_err(|e| format!("Transfer failed: {}", e))?;

    logger::debug(
        LogTag::Tools,
        &format!(
            "SOL transfer: {} -> {}, amount={:.6} SOL, sig={}",
            &from_pubkey.to_string()[..8],
            &to_address[..8],
            amount_sol,
            &signature.to_string()[..16]
        ),
    );

    Ok(signature.to_string())
}

// =============================================================================
// TOKEN TRANSFER
// =============================================================================

/// Transfer SPL tokens from one wallet to another
///
/// Fetches token decimals directly from the mint account to ensure accuracy.
///
/// # Arguments
/// * `from_keypair` - Keypair of the sending wallet
/// * `to_address` - Recipient wallet address (base58)
/// * `mint` - Token mint address
/// * `amount` - Raw token amount (in smallest units)
/// * `is_token_2022` - Whether this is a Token-2022 token
///
/// # Returns
/// Transaction signature on success
pub async fn transfer_token(
    from_keypair: &Keypair,
    to_address: &str,
    mint: &str,
    amount: u64,
    is_token_2022: bool,
) -> Result<String, String> {
    let rpc_client = get_rpc_client();

    let from_pubkey = from_keypair.pubkey();
    let to_pubkey = Pubkey::from_str(to_address)
        .map_err(|e| format!("Invalid recipient address: {}", e))?;
    let mint_pubkey = Pubkey::from_str(mint)
        .map_err(|e| format!("Invalid mint address: {}", e))?;

    // Fetch mint account to get decimals
    let mint_account = rpc_client
        .get_account(&mint_pubkey)
        .await
        .map_err(|e| format!("Failed to get mint account: {}", e))?
        .ok_or_else(|| "Mint account not found".to_string())?;

    // Parse decimals from mint data (offset 44, 1 byte for SPL Token)
    let decimals = if mint_account.data.len() >= 45 {
        mint_account.data[44]
    } else {
        return Err("Invalid mint account data".to_string());
    };

    let token_program_id = if is_token_2022 {
        Pubkey::from_str(TOKEN_2022_PROGRAM_ID).unwrap()
    } else {
        spl_token::id()
    };

    // Get source ATA
    let source_ata = if is_token_2022 {
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &from_pubkey,
            &mint_pubkey,
            &token_program_id,
        )
    } else {
        spl_associated_token_account::get_associated_token_address(&from_pubkey, &mint_pubkey)
    };

    // Get destination ATA
    let dest_ata = if is_token_2022 {
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &to_pubkey,
            &mint_pubkey,
            &token_program_id,
        )
    } else {
        spl_associated_token_account::get_associated_token_address(&to_pubkey, &mint_pubkey)
    };

    let mut instructions = Vec::new();

    // Check if destination ATA exists, create if not
    let dest_account = rpc_client.get_account(&dest_ata).await?;
    if dest_account.is_none() {
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account(
                &from_pubkey,
                &to_pubkey,
                &mint_pubkey,
                &token_program_id,
            ),
        );
    }

    // Build transfer_checked instruction
    let transfer_ix = spl_token::instruction::transfer_checked(
        &token_program_id,
        &source_ata,
        &mint_pubkey,
        &dest_ata,
        &from_pubkey,
        &[],
        amount,
        decimals,
    )
    .map_err(|e| format!("Failed to build transfer instruction: {}", e))?;

    instructions.push(transfer_ix);

    // Get recent blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| format!("Failed to get blockhash: {}", e))?;

    // Build and sign transaction
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&from_pubkey),
        &[from_keypair],
        recent_blockhash,
    );

    // Send and confirm
    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map_err(|e| format!("Token transfer failed: {}", e))?;

    let ui_amount = amount as f64 / 10f64.powi(decimals as i32);
    logger::debug(
        LogTag::Tools,
        &format!(
            "Token transfer: {} -> {}, mint={}, amount={:.6}, sig={}",
            &from_pubkey.to_string()[..8],
            &to_address[..8],
            &mint[..8],
            ui_amount,
            &signature.to_string()[..16]
        ),
    );

    Ok(signature.to_string())
}

// =============================================================================
// BULK FUNDING
// =============================================================================

/// Fund multiple wallets from a source wallet
///
/// # Arguments
/// * `from_keypair` - Keypair of the funding wallet
/// * `targets` - List of (address, amount_sol) tuples
/// * `concurrency` - Number of concurrent transfers
///
/// # Returns
/// List of operation results
pub async fn fund_wallets(
    from_keypair: &Keypair,
    targets: Vec<(String, f64)>,
    concurrency: usize,
) -> Vec<WalletOpResult> {
    if targets.is_empty() {
        return Vec::new();
    }

    let concurrency = std::cmp::max(1, concurrency);

    logger::info(
        LogTag::Tools,
        &format!(
            "Funding {} wallets with concurrency {}",
            targets.len(),
            concurrency
        ),
    );

    let results: Vec<WalletOpResult> = stream::iter(targets)
        .map(|(address, amount)| {
            let keypair = from_keypair;
            async move {
                match transfer_sol(keypair, &address, amount).await {
                    Ok(sig) => WalletOpResult::success(0, address, sig, amount, None),
                    Err(e) => WalletOpResult::failure(0, address, e),
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let success_count = results.iter().filter(|r| r.success).count();
    let total_funded: f64 = results
        .iter()
        .filter(|r| r.success)
        .filter_map(|r| r.amount_sol)
        .sum();

    logger::info(
        LogTag::Tools,
        &format!(
            "Funding complete: {}/{} successful, {:.6} SOL transferred",
            success_count,
            results.len(),
            total_funded
        ),
    );

    results
}

// =============================================================================
// SOL COLLECTION
// =============================================================================

/// Collect SOL from multiple wallets to a destination
///
/// # Arguments
/// * `wallets` - List of wallets with keypairs
/// * `to_address` - Destination wallet address
/// * `leave_rent` - Whether to leave rent-exempt minimum in source wallets
///
/// # Returns
/// List of operation results
pub async fn collect_sol(
    wallets: Vec<WalletWithKey>,
    to_address: &str,
    leave_rent: bool,
) -> Vec<WalletOpResult> {
    if wallets.is_empty() {
        return Vec::new();
    }

    let rpc_client = get_rpc_client();

    logger::info(
        LogTag::Tools,
        &format!(
            "Collecting SOL from {} wallets to {}",
            wallets.len(),
            &to_address[..8]
        ),
    );

    let mut results = Vec::new();

    for wallet in wallets {
        let wallet_id = wallet.wallet.id;
        let wallet_address = wallet.wallet.address.clone();

        // Get current balance
        let balance = match rpc_client.get_sol_balance(&wallet_address).await {
            Ok(b) => b,
            Err(e) => {
                results.push(WalletOpResult::failure(
                    wallet_id,
                    wallet_address,
                    format!("Failed to get balance: {}", e),
                ));
                continue;
            }
        };

        // Calculate transfer amount
        let rent_reserve = if leave_rent {
            lamports_to_sol(RENT_EXEMPT_MINIMUM)
        } else {
            0.0
        };

        // Estimate transaction fee (~5000 lamports)
        let tx_fee = 0.000005;
        let transfer_amount = balance - rent_reserve - tx_fee;

        if transfer_amount <= 0.0 {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "Wallet {} has insufficient balance ({:.6} SOL) for collection",
                    &wallet_address[..8],
                    balance
                ),
            );
            continue;
        }

        // Execute transfer
        match transfer_sol(&wallet.keypair, to_address, transfer_amount).await {
            Ok(sig) => {
                results.push(WalletOpResult::success(
                    wallet_id,
                    wallet_address,
                    sig,
                    transfer_amount,
                    None,
                ));
            }
            Err(e) => {
                results.push(WalletOpResult::failure(wallet_id, wallet_address, e));
            }
        }

        // Small delay between transfers to avoid rate limiting
        sleep(Duration::from_millis(100)).await;
    }

    let success_count = results.iter().filter(|r| r.success).count();
    let total_collected: f64 = results
        .iter()
        .filter(|r| r.success)
        .filter_map(|r| r.amount_sol)
        .sum();

    logger::info(
        LogTag::Tools,
        &format!(
            "Collection complete: {}/{} successful, {:.6} SOL collected",
            success_count,
            results.len(),
            total_collected
        ),
    );

    results
}

// =============================================================================
// ATA CLOSE
// =============================================================================

/// Close an Associated Token Account to reclaim rent
///
/// # Arguments
/// * `owner_keypair` - Keypair of the ATA owner
/// * `mint` - Token mint address
/// * `is_token_2022` - Whether this is a Token-2022 token
///
/// # Returns
/// Transaction signature on success
pub async fn close_ata(
    owner_keypair: &Keypair,
    mint: &str,
    is_token_2022: bool,
) -> Result<String, String> {
    let rpc_client = get_rpc_client();

    let owner_pubkey = owner_keypair.pubkey();
    let mint_pubkey = Pubkey::from_str(mint)
        .map_err(|e| format!("Invalid mint address: {}", e))?;

    let token_program_id = if is_token_2022 {
        Pubkey::from_str(TOKEN_2022_PROGRAM_ID).unwrap()
    } else {
        spl_token::id()
    };

    // Get ATA address
    let ata = if is_token_2022 {
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &owner_pubkey,
            &mint_pubkey,
            &token_program_id,
        )
    } else {
        spl_associated_token_account::get_associated_token_address(&owner_pubkey, &mint_pubkey)
    };

    // Build close instruction
    let close_instruction = if is_token_2022 {
        build_token_2022_close_instruction(&ata, &owner_pubkey)?
    } else {
        spl_token::instruction::close_account(
            &spl_token::id(),
            &ata,
            &owner_pubkey,
            &owner_pubkey,
            &[],
        )
        .map_err(|e| format!("Failed to build close instruction: {}", e))?
    };

    // Get recent blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| format!("Failed to get blockhash: {}", e))?;

    // Build and sign transaction
    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[owner_keypair],
        recent_blockhash,
    );

    // Send and confirm
    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map_err(|e| format!("Close ATA failed: {}", e))?;

    logger::debug(
        LogTag::Tools,
        &format!(
            "Closed ATA: owner={}, mint={}, sig={}",
            &owner_pubkey.to_string()[..8],
            &mint[..8],
            &signature.to_string()[..16]
        ),
    );

    Ok(signature.to_string())
}

/// Build close instruction for Token-2022 accounts
fn build_token_2022_close_instruction(
    token_account: &Pubkey,
    owner: &Pubkey,
) -> Result<Instruction, String> {
    let token_2022_program_id = Pubkey::from_str(TOKEN_2022_PROGRAM_ID)
        .map_err(|e| format!("Invalid Token-2022 program ID: {}", e))?;

    // CloseAccount instruction: [9] (instruction discriminator)
    let instruction_data = vec![9u8];

    let accounts = vec![
        AccountMeta::new(*token_account, false), // Token account to close
        AccountMeta::new(*owner, false),         // Destination for lamports
        AccountMeta::new_readonly(*owner, true), // Owner/authority
    ];

    Ok(Instruction {
        program_id: token_2022_program_id,
        accounts,
        data: instruction_data,
    })
}
