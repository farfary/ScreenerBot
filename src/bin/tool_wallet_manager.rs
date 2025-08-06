use clap::{Arg, Command};
use screenerbot::wallets_manager::{get_wallets_manager, init_wallets_manager};
use screenerbot::tokens::api::init_dexscreener_api;
use screenerbot::rpc::init_rpc_client;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize required services
    init_dexscreener_api().await?;
    init_rpc_client()?;
    init_wallets_manager()?;

    let matches = Command::new("Wallet Manager")
        .version("1.0")
        .about("ScreenerBot Wallet Management Tool")
        .subcommand(
            Command::new("create")
                .about("Create a new wallet")
                .arg(
                    Arg::new("label")
                        .short('l')
                        .long("label")
                        .value_name("LABEL")
                        .help("Optional label for the wallet")
                )
        )
        .subcommand(
            Command::new("list")
                .about("List all wallets with balances")
        )
        .subcommand(
            Command::new("show")
                .about("Show detailed wallet information")
                .arg(
                    Arg::new("public_key")
                        .value_name("PUBLIC_KEY")
                        .help("Public key of the wallet to show")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("update")
                .about("Update wallet balances")
                .arg(
                    Arg::new("public_key")
                        .value_name("PUBLIC_KEY")
                        .help("Public key of the wallet to update (optional - updates all if not provided")
                )
        )
        .subcommand(
            Command::new("backup")
                .about("Create backup of all wallets")
        )
        .subcommand(
            Command::new("restore")
                .about("Restore wallets from backup")
                .arg(
                    Arg::new("backup_file")
                        .value_name("BACKUP_FILE")
                        .help("Backup filename to restore from")
                        .required(true)
                )
        )
        .subcommand(
            Command::new("list-backups")
                .about("List all available backup files")
        )
        .subcommand(
            Command::new("stats")
                .about("Show wallet statistics")
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a wallet (DANGEROUS)")
                .arg(
                    Arg::new("public_key")
                        .value_name("PUBLIC_KEY")
                        .help("Public key of the wallet to delete")
                        .required(true)
                )
                .arg(
                    Arg::new("confirm")
                        .long("confirm")
                        .action(clap::ArgAction::SetTrue)
                        .help("Confirm deletion (required)")
                )
        )
        .get_matches();

    let manager = get_wallets_manager()?;

    match matches.subcommand() {
        Some(("create", sub_matches)) => {
            let label = sub_matches.get_one::<String>("label").cloned();
            
            println!("🔐 Creating new wallet...");
            let wallet = manager.create_wallet(label).await?;
            
            println!("✅ Successfully created new wallet!");
            println!("📍 Public Key:  {}", wallet.public_key);
            println!("🔑 Private Key: {}", wallet.private_key);
            if let Some(label) = &wallet.label {
                println!("🏷️  Label:       {}", label);
            }
            println!("📅 Created:     {}", wallet.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("\n⚠️  IMPORTANT: Save your private key securely!");
            println!("💾 Wallet backup saved to: data/wallets/{}.json", wallet.public_key);
        }

        Some(("list", _)) => {
            println!("📋 Listing all wallets...\n");
            let wallets = manager.get_all_wallets().await?;
            
            if wallets.is_empty() {
                println!("No wallets found. Use 'create' to create a new wallet.");
                return Ok(());
            }

            for wallet in wallets {
                println!("🔑 Wallet: {}", wallet.public_key);
                if let Some(label) = &wallet.label {
                    println!("   🏷️  Label: {}", label);
                }
                println!("   💰 SOL Balance: {:.6}", wallet.sol_balance);
                println!("   🪙 Token Accounts: {}", wallet.token_balances.len());
                println!("   📅 Last Updated: {}", wallet.last_updated.format("%Y-%m-%d %H:%M:%S UTC"));
                
                if !wallet.token_balances.is_empty() {
                    println!("   🪙 Token Balances:");
                    for (mint, balance) in &wallet.token_balances {
                        println!("      • {} {} ({})", 
                            balance.ui_amount, 
                            balance.symbol.as_ref().unwrap_or(&"Unknown".to_string()),
                            &mint[..8]
                        );
                    }
                }
                println!();
            }
        }

        Some(("show", sub_matches)) => {
            let public_key = sub_matches.get_one::<String>("public_key").unwrap();
            
            if let Some(wallet) = manager.get_wallet(public_key).await? {
                println!("🔍 Wallet Details:");
                println!("📍 Public Key:    {}", wallet.public_key);
                println!("🔑 Private Key:   {}", wallet.private_key);
                if let Some(label) = &wallet.label {
                    println!("🏷️  Label:         {}", label);
                }
                println!("💰 SOL Balance:   {:.9} SOL", wallet.sol_balance);
                println!("📅 Created:       {}", wallet.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                println!("🔄 Last Updated:  {}", wallet.last_updated.format("%Y-%m-%d %H:%M:%S UTC"));
                
                if !wallet.token_balances.is_empty() {
                    println!("\n🪙 Token Balances:");
                    for (mint, balance) in &wallet.token_balances {
                        println!("   • Mint: {}", mint);
                        println!("     Amount: {} (raw: {})", balance.ui_amount, balance.amount);
                        println!("     Decimals: {}", balance.decimals);
                        if let Some(symbol) = &balance.symbol {
                            println!("     Symbol: {}", symbol);
                        }
                        println!("     Updated: {}", balance.last_updated.format("%Y-%m-%d %H:%M:%S UTC"));
                        println!();
                    }
                }
            } else {
                println!("❌ Wallet not found: {}", public_key);
            }
        }

        Some(("update", sub_matches)) => {
            if let Some(public_key) = sub_matches.get_one::<String>("public_key") {
                println!("🔄 Updating balances for wallet: {}", public_key);
                manager.update_wallet_balances(public_key).await?;
                println!("✅ Wallet balances updated successfully!");
            } else {
                println!("🔄 Updating balances for all wallets...");
                manager.update_all_wallet_balances().await?;
                println!("✅ All wallet balances updated successfully!");
            }
        }

        Some(("backup", _)) => {
            println!("💾 Creating wallet backup...");
            let backup_filename = manager.create_backup().await?;
            println!("✅ Backup created successfully: {}", backup_filename);
            println!("📁 Location: data/wallets/{}", backup_filename);
        }

        Some(("restore", sub_matches)) => {
            let backup_file = sub_matches.get_one::<String>("backup_file").unwrap();
            
            println!("⚠️  WARNING: This will replace all current wallets!");
            print!("Are you sure you want to continue? (y/N): ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
                println!("🔄 Restoring wallets from backup: {}", backup_file);
                let restored_count = manager.restore_from_backup(backup_file).await?;
                println!("✅ Successfully restored {} wallets!", restored_count);
            } else {
                println!("❌ Restore cancelled.");
            }
        }

        Some(("list-backups", _)) => {
            println!("📋 Available backups:\n");
            let backups = manager.list_backups()?;
            
            if backups.is_empty() {
                println!("No backup files found.");
            } else {
                for (i, backup) in backups.iter().enumerate() {
                    println!("{}. {}", i + 1, backup);
                }
            }
        }

        Some(("stats", _)) => {
            println!("📊 Wallet Statistics:\n");
            let stats = manager.get_wallet_stats().await?;
            
            println!("🔢 Total Wallets:      {}", stats.total_wallets);
            println!("💰 Total SOL Balance:  {:.6} SOL", stats.total_sol_balance);
            println!("🪙 Total Token Accounts: {}", stats.total_token_accounts);
            println!("🔄 Last Updated:       {}", stats.last_updated.format("%Y-%m-%d %H:%M:%S UTC"));
        }

        Some(("delete", sub_matches)) => {
            let public_key = sub_matches.get_one::<String>("public_key").unwrap();
            let confirm = sub_matches.get_flag("confirm");
            
            if !confirm {
                println!("❌ Wallet deletion requires --confirm flag for safety");
                println!("Usage: delete <PUBLIC_KEY> --confirm");
                return Ok(());
            }

            println!("⚠️  WARNING: You are about to permanently delete wallet: {}", public_key);
            print!("Type 'DELETE' to confirm: ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            if input.trim() == "DELETE" {
                manager.delete_wallet(public_key, true).await?;
                println!("✅ Wallet deleted successfully");
                
                // Create backup after deletion
                let backup_filename = manager.create_backup().await?;
                println!("💾 Backup created: {}", backup_filename);
            } else {
                println!("❌ Deletion cancelled - incorrect confirmation");
            }
        }

        _ => {
            println!("❌ No valid subcommand provided. Use --help for usage information.");
        }
    }

    Ok(())
}
