use screenerbot::wallet::*;
use screenerbot::global::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configs
    let configs = read_configs("configs.json")?;

    // The problematic transaction
    let transaction_signature =
        "5Lw1frVV4jrAX8GWPtCfRjcUYNPcTv4bhppSabsycdcKiFe8GsumSNpdyxFpSLdUP3tGy38KrFv2Y5aDU3Bo5c76";
    let wallet_address = get_wallet_address()?;

    println!("\n=== DEBUGGING EFFECTIVE PRICE CALCULATION ===");
    println!("Transaction: {}", transaction_signature);
    println!("Expected entry_price from API: 0.00001937 SOL");
    println!("Calculated effective_entry_price: 0.0004184649043498479 SOL");
    println!("Difference: {:.2}x higher than expected", 0.0004184649043498479 / 0.00001937);

    // SOL to Token swap (buying stonks with SOL)
    let input_mint = SOL_MINT; // SOL
    let output_mint = "27U6sAYSDUJLpeCTTL5gW2wSwLGNRZRZKWJEqTWGbonk"; // stonks

    println!("\nAnalyzing transaction details...");

    let client = reqwest::Client::new();

    // Call the existing calculate_effective_price function to debug it
    println!("\nCalling calculate_effective_price function...");
    match
        calculate_effective_price(
            &client,
            transaction_signature,
            input_mint,
            output_mint,
            &wallet_address,
            &configs.rpc_url,
            &configs
        ).await
    {
        Ok((effective_price, actual_input_change, actual_output_change, _diff)) => {
            println!("✓ calculate_effective_price succeeded");
            println!("  Calculated effective_price: {:.15} SOL/token", effective_price);
            println!("  Expected API price: {:.15} SOL/token", 0.00001937);
            println!("  Ratio (calculated/expected): {:.2}x", effective_price / 0.00001937);
            println!("  Input change: {} lamports", actual_input_change);
            println!("  Output change: {} raw token units", actual_output_change);

            // Let's manually check what the effective price calculation is doing wrong
            println!("\n=== Manual Analysis ===");

            // The token has these details from positions.json:
            // - entry_size_sol: 0.0001
            // - token_amount: 5126547 (raw units)
            // - effective_entry_price: 0.0004184649043498479

            let expected_sol_spent = 0.0001; // From entry_size_sol
            let tokens_received_raw = 5126547; // From token_amount

            // Assume 6 decimals for this token (common for meme tokens)
            let assumed_decimals = 6;
            let ui_tokens_received = (tokens_received_raw as f64) / (10_f64).powi(assumed_decimals);

            println!("From position data:");
            println!("  Expected SOL spent: {} SOL", expected_sol_spent);
            println!("  Raw tokens received: {}", tokens_received_raw);
            println!("  Assuming {} decimals, UI tokens: {}", assumed_decimals, ui_tokens_received);
            println!(
                "  Manual calculation: {} / {} = {:.15} SOL/token",
                expected_sol_spent,
                ui_tokens_received,
                expected_sol_spent / ui_tokens_received
            );

            // Try different decimal assumptions
            for decimals in [6, 9, 18] {
                let ui_tokens = (tokens_received_raw as f64) / (10_f64).powi(decimals);
                let manual_price = expected_sol_spent / ui_tokens;
                println!(
                    "  With {} decimals: {} SOL / {} tokens = {:.15} SOL/token",
                    decimals,
                    expected_sol_spent,
                    ui_tokens,
                    manual_price
                );
            }
        }
        Err(e) => {
            println!("✗ calculate_effective_price failed: {}", e);
        }
    }

    println!("\n=== RECOMMENDATION ===");
    println!("The effective price calculation should exclude transaction fees:");
    println!("  effective_price = (sol_spent - transaction_fee) / tokens_received");
    println!("This will give a more accurate price that's closer to the API price.");

    Ok(())
}
