use screenerbot::wallet::*;
use screenerbot::global::*;
use screenerbot::logger::{ log, LogTag, init_file_logging };
use std::str::FromStr;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    transaction::Transaction,
    instruction::Instruction,
};
use spl_token::instruction::close_account;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_file_logging();

    log(LogTag::System, "TOOL", "🧹 Starting ATA Cleanup Tool");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string());
    let force = args.contains(&"--force".to_string());
    let verbose = args.contains(&"--verbose".to_string());

    if dry_run {
        log(LogTag::System, "MODE", "🔍 DRY RUN MODE - No actual transactions will be sent");
    }

    if verbose {
        log(LogTag::System, "MODE", "📝 VERBOSE MODE - Detailed logging enabled");
    }

    // Get wallet address from configs
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            return Err(e.into());
        }
    };

    log(LogTag::System, "INFO", &format!("Wallet: {}", wallet_address));

    // Step 1: Analyze all token accounts
    log(LogTag::System, "ANALYZE", "🔍 Analyzing all token accounts...");

    let accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get token accounts: {}", e));
            return Err(e.into());
        }
    };

    log(LogTag::System, "INFO", &format!("Found {} total token accounts", accounts.len()));

    let mut empty_accounts = Vec::new();
    let mut non_empty_accounts = Vec::new();
    let mut detection_errors = 0;

    for (i, account) in accounts.iter().enumerate() {
        if verbose {
            log(
                LogTag::System,
                "ACCOUNT",
                &format!(
                    "#{}: {} | Mint: {} | Balance: {}",
                    i + 1,
                    &account.account[..8],
                    &account.mint[..8],
                    account.balance
                )
            );
        }

        // Verify the token account program ownership
        match check_token_account_program(&account.account).await {
            Ok(actual_is_token_2022) => {
                let detection_correct = actual_is_token_2022 == account.is_token_2022;

                if verbose || !detection_correct {
                    log(
                        LogTag::System,
                        "VERIFY",
                        &format!(
                            "Account {}: Actual Token-2022: {} | Current detection: {} {}",
                            &account.account[..8],
                            actual_is_token_2022,
                            account.is_token_2022,
                            if detection_correct {
                                "✅"
                            } else {
                                "❌ MISMATCH!"
                            }
                        )
                    );
                }

                if !detection_correct {
                    detection_errors += 1;
                }

                if account.balance == 0 {
                    empty_accounts.push((account, actual_is_token_2022));
                } else {
                    non_empty_accounts.push(account);
                }
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!(
                        "Could not verify program for account {}: {}",
                        &account.account[..8],
                        e
                    )
                );
                detection_errors += 1;
            }
        }
    }

    // Summary
    log(
        LogTag::System,
        "SUMMARY",
        &format!(
            "📊 Analysis complete: {} total accounts, {} empty, {} non-empty, {} detection errors",
            accounts.len(),
            empty_accounts.len(),
            non_empty_accounts.len(),
            detection_errors
        )
    );

    if empty_accounts.is_empty() {
        log(LogTag::System, "INFO", "✅ No empty token accounts found. Nothing to clean up!");
        return Ok(());
    }

    // Show rent calculation
    let estimated_rent_reclaim = (empty_accounts.len() as f64) * 0.00203928;
    log(
        LogTag::System,
        "RENT",
        &format!(
            "💰 Estimated rent to reclaim: ~{:.6} SOL from {} empty accounts",
            estimated_rent_reclaim,
            empty_accounts.len()
        )
    );

    if !force && !dry_run {
        log(
            LogTag::System,
            "CONFIRM",
            "⚠️  Run with --force to proceed with cleanup, or --dry-run to simulate"
        );
        return Ok(());
    }

    // Step 2: Close empty accounts
    log(LogTag::System, "CLEANUP", "🧹 Starting ATA cleanup...");

    let mut closed_count = 0;
    let mut failed_count = 0;
    let mut signatures = Vec::new();

    for (i, (account, is_token_2022)) in empty_accounts.iter().enumerate() {
        log(
            LogTag::System,
            "CLOSE",
            &format!(
                "({}/{}) Closing {} account: {} (mint: {})",
                i + 1,
                empty_accounts.len(),
                if *is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                &account.account[..8],
                &account.mint[..8]
            )
        );

        if dry_run {
            log(LogTag::System, "DRY_RUN", "Would close this account");
            closed_count += 1;
        } else {
            match
                close_ata_fixed(
                    &wallet_address,
                    &account.account,
                    &account.mint,
                    *is_token_2022
                ).await
            {
                Ok(signature) => {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!("✅ Closed ATA {} | TX: {}", &account.account[..8], signature)
                    );
                    signatures.push(signature);
                    closed_count += 1;

                    // Small delay between closures to avoid overwhelming the network
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("❌ Failed to close ATA {}: {}", &account.account[..8], e)
                    );
                    failed_count += 1;
                }
            }
        }
    }

    // Final summary
    let actual_rent_reclaimed = (closed_count as f64) * 0.00203928;
    log(
        LogTag::System,
        "COMPLETE",
        &format!(
            "🎉 ATA cleanup complete! {} closed, {} failed, ~{:.6} SOL reclaimed",
            closed_count,
            failed_count,
            actual_rent_reclaimed
        )
    );

    if !dry_run && !signatures.is_empty() {
        log(LogTag::System, "SIGNATURES", "Transaction signatures:");
        for (i, sig) in signatures.iter().enumerate() {
            log(LogTag::System, "TX", &format!("  {}: {}", i + 1, sig));
        }
    }

    if detection_errors > 0 {
        log(
            LogTag::System,
            "WARNING",
            &format!("⚠️  {} detection errors occurred. Consider investigating the wallet detection logic.", detection_errors)
        );
    }

    Ok(())
}

