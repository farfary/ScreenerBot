use serde_json::Value;
use crate::{ wallet::SwapError, global::is_debug_profit_enabled, logger::{ log, LogTag } };

/// SOL mint address (native SOL)
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Comprehensive swap analysis result containing all important details
#[derive(Debug, Clone)]
pub struct SwapAnalysisResult {
    // Transaction status
    pub success: bool,
    pub transaction_signature: String,
    pub error_message: Option<String>,

    // Amounts (in human-readable units with decimals)
    pub input_amount: f64,
    pub output_amount: f64,
    pub input_decimals: u8,
    pub output_decimals: u8,

    // Raw amounts (in smallest token units)
    pub input_amount_raw: u64,
    pub output_amount_raw: u64,

    // Price analysis
    pub effective_price: f64,
    pub expected_price: Option<f64>,
    pub price_difference_percent: f64,
    pub slippage_percent: f64,

    // Fee analysis
    pub transaction_fee_sol: f64,
    pub transaction_fee_lamports: u64,
    pub platform_fee_sol: Option<f64>,
    pub total_fees_sol: f64,

    // ATA analysis
    pub ata_creation_detected: bool,
    pub ata_rent_lamports: u64,
    pub ata_rent_sol: f64,

    // Analysis metadata
    pub analysis_method: String,
    pub confidence_score: f64,
    pub analysis_time_ms: u64,

    // Token information
    pub input_mint: String,
    pub output_mint: String,
    pub is_buy: bool, // true for SOL->Token, false for Token->SOL

    // Additional details
    pub wallet_address: String,
    pub block_height: Option<u64>,
    pub block_time: Option<i64>,
}

/// Token transfer data extracted from transaction
#[derive(Debug, Clone)]
struct TokenTransferData {
    input_amount: f64,
    output_amount: f64,
    input_decimals: u8,
    output_decimals: u8,
    confidence: f64,
    method: String,
}

/// Convert lamports to SOL
fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / 1_000_000_000.0
}

/// Convert SOL to lamports
fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

/// Get transaction details from RPC
async fn get_transaction_details(
    client: &reqwest::Client,
    transaction_signature: &str,
    rpc_endpoint: &str
) -> Result<String, SwapError> {
    let request_body =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            transaction_signature,
            {
                "encoding": "json",
                "maxSupportedTransactionVersion": 0,
                "commitment": "confirmed"
            }
        ]
    });

    let response = client
        .post(rpc_endpoint)
        .json(&request_body)
        .send().await
        .map_err(|e| SwapError::NetworkError(e))?;

    let response_text = response.text().await.map_err(|e| SwapError::NetworkError(e))?;

    let json: Value = serde_json
        ::from_str(&response_text)
        .map_err(|e| SwapError::InvalidResponse(format!("Failed to parse response: {}", e)))?;

    if let Some(result) = json.get("result") {
        if result.is_null() {
            return Err(SwapError::InvalidResponse("Transaction not found".to_string()));
        }
        Ok(serde_json::to_string(result).unwrap())
    } else if let Some(error) = json.get("error") {
        Err(SwapError::InvalidResponse(format!("RPC error: {}", error)))
    } else {
        Err(SwapError::InvalidResponse("Invalid RPC response format".to_string()))
    }
}

