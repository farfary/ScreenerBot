use anyhow::Result;
use screenerbot::trader::database::TraderDatabase;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Checking for duplicate positions in database...");
    
    let database = TraderDatabase::new("trader.db")?;
    let active_positions = database.get_active_positions()?;
    
    println!("\n📊 Active positions in database:");
    for (id, summary) in &active_positions {
        println!("ID: {}, Token: {}, Symbol: {}, Invested: {:.5} SOL", 
                 id, summary.token_address, summary.token_symbol, summary.total_invested_sol);
    }
    
    // Check for duplicates by token address
    let mut token_counts = std::collections::HashMap::new();
    for (id, summary) in &active_positions {
        let count = token_counts.entry(summary.token_address.clone()).or_insert(Vec::new());
        count.push(*id);
    }
    
    println!("\n🔍 Checking for duplicate tokens:");
    let mut found_duplicates = false;
    for (token, ids) in token_counts {
        if ids.len() > 1 {
            println!("❌ DUPLICATE: Token {} has {} positions with IDs: {:?}", 
                     token, ids.len(), ids);
            found_duplicates = true;
        }
    }
    
    if !found_duplicates {
        println!("✅ No duplicate positions found");
    }
    
    Ok(())
}
