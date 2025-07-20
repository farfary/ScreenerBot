use screenerbot::{
    global::read_configs,
    wallet::{ get_wallet_address, get_sol_balance },
    logger::{ log, LogTag },
};
use serde_json::Value;

/// Find all token accounts in the wallet and identify empty ones that can be closed
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Finding Empty Token Accounts (ATAs) to Close");
    println!("==============================================");

    // Load configurations
    let configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address()?;
    println!("✅ Wallet address: {}", wallet_address);

    let sol_balance = get_sol_balance(&wallet_address).await?;
    println!("💰 SOL Balance: {:.6} SOL", sol_balance);

    println!("\n🔎 Scanning for token accounts...");

    // Get all token accounts for this wallet
    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet_address,
            {
                "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" // SPL Token program
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&configs.rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send().await?;

    let rpc_response: Value = response.json().await?;

    if let Some(result) = rpc_response.get("result") {
        if let Some(value) = result.get("value") {
            if let Some(accounts) = value.as_array() {
                println!("📊 Found {} token accounts", accounts.len());

                let mut empty_accounts = Vec::new();
                let mut accounts_with_balance = Vec::new();

                for (i, account) in accounts.iter().enumerate() {
                    if let Some(account_info) = account.get("account") {
                        if let Some(data) = account_info.get("data") {
                            if let Some(parsed) = data.get("parsed") {
                                if let Some(info) = parsed.get("info") {
                                    let pubkey = account
                                        .get("pubkey")
                                        .and_then(|p| p.as_str())
                                        .unwrap_or("unknown");
                                    let mint = info
                                        .get("mint")
                                        .and_then(|m| m.as_str())
                                        .unwrap_or("unknown");
                                    let owner = info
                                        .get("owner")
                                        .and_then(|o| o.as_str())
                                        .unwrap_or("unknown");

                                    // Get token amount
                                    let token_amount = info
                                        .get("tokenAmount")
                                        .and_then(|ta| ta.get("amount"))
                                        .and_then(|a| a.as_str())
                                        .unwrap_or("0");

                                    let ui_amount = info
                                        .get("tokenAmount")
                                        .and_then(|ta| ta.get("uiAmount"))
                                        .and_then(|ua| ua.as_f64())
                                        .unwrap_or(0.0);

                                    let decimals = info
                                        .get("tokenAmount")
                                        .and_then(|ta| ta.get("decimals"))
                                        .and_then(|d| d.as_u64())
                                        .unwrap_or(0);

                                    println!("\n{:2}. 🏦 Token Account: {}", i + 1, &pubkey[..8]);
                                    println!("    📄 Mint: {}", &mint[..8]);
                                    println!("    👤 Owner: {}", if owner == wallet_address {
                                        "✅ You"
                                    } else {
                                        "❌ Other"
                                    });
                                    println!(
                                        "    💰 Amount: {} raw ({:.6} UI, {} decimals)",
                                        token_amount,
                                        ui_amount,
                                        decimals
                                    );

                                    if token_amount == "0" || ui_amount == 0.0 {
                                        println!("    🗑️  Status: EMPTY - Can be closed!");
                                        empty_accounts.push((pubkey.to_string(), mint.to_string()));
                                    } else {
                                        println!("    ✅ Status: Has tokens - Cannot close");
                                        accounts_with_balance.push((
                                            pubkey.to_string(),
                                            mint.to_string(),
                                            ui_amount,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }

                println!("\n📋 Summary");
                println!("=========");
                println!("Total token accounts: {}", accounts.len());
                println!("Empty accounts (closable): {}", empty_accounts.len());
                println!("Accounts with balance: {}", accounts_with_balance.len());

                if !empty_accounts.is_empty() {
                    println!("\n🗑️  Empty Accounts That Can Be Closed:");
                    for (i, (account, mint)) in empty_accounts.iter().enumerate() {
                        println!("{}. Account: {} | Mint: {}", i + 1, &account[..16], &mint[..16]);
                    }

                    // Calculate potential rent reclaim
                    let potential_rent = (empty_accounts.len() as f64) * 0.00203928; // ~0.002 SOL per account
                    println!("\n💰 Potential rent to reclaim: ~{:.6} SOL", potential_rent);

                    // Show the first empty account for testing
                    if let Some((account, mint)) = empty_accounts.first() {
                        println!("\n🧪 Test Target (First Empty Account):");
                        println!("Account: {}", account);
                        println!("Mint: {}", mint);
                    }
                }

                if !accounts_with_balance.is_empty() {
                    println!("\n💰 Accounts With Balance:");
                    for (i, (account, mint, balance)) in accounts_with_balance.iter().enumerate() {
                        println!(
                            "{}. Account: {} | Mint: {} | Balance: {:.6}",
                            i + 1,
                            &account[..16],
                            &mint[..16],
                            balance
                        );
                    }
                }
            } else {
                println!("❌ No token accounts found");
            }
        }
    }

    // Also check Token-2022 accounts
    println!("\n🔎 Scanning for Token-2022 accounts...");

    let token2022_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet_address,
            {
                "programId": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" // Token Extensions (Token-2022) program
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let response2022 = client
        .post(&configs.rpc_url)
        .header("Content-Type", "application/json")
        .json(&token2022_payload)
        .send().await?;

    let rpc_response2022: Value = response2022.json().await?;

    if let Some(result) = rpc_response2022.get("result") {
        if let Some(value) = result.get("value") {
            if let Some(accounts) = value.as_array() {
                if accounts.is_empty() {
                    println!("📊 No Token-2022 accounts found");
                } else {
                    println!("📊 Found {} Token-2022 accounts", accounts.len());
                    // Process Token-2022 accounts similar to above
                }
            }
        }
    }

    Ok(())
}