/// Method 1: Comprehensive Analysis (Combines all methods)
pub async fn analyze_swap_comprehensive(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    rpc_endpoint: &str,
    intended_amount: Option<f64>
) -> Result<SwapAnalysisResult, SwapError> {
    let start_time = std::time::Instant::now();

    if is_debug_profit_enabled() {
        log(
            LogTag::Wallet,
            "SWAP_ANALYSIS",
            &format!("Starting comprehensive swap analysis for TX: {}", transaction_signature)
        );
    }

    // Wait for transaction to be fully confirmed
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Get transaction details
    let tx_response = get_transaction_details(client, transaction_signature, rpc_endpoint).await?;
    let transaction_json: Value = serde_json
        ::from_str(&tx_response)
        .map_err(|e| SwapError::InvalidResponse(format!("Failed to parse transaction: {}", e)))?;

    // Check transaction success
    let success = check_transaction_success(&transaction_json)?;
    let error_message = if !success { extract_error_message(&transaction_json) } else { None };

    if !success {
        return Ok(SwapAnalysisResult {
            success: false,
            transaction_signature: transaction_signature.to_string(),
            error_message,
            input_amount: 0.0,
            output_amount: 0.0,
            input_decimals: 0,
            output_decimals: 0,
            input_amount_raw: 0,
            output_amount_raw: 0,
            effective_price: 0.0,
            expected_price: intended_amount,
            price_difference_percent: 0.0,
            slippage_percent: 0.0,
            transaction_fee_sol: 0.0,
            transaction_fee_lamports: 0,
            platform_fee_sol: None,
            total_fees_sol: 0.0,
            ata_creation_detected: false,
            ata_rent_lamports: 0,
            ata_rent_sol: 0.0,
            analysis_method: "Failed Transaction".to_string(),
            confidence_score: 1.0,
            analysis_time_ms: start_time.elapsed().as_millis() as u64,
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            is_buy: input_mint == SOL_MINT,
            wallet_address: wallet_address.to_string(),
            block_height: extract_block_height(&transaction_json),
            block_time: extract_block_time(&transaction_json),
        });
    }

    // Try multiple analysis methods
    let methods = vec![
        analyze_inner_instructions(&transaction_json, input_mint, output_mint, wallet_address),
        analyze_token_balances(&transaction_json, input_mint, output_mint, wallet_address),
        analyze_log_messages(&transaction_json, input_mint, output_mint)
    ];

    // Get valid results
    let valid_results: Vec<_> = methods
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();

    if valid_results.is_empty() {
        return Err(SwapError::InvalidResponse("No valid analysis methods succeeded".to_string()));
    }

    // Calculate consensus result
    let consensus_result = calculate_consensus_result(valid_results, intended_amount)?;

    // Extract fee information
    let (tx_fee_lamports, tx_fee_sol) = extract_transaction_fee(&transaction_json);
    let platform_fee_sol = extract_platform_fee(&transaction_json);
    let total_fees_sol = tx_fee_sol + platform_fee_sol.unwrap_or(0.0);

    // Detect ATA creation
    let (ata_detected, ata_rent_lamports, ata_rent_sol) = detect_ata_creation(
        &transaction_json,
        wallet_address
    );

    // Calculate effective price
    let effective_price = if input_mint == SOL_MINT {
        consensus_result.input_amount / consensus_result.output_amount
    } else {
        consensus_result.output_amount / consensus_result.input_amount
    };

    // Calculate price difference and slippage
    let (price_diff_percent, slippage_percent) = if let Some(expected) = intended_amount {
        let price_diff = ((effective_price - expected) / expected) * 100.0;
        let slippage = price_diff.abs();
        (price_diff, slippage)
    } else {
        (0.0, 0.0)
    };

    // Convert to raw amounts
    let input_raw = (consensus_result.input_amount *
        (10_f64).powi(consensus_result.input_decimals as i32)) as u64;
    let output_raw = (consensus_result.output_amount *
        (10_f64).powi(consensus_result.output_decimals as i32)) as u64;

    let analysis_time = start_time.elapsed().as_millis() as u64;

    if is_debug_profit_enabled() {
        log(
            LogTag::Wallet,
            "SWAP_ANALYSIS",
            &format!(
                "Analysis complete: method={}, confidence={:.2}, price={:.12}, slippage={:.3}%, time={}ms",
                consensus_result.method,
                consensus_result.confidence,
                effective_price,
                slippage_percent,
                analysis_time
            )
        );
    }

    Ok(SwapAnalysisResult {
        success: true,
        transaction_signature: transaction_signature.to_string(),
        error_message: None,
        input_amount: consensus_result.input_amount,
        output_amount: consensus_result.output_amount,
        input_decimals: consensus_result.input_decimals,
        output_decimals: consensus_result.output_decimals,
        input_amount_raw: input_raw,
        output_amount_raw: output_raw,
        effective_price,
        expected_price: intended_amount,
        price_difference_percent: price_diff_percent,
        slippage_percent,
        transaction_fee_sol: tx_fee_sol,
        transaction_fee_lamports: tx_fee_lamports,
        platform_fee_sol,
        total_fees_sol,
        ata_creation_detected: ata_detected,
        ata_rent_lamports,
        ata_rent_sol,
        analysis_method: consensus_result.method,
        confidence_score: consensus_result.confidence,
        analysis_time_ms: analysis_time,
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        is_buy: input_mint == SOL_MINT,
        wallet_address: wallet_address.to_string(),
        block_height: extract_block_height(&transaction_json),
        block_time: extract_block_time(&transaction_json),
    })
}

