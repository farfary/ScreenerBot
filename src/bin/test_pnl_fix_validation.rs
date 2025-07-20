use screenerbot::{
    global::{ read_configs, Token },
    trader::{ Position, calculate_position_pnl_from_swaps },
    logger::{ log, LogTag },
};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing P&L Calculation Fix");
    println!("===============================\n");

    // Example position with real swap data
    let position = Position {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "SOL".to_string(),
        name: "Solana".to_string(),
        entry_price: 0.00001, // Original DexScreener price
        entry_time: Utc::now(),
        exit_price: None,
        exit_time: None,
        pnl_sol: None,
        pnl_percent: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.0005, // 0.0005 SOL invested
        total_size_sol: 0.0005,
        drawdown_percent: 0.0,
        price_highest: 0.00001,
        price_lowest: 0.00001,
        entry_transaction_signature: Some("example_tx".to_string()),
        exit_transaction_signature: None,
        token_amount: Some(50_000_000), // 50M raw tokens (with 6 decimals = 50 UI tokens)
        effective_entry_price: Some(0.00001), // Actual on-chain price
        effective_exit_price: None,
    };

    let current_price = 0.000012; // 20% price increase
    let token_decimals = Some(6u8);

    println!("📊 Position Details:");
    println!("   Symbol: {}", position.symbol);
    println!("   Entry Size: {:.6} SOL", position.entry_size_sol);
    println!("   Original Entry Price: {:.8} SOL", position.entry_price);
    println!("   Effective Entry Price: {:.8} SOL", position.effective_entry_price.unwrap());
    println!("   Token Amount (raw): {}", position.token_amount.unwrap());
    println!("   Token Decimals: {}", token_decimals.unwrap());
    println!("   Current Price: {:.8} SOL", current_price);
    println!();

    // Calculate UI token amount
    let ui_tokens =
        (position.token_amount.unwrap() as f64) / (10_f64).powi(token_decimals.unwrap() as i32);
    println!("🔢 Token Amount Calculation:");
    println!("   Raw tokens: {}", position.token_amount.unwrap());
    println!("   UI tokens: {:.2}", ui_tokens);
    println!();

    // Test the new accurate P&L calculation
    let (pnl_sol, pnl_percent) = calculate_position_pnl_from_swaps(
        &position,
        current_price,
        token_decimals
    );

    println!("💰 P&L Calculation Results:");
    println!(
        "   Current value of tokens: {:.6} SOL ({:.2} tokens × {:.8} SOL/token)",
        ui_tokens * current_price,
        ui_tokens,
        current_price
    );
    println!("   Initial investment: {:.6} SOL", position.entry_size_sol);
    println!("   Net P&L: {:.6} SOL", pnl_sol);
    println!("   P&L Percentage: {:.2}%", pnl_percent);
    println!();

    // Manual verification
    let expected_current_value = ui_tokens * current_price;
    let expected_pnl = expected_current_value - position.entry_size_sol;
    let expected_pnl_percent = (expected_pnl / position.entry_size_sol) * 100.0;

    println!("✅ Manual Verification:");
    println!("   Expected current value: {:.6} SOL", expected_current_value);
    println!("   Expected P&L: {:.6} SOL", expected_pnl);
    println!("   Expected P&L %: {:.2}%", expected_pnl_percent);
    println!();

    // Check if calculations match
    let sol_match = (pnl_sol - expected_pnl).abs() < 0.000001;
    let percent_match = (pnl_percent - expected_pnl_percent).abs() < 0.01;

    if sol_match && percent_match {
        println!("🎉 SUCCESS: P&L calculations are accurate!");
        println!("   SOL P&L matches: ✅");
        println!("   Percentage P&L matches: ✅");
    } else {
        println!("❌ ERROR: P&L calculations don't match expected values");
        println!("   SOL P&L matches: {}", if sol_match { "✅" } else { "❌" });
        println!("   Percentage P&L matches: {}", if percent_match { "✅" } else { "❌" });
    }

    println!("\n🔍 Key Benefits of Fixed P&L Calculation:");
    println!("   • Uses actual token amounts from swap transactions");
    println!("   • Uses effective entry prices from on-chain data");
    println!("   • Accounts for slippage and fees accurately");
    println!("   • No longer relies on theoretical price calculations");
    println!("   • Handles token decimals correctly");

    Ok(())
}
