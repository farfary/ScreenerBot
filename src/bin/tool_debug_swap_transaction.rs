use anyhow::{ anyhow, Result };
use base64::{ engine::general_purpose, Engine as _ };
use solana_client::rpc_client::RpcClient;
use solana_sdk::{ signature::Signature };
use solana_transaction_status::{ UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta };
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

// Known DEX program IDs
const RAYDIUM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const RAYDIUM_CPMM: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
const ORCA_WHIRLPOOL: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const JUPITER_V6: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const METEORA_DLMM: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
const PUMP_FUN: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

#[derive(Debug, Clone)]
struct TransactionDebugInfo {
    signature: String,
    slot: u64,
    block_time: Option<i64>,
    fee: u64,
    success: bool,
    error: Option<String>,
    compute_units_consumed: Option<u64>,
    pre_balances: Vec<u64>,
    post_balances: Vec<u64>,
    pre_token_balances: Vec<TokenBalance>,
    post_token_balances: Vec<TokenBalance>,
    instructions: Vec<InstructionInfo>,
    inner_instructions: Vec<InnerInstructionInfo>,
    accounts: Vec<String>,
    log_messages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct TokenBalance {
    account_index: usize,
    mint: String,
    ui_token_amount: f64,
    raw_amount: String,
    decimals: u8,
    owner: Option<String>,
}

#[derive(Debug, Clone)]
struct InstructionInfo {
    program_id: String,
    program_name: String,
    data: String,
    decoded_data: Option<String>,
}

#[derive(Debug, Clone)]
struct InnerInstructionInfo {
    instruction_index: usize,
    instructions: Vec<InstructionInfo>,
}

#[derive(Debug)]
struct SwapAnalysis {
    swap_detected: bool,
    dex_program: Option<String>,
    input_token: Option<String>,
    output_token: Option<String>,
    input_amount: Option<f64>,
    output_amount: Option<f64>,
    wallet_address: Option<String>,
    sol_balance_change: i64,
    ata_operations: Vec<ATAOperation>,
    transfer_operations: Vec<TransferOperation>,
}

#[derive(Debug)]
struct ATAOperation {
    operation_type: String, // "create" or "close"
    ata_address: String,
    mint: String,
    owner: String,
    rent_lamports: Option<u64>,
}

#[derive(Debug)]
struct TransferOperation {
    from: String,
    to: String,
    mint: Option<String>, // None for SOL transfers
    amount: f64,
    raw_amount: String,
    decimals: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <transaction_signature>", args[0]);
        std::process::exit(1);
    }

    let signature_str = &args[1];
    let signature = Signature::from_str(signature_str).map_err(|e|
        anyhow!("Invalid signature: {}", e)
    )?;

    // Initialize RPC client
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let client = RpcClient::new(rpc_url.to_string());

    println!("🔍 Debugging Transaction: {}", signature_str);
    println!("{}", "=".repeat(80));

    // Fetch transaction with full details
    let transaction = fetch_transaction_details(&client, &signature).await?;

    // Parse and analyze transaction
    let debug_info = parse_transaction_details(&transaction)?;

    // Perform swap analysis
    let swap_analysis = analyze_swap_transaction(&debug_info)?;

    // Print comprehensive debug information
    print_transaction_overview(&debug_info);
    print_account_changes(&debug_info);
    print_instruction_analysis(&debug_info);
    print_token_operations(&debug_info);
    print_swap_analysis(&swap_analysis);
    print_log_analysis(&debug_info);

    Ok(())
}

async fn fetch_transaction_details(
    client: &RpcClient,
    signature: &Signature
) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
    println!("📡 Fetching transaction details from RPC...");

    let config = solana_client::rpc_config::RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(solana_sdk::commitment_config::CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let transaction = client
        .get_transaction_with_config(signature, config)
        .map_err(|e| anyhow!("Failed to fetch transaction: {}", e))?;

    println!("✅ Transaction fetched successfully");
    Ok(transaction)
}

