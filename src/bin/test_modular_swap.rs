/// Example usage of the new modular swap system
///
/// This binary demonstrates how to use the new direct swap functionality
/// integrated into the pools module.

use screenerbot::pools::swap::{ SwapBuilder, SwapDirection };
use screenerbot::arguments::{ get_arg_value, has_arg, set_cmd_args };
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set command line arguments for global access
    set_cmd_args(std::env::args().collect());

    if has_arg("--help") || has_arg("-h") {
        print_help();
        return Ok(());
    }

    log(LogTag::System, "STARTUP", "🚀 Testing New Modular Swap System");

    // Parse command line arguments
    let pool_address = get_arg_value("--pool").ok_or(
        "Pool address is required. Use --pool <address>"
    )?;
    let token_mint = get_arg_value("--token").ok_or("Token mint is required. Use --token <mint>")?;
    let amount_str = get_arg_value("--amount").ok_or(
        "Amount is required. Use --amount <amount_in_sol>"
    )?;
    let amount_sol: f64 = amount_str.parse().map_err(|_| "Invalid amount format")?;

    let direction = if has_arg("--sell") { SwapDirection::Sell } else { SwapDirection::Buy };

    let dry_run = has_arg("--dry-run");

    log(
        LogTag::System,
        "INFO",
        &format!(
            "📋 Swap Configuration:
        Pool: {}
        Token: {}
        Amount: {} {}
        Direction: {:?}
        Dry Run: {}",
            pool_address,
            token_mint,
            amount_sol,
            match direction {
                SwapDirection::Buy => "SOL",
                SwapDirection::Sell => "tokens",
            },
            direction,
            dry_run
        )
    );

    // Use the new swap builder API
    let result = SwapBuilder::new()
        .pool_address(&pool_address)?
        .token_mint(&token_mint)?
        .amount(amount_sol)
        .direction(direction)
        .slippage_percent(1.0) // 1% slippage
        .execute().await?;

    // Display result
    if result.success {
        log(
            LogTag::System,
            "SUCCESS",
            &format!(
                "✅ Swap completed successfully!
                Input: {:.6}
                Output: {:.6} (min: {:.6})
                Signature: {:?}",
                result.params.input_amount,
                result.params.expected_output,
                result.params.minimum_output,
                result.signature
            )
        );
    } else {
        log(LogTag::System, "ERROR", &format!("❌ Swap failed: {:?}", result.error));
    }

    Ok(())
}

fn print_help() {
    println!("Modular Direct Swap System Test");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin test_modular_swap [FLAGS] [OPTIONS]");
    println!();
    println!("REQUIRED OPTIONS:");
    println!("    --pool <ADDRESS>       Pool address to swap in");
    println!("    --token <MINT>         Token mint address");
    println!("    --amount <AMOUNT>      Amount in SOL");
    println!();
    println!("FLAGS:");
    println!("    --sell                 Sell tokens for SOL (default: buy)");
    println!("    --dry-run              Don't send transaction, just build it");
    println!("    --help, -h             Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    # Buy tokens with 0.01 SOL");
    println!(
        "    cargo run --bin test_modular_swap -- --pool 2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ --token 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t --amount 0.01"
    );
    println!();
    println!("    # Sell tokens worth ~0.01 SOL (specify token amount)");
    println!(
        "    cargo run --bin test_modular_swap -- --pool 2SNwf41oZyqVyCuX6PtZCenCnTWzsDR2bcqQzMPyp1NQ --token 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t --amount 1000 --sell"
    );
}