/// Method 2: Inner Instructions Analysis
pub async fn analyze_swap_inner_instructions(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    rpc_endpoint: &str,
    intended_amount: Option<f64>
) -> Result<SwapAnalysisResult, SwapError> {
    let start_time = std::time::Instant::now();

    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let tx_response = get_transaction_details(client, transaction_signature, rpc_endpoint).await?;
    let transaction_json: Value = serde_json::from_str(&tx_response)?;

    let success = check_transaction_success(&transaction_json)?;
    if !success {
        return Err(SwapError::TransactionError("Transaction failed".to_string()));
    }

    let result = analyze_inner_instructions(
        &transaction_json,
        input_mint,
        output_mint,
        wallet_address
    )?;

    // Build result using inner instructions data
    build_swap_result(
        transaction_signature,
        &transaction_json,
        &result,
        input_mint,
        output_mint,
        wallet_address,
        intended_amount,
        start_time.elapsed().as_millis() as u64
    )
}

/// Method 3: Token Balance Changes Analysis
pub async fn analyze_swap_balance_changes(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    rpc_endpoint: &str,
    intended_amount: Option<f64>
) -> Result<SwapAnalysisResult, SwapError> {
    let start_time = std::time::Instant::now();

    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let tx_response = get_transaction_details(client, transaction_signature, rpc_endpoint).await?;
    let transaction_json: Value = serde_json::from_str(&tx_response)?;

    let success = check_transaction_success(&transaction_json)?;
    if !success {
        return Err(SwapError::TransactionError("Transaction failed".to_string()));
    }

    let result = analyze_token_balances(
        &transaction_json,
        input_mint,
        output_mint,
        wallet_address
    )?;

    build_swap_result(
        transaction_signature,
        &transaction_json,
        &result,
        input_mint,
        output_mint,
        wallet_address,
        intended_amount,
        start_time.elapsed().as_millis() as u64
    )
}

/// Method 4: Log Messages Analysis
pub async fn analyze_swap_log_messages(
    client: &reqwest::Client,
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    rpc_endpoint: &str,
    intended_amount: Option<f64>
) -> Result<SwapAnalysisResult, SwapError> {
    let start_time = std::time::Instant::now();

    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    let tx_response = get_transaction_details(client, transaction_signature, rpc_endpoint).await?;
    let transaction_json: Value = serde_json::from_str(&tx_response)?;

    let success = check_transaction_success(&transaction_json)?;
    if !success {
        return Err(SwapError::TransactionError("Transaction failed".to_string()));
    }

    let result = analyze_log_messages(&transaction_json, input_mint, output_mint)?;

    build_swap_result(
        transaction_signature,
        &transaction_json,
        &result,
        input_mint,
        output_mint,
        wallet_address,
        intended_amount,
        start_time.elapsed().as_millis() as u64
    )
}

// Helper functions for analysis methods

fn check_transaction_success(transaction_json: &Value) -> Result<bool, SwapError> {
    if let Some(meta) = transaction_json.get("meta") {
        Ok(meta.get("err").is_none() || meta.get("err").unwrap().is_null())
    } else {
        Err(SwapError::InvalidResponse("Missing transaction metadata".to_string()))
    }
}

fn extract_error_message(transaction_json: &Value) -> Option<String> {
    transaction_json
        .get("meta")
        .and_then(|meta| meta.get("err"))
        .and_then(|err| err.as_str())
        .map(|s| s.to_string())
}

fn extract_block_height(transaction_json: &Value) -> Option<u64> {
    transaction_json.get("slot").and_then(|slot| slot.as_u64())
}

fn extract_block_time(transaction_json: &Value) -> Option<i64> {
    transaction_json.get("blockTime").and_then(|time| time.as_i64())
}