fn parse_transaction_details(
    transaction: &EncodedConfirmedTransactionWithStatusMeta
) -> Result<TransactionDebugInfo> {
    println!("🔄 Parsing transaction details...");

    let meta = transaction.transaction.meta
        .as_ref()
        .ok_or_else(|| anyhow!("Transaction meta not found"))?;

    let ui_transaction = &transaction.transaction.transaction;

    // Parse the transaction as JSON to extract basic information
    let transaction_json = serde_json::to_value(&transaction)?;

    // Extract accounts from transaction
    let accounts = extract_accounts_from_transaction(&transaction_json);

    // Extract instructions
    let instructions = extract_instructions_from_transaction(&transaction_json);

    // Extract inner instructions
    let inner_instructions = extract_inner_instructions_from_transaction(&transaction_json);

    // Parse token balances
    let pre_token_balances = extract_token_balances(&transaction_json, "preTokenBalances");
    let post_token_balances = extract_token_balances(&transaction_json, "postTokenBalances");

    // Extract basic transaction info
    let compute_units = transaction_json
        .get("meta")
        .and_then(|m| m.get("computeUnitsConsumed"))
        .and_then(|c| c.as_u64());

    let log_messages = transaction_json
        .get("meta")
        .and_then(|m| m.get("logMessages"))
        .and_then(|l| l.as_array())
        .map(|arr|
            arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        )
        .unwrap_or_default();

    let signatures = transaction_json
        .get("transaction")
        .and_then(|t| t.get("signatures"))
        .and_then(|s| s.as_array())
        .and_then(|arr| arr.first())
        .and_then(|s| s.as_str())
        .unwrap_or("unknown")
        .to_string();

    let debug_info = TransactionDebugInfo {
        signature: signatures,
        slot: transaction.slot,
        block_time: transaction.block_time,
        fee: meta.fee,
        success: meta.err.is_none(),
        error: meta.err.as_ref().map(|e| format!("{:?}", e)),
        compute_units_consumed: compute_units,
        pre_balances: meta.pre_balances.clone(),
        post_balances: meta.post_balances.clone(),
        pre_token_balances,
        post_token_balances,
        instructions,
        inner_instructions,
        accounts,
        log_messages,
    };

    println!("✅ Transaction details parsed successfully");
    Ok(debug_info)
}

