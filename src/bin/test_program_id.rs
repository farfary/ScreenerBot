/// Test program to verify program ID detection functionality

use screenerbot::pools::types::ProgramKind;
use screenerbot::rpc::get_rpc_client;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Get program type from pool address by fetching the account owner
async fn get_pool_program_info(pool_address: &str) -> (String, String) {
    let pool_pubkey = match Pubkey::from_str(pool_address) {
        Ok(pubkey) => pubkey,
        Err(_) => return ("INVALID_ADDRESS".to_string(), "unknown".to_string()),
    };

    let rpc_client = get_rpc_client();
    match rpc_client.get_account(&pool_pubkey).await {
        Ok(account) => {
            let program_id = account.owner.to_string();
            let program_kind = ProgramKind::from_program_id(&program_id);
            (program_id, program_kind.display_name().to_string())
        }
        Err(_) => ("FETCH_ERROR".to_string(), "unknown".to_string()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing program ID detection...");
    
    // Test with a few different pool addresses
    let test_pools = vec![
        "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2", // Should be a real pool
        "9d9mb8kooFfaD3SctgZtkxQypkshx6ezhbKio89ixyy2", // Should be a real pool
        "3ucNos4NbumPLZNWztqGHNFFgkHeRMBQAVemeeomsUxv", // Should be a real pool
    ];
    
    for pool_address in test_pools {
        println!("\n🔍 Testing pool: {}", pool_address);
        let (program_id, program_name) = get_pool_program_info(pool_address).await;
        
        let program_display = if program_id.len() > 8 && !program_id.starts_with("INVALID") && !program_id.starts_with("FETCH") {
            format!("{} ({}...{})", program_name, &program_id[..8], &program_id[program_id.len()-8..])
        } else {
            program_name
        };
        
        println!("  📊 Program ID: {}", program_id);
        println!("  🏷️  Program Type: {}", program_display);
    }
    
    println!("\n✅ Program ID detection test completed!");
    Ok(())
}
