use screenerbot::pool_price::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧮 Debugging sqrt_price calculation for Meteora DAMM v2");

    // Raw values from the test
    let sqrt_price = 128431947757712715u128;
    let token_a_decimals = 6; // BDNPD38e token
    let token_b_decimals = 9; // SOL

    println!("📊 Raw Values:");
    println!("   sqrt_price: {}", sqrt_price);
    println!("   token_a_decimals: {}", token_a_decimals);
    println!("   token_b_decimals: {}", token_b_decimals);

    // Try different sqrt_price interpretations
    println!("\n🔬 Testing different sqrt_price interpretations:");

    // Current calculation (what we're using)
    let current_price = {
        let sqrt_price_normalized = (sqrt_price as f64) / (2_f64).powi(64);
        let price_raw = sqrt_price_normalized * sqrt_price_normalized;
        let decimal_adjustment =
            (10_f64).powi(token_a_decimals as i32) / (10_f64).powi(token_b_decimals as i32);
        price_raw * decimal_adjustment
    };
    println!("1. Current calculation: {} SOL per token", current_price);

    // Alternative 1: Different Q64.64 interpretation
    let alt1_price = {
        let sqrt_price_f64 = sqrt_price as f64;
        let price_raw = (sqrt_price_f64 / (2_f64).powi(32)).powi(2);
        let decimal_adjustment =
            (10_f64).powi(token_a_decimals as i32) / (10_f64).powi(token_b_decimals as i32);
        (price_raw / (2_f64).powi(64)) * decimal_adjustment
    };
    println!("2. Alternative Q32.32: {} SOL per token", alt1_price);

    // Alternative 2: Simple sqrt_price / 2^64
    let alt2_price = {
        let price_raw = (sqrt_price as f64) / (2_f64).powi(64);
        let decimal_adjustment =
            (10_f64).powi(token_a_decimals as i32) / (10_f64).powi(token_b_decimals as i32);
        price_raw * decimal_adjustment
    };
    println!("3. Linear sqrt_price: {} SOL per token", alt2_price);

    // Alternative 3: Inverted orientation (token B / token A)
    let alt3_price = {
        let sqrt_price_normalized = (sqrt_price as f64) / (2_f64).powi(64);
        let price_raw = sqrt_price_normalized * sqrt_price_normalized;
        let decimal_adjustment =
            (10_f64).powi(token_b_decimals as i32) / (10_f64).powi(token_a_decimals as i32);
        price_raw * decimal_adjustment
    };
    println!("4. Inverted orientation: {} SOL per token", alt3_price);

    // Alternative 4: Much smaller Q notation (maybe Q128.128 or similar)
    let alt4_price = {
        let sqrt_price_normalized = (sqrt_price as f64) / (2_f64).powi(128);
        let price_raw = sqrt_price_normalized * sqrt_price_normalized;
        let decimal_adjustment =
            (10_f64).powi(token_a_decimals as i32) / (10_f64).powi(token_b_decimals as i32);
        price_raw * decimal_adjustment
    };
    println!("5. Q128.128 format: {} SOL per token", alt4_price);

    // Alternative 5: No decimal adjustment
    let alt5_price = {
        let sqrt_price_normalized = (sqrt_price as f64) / (2_f64).powi(64);
        sqrt_price_normalized * sqrt_price_normalized
    };
    println!("6. No decimal adjustment: {} raw price", alt5_price);

    // Alternative 6: Different scaling entirely
    let alt6_price = {
        let price_raw = (sqrt_price as f64) / (10_f64).powi(18); // Maybe it's already in a different format
        price_raw
    };
    println!("7. Direct scaling: {} SOL per token", alt6_price);

    println!("\n🎯 Target DexScreener price: ~0.00000004 SOL per token");
    println!("   Looking for calculation closest to this value...");

    // Compare to target
    let target = 0.00000004;
    let differences = vec![
        ("Current", (current_price - target).abs()),
        ("Q32.32", (alt1_price - target).abs()),
        ("Linear", (alt2_price - target).abs()),
        ("Inverted", (alt3_price - target).abs()),
        ("Q128.128", (alt4_price - target).abs()),
        ("No decimals", (alt5_price - target).abs()),
        ("Direct", (alt6_price - target).abs())
    ];

    println!("\n📈 Differences from target:");
    for (name, diff) in differences {
        println!("   {}: {:.2e}", name, diff);
    }

    Ok(())
}