fn analyze_inner_instructions(
    transaction_json: &Value,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> Result<TokenTransferData, SwapError> {
    let meta = transaction_json
        .get("meta")
        .ok_or_else(|| SwapError::InvalidResponse("Missing metadata".to_string()))?;

    let inner_instructions = meta
        .get("innerInstructions")
        .ok_or_else(|| SwapError::InvalidResponse("Missing inner instructions".to_string()))?
        .as_array()
        .ok_or_else(|| SwapError::InvalidResponse("Inner instructions not an array".to_string()))?;

    let mut input_amount = 0.0;
    let mut output_amount = 0.0;
    let mut input_decimals = 0u8;
    let mut output_decimals = 0u8;

    for inner_ix_group in inner_instructions {
        if let Some(instructions) = inner_ix_group.get("instructions").and_then(|i| i.as_array()) {
            for instruction in instructions {
                if let Some(parsed) = instruction.get("parsed") {
                    if let Some(info) = parsed.get("info") {
                        if let Some(instruction_type) = parsed.get("type").and_then(|t| t.as_str()) {
                            if instruction_type == "transferChecked" {
                                let mint = info
                                    .get("mint")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("");
                                let amount = info
                                    .get("tokenAmount")
                                    .and_then(|ta| ta.get("uiAmount"))
                                    .and_then(|ua| ua.as_f64())
                                    .unwrap_or(0.0);
                                let decimals = info
                                    .get("tokenAmount")
                                    .and_then(|ta| ta.get("decimals"))
                                    .and_then(|d| d.as_u64())
                                    .unwrap_or(0) as u8;

                                let source = info
                                    .get("source")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("");
                                let destination = info
                                    .get("destination")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("");

                                // Determine if this is input or output based on wallet involvement
                                if mint == input_mint && source.contains(wallet_address) {
                                    input_amount = amount;
                                    input_decimals = decimals;
                                } else if
                                    mint == output_mint &&
                                    destination.contains(wallet_address)
                                {
                                    output_amount = amount;
                                    output_decimals = decimals;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Handle SOL transfers for SOL-token swaps
    if input_mint == SOL_MINT || output_mint == SOL_MINT {
        let sol_change = calculate_sol_balance_change(transaction_json, wallet_address)?;
        if input_mint == SOL_MINT {
            input_amount = sol_change;
            input_decimals = 9;
        } else {
            output_amount = sol_change;
            output_decimals = 9;
        }
    }

    if input_amount > 0.0 && output_amount > 0.0 {
        Ok(TokenTransferData {
            input_amount,
            output_amount,
            input_decimals,
            output_decimals,
            confidence: 0.95,
            method: "Inner Instructions".to_string(),
        })
    } else {
        Err(
            SwapError::InvalidResponse(
                "Could not extract transfer amounts from inner instructions".to_string()
            )
        )
    }
}

fn analyze_token_balances(
    transaction_json: &Value,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> Result<TokenTransferData, SwapError> {
    let meta = transaction_json
        .get("meta")
        .ok_or_else(|| SwapError::InvalidResponse("Missing metadata".to_string()))?;

    let empty_vec = vec![];
    let pre_token_balances = meta
        .get("preTokenBalances")
        .and_then(|b| b.as_array())
        .unwrap_or(&empty_vec);
    let post_token_balances = meta
        .get("postTokenBalances")
        .and_then(|b| b.as_array())
        .unwrap_or(&empty_vec);

    let mut input_amount = 0.0;
    let mut output_amount = 0.0;
    let mut input_decimals = 0u8;
    let mut output_decimals = 0u8;

    // Find token balance changes
    let input_change = calculate_token_balance_change(
        pre_token_balances,
        post_token_balances,
        input_mint,
        wallet_address
    )?;
    let output_change = calculate_token_balance_change(
        pre_token_balances,
        post_token_balances,
        output_mint,
        wallet_address
    )?;

    // Get decimals
    input_decimals = get_decimals_from_balances(
        pre_token_balances,
        post_token_balances,
        input_mint
    )?;
    output_decimals = get_decimals_from_balances(
        pre_token_balances,
        post_token_balances,
        output_mint
    )?;

    // Handle SOL separately
    if input_mint == SOL_MINT || output_mint == SOL_MINT {
        let sol_change = calculate_sol_balance_change(transaction_json, wallet_address)?;
        if input_mint == SOL_MINT {
            input_amount = sol_change;
            input_decimals = 9;
            output_amount = output_change;
        } else {
            input_amount = input_change;
            output_amount = sol_change;
            output_decimals = 9;
        }
    } else {
        input_amount = input_change;
        output_amount = output_change;
    }

    if input_amount > 0.0 && output_amount > 0.0 {
        Ok(TokenTransferData {
            input_amount,
            output_amount,
            input_decimals,
            output_decimals,
            confidence: 0.9,
            method: "Token Balances".to_string(),
        })
    } else {
        Err(SwapError::InvalidResponse("Could not extract amounts from token balances".to_string()))
    }
}

fn analyze_log_messages(
    transaction_json: &Value,
    input_mint: &str,
    output_mint: &str
) -> Result<TokenTransferData, SwapError> {
    let meta = transaction_json
        .get("meta")
        .ok_or_else(|| SwapError::InvalidResponse("Missing metadata".to_string()))?;

    if let Some(log_messages) = meta.get("logMessages").and_then(|logs| logs.as_array()) {
        for log in log_messages {
            if let Some(log_text) = log.as_str() {
                // Try to parse GMGN swap logs
                if log_text.contains("swap") || log_text.contains("Swap") {
                    if let Ok(parsed) = parse_swap_log(log_text, input_mint, output_mint) {
                        return Ok(parsed);
                    }
                }
            }
        }
    }

    Err(SwapError::InvalidResponse("No recognizable swap logs found".to_string()))
}

fn calculate_token_balance_change(
    pre_balances: &[Value],
    post_balances: &[Value],
    mint: &str,
    wallet_address: &str
) -> Result<f64, SwapError> {
    let mut pre_amount = 0.0;
    let mut post_amount = 0.0;

    // Find pre-balance
    for balance in pre_balances {
        if
            let (Some(balance_mint), Some(ui_amount)) = (
                balance.get("mint").and_then(|m| m.as_str()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|ta| ta.get("uiAmount"))
                    .and_then(|ua| ua.as_f64()),
            )
        {
            if balance_mint == mint {
                if let Some(owner) = balance.get("owner").and_then(|o| o.as_str()) {
                    if owner == wallet_address {
                        pre_amount = ui_amount;
                        break;
                    }
                }
            }
        }
    }

    // Find post-balance
    for balance in post_balances {
        if
            let (Some(balance_mint), Some(ui_amount)) = (
                balance.get("mint").and_then(|m| m.as_str()),
                balance
                    .get("uiTokenAmount")
                    .and_then(|ta| ta.get("uiAmount"))
                    .and_then(|ua| ua.as_f64()),
            )
        {
            if balance_mint == mint {
                if let Some(owner) = balance.get("owner").and_then(|o| o.as_str()) {
                    if owner == wallet_address {
                        post_amount = ui_amount;
                        break;
                    }
                }
            }
        }
    }

    Ok((post_amount - pre_amount).abs())
}

fn calculate_sol_balance_change(
    transaction_json: &Value,
    wallet_address: &str
) -> Result<f64, SwapError> {
    let meta = transaction_json
        .get("meta")
        .ok_or_else(|| SwapError::InvalidResponse("Missing metadata".to_string()))?;

    let transaction = transaction_json
        .get("transaction")
        .ok_or_else(|| SwapError::InvalidResponse("Missing transaction".to_string()))?;

    let message = transaction
        .get("message")
        .ok_or_else(|| SwapError::InvalidResponse("Missing message".to_string()))?;

    let account_keys = message
        .get("accountKeys")
        .ok_or_else(|| SwapError::InvalidResponse("Missing accountKeys".to_string()))?
        .as_array()
        .ok_or_else(|| SwapError::InvalidResponse("accountKeys not array".to_string()))?;

    // Find wallet index
    let mut wallet_index = None;
    for (i, key) in account_keys.iter().enumerate() {
        if let Some(pubkey) = key.as_str() {
            if pubkey == wallet_address {
                wallet_index = Some(i);
                break;
            }
        }
    }

    let wallet_index = wallet_index.ok_or_else(||
        SwapError::InvalidResponse("Wallet not found in transaction".to_string())
    )?;

    let pre_balances = meta
        .get("preBalances")
        .ok_or_else(|| SwapError::InvalidResponse("Missing preBalances".to_string()))?
        .as_array()
        .ok_or_else(|| SwapError::InvalidResponse("preBalances not array".to_string()))?;

    let post_balances = meta
        .get("postBalances")
        .ok_or_else(|| SwapError::InvalidResponse("Missing postBalances".to_string()))?
        .as_array()
        .ok_or_else(|| SwapError::InvalidResponse("postBalances not array".to_string()))?;

    let pre_balance = pre_balances
        .get(wallet_index)
        .ok_or_else(||
            SwapError::InvalidResponse("Wallet index out of bounds in preBalances".to_string())
        )?
        .as_u64()
        .ok_or_else(|| SwapError::InvalidResponse("Invalid preBalance".to_string()))?;

    let post_balance = post_balances
        .get(wallet_index)
        .ok_or_else(||
            SwapError::InvalidResponse("Wallet index out of bounds in postBalances".to_string())
        )?
        .as_u64()
        .ok_or_else(|| SwapError::InvalidResponse("Invalid postBalance".to_string()))?;

    // Calculate change (excluding fees for more accuracy)
    let sol_change_lamports = if post_balance > pre_balance {
        post_balance - pre_balance
    } else {
        pre_balance - post_balance
    };

    // Exclude transaction fee
    let fee = meta
        .get("fee")
        .and_then(|f| f.as_u64())
        .unwrap_or(0);

    let adjusted_lamports = if post_balance < pre_balance && fee > 0 {
        // For outgoing transactions, subtract the fee
        sol_change_lamports.saturating_sub(fee)
    } else {
        sol_change_lamports
    };

    Ok(lamports_to_sol(adjusted_lamports))
}

fn get_decimals_from_balances(
    pre_balances: &[Value],
    post_balances: &[Value],
    mint: &str
) -> Result<u8, SwapError> {
    // Try post balances first
    for balance in post_balances {
        if let Some(balance_mint) = balance.get("mint").and_then(|m| m.as_str()) {
            if balance_mint == mint {
                if
                    let Some(decimals) = balance
                        .get("uiTokenAmount")
                        .and_then(|ta| ta.get("decimals"))
                        .and_then(|d| d.as_u64())
                {
                    return Ok(decimals as u8);
                }
            }
        }
    }

    // Try pre balances
    for balance in pre_balances {
        if let Some(balance_mint) = balance.get("mint").and_then(|m| m.as_str()) {
            if balance_mint == mint {
                if
                    let Some(decimals) = balance
                        .get("uiTokenAmount")
                        .and_then(|ta| ta.get("decimals"))
                        .and_then(|d| d.as_u64())
                {
                    return Ok(decimals as u8);
                }
            }
        }
    }

    // Default decimals
    if mint == SOL_MINT {
        Ok(9)
    } else {
        Ok(6) // Common default for SPL tokens
    }
}

fn parse_swap_log(
    log_text: &str,
    _input_mint: &str,
    _output_mint: &str
) -> Result<TokenTransferData, SwapError> {
    // This is a simplified parser - you can extend this to handle specific DEX log formats
    if log_text.contains("swap") {
        // Extract numbers from log if possible
        // This is a placeholder implementation
        return Err(
            SwapError::InvalidResponse("Log parsing not implemented for this format".to_string())
        );
    }

    Err(SwapError::InvalidResponse("No swap data found in log".to_string()))
}

fn calculate_consensus_result(
    valid_results: Vec<TokenTransferData>,
    _intended_amount: Option<f64>
) -> Result<TokenTransferData, SwapError> {
    if valid_results.is_empty() {
        return Err(
            SwapError::InvalidResponse("No valid results to calculate consensus".to_string())
        );
    }

    // For now, return the result with highest confidence
    // You can implement more sophisticated consensus logic here
    let best_result = valid_results
        .into_iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
        .unwrap();

    Ok(best_result)
}

fn extract_transaction_fee(transaction_json: &Value) -> (u64, f64) {
    let fee_lamports = transaction_json
        .get("meta")
        .and_then(|meta| meta.get("fee"))
        .and_then(|fee| fee.as_u64())
        .unwrap_or(0);

    (fee_lamports, lamports_to_sol(fee_lamports))
}

fn extract_platform_fee(transaction_json: &Value) -> Option<f64> {
    // Look for platform-specific fees in logs
    if let Some(meta) = transaction_json.get("meta") {
        if let Some(logs) = meta.get("logMessages").and_then(|l| l.as_array()) {
            for log in logs {
                if let Some(log_text) = log.as_str() {
                    if log_text.contains("platform fee") || log_text.contains("Platform Fee") {
                        // Parse platform fee from log message
                        // This is implementation specific to each DEX
                    }
                }
            }
        }
    }
    None
}

fn detect_ata_creation(transaction_json: &Value, _wallet_address: &str) -> (bool, u64, f64) {
    // Look for ATA creation in inner instructions
    if let Some(meta) = transaction_json.get("meta") {
        if
            let Some(inner_instructions) = meta
                .get("innerInstructions")
                .and_then(|ii| ii.as_array())
        {
            for inner_ix_group in inner_instructions {
                if
                    let Some(instructions) = inner_ix_group
                        .get("instructions")
                        .and_then(|i| i.as_array())
                {
                    for instruction in instructions {
                        if let Some(parsed) = instruction.get("parsed") {
                            if
                                let Some(instruction_type) = parsed
                                    .get("type")
                                    .and_then(|t| t.as_str())
                            {
                                if
                                    instruction_type == "createAccount" ||
                                    instruction_type == "create"
                                {
                                    // Common ATA rent is ~2,039,280 lamports
                                    let ata_rent = 2_039_280u64;
                                    return (true, ata_rent, lamports_to_sol(ata_rent));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (false, 0, 0.0)
}

fn build_swap_result(
    transaction_signature: &str,
    transaction_json: &Value,
    result: &TokenTransferData,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str,
    intended_amount: Option<f64>,
    analysis_time_ms: u64
) -> Result<SwapAnalysisResult, SwapError> {
    let (tx_fee_lamports, tx_fee_sol) = extract_transaction_fee(transaction_json);
    let platform_fee_sol = extract_platform_fee(transaction_json);
    let total_fees_sol = tx_fee_sol + platform_fee_sol.unwrap_or(0.0);
    let (ata_detected, ata_rent_lamports, ata_rent_sol) = detect_ata_creation(
        transaction_json,
        wallet_address
    );

    let effective_price = if input_mint == SOL_MINT {
        result.input_amount / result.output_amount
    } else {
        result.output_amount / result.input_amount
    };

    let (price_diff_percent, slippage_percent) = if let Some(expected) = intended_amount {
        let price_diff = ((effective_price - expected) / expected) * 100.0;
        let slippage = price_diff.abs();
        (price_diff, slippage)
    } else {
        (0.0, 0.0)
    };

    let input_raw = (result.input_amount * (10_f64).powi(result.input_decimals as i32)) as u64;
    let output_raw = (result.output_amount * (10_f64).powi(result.output_decimals as i32)) as u64;

    Ok(SwapAnalysisResult {
        success: true,
        transaction_signature: transaction_signature.to_string(),
        error_message: None,
        input_amount: result.input_amount,
        output_amount: result.output_amount,
        input_decimals: result.input_decimals,
        output_decimals: result.output_decimals,
        input_amount_raw: input_raw,
        output_amount_raw: output_raw,
        effective_price,
        expected_price: intended_amount,
        price_difference_percent: price_diff_percent,
        slippage_percent,
        transaction_fee_sol: tx_fee_sol,
        transaction_fee_lamports: tx_fee_lamports,
        platform_fee_sol,
        total_fees_sol,
        ata_creation_detected: ata_detected,
        ata_rent_lamports,
        ata_rent_sol,
        analysis_method: result.method.clone(),
        confidence_score: result.confidence,
        analysis_time_ms,
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        is_buy: input_mint == SOL_MINT,
        wallet_address: wallet_address.to_string(),
        block_height: extract_block_height(transaction_json),
        block_time: extract_block_time(transaction_json),
    })
}
