use std::fs;
use serde::{Serialize, Deserialize};
use serde_json;

// Copy the TokenBalance and related structs from rpc.rs
#[derive(Debug, Serialize, Deserialize)]
struct TokenBalance {
    #[serde(rename = "accountIndex")]
    account_index: u32,
    mint: String,
    owner: Option<String>,
    #[serde(rename = "programId")]
    program_id: Option<String>,
    #[serde(rename = "uiTokenAmount")]
    ui_token_amount: UiTokenAmount,
}

#[derive(Debug, Serialize, Deserialize)]
struct UiTokenAmount {
    amount: String,
    decimals: u8,
    #[serde(rename = "uiAmount")]
    ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    ui_amount_string: Option<String>,
}

fn main() {
    let file_path = "/Users/farhad/Desktop/ScreenerBot/data/transactions/dy2Rnm3MsysvW2oijXuMtsN6JGpXb5Fu6u29jyhPL8Ft5fg1n51MAqBetKxS3K4LZ5ewHhGhsgGST1Bs1P6ipzB.json";
    
    println!("🔍 Testing token balance conversion");
    
    let content = fs::read_to_string(file_path).expect("Failed to read file");
    let json_data: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse JSON");
    
    let meta = json_data.get("transaction_data").unwrap().get("meta").unwrap();
    
    println!("📊 Meta keys: {:?}", meta.as_object().unwrap().keys().collect::<Vec<_>>());
    
    // Test preTokenBalances conversion
    if let Some(pre_balances_json) = meta.get("preTokenBalances") {
        println!("📊 Pre token balances JSON type: {:?}", pre_balances_json.is_array());
        if let Some(pre_array) = pre_balances_json.as_array() {
            println!("📊 Pre token balances array length: {}", pre_array.len());
            
            // Try to convert manually
            let mut pre_balances = Vec::new();
            for (i, balance_json) in pre_array.iter().enumerate() {
                match serde_json::from_value::<TokenBalance>(balance_json.clone()) {
                    Ok(balance) => {
                        println!("✅ Pre balance {}: mint={}, owner={:?}", i, balance.mint, balance.owner);
                        pre_balances.push(balance);
                    }
                    Err(e) => {
                        println!("❌ Failed to convert pre balance {}: {}", i, e);
                        println!("   JSON: {}", balance_json);
                    }
                }
            }
            println!("📊 Successfully converted {} pre balances", pre_balances.len());
        }
    }
    
    // Test postTokenBalances conversion
    if let Some(post_balances_json) = meta.get("postTokenBalances") {
        println!("\n📊 Post token balances JSON type: {:?}", post_balances_json.is_array());
        if let Some(post_array) = post_balances_json.as_array() {
            println!("📊 Post token balances array length: {}", post_array.len());
            
            // Try to convert manually
            let mut post_balances = Vec::new();
            for (i, balance_json) in post_array.iter().enumerate() {
                match serde_json::from_value::<TokenBalance>(balance_json.clone()) {
                    Ok(balance) => {
                        println!("✅ Post balance {}: mint={}, owner={:?}", i, balance.mint, balance.owner);
                        if balance.owner.as_ref().map(|o| o == "FYmfcfwyx8K1MnBmk6d66eeNPoPMbTXEMve5Tk1pGgiC").unwrap_or(false) {
                            println!("   🎯 THIS IS THE WALLET'S TOKEN BALANCE!");
                            println!("   Amount: {}, UI Amount: {:?}", balance.ui_token_amount.amount, balance.ui_token_amount.ui_amount);
                        }
                        post_balances.push(balance);
                    }
                    Err(e) => {
                        println!("❌ Failed to convert post balance {}: {}", i, e);
                        println!("   JSON: {}", balance_json);
                    }
                }
            }
            println!("📊 Successfully converted {} post balances", post_balances.len());
        }
    }
}
