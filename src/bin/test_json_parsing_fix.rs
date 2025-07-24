// test_json_parsing_fix.rs - Test the JSON parsing fix for slippageBps and sol_cost fields
use screenerbot::wallet::{ SwapQuote, SwapData, RawTransaction };
use serde_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing JSON parsing fix for slippageBps and sol_cost fields...");

    // Test case 1: slippageBps as integer (from the error message)
    let json_with_int_slippage =
        r#"
    {
        "inputMint": "3oqcUejEoAjGKcqBRs98XRmB4grsBk2rjjPZS7wEbonk",
        "inAmount": "383425912",
        "outputMint": "So11111111111111111111111111111111111111112",
        "outAmount": "956381",
        "otherAmountThreshold": "908561",
        "inDecimals": 6,
        "outDecimals": 9,
        "swapMode": "ExactIn",
        "slippageBps": 500,
        "platformFee": "9564",
        "priceImpactPct": "0.14",
        "routePlan": [],
        "timeTaken": 0.316
    }
    "#;

    println!("\n1. Testing slippageBps as integer...");
    let quote_result: Result<SwapQuote, _> = serde_json::from_str(json_with_int_slippage);
    match quote_result {
        Ok(quote) => {
            println!("✅ Successfully parsed slippageBps as integer: {}", quote.slippage_bps);
        }
        Err(e) => {
            println!("❌ Failed to parse slippageBps as integer: {}", e);
        }
    }

    // Test case 2: slippageBps as string (original format)
    let json_with_string_slippage =
        r#"
    {
        "inputMint": "3oqcUejEoAjGKcqBRs98XRmB4grsBk2rjjPZS7wEbonk",
        "inAmount": "383425912",
        "outputMint": "So11111111111111111111111111111111111111112",
        "outAmount": "956381",
        "otherAmountThreshold": "908561",
        "inDecimals": 6,
        "outDecimals": 9,
        "swapMode": "ExactIn",
        "slippageBps": "500",
        "platformFee": "9564",
        "priceImpactPct": "0.14",
        "routePlan": [],
        "timeTaken": 0.316
    }
    "#;

    println!("\n2. Testing slippageBps as string...");
    let quote_result: Result<SwapQuote, _> = serde_json::from_str(json_with_string_slippage);
    match quote_result {
        Ok(quote) => {
            println!("✅ Successfully parsed slippageBps as string: {}", quote.slippage_bps);
        }
        Err(e) => {
            println!("❌ Failed to parse slippageBps as string: {}", e);
        }
    }

    // Test case 3: sol_cost as integer (from the error message)
    let json_with_int_sol_cost =
        r#"
    {
        "quote": {
            "inputMint": "3oqcUejEoAjGKcqBRs98XRmB4grsBk2rjjPZS7wEbonk",
            "inAmount": "383425912",
            "outputMint": "So11111111111111111111111111111111111111112",
            "outAmount": "956381",
            "otherAmountThreshold": "908561",
            "inDecimals": 6,
            "outDecimals": 9,
            "swapMode": "ExactIn",
            "slippageBps": 500,
            "platformFee": "9564",
            "priceImpactPct": "0.14",
            "routePlan": [],
            "timeTaken": 0.316
        },
        "raw_tx": {
            "swapTransaction": "AbhcR6grmy8wUqM5IjMeXjUssEV",
            "lastValidBlockHeight": 355507895,
            "prioritizationFeeLamports": 0,
            "recentBlockhash": "6ra6BaDgdq3Fspmd1XQBar2QNHh5BwmpEYJtRbK3n13e",
            "version": "0"
        },
        "amount_in_usd": "0.17886032",
        "amount_out_usd": "0.17893010",
        "sol_cost": 2044280,
        "jito_order_id": null
    }
    "#;

    println!("\n3. Testing sol_cost as integer...");
    let swap_data_result: Result<SwapData, _> = serde_json::from_str(json_with_int_sol_cost);
    match swap_data_result {
        Ok(swap_data) => {
            println!("✅ Successfully parsed sol_cost as integer: {:?}", swap_data.sol_cost);
        }
        Err(e) => {
            println!("❌ Failed to parse sol_cost as integer: {}", e);
        }
    }

    // Test case 4: sol_cost as string (original format)
    let json_with_string_sol_cost =
        r#"
    {
        "quote": {
            "inputMint": "3oqcUejEoAjGKcqBRs98XRmB4grsBk2rjjPZS7wEbonk",
            "inAmount": "383425912",
            "outputMint": "So11111111111111111111111111111111111111112",
            "outAmount": "956381",
            "otherAmountThreshold": "908561",
            "inDecimals": 6,
            "outDecimals": 9,
            "swapMode": "ExactIn",
            "slippageBps": "500",
            "platformFee": "9564",
            "priceImpactPct": "0.14",
            "routePlan": [],
            "timeTaken": 0.316
        },
        "raw_tx": {
            "swapTransaction": "AbhcR6grmy8wUqM5IjMeXjUssEV",
            "lastValidBlockHeight": 355507895,
            "prioritizationFeeLamports": 0,
            "recentBlockhash": "6ra6BaDgdq3Fspmd1XQBar2QNHh5BwmpEYJtRbK3n13e",
            "version": "0"
        },
        "amount_in_usd": "0.17886032",
        "amount_out_usd": "0.17893010",
        "sol_cost": "2044280",
        "jito_order_id": null
    }
    "#;

    println!("\n4. Testing sol_cost as string...");
    let swap_data_result: Result<SwapData, _> = serde_json::from_str(json_with_string_sol_cost);
    match swap_data_result {
        Ok(swap_data) => {
            println!("✅ Successfully parsed sol_cost as string: {:?}", swap_data.sol_cost);
        }
        Err(e) => {
            println!("❌ Failed to parse sol_cost as string: {}", e);
        }
    }

    // Test case 5: sol_cost as null
    let json_with_null_sol_cost =
        r#"
    {
        "quote": {
            "inputMint": "3oqcUejEoAjGKcqBRs98XRmB4grsBk2rjjPZS7wEbonk",
            "inAmount": "383425912",
            "outputMint": "So11111111111111111111111111111111111111112",
            "outAmount": "956381",
            "otherAmountThreshold": "908561",
            "inDecimals": 6,
            "outDecimals": 9,
            "swapMode": "ExactIn",
            "slippageBps": "500",
            "platformFee": "9564",
            "priceImpactPct": "0.14",
            "routePlan": [],
            "timeTaken": 0.316
        },
        "raw_tx": {
            "swapTransaction": "AbhcR6grmy8wUqM5IjMeXjUssEV",
            "lastValidBlockHeight": 355507895,
            "prioritizationFeeLamports": 0,
            "recentBlockhash": "6ra6BaDgdq3Fspmd1XQBar2QNHh5BwmpEYJtRbK3n13e",
            "version": "0"
        },
        "amount_in_usd": "0.17886032",
        "amount_out_usd": "0.17893010",
        "sol_cost": null,
        "jito_order_id": null
    }
    "#;

    println!("\n5. Testing sol_cost as null...");
    let swap_data_result: Result<SwapData, _> = serde_json::from_str(json_with_null_sol_cost);
    match swap_data_result {
        Ok(swap_data) => {
            println!("✅ Successfully parsed sol_cost as null: {:?}", swap_data.sol_cost);
        }
        Err(e) => {
            println!("❌ Failed to parse sol_cost as null: {}", e);
        }
    }

    println!("\n✅ All JSON parsing tests completed!");
    Ok(())
}