fn extract_accounts_from_transaction(transaction_json: &Value) -> Vec<String> {
    transaction_json
        .get("transaction")
        .and_then(|t| t.get("message"))
        .and_then(|m| m.get("accountKeys"))
        .and_then(|keys| keys.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|key| {
                    if let Some(s) = key.as_str() {
                        Some(s.to_string())
                    } else if let Some(obj) = key.as_object() {
                        obj.get("pubkey")
                            .and_then(|p| p.as_str())
                            .map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn extract_instructions_from_transaction(transaction_json: &Value) -> Vec<InstructionInfo> {
    transaction_json
        .get("transaction")
        .and_then(|t| t.get("message"))
        .and_then(|m| m.get("instructions"))
        .and_then(|insts| insts.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|inst| parse_instruction_from_json(inst))
                .collect()
        })
        .unwrap_or_default()
}

fn extract_inner_instructions_from_transaction(
    transaction_json: &Value
) -> Vec<InnerInstructionInfo> {
    transaction_json
        .get("meta")
        .and_then(|m| m.get("innerInstructions"))
        .and_then(|inner| inner.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|inner_group| {
                    let index = inner_group.get("index")?.as_u64()? as usize;
                    let instructions = inner_group
                        .get("instructions")?
                        .as_array()?
                        .iter()
                        .filter_map(|inst| parse_instruction_from_json(inst))
                        .collect();

                    Some(InnerInstructionInfo {
                        instruction_index: index,
                        instructions,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_instruction_from_json(inst: &Value) -> Option<InstructionInfo> {
    if let Some(parsed) = inst.get("parsed") {
        // Parsed instruction
        let program_id = inst.get("program")?.as_str()?.to_string();
        let program_name = get_program_name(&program_id);

        Some(InstructionInfo {
            program_id,
            program_name,
            data: "parsed".to_string(),
            decoded_data: Some(format!("{:?}", parsed)),
        })
    } else {
        // Compiled instruction
        let program_id_index = inst.get("programIdIndex")?.as_u64()? as usize;
        let data = inst.get("data")?.as_str()?.to_string();

        // For now, we'll use a placeholder program ID since we don't have account resolution
        let program_id = format!("account_{}", program_id_index);
        let program_name = get_program_name(&program_id);

        Some(InstructionInfo {
            program_id: program_id.clone(),
            program_name,
            data: data.clone(),
            decoded_data: decode_instruction_data(&program_id, &data),
        })
    }
}

fn extract_token_balances(transaction_json: &Value, field_name: &str) -> Vec<TokenBalance> {
    transaction_json
        .get("meta")
        .and_then(|m| m.get(field_name))
        .and_then(|balances| balances.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|balance| {
                    let account_index = balance.get("accountIndex")?.as_u64()? as usize;
                    let mint = balance.get("mint")?.as_str()?.to_string();
                    let ui_token_amount = balance
                        .get("uiTokenAmount")
                        .and_then(|uta| uta.get("uiAmount"))
                        .and_then(|ui| ui.as_f64())
                        .unwrap_or(0.0);
                    let raw_amount = balance
                        .get("uiTokenAmount")
                        .and_then(|uta| uta.get("amount"))
                        .and_then(|amt| amt.as_str())
                        .unwrap_or("0")
                        .to_string();
                    let decimals = balance
                        .get("uiTokenAmount")
                        .and_then(|uta| uta.get("decimals"))
                        .and_then(|dec| dec.as_u64())
                        .unwrap_or(0) as u8;
                    let owner = balance
                        .get("owner")
                        .and_then(|o| o.as_str())
                        .map(|s| s.to_string());

                    Some(TokenBalance {
                        account_index,
                        mint,
                        ui_token_amount,
                        raw_amount,
                        decimals,
                        owner,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn get_program_name(program_id: &str) -> String {
    match program_id {
        SPL_TOKEN_PROGRAM_ID => "SPL Token".to_string(),
        SYSTEM_PROGRAM_ID => "System Program".to_string(),
        ASSOCIATED_TOKEN_PROGRAM_ID => "Associated Token Program".to_string(),
        RAYDIUM_V4 => "Raydium V4".to_string(),
        RAYDIUM_CPMM => "Raydium CPMM".to_string(),
        ORCA_WHIRLPOOL => "Orca Whirlpool".to_string(),
        JUPITER_V6 => "Jupiter V6".to_string(),
        METEORA_DLMM => "Meteora DLMM".to_string(),
        PUMP_FUN => "Pump.fun".to_string(),
        _ => "Unknown Program".to_string(),
    }
}

fn decode_instruction_data(program_id: &str, data: &str) -> Option<String> {
    // Basic instruction data decoding - can be expanded
    if data.is_empty() {
        return None;
    }

    match program_id {
        SPL_TOKEN_PROGRAM_ID => decode_spl_token_instruction(data),
        SYSTEM_PROGRAM_ID => decode_system_instruction(data),
        _ => Some(format!("Raw data: {}", data)),
    }
}

fn decode_spl_token_instruction(data: &str) -> Option<String> {
    // Decode common SPL Token instructions
    if let Ok(decoded) = general_purpose::STANDARD.decode(data) {
        if !decoded.is_empty() {
            match decoded[0] {
                3 => Some("Transfer".to_string()),
                7 => Some("MintTo".to_string()),
                8 => Some("Burn".to_string()),
                9 => Some("CloseAccount".to_string()),
                12 => Some("TransferChecked".to_string()),
                _ => Some(format!("SPL Token instruction: {}", decoded[0])),
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn decode_system_instruction(data: &str) -> Option<String> {
    // Decode common System Program instructions
    if let Ok(decoded) = general_purpose::STANDARD.decode(data) {
        if !decoded.is_empty() {
            match decoded[0] {
                0 => Some("CreateAccount".to_string()),
                2 => Some("Transfer".to_string()),
                3 => Some("CreateAccountWithSeed".to_string()),
                _ => Some(format!("System instruction: {}", decoded[0])),
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn analyze_swap_transaction(debug_info: &TransactionDebugInfo) -> Result<SwapAnalysis> {
    println!("🔍 Analyzing swap transaction...");

    let mut analysis = SwapAnalysis {
        swap_detected: false,
        dex_program: None,
        input_token: None,
        output_token: None,
        input_amount: None,
        output_amount: None,
        wallet_address: None,
        sol_balance_change: 0,
        ata_operations: vec![],
        transfer_operations: vec![],
    };

    // Detect DEX programs
    for instruction in &debug_info.instructions {
        if is_dex_program(&instruction.program_id) {
            analysis.swap_detected = true;
            analysis.dex_program = Some(instruction.program_name.clone());
            break;
        }
    }

    // Analyze SOL balance changes
    if debug_info.pre_balances.len() == debug_info.post_balances.len() {
        for (i, (pre, post)) in debug_info.pre_balances
            .iter()
            .zip(debug_info.post_balances.iter())
            .enumerate() {
            let change = (*post as i64) - (*pre as i64);
            if change.abs() > 1000000 {
                // Significant change (> 0.001 SOL)
                if let Some(account) = debug_info.accounts.get(i) {
                    if analysis.wallet_address.is_none() {
                        analysis.wallet_address = Some(account.clone());
                        analysis.sol_balance_change = change;
                    }
                }
            }
        }
    }

    // Analyze token balance changes
    let mut token_changes: HashMap<String, (f64, f64)> = HashMap::new();

    // Get pre-swap token balances
    for balance in &debug_info.pre_token_balances {
        token_changes.insert(balance.mint.clone(), (balance.ui_token_amount, 0.0));
    }

    // Update with post-swap token balances
    for balance in &debug_info.post_token_balances {
        let entry = token_changes.entry(balance.mint.clone()).or_insert((0.0, 0.0));
        entry.1 = balance.ui_token_amount;
    }

    // Identify input and output tokens
    for (mint, (pre_amount, post_amount)) in token_changes {
        let change = post_amount - pre_amount;
        if change > 0.0 && analysis.output_token.is_none() {
            analysis.output_token = Some(mint.clone());
            analysis.output_amount = Some(change);
        } else if change < 0.0 && analysis.input_token.is_none() {
            analysis.input_token = Some(mint.clone());
            analysis.input_amount = Some(change.abs());
        }
    }

    // Analyze ATA operations
    analysis.ata_operations = detect_ata_operations(debug_info);

    // Analyze transfer operations
    analysis.transfer_operations = detect_transfer_operations(debug_info);

    println!("✅ Swap analysis completed");
    Ok(analysis)
}

fn is_dex_program(program_id: &str) -> bool {
    matches!(
        program_id,
        RAYDIUM_V4 | RAYDIUM_CPMM | ORCA_WHIRLPOOL | JUPITER_V6 | METEORA_DLMM | PUMP_FUN
    )
}

fn detect_ata_operations(debug_info: &TransactionDebugInfo) -> Vec<ATAOperation> {
    let mut operations = vec![];

    // Check instructions for ATA operations
    for instruction in &debug_info.instructions {
        if instruction.program_id == ASSOCIATED_TOKEN_PROGRAM_ID {
            if let Some(decoded) = &instruction.decoded_data {
                if decoded.contains("Create") {
                    operations.push(ATAOperation {
                        operation_type: "create".to_string(),
                        ata_address: "parsed_instruction".to_string(),
                        mint: "unknown".to_string(),
                        owner: "unknown".to_string(),
                        rent_lamports: Some(2039280), // Standard ATA rent
                    });
                }
            }
        }

        if instruction.program_id == SPL_TOKEN_PROGRAM_ID {
            if let Some(decoded) = &instruction.decoded_data {
                if decoded == "CloseAccount" {
                    operations.push(ATAOperation {
                        operation_type: "close".to_string(),
                        ata_address: "parsed_instruction".to_string(),
                        mint: "unknown".to_string(),
                        owner: "unknown".to_string(),
                        rent_lamports: Some(2039280), // Standard ATA rent
                    });
                }
            }
        }
    }

    // Also check inner instructions
    for inner_group in &debug_info.inner_instructions {
        for instruction in &inner_group.instructions {
            if instruction.program_id == SPL_TOKEN_PROGRAM_ID {
                if let Some(decoded) = &instruction.decoded_data {
                    if decoded == "CloseAccount" {
                        operations.push(ATAOperation {
                            operation_type: "close".to_string(),
                            ata_address: "parsed_instruction".to_string(),
                            mint: "unknown".to_string(),
                            owner: "unknown".to_string(),
                            rent_lamports: Some(2039280),
                        });
                    }
                }
            }
        }
    }

    operations
}

fn detect_transfer_operations(debug_info: &TransactionDebugInfo) -> Vec<TransferOperation> {
    let mut operations = vec![];

    // Analyze token balance changes for transfers
    let mut token_balances: HashMap<String, Vec<&TokenBalance>> = HashMap::new();

    for balance in &debug_info.pre_token_balances {
        token_balances.entry(balance.mint.clone()).or_default().push(balance);
    }

    for balance in &debug_info.post_token_balances {
        token_balances.entry(balance.mint.clone()).or_default().push(balance);
    }

    for (mint, balances) in token_balances {
        if balances.len() >= 2 {
            let pre_balance = balances
                .iter()
                .find(|b| {
                    debug_info.pre_token_balances
                        .iter()
                        .any(|pb| pb.mint == b.mint && pb.account_index == b.account_index)
                });
            let post_balance = balances
                .iter()
                .find(|b| {
                    debug_info.post_token_balances
                        .iter()
                        .any(|pb| pb.mint == b.mint && pb.account_index == b.account_index)
                });

            if let (Some(pre), Some(post)) = (pre_balance, post_balance) {
                if pre.ui_token_amount != post.ui_token_amount {
                    let amount_change = post.ui_token_amount - pre.ui_token_amount;
                    operations.push(TransferOperation {
                        from: if amount_change < 0.0 {
                            "wallet".to_string()
                        } else {
                            "pool".to_string()
                        },
                        to: if amount_change > 0.0 {
                            "wallet".to_string()
                        } else {
                            "pool".to_string()
                        },
                        mint: Some(mint.clone()),
                        amount: amount_change.abs(),
                        raw_amount: post.raw_amount.clone(),
                        decimals: post.decimals,
                    });
                }
            }
        }
    }

    operations
}

fn print_transaction_overview(debug_info: &TransactionDebugInfo) {
    println!("\n📋 TRANSACTION OVERVIEW");
    println!("{}", "=".repeat(50));
    println!("Signature: {}", debug_info.signature);
    println!("Slot: {}", debug_info.slot);
    if let Some(block_time) = debug_info.block_time {
        let datetime = chrono::DateTime::from_timestamp(block_time, 0).unwrap_or_default();
        println!("Block Time: {} ({})", block_time, datetime.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    println!("Fee: {} lamports ({:.9} SOL)", debug_info.fee, (debug_info.fee as f64) / 1e9);
    println!("Status: {}", if debug_info.success { "✅ Success" } else { "❌ Failed" });
    if let Some(error) = &debug_info.error {
        println!("Error: {}", error);
    }
    if let Some(compute_units) = debug_info.compute_units_consumed {
        println!("Compute Units Consumed: {}", compute_units);
    }
    println!("Number of Accounts: {}", debug_info.accounts.len());
    println!("Number of Instructions: {}", debug_info.instructions.len());
    println!("Number of Inner Instructions: {}", debug_info.inner_instructions.len());
}

fn print_account_changes(debug_info: &TransactionDebugInfo) {
    println!("\n💰 ACCOUNT BALANCE CHANGES");
    println!("{}", "=".repeat(50));

    for (i, account) in debug_info.accounts.iter().enumerate() {
        if
            let (Some(pre), Some(post)) = (
                debug_info.pre_balances.get(i),
                debug_info.post_balances.get(i),
            )
        {
            let change = (*post as i64) - (*pre as i64);
            if change != 0 {
                println!("Account {}: {}", i, account);
                println!("  Pre:  {} lamports ({:.9} SOL)", pre, (*pre as f64) / 1e9);
                println!("  Post: {} lamports ({:.9} SOL)", post, (*post as f64) / 1e9);
                println!("  Change: {} lamports ({:.9} SOL)", change, (change as f64) / 1e9);
                println!();
            }
        }
    }
}

fn print_instruction_analysis(debug_info: &TransactionDebugInfo) {
    println!("\n🔧 INSTRUCTION ANALYSIS");
    println!("{}", "=".repeat(50));

    for (i, instruction) in debug_info.instructions.iter().enumerate() {
        println!("Instruction {}: {}", i, instruction.program_name);
        println!("  Program ID: {}", instruction.program_id);
        if !instruction.data.is_empty() && instruction.data != "parsed" {
            println!("  Data: {}", instruction.data);
        }
        if let Some(decoded) = &instruction.decoded_data {
            println!("  Decoded: {}", decoded);
        }
        println!();
    }

    // Print inner instructions
    if !debug_info.inner_instructions.is_empty() {
        println!("\n🔄 INNER INSTRUCTIONS");
        println!("{}", "=".repeat(50));

        for inner_group in &debug_info.inner_instructions {
            println!("Inner instructions for instruction {}:", inner_group.instruction_index);
            for (i, instruction) in inner_group.instructions.iter().enumerate() {
                println!("  Inner {}: {}", i, instruction.program_name);
                println!("    Program ID: {}", instruction.program_id);
                if let Some(decoded) = &instruction.decoded_data {
                    println!("    Decoded: {}", decoded);
                }
            }
            println!();
        }
    }
}

fn print_token_operations(debug_info: &TransactionDebugInfo) {
    println!("\n🪙 TOKEN BALANCE CHANGES");
    println!("{}", "=".repeat(50));

    // Group token balances by mint
    let mut token_changes: HashMap<
        String,
        (Option<&TokenBalance>, Option<&TokenBalance>)
    > = HashMap::new();

    for balance in &debug_info.pre_token_balances {
        token_changes.insert(balance.mint.clone(), (Some(balance), None));
    }

    for balance in &debug_info.post_token_balances {
        let entry = token_changes.entry(balance.mint.clone()).or_insert((None, None));
        entry.1 = Some(balance);
    }

    for (mint, (pre_balance, post_balance)) in token_changes {
        println!("Token: {}", mint);

        match (pre_balance, post_balance) {
            (Some(pre), Some(post)) => {
                let change = post.ui_token_amount - pre.ui_token_amount;
                println!("  Pre:  {} tokens", pre.ui_token_amount);
                println!("  Post: {} tokens", post.ui_token_amount);
                println!("  Change: {} tokens", change);
                println!("  Decimals: {}", post.decimals);
                if let Some(owner) = &post.owner {
                    println!("  Owner: {}", owner);
                }
            }
            (Some(pre), None) => {
                println!("  Pre:  {} tokens", pre.ui_token_amount);
                println!("  Post: Account closed/burned");
                println!("  Change: -{} tokens", pre.ui_token_amount);
            }
            (None, Some(post)) => {
                println!("  Pre:  Account didn't exist");
                println!("  Post: {} tokens", post.ui_token_amount);
                println!("  Change: +{} tokens", post.ui_token_amount);
            }
            (None, None) => unreachable!(),
        }
        println!();
    }
}

fn print_swap_analysis(analysis: &SwapAnalysis) {
    println!("\n🔄 SWAP ANALYSIS");
    println!("{}", "=".repeat(50));

    println!("Swap Detected: {}", if analysis.swap_detected { "✅ Yes" } else { "❌ No" });

    if let Some(dex) = &analysis.dex_program {
        println!("DEX Program: {}", dex);
    }

    if let Some(wallet) = &analysis.wallet_address {
        println!("Wallet Address: {}", wallet);
        println!(
            "SOL Balance Change: {} lamports ({:.9} SOL)",
            analysis.sol_balance_change,
            (analysis.sol_balance_change as f64) / 1e9
        );
    }

    if let Some(input_token) = &analysis.input_token {
        println!("Input Token: {}", input_token);
        if let Some(amount) = analysis.input_amount {
            println!("Input Amount: {}", amount);
        }
    }

    if let Some(output_token) = &analysis.output_token {
        println!("Output Token: {}", output_token);
        if let Some(amount) = analysis.output_amount {
            println!("Output Amount: {}", amount);
        }
    }

    if !analysis.ata_operations.is_empty() {
        println!("\n🏦 ATA OPERATIONS:");
        for (i, ata_op) in analysis.ata_operations.iter().enumerate() {
            println!("  {}: {} ATA", i + 1, ata_op.operation_type.to_uppercase());
            println!("     ATA Address: {}", ata_op.ata_address);
            println!("     Mint: {}", ata_op.mint);
            println!("     Owner: {}", ata_op.owner);
            if let Some(rent) = ata_op.rent_lamports {
                println!("     Rent: {} lamports ({:.9} SOL)", rent, (rent as f64) / 1e9);
            }
            println!();
        }
    }

    if !analysis.transfer_operations.is_empty() {
        println!("\n💸 TRANSFER OPERATIONS:");
        for (i, transfer) in analysis.transfer_operations.iter().enumerate() {
            println!("  {}: {} -> {}", i + 1, transfer.from, transfer.to);
            println!("     Amount: {} tokens", transfer.amount);
            if let Some(mint) = &transfer.mint {
                println!("     Token: {}", mint);
            }
            println!("     Decimals: {}", transfer.decimals);
            println!();
        }
    }
}

fn print_log_analysis(debug_info: &TransactionDebugInfo) {
    if debug_info.log_messages.is_empty() {
        return;
    }

    println!("\n📝 TRANSACTION LOGS");
    println!("{}", "=".repeat(50));

    for (i, log) in debug_info.log_messages.iter().enumerate() {
        println!("{}: {}", i + 1, log);
    }

    println!("\n🔍 LOG ANALYSIS");
    println!("{}", "=".repeat(30));

    // Analyze logs for important patterns
    let mut program_invocations = vec![];
    let mut compute_units = vec![];
    let mut errors = vec![];

    for log in &debug_info.log_messages {
        if log.contains("Program ") && log.contains(" invoke") {
            program_invocations.push(log);
        }
        if log.contains("consumed") && log.contains("compute units") {
            compute_units.push(log);
        }
        if log.contains("Error") || log.contains("failed") {
            errors.push(log);
        }
    }

    if !program_invocations.is_empty() {
        println!("Program Invocations:");
        for inv in program_invocations {
            println!("  • {}", inv);
        }
        println!();
    }

    if !compute_units.is_empty() {
        println!("Compute Unit Usage:");
        for cu in compute_units {
            println!("  • {}", cu);
        }
        println!();
    }

    if !errors.is_empty() {
        println!("Errors/Warnings:");
        for error in errors {
            println!("  • {}", error);
        }
        println!();
    }
}
