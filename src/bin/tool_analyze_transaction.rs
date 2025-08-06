use clap::Parser;
use screenerbot::rpc::{init_rpc_client, get_rpc_client};
use screenerbot::logger::{log, LogTag};

#[derive(Parser)]
struct Args {
    /// Transaction signature to analyze
    signature: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    log(LogTag::System, "START", &format!("Analyzing transaction: {}", args.signature));
    
    // Initialize RPC client
    init_rpc_client()?;
    let rpc_client = get_rpc_client();
    
    // Get transaction details
    match rpc_client.get_transaction_details(&args.signature).await {
        Ok(details) => {
            println!("🔍 Transaction Analysis: {}", args.signature);
            println!("✅ Successfully fetched transaction details");
            
            println!("🔢 Slot: {}", details.slot);
            
            if let Some(meta) = &details.meta {
                let success = meta.err.is_none();
                println!("📊 Status: {}", if success { "SUCCESS" } else { "FAILED" });
                
                if !success {
                    if let Some(err) = &meta.err {
                        println!("❌ Error: {:?}", err);
                    }
                }
                
                println!("⚡ Fee: {:.9} SOL", meta.fee as f64 / 1_000_000_000.0);
                
                // Show balance changes
                if !meta.pre_balances.is_empty() && !meta.post_balances.is_empty() {
                    println!("\n💰 SOL Balance Changes:");
                    for (i, (pre, post)) in meta.pre_balances.iter().zip(meta.post_balances.iter()).enumerate() {
                        let change = *post as i64 - *pre as i64;
                        if change != 0 {
                            let change_sol = change as f64 / 1_000_000_000.0;
                            println!("  Account {}: {:.9} SOL", i, change_sol);
                        }
                    }
                }
                
                // Analyze all fees paid
                println!("\n💳 Fee Analysis:");
                analyze_all_fees(&details);
                
                // Analyze token transfers
                println!("\n🔄 Token Transfer Analysis:");
                analyze_token_transfers(&details);
                
                // Analyze all instructions
                println!("\n📋 Instruction Analysis:");
                analyze_instructions(&details);
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch transaction: {}", e);
            return Err(format!("Transaction fetch failed: {}", e).into());
        }
    }
    
    Ok(())
}

fn analyze_all_fees(details: &screenerbot::rpc::TransactionDetails) {
    if let Some(meta) = &details.meta {
        println!("📊 Comprehensive Fee Breakdown:");
        
        // 1. Transaction fee (already shown above, but let's detail it)
        let transaction_fee_sol = meta.fee as f64 / 1_000_000_000.0;
        println!("  🏦 Transaction Fee: {:.9} SOL ({} lamports)", 
            transaction_fee_sol, meta.fee);
        
        let mut total_fees_lamports = meta.fee;
        let mut fee_details = Vec::new();
        
        // 2. Look for compute budget instructions that set priority fees
        if let Ok(message) = serde_json::from_value::<serde_json::Value>(details.transaction.message.clone()) {
            if let Some(instructions) = message.get("instructions").and_then(|i| i.as_array()) {
                for (i, instruction) in instructions.iter().enumerate() {
                    if let Some(program_id_index) = instruction.get("programIdIndex").and_then(|p| p.as_u64()) {
                        if let Some(account_keys) = message.get("accountKeys").and_then(|keys| keys.as_array()) {
                            if (program_id_index as usize) < account_keys.len() {
                                if let Some(program_id) = account_keys[program_id_index as usize].as_str() {
                                    if program_id == "ComputeBudget111111111111111111111111111111" {
                                        if let Some(data) = instruction.get("data").and_then(|d| d.as_str()) {
                                            if let Ok(decoded_bytes) = bs58::decode(data).into_vec() {
                                                analyze_compute_budget_fees(&decoded_bytes, i + 1, &mut fee_details);
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
        
        // 3. Look for rent payments in SOL balance changes
        let mut rent_payments = Vec::new();
        if !meta.pre_balances.is_empty() && !meta.post_balances.is_empty() {
            for (i, (pre, post)) in meta.pre_balances.iter().zip(meta.post_balances.iter()).enumerate() {
                let change = *post as i64 - *pre as i64;
                if change < 0 {
                    let loss_lamports = (-change) as u64;
                    let loss_sol = loss_lamports as f64 / 1_000_000_000.0;
                    
                    // Check if this might be rent payment (common amounts: ~0.00203928 SOL for token accounts)
                    if loss_lamports >= 2_000_000 && loss_lamports <= 3_000_000 {
                        rent_payments.push((i, loss_lamports, loss_sol, "Token Account Creation"));
                    } else if loss_lamports >= 890_000 && loss_lamports <= 1_500_000 {
                        rent_payments.push((i, loss_lamports, loss_sol, "System Account Creation"));
                    } else if loss_sol > 0.0001 && loss_sol < 0.01 {
                        rent_payments.push((i, loss_lamports, loss_sol, "Potential Rent/Fee"));
                    }
                }
            }
        }
        
        // 4. Look for swap/trading fees in logs
        let mut trading_fees = Vec::new();
        if let Some(log_messages) = &meta.log_messages {
            for (i, log) in log_messages.iter().enumerate() {
                // Look for fee-related messages in logs
                if log.contains("fee") || log.contains("Fee") {
                    trading_fees.push((i, log.clone()));
                }
            }
        }
        
        // 5. Account creation costs (ATA creation, new accounts)
        let mut account_creation_costs = Vec::new();
        if let Some(log_messages) = &meta.log_messages {
            let mut ata_creations = 0;
            let mut account_creations = 0;
            
            for log in log_messages {
                if log.contains("InitializeAccount") || log.contains("CreateIdempotent") {
                    ata_creations += 1;
                } else if log.contains("CreateAccount") || log.contains("Allocate") {
                    account_creations += 1;
                }
            }
            
            if ata_creations > 0 {
                let ata_rent = 2_039_280_u64 * ata_creations as u64; // Standard ATA rent
                account_creation_costs.push(("Associated Token Accounts", ata_creations, ata_rent));
                total_fees_lamports += ata_rent;
            }
            
            if account_creations > 0 {
                let account_rent = 890_880_u64 * account_creations as u64; // Approximate system account rent
                account_creation_costs.push(("System Accounts", account_creations, account_rent));
                total_fees_lamports += account_rent;
            }
        }
        
        // Display all fee components
        if !fee_details.is_empty() {
            println!("\n  ⚡ Compute Budget Fees:");
            for detail in &fee_details {
                println!("    {}", detail);
            }
        }
        
        if !rent_payments.is_empty() {
            println!("\n  🏠 Rent/Account Costs:");
            for (account_idx, lamports, sol, description) in &rent_payments {
                println!("    Account {}: {:.9} SOL ({} lamports) - {}", 
                    account_idx, sol, lamports, description);
            }
        }
        
        if !account_creation_costs.is_empty() {
            println!("\n  🆕 Account Creation Costs:");
            for (account_type, count, total_lamports) in &account_creation_costs {
                let total_sol = *total_lamports as f64 / 1_000_000_000.0;
                println!("    {} x{}: {:.9} SOL ({} lamports)", 
                    account_type, count, total_sol, total_lamports);
            }
        }
        
        if !trading_fees.is_empty() {
            println!("\n  💱 Trading/Protocol Fees (from logs):");
            for (log_idx, log_msg) in &trading_fees {
                println!("    Log {}: {}", log_idx, log_msg);
            }
        }
        
        // 6. Calculate actual SOL spent on tokens (excluding reclaimable costs)
        let mut net_sol_spent = 0.0;
        let mut reclaimable_ata_rent = 0.0;
        let mut permanent_costs = 0.0;
        
        // Transaction fee is permanent cost
        permanent_costs += transaction_fee_sol;
        
        // ATA rent is reclaimable, so subtract from total
        for (account_type, count, total_lamports) in &account_creation_costs {
            let cost_sol = *total_lamports as f64 / 1_000_000_000.0;
            if account_type.contains("Associated Token Accounts") {
                reclaimable_ata_rent += cost_sol;
            } else {
                permanent_costs += cost_sol;
            }
        }
        
        // Calculate net SOL spent on tokens from balance changes
        if !meta.pre_balances.is_empty() && !meta.post_balances.is_empty() {
            for (_i, (pre, post)) in meta.pre_balances.iter().zip(meta.post_balances.iter()).enumerate() {
                let change = *post as i64 - *pre as i64;
                if change < 0 {
                    let loss_sol = (-change) as f64 / 1_000_000_000.0;
                    net_sol_spent += loss_sol;
                }
            }
        }
        
        // Account creation/closure analysis
        analyze_account_lifecycle(&details, &mut permanent_costs, &mut reclaimable_ata_rent);
        
        // Total fee summary
        let total_fees_sol = total_fees_lamports as f64 / 1_000_000_000.0;
        println!("\n  💰 Total Estimated Fees: {:.9} SOL ({} lamports)", 
            total_fees_sol, total_fees_lamports);
        
        // 7. Net SOL analysis
        println!("\n  🧮 Net SOL Analysis:");
        println!("    💸 Total SOL Spent: {:.9} SOL", net_sol_spent);
        println!("    🔄 Reclaimable ATA Rent: {:.9} SOL", reclaimable_ata_rent);
        println!("    🔥 Permanent Costs: {:.9} SOL", permanent_costs);
        
        let actual_token_cost = net_sol_spent - reclaimable_ata_rent;
        if actual_token_cost > 0.0 {
            println!("    ⭐ Net SOL for Tokens: {:.9} SOL", actual_token_cost);
            
            // Calculate efficiency based on net cost
            if let (Some(pre_token_balances), Some(post_token_balances)) = 
                (&meta.pre_token_balances, &meta.post_token_balances) {
                
                let mut token_value_received = 0.0;
                
                // Try to estimate token value received
                for post_balance in post_token_balances {
                    if let Some(pre_balance) = pre_token_balances.iter()
                        .find(|pb| pb.account_index == post_balance.account_index) {
                        
                        let pre_amount = pre_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                        let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                        
                        if post_amount > pre_amount {
                            let gain = post_amount - pre_amount;
                            let decimals = post_balance.ui_token_amount.decimals;
                            let gain_formatted = gain as f64 / 10_f64.powi(decimals as i32);
                            
                            // For USDC/USDT (6 decimals), treat 1:1 with USD
                            if decimals == 6 && (post_balance.mint.contains("USDC") || 
                                post_balance.mint.contains("USDT") || 
                                post_balance.mint == "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v") {
                                // Rough USD to SOL conversion (very approximate)
                                token_value_received += gain_formatted * 0.01; // Assume 1 USD = 0.01 SOL
                            }
                        }
                    } else {
                        // New token account
                        let amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                        if amount > 0 {
                            let decimals = post_balance.ui_token_amount.decimals;
                            let amount_formatted = amount as f64 / 10_f64.powi(decimals as i32);
                            
                            if decimals == 6 && (post_balance.mint.contains("USDC") || 
                                post_balance.mint.contains("USDT") || 
                                post_balance.mint == "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v") {
                                token_value_received += amount_formatted * 0.01;
                            }
                        }
                    }
                }
                
                if token_value_received > 0.0 {
                    let net_trade_efficiency = (actual_token_cost / token_value_received) * 100.0;
                    println!("    📊 Estimated Token Value: {:.6} SOL equivalent", token_value_received);
                    println!("    💹 Net Trading Cost: {:.4}% of token value", net_trade_efficiency);
                }
            }
        } else {
            println!("    ✅ Net Gain: {:.9} SOL (after reclaimable rent)", -actual_token_cost);
        }
        
        // 8. Fee breakdown percentage
        if total_fees_lamports > 0 {
            println!("\n  📊 Fee Breakdown:");
            let tx_fee_percent = (meta.fee as f64 / total_fees_lamports as f64) * 100.0;
            println!("    Network Transaction Fee: {:.2}%", tx_fee_percent);
            
            for (account_type, _count, lamports) in &account_creation_costs {
                let percent = (*lamports as f64 / total_fees_lamports as f64) * 100.0;
                println!("    {}: {:.2}%", account_type, percent);
            }
        }
        
        // 9. Fee efficiency analysis (original)
        if let (Some(pre_token_balances), Some(post_token_balances)) = 
            (&meta.pre_token_balances, &meta.post_token_balances) {
            
            // Try to estimate the trade volume for fee efficiency
            let mut total_trade_volume_sol = 0.0;
            
            for post_balance in post_token_balances {
                if let Some(pre_balance) = pre_token_balances.iter()
                    .find(|pb| pb.account_index == post_balance.account_index) {
                    
                    let pre_amount = pre_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    
                    if pre_amount != post_amount {
                        let change = (post_amount as i64 - pre_amount as i64).abs() as u64;
                        let decimals = post_balance.ui_token_amount.decimals;
                        let change_formatted = change as f64 / 10_f64.powi(decimals as i32);
                        
                        // Rough estimation: assume 1 token unit = some SOL value
                        // This is very rough without price data
                        if post_balance.mint == "So11111111111111111111111111111111111111112" {
                            total_trade_volume_sol += change_formatted;
                        }
                    }
                }
            }
            
            if total_trade_volume_sol > 0.0 {
                let fee_percentage = (total_fees_sol / total_trade_volume_sol) * 100.0;
                println!("\n  📈 Fee Efficiency:");
                println!("    Estimated Trade Volume: {:.6} SOL", total_trade_volume_sol);
                println!("    Total Fees as % of Volume: {:.4}%", fee_percentage);
            }
        }
    }
}

fn analyze_compute_budget_fees(data: &[u8], instruction_num: usize, fee_details: &mut Vec<String>) {
    if data.len() >= 4 {
        let instruction_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        match instruction_type {
            2 => { // SetComputeUnitPrice
                if data.len() >= 12 {
                    let price = u64::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                        data[8], data[9], data[10], data[11]
                    ]);
                    fee_details.push(format!(
                        "Instruction #{}: Priority fee set to {} micro-lamports per CU", 
                        instruction_num, price
                    ));
                }
            }
            3 => { // SetComputeUnitLimit
                if data.len() >= 8 {
                    let limit = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                    fee_details.push(format!(
                        "Instruction #{}: Compute unit limit set to {} CU", 
                        instruction_num, limit
                    ));
                }
            }
            _ => {}
        }
    }
}

fn analyze_account_lifecycle(details: &screenerbot::rpc::TransactionDetails, permanent_costs: &mut f64, reclaimable_rent: &mut f64) {
    println!("\n  🏗️ Account Lifecycle Analysis:");
    
    if let Some(meta) = &details.meta {
        let mut accounts_created = 0;
        let mut accounts_closed = 0;
        let mut ata_created = 0;
        let mut ata_closed = 0;
        let mut system_accounts_created = 0;
        
        // Analyze from logs
        if let Some(log_messages) = &meta.log_messages {
            for (i, log) in log_messages.iter().enumerate() {
                if log.contains("InitializeAccount3") || log.contains("InitializeAccount2") || log.contains("InitializeAccount") {
                    if !log.contains("InitializeMultisig") {
                        ata_created += 1;
                        println!("    🆕 Log {}: ATA account created", i);
                    }
                } else if log.contains("CreateIdempotent") {
                    ata_created += 1;
                    println!("    🆕 Log {}: ATA account created (idempotent)", i);
                } else if log.contains("CloseAccount") {
                    ata_closed += 1;
                    println!("    🗑️ Log {}: Token account closed", i);
                } else if log.contains("CreateAccount") || log.contains("Allocate") {
                    system_accounts_created += 1;
                    println!("    🆕 Log {}: System account created", i);
                }
            }
        }
        
        // Analyze from balance changes to detect account creation/closure
        if !meta.pre_balances.is_empty() && !meta.post_balances.is_empty() {
            // Check for new accounts (0 pre-balance, non-zero post-balance)
            for (i, (pre, post)) in meta.pre_balances.iter().zip(meta.post_balances.iter()).enumerate() {
                if *pre == 0 && *post > 0 {
                    accounts_created += 1;
                    let post_sol = *post as f64 / 1_000_000_000.0;
                    
                    // Determine account type by rent amount
                    if *post >= 2_000_000 && *post <= 3_000_000 {
                        println!("    💰 Account {}: Token account created with {:.9} SOL rent", i, post_sol);
                    } else if *post >= 800_000 && *post <= 1_500_000 {
                        println!("    💰 Account {}: System account created with {:.9} SOL rent", i, post_sol);
                    } else if *post > 0 {
                        println!("    💰 Account {}: Account created with {:.9} SOL", i, post_sol);
                    }
                } else if *pre > 0 && *post == 0 {
                    accounts_closed += 1;
                    let pre_sol = *pre as f64 / 1_000_000_000.0;
                    
                    // This SOL was reclaimed
                    *reclaimable_rent += pre_sol;
                    println!("    ♻️ Account {}: Account closed, {:.9} SOL reclaimed", i, pre_sol);
                }
            }
        }
        
        // Analyze token account changes
        if let (Some(pre_token_balances), Some(post_token_balances)) = 
            (&meta.pre_token_balances, &meta.post_token_balances) {
            
            // Find new token accounts
            for post_balance in post_token_balances {
                let found_in_pre = pre_token_balances.iter()
                    .any(|pre| pre.account_index == post_balance.account_index);
                
                if !found_in_pre {
                    println!("    🎯 New token account {} for mint: {}", 
                        post_balance.account_index, 
                        &post_balance.mint[..8]);
                }
            }
            
            // Find closed token accounts
            for pre_balance in pre_token_balances {
                let found_in_post = post_token_balances.iter()
                    .any(|post| post.account_index == pre_balance.account_index);
                
                if !found_in_post {
                    println!("    🗑️ Token account {} closed for mint: {}", 
                        pre_balance.account_index, 
                        &pre_balance.mint[..8]);
                }
            }
        }
        
        // Summary
        println!("\n    📊 Account Lifecycle Summary:");
        println!("      🆕 Total Accounts Created: {}", accounts_created);
        println!("      🗑️ Total Accounts Closed: {}", accounts_closed);
        
        if ata_created > 0 {
            let ata_rent_total = ata_created as f64 * 0.002039280; // Standard ATA rent
            println!("      🎯 ATA Accounts Created: {} (Rent: {:.9} SOL)", ata_created, ata_rent_total);
        }
        
        if ata_closed > 0 {
            let ata_rent_reclaimed = ata_closed as f64 * 0.002039280;
            println!("      ♻️ ATA Accounts Closed: {} (Rent Reclaimed: {:.9} SOL)", ata_closed, ata_rent_reclaimed);
        }
        
        if system_accounts_created > 0 {
            let system_rent_total = system_accounts_created as f64 * 0.000890880; // Approximate system account rent
            println!("      🏛️ System Accounts Created: {} (Rent: {:.9} SOL)", system_accounts_created, system_rent_total);
            *permanent_costs += system_rent_total;
        }
        
        // Calculate net account rent impact
        let net_ata_impact = (ata_created - ata_closed) as f64 * 0.002039280;
        if net_ata_impact > 0.0 {
            println!("      📈 Net ATA Rent Locked: {:.9} SOL", net_ata_impact);
        } else if net_ata_impact < 0.0 {
            println!("      📉 Net ATA Rent Freed: {:.9} SOL", -net_ata_impact);
        } else {
            println!("      ⚖️ Net ATA Rent Impact: 0 SOL (balanced)");
        }
        
        // Efficiency analysis
        if accounts_created > 0 {
            let avg_rent_per_account = (ata_created as f64 * 0.002039280 + system_accounts_created as f64 * 0.000890880) / accounts_created as f64;
            println!("      💡 Average Rent per Account: {:.9} SOL", avg_rent_per_account);
        }
    }
}

fn analyze_token_transfers(details: &screenerbot::rpc::TransactionDetails) {
    if let Some(meta) = &details.meta {
        // Check for token account balance changes
        if let (Some(pre_token_balances), Some(post_token_balances)) = 
            (&meta.pre_token_balances, &meta.post_token_balances) {
            
            println!("📊 Token Balance Changes:");
            
            // Create a map of account indices to pre-balances
            let mut pre_balances_map = std::collections::HashMap::new();
            for balance in pre_token_balances {
                pre_balances_map.insert(balance.account_index, balance);
            }
            
            // Check post balances and compare
            for post_balance in post_token_balances {
                let account_index = post_balance.account_index;
                
                if let Some(pre_balance) = pre_balances_map.get(&account_index) {
                    // Calculate change
                    let pre_amount = pre_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    
                    if pre_amount != post_amount {
                        let change = post_amount as i64 - pre_amount as i64;
                        let decimals = post_balance.ui_token_amount.decimals;
                        let change_formatted = change as f64 / 10_f64.powi(decimals as i32);
                        
                        let direction = if change > 0 { "+" } else { "" };
                        println!("  Account {}: {}{:.6} tokens (Mint: {})", 
                            account_index, 
                            direction, 
                            change_formatted,
                            post_balance.mint
                        );
                    }
                } else {
                    // New token account (created during transaction)
                    let amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    if amount > 0 {
                        let decimals = post_balance.ui_token_amount.decimals;
                        let amount_formatted = amount as f64 / 10_f64.powi(decimals as i32);
                        println!("  Account {} (NEW): +{:.6} tokens (Mint: {})", 
                            account_index, 
                            amount_formatted,
                            post_balance.mint
                        );
                    }
                }
            }
            
            // Check for closed token accounts
            for pre_balance in pre_token_balances {
                let account_index = pre_balance.account_index;
                let found_in_post = post_token_balances.iter()
                    .any(|post| post.account_index == account_index);
                
                if !found_in_post {
                    let amount = pre_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    if amount > 0 {
                        let decimals = pre_balance.ui_token_amount.decimals;
                        let amount_formatted = amount as f64 / 10_f64.powi(decimals as i32);
                        println!("  Account {} (CLOSED): -{:.6} tokens (Mint: {})", 
                            account_index, 
                            amount_formatted,
                            pre_balance.mint
                        );
                    }
                }
            }
        } else {
            println!("  No token balance changes detected");
        }
        
        // Analyze instruction logs for transfer events
        if let Some(log_messages) = &meta.log_messages {
            println!("\n📋 Transfer Events from Logs:");
            let mut transfer_count = 0;
            
            for (i, log) in log_messages.iter().enumerate() {
                // Look for transfer-related log messages
                if log.contains("Transfer") || log.contains("transfer") {
                    println!("  Log {}: {}", i, log);
                    transfer_count += 1;
                } else if log.contains("Instruction:") {
                    println!("  Log {}: {}", i, log);
                }
            }
            
            if transfer_count == 0 {
                println!("  No explicit transfer events found in logs");
            } else {
                println!("  Found {} transfer-related log entries", transfer_count);
            }
        }
    }
}

fn analyze_instructions(details: &screenerbot::rpc::TransactionDetails) {
    println!("📝 Transaction Instructions Analysis:");
    
    // Parse the message from JSON
    if let Ok(message) = serde_json::from_value::<serde_json::Value>(details.transaction.message.clone()) {
        // Extract instructions from the message
        if let Some(instructions) = message.get("instructions").and_then(|i| i.as_array()) {
            println!("� Main Instructions ({} total):", instructions.len());
            
            for (i, instruction) in instructions.iter().enumerate() {
                println!("\n  � Instruction #{}", i + 1);
                
                if let Some(program_id_index) = instruction.get("programIdIndex").and_then(|p| p.as_u64()) {
                    println!("    Program Index: {}", program_id_index);
                    
                    // Try to get account keys and identify the program
                    if let Some(account_keys) = message.get("accountKeys").and_then(|keys| keys.as_array()) {
                        if (program_id_index as usize) < account_keys.len() {
                            if let Some(program_id) = account_keys[program_id_index as usize].as_str() {
                                println!("    Program ID: {}", program_id);
                                
                                let program_name = identify_program(program_id);
                                if !program_name.is_empty() {
                                    println!("    Program Name: {}", program_name);
                                }
                            }
                        }
                    }
                }
                
                if let Some(accounts) = instruction.get("accounts").and_then(|a| a.as_array()) {
                    println!("    Accounts: {} account indices", accounts.len());
                    let account_indices: Vec<String> = accounts
                        .iter()
                        .filter_map(|a| a.as_u64().map(|n| n.to_string()))
                        .collect();
                    if !account_indices.is_empty() {
                        println!("    Account Indices: [{}]", account_indices.join(", "));
                    }
                }
                
                if let Some(data) = instruction.get("data").and_then(|d| d.as_str()) {
                    println!("    Data Length: {} characters", data.len());
                    
                    // Show first part of instruction data
                    let preview_len = std::cmp::min(data.len(), 32);
                    println!("    Data Preview: {}{}", 
                        &data[..preview_len],
                        if data.len() > preview_len { "..." } else { "" }
                    );
                    
                    // Try to decode and display the full data
                    let program_name = if let (Some(program_id_index), Some(account_keys)) = 
                        (instruction.get("programIdIndex").and_then(|p| p.as_u64()),
                         message.get("accountKeys").and_then(|keys| keys.as_array())) {
                        if (program_id_index as usize) < account_keys.len() {
                            if let Some(program_id) = account_keys[program_id_index as usize].as_str() {
                                identify_program(program_id)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };
                    
                    print_instruction_data(data, &program_name);
                }
            }
        } else {
            println!("  Could not parse instructions from message");
        }
        
        // Look for address lookup table info
        if let Some(address_lookup_tables) = message.get("addressTableLookups").and_then(|alt| alt.as_array()) {
            if !address_lookup_tables.is_empty() {
                println!("\n🔹 Address Lookup Tables ({} tables):", address_lookup_tables.len());
                
                for (i, alt) in address_lookup_tables.iter().enumerate() {
                    println!("  📋 Table #{}", i + 1);
                    
                    if let Some(account_key) = alt.get("accountKey").and_then(|k| k.as_str()) {
                        println!("    Account Key: {}", account_key);
                    }
                    
                    if let Some(writable_indexes) = alt.get("writableIndexes").and_then(|w| w.as_array()) {
                        let indexes: Vec<String> = writable_indexes
                            .iter()
                            .filter_map(|i| i.as_u64().map(|n| n.to_string()))
                            .collect();
                        println!("    Writable Indexes: [{}]", indexes.join(", "));
                    }
                    
                    if let Some(readonly_indexes) = alt.get("readonlyIndexes").and_then(|r| r.as_array()) {
                        let indexes: Vec<String> = readonly_indexes
                            .iter()
                            .filter_map(|i| i.as_u64().map(|n| n.to_string()))
                            .collect();
                        println!("    Readonly Indexes: [{}]", indexes.join(", "));
                    }
                }
            }
        }
    } else {
        println!("  Could not parse transaction message");
    }
    
    // Analyze logs for instruction details
    if let Some(meta) = &details.meta {
        if let Some(log_messages) = &meta.log_messages {
            println!("\n🔹 Instruction Logs Analysis:");
            
            let mut instruction_count = 0;
            let mut program_invocations = Vec::new();
            
            for (i, log) in log_messages.iter().enumerate() {
                if log.starts_with("Program ") && log.contains(" invoke") {
                    instruction_count += 1;
                    
                    // Extract program ID from log
                    if let Some(program_start) = log.find("Program ") {
                        if let Some(invoke_pos) = log.find(" invoke") {
                            let program_part = &log[program_start + 8..invoke_pos];
                            let program_name = identify_program(program_part);
                            let display_name = if program_name.is_empty() {
                                program_part.to_string()
                            } else {
                                format!("{} ({})", program_name, &program_part[..8])
                            };
                            program_invocations.push(display_name.clone());
                            
                            println!("  📋 Log {}: {} invoke", i, display_name);
                        }
                    }
                } else if log.contains("success") && log.contains("Program ") {
                    println!("  ✅ Log {}: {}", i, log);
                } else if log.contains("Instruction:") {
                    println!("  🔸 Log {}: {}", i, log);
                }
            }
            
            if instruction_count > 0 {
                println!("\n  📊 Summary:");
                println!("    Total Program Invocations: {}", instruction_count);
                println!("    Programs Used: {}", program_invocations.len());
                
                // Count unique programs
                let mut unique_programs = std::collections::HashSet::new();
                for program in &program_invocations {
                    unique_programs.insert(program.clone());
                }
                
                if unique_programs.len() != program_invocations.len() {
                    println!("    Unique Programs: {}", unique_programs.len());
                }
            }
        }
    }
}

fn identify_program(program_id: &str) -> String {
    match program_id {
        "11111111111111111111111111111111" => "System Program".to_string(),
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "SPL Token Program".to_string(),
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => "Associated Token Account Program".to_string(),
        "ComputeBudget111111111111111111111111111111" => "Compute Budget Program".to_string(),
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "Jupiter Aggregator v6".to_string(),
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "Meteora DLMM Program".to_string(),
        "HyaB3W9q6XdA5xwpU4XnSZV94htfmbmqJXZcEbRaJutt" => "Invariant Swap".to_string(),
        "SoLFiHG9TfgtdUXUjWAxi3LtvYuFyDLVhBWxdMZxyCe" => "SolFi Protocol".to_string(),
        "So11111111111111111111111111111111111111112" => "Wrapped SOL".to_string(),
        "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM" => "Raydium AMM Program".to_string(),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium AMM Program v4".to_string(),
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => "Raydium CPMM Program".to_string(),
        _ => "".to_string(),
    }
}

fn print_instruction_data(data: &str, program_name: &str) {
    println!("    📄 Full Instruction Data:");
    
    // Try to decode from base58
    if let Ok(decoded_bytes) = bs58::decode(data).into_vec() {
        println!("      🔢 Decoded from Base58 ({} bytes):", decoded_bytes.len());
        
        // Print as hex dump with ASCII
        print_hex_dump(&decoded_bytes);
        
        // Try to interpret based on program type
        interpret_instruction_data(&decoded_bytes, program_name);
    } else {
        println!("      ⚠️ Could not decode as Base58, showing raw data:");
        println!("      Raw: {}", data);
    }
}

fn print_hex_dump(data: &[u8]) {
    println!("      📊 Hex Dump:");
    
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = i * 16;
        print!("      {:04x}: ", offset);
        
        // Print hex bytes
        for (j, byte) in chunk.iter().enumerate() {
            if j == 8 {
                print!(" ");
            }
            print!("{:02x} ", byte);
        }
        
        // Pad to align ASCII section
        for _ in chunk.len()..16 {
            if chunk.len() <= 8 {
                print!("   ");
            } else {
                print!("   ");
            }
        }
        if chunk.len() <= 8 {
            print!(" ");
        }
        
        // Print ASCII representation
        print!(" |");
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}

fn interpret_instruction_data(data: &[u8], program_name: &str) {
    if data.is_empty() {
        return;
    }
    
    println!("      🔍 Instruction Interpretation:");
    
    match program_name {
        "Compute Budget Program" => {
            interpret_compute_budget_instruction(data);
        }
        "SPL Token Program" => {
            interpret_spl_token_instruction(data);
        }
        "Associated Token Account Program" => {
            interpret_ata_instruction(data);
        }
        "Jupiter Aggregator v6" => {
            interpret_jupiter_instruction(data);
        }
        _ => {
            println!("        💡 First 4 bytes (instruction discriminator): {:02x} {:02x} {:02x} {:02x}", 
                data.get(0).unwrap_or(&0),
                data.get(1).unwrap_or(&0),
                data.get(2).unwrap_or(&0),
                data.get(3).unwrap_or(&0)
            );
            
            if data.len() >= 8 {
                let first_u64 = u64::from_le_bytes([
                    data[0], data[1], data[2], data[3],
                    data[4], data[5], data[6], data[7]
                ]);
                println!("        💡 First 8 bytes as u64 (little-endian): {}", first_u64);
            }
        }
    }
}

fn interpret_compute_budget_instruction(data: &[u8]) {
    if data.len() >= 4 {
        let instruction_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        match instruction_type {
            0 => {
                if data.len() >= 8 {
                    let units = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                    println!("        🔧 RequestUnits: {} compute units", units);
                }
            }
            1 => {
                if data.len() >= 12 {
                    let units = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                    let price = u64::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                        data[8], data[9], data[10], data[11]
                    ]);
                    println!("        🔧 RequestHeapFrame: {} bytes", units);
                    println!("        💰 Priority fee: {} micro-lamports per CU", price);
                }
            }
            2 => {
                if data.len() >= 12 {
                    let price = u64::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                        data[8], data[9], data[10], data[11]
                    ]);
                    println!("        💰 SetComputeUnitPrice: {} micro-lamports per CU", price);
                }
            }
            3 => {
                if data.len() >= 8 {
                    let limit = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                    println!("        ⚡ SetComputeUnitLimit: {} compute units", limit);
                }
            }
            _ => {
                println!("        ❓ Unknown compute budget instruction type: {}", instruction_type);
            }
        }
    }
}

fn interpret_spl_token_instruction(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    
    match data[0] {
        0 => println!("        🔧 InitializeMint"),
        1 => println!("        🔧 InitializeAccount"),
        2 => println!("        🔧 InitializeMultisig"),
        3 => {
            if data.len() >= 9 {
                let amount = u64::from_le_bytes([
                    data[1], data[2], data[3], data[4],
                    data[5], data[6], data[7], data[8]
                ]);
                println!("        💸 Transfer: {} token units", amount);
            }
        }
        4 => println!("        ✅ Approve"),
        5 => println!("        ❌ Revoke"),
        6 => println!("        🔧 SetAuthority"),
        7 => println!("        🔥 MintTo"),
        8 => println!("        🔥 Burn"),
        9 => println!("        🗑️ CloseAccount"),
        10 => println!("        🧊 FreezeAccount"),
        11 => println!("        🔓 ThawAccount"),
        12 => {
            if data.len() >= 17 {
                let amount = u64::from_le_bytes([
                    data[1], data[2], data[3], data[4],
                    data[5], data[6], data[7], data[8]
                ]);
                let decimals = data[9];
                println!("        💸 TransferChecked: {} token units (decimals: {})", amount, decimals);
            }
        }
        13 => println!("        ✅ ApproveChecked"),
        14 => println!("        🔥 MintToChecked"),
        15 => println!("        🔥 BurnChecked"),
        16 => println!("        🔧 InitializeAccount2"),
        17 => println!("        🔧 SyncNative"),
        18 => println!("        🔧 InitializeAccount3"),
        19 => println!("        🔧 InitializeMultisig2"),
        20 => println!("        🔧 InitializeMint2"),
        21 => println!("        🔧 GetAccountDataSize"),
        22 => println!("        🔧 InitializeImmutableOwner"),
        23 => println!("        🔧 AmountToUiAmount"),
        24 => println!("        🔧 UiAmountToAmount"),
        _ => println!("        ❓ Unknown SPL Token instruction: {}", data[0]),
    }
}

fn interpret_ata_instruction(data: &[u8]) {
    if data.is_empty() {
        println!("        🔧 Create (default ATA instruction)");
    } else {
        match data[0] {
            0 => println!("        🔧 Create"),
            1 => println!("        🔧 CreateIdempotent"),
            2 => println!("        🔧 RecoverNested"),
            _ => println!("        ❓ Unknown ATA instruction: {}", data[0]),
        }
    }
}

fn interpret_jupiter_instruction(data: &[u8]) {
    if data.len() >= 8 {
        // Jupiter uses 8-byte discriminators
        let discriminator = &data[0..8];
        println!("        🪐 Jupiter instruction discriminator: {:02x?}", discriminator);
        
        // Common Jupiter discriminators (these are examples, actual values may vary)
        match discriminator {
            [0xe4, 0x45, 0xb7, 0x24, 0x89, 0x5e, 0x97, 0x6b] => {
                println!("        🔄 Route (Jupiter swap routing)");
            }
            [0xb3, 0x2c, 0x3f, 0x7f, 0x4a, 0x6d, 0x8e, 0x91] => {
                println!("        🔄 SharedAccountsRoute (Jupiter shared accounts routing)");
            }
            _ => {
                println!("        🔄 Jupiter swap instruction (unknown discriminator)");
            }
        }
        
        if data.len() > 8 {
            println!("        📊 Additional data: {} bytes", data.len() - 8);
        }
    }
}