/// Check which program owns a token account (corrected detection logic)
async fn check_token_account_program(token_account: &str) -> Result<bool, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [
            token_account,
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let client = reqwest::Client::new();

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for rpc_url in rpc_endpoints {
        match
            client
                .post(rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(owner) = value.get("owner") {
                                if let Some(owner_str) = owner.as_str() {
                                    // Check if owned by Token Extensions Program (Token-2022)
                                    let is_token_2022 =
                                        owner_str == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

                                    // Also check for SPL Token program
                                    let is_spl_token =
                                        owner_str == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

                                    if !is_token_2022 && !is_spl_token {
                                        return Err(
                                            SwapError::TransactionError(
                                                format!(
                                                    "Token account {} has unexpected owner: {}",
                                                    token_account,
                                                    owner_str
                                                )
                                            )
                                        );
                                    }

                                    return Ok(is_token_2022);
                                }
                            }
                        }
                    }
                }
            }
            Err(_e) => {
                continue;
            }
        }
    }

    Err(SwapError::TransactionError("Failed to check token account program".to_string()))
}

/// Fixed version of close_ata that uses correct program detection
async fn close_ata_fixed(
    wallet_address: &str,
    token_account: &str,
    _mint: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    // Use proper Solana SDK to build and send close instruction
    build_and_send_close_instruction_fixed(wallet_address, token_account, is_token_2022).await
}

/// Fixed version that properly handles both program types
async fn build_and_send_close_instruction_fixed(
    wallet_address: &str,
    token_account: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Parse addresses
    let owner_pubkey = Pubkey::from_str(wallet_address).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid wallet address: {}", e))
    )?;

    let token_account_pubkey = Pubkey::from_str(token_account).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid token account: {}", e))
    )?;

    // Decode private key
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key: {}", e)))?;

    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    // Build close account instruction with correct program ID
    let close_instruction = if is_token_2022 {
        // For Token-2022, manually build the close instruction
        let token_2022_program_id = Pubkey::from_str(
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        ).map_err(|e|
            SwapError::TransactionError(format!("Invalid Token-2022 program ID: {}", e))
        )?;

        // Close account instruction data: [9] (close account instruction discriminator)
        let instruction_data = vec![9u8];

        Instruction {
            program_id: token_2022_program_id,
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(token_account_pubkey, false),
                solana_sdk::instruction::AccountMeta::new(owner_pubkey, false),
                solana_sdk::instruction::AccountMeta::new_readonly(owner_pubkey, true)
            ],
            data: instruction_data,
        }
    } else {
        // Standard SPL Token close instruction
        close_account(
            &spl_token::id(),
            &token_account_pubkey,
            &owner_pubkey,
            &owner_pubkey,
            &[]
        ).map_err(|e|
            SwapError::TransactionError(
                format!("Failed to build SPL Token close instruction: {}", e)
            )
        )?
    };

    // Get recent blockhash via RPC
    let recent_blockhash = get_latest_blockhash(&configs.rpc_url).await?;

    // Build transaction
    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[&keypair],
        recent_blockhash
    );

    // Send transaction via RPC
    send_close_transaction_via_rpc(&transaction, &configs).await
}
