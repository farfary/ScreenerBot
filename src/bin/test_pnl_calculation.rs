use screenerbot::trader::position::Position;

fn main() -> anyhow::Result<()> {
    println!("🧪 Testing P&L Calculation Fix");
    
    // Create a test position
    let mut position = Position::new(
        "TestToken123".to_string(),
        "TEST".to_string()
    );
    
    // Simulate a buy trade: 0.002 SOL at 0.000001 SOL per token
    let investment = 0.002;
    let entry_price = 0.000001; // 1e-6 SOL per token
    let tokens_bought = investment / entry_price; // 2000 tokens
    
    position.add_buy_trade(investment, tokens_bought, entry_price);
    
    println!("📊 Position after buy:");
    println!("   💰 Invested: {:.6} SOL", position.total_invested_sol);
    println!("   🪙 Tokens: {:.2} tokens", position.total_tokens);
    println!("   💵 Entry Price: {:.9} SOL per token", position.average_buy_price);
    
    // Simulate price increase to 0.000002 SOL per token (2x)
    let new_price = 0.000002; // 2e-6 SOL per token
    position.update_price(new_price);
    
    println!("\n📈 After price update to {:.9} SOL per token:", new_price);
    println!("   📊 Unrealized P&L: {:.6} SOL ({:.1}%)", 
             position.unrealized_pnl_sol, 
             position.unrealized_pnl_percent);
    
    // Simulate selling all tokens at the new price
    // Gross proceeds (before fees): 2000 tokens × 0.000002 = 0.004 SOL
    let gross_proceeds = tokens_bought * new_price;
    position.add_sell_trade(gross_proceeds, tokens_bought, new_price);
    
    println!("\n🔴 After selling all tokens:");
    println!("   💰 Gross Proceeds: {:.6} SOL", gross_proceeds);
    println!("   📊 Realized P&L: {:.6} SOL", position.realized_pnl_sol);
    println!("   📈 Expected P&L: {:.6} SOL (should be {:.6})", 
             gross_proceeds - investment, 
             gross_proceeds - investment);
    
    // Verify the calculation is correct
    let expected_pnl = gross_proceeds - investment;
    let actual_pnl = position.realized_pnl_sol;
    
    if (actual_pnl - expected_pnl).abs() < 0.000001 {
        println!("✅ P&L calculation is CORRECT!");
    } else {
        println!("❌ P&L calculation is WRONG!");
        println!("   Expected: {:.6} SOL", expected_pnl);
        println!("   Actual: {:.6} SOL", actual_pnl);
    }
    
    Ok(())
}
