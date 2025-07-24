use anyhow::Result;
use screenerbot::pool_price::PoolDiscoveryAndPricing;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Debug: Raw Meteora DAMM v2 Pool Data Analysis");
    println!("=================================================");

    // Load configuration
    let configs_json = std::fs
        ::read_to_string("configs.json")
        .expect("Failed to read configs.json");
    let configs: serde_json::Value = serde_json
        ::from_str(&configs_json)
        .expect("Failed to parse configs.json");

    let rpc_url = configs["rpc_url"].as_str().expect("rpc_url not found in configs.json");

    let rpc_client = RpcClient::new(rpc_url.to_string());

    // The problematic pool
    let pool_address = "GAAxbSVm3sLKjFnAhQCmFjDPXkmbWWYP1F3xrN5LQs4n";
    let pool_pubkey = Pubkey::from_str(pool_address)?;

    println!("Pool Address: {}", pool_address);
    println!("Getting pool account data...\n");

    let account_info = rpc_client.get_account(&pool_pubkey)?;
    let data = &account_info.data;

    println!("Account Owner: {}", account_info.owner);
    println!("Account Data Size: {} bytes", data.len());

    // Based on the JSON structure you provided, let's manually examine the data
    println!("\n📊 Raw Data Analysis:");
    println!("======================");

    // First, let's examine the discriminator and initial structure
    if data.len() >= 8 {
        println!("First 8 bytes (discriminator): {:02X?}", &data[0..8]);
    }

    // Let's examine data around likely field positions
    let positions_to_check = vec![
        (192, "token_a_mint candidate 1"),
        (200, "token_a_mint candidate 2"),
        (208, "token_a_mint candidate 3"),
        (216, "token_a_mint candidate 4"),
        (224, "token_b_mint candidate 1"),
        (232, "token_b_mint candidate 2"),
        (240, "token_b_mint candidate 3"),
        (248, "token_b_mint candidate 4"),
        (256, "token_a_vault candidate 1"),
        (264, "token_a_vault candidate 2"),
        (272, "token_a_vault candidate 3"),
        (280, "token_a_vault candidate 4"),
        (288, "token_b_vault candidate 1"),
        (296, "token_b_vault candidate 2"),
        (304, "token_b_vault candidate 3"),
        (312, "token_b_vault candidate 4")
    ];

    for (offset, description) in positions_to_check {
        if data.len() >= offset + 32 {
            let pubkey_bytes: [u8; 32] = data[offset..offset + 32].try_into().unwrap();
            let pubkey = Pubkey::new_from_array(pubkey_bytes);
            println!("Offset {}: {} -> {}", offset, description, pubkey);
        }
    }

    // Let's also check some known values from the JSON you provided
    println!("\n🎯 Searching for Known Values:");
    println!("==============================");

    // Look for the token mints we know from the JSON:
    // token_a_mint: BDNPD38erhzRmu5qYLTLFAwmyyW5UvGryUu6TsJFpump
    // token_b_mint: So11111111111111111111111111111111111111112

    let expected_token_a = "BDNPD38erhzRmu5qYLTLFAwmyyW5UvGryUu6TsJFpump";
    let expected_token_b = "So11111111111111111111111111111111111111112";
    let expected_token_a_vault = "C1nQuvxfRdp9vDyGFCihpXuCU3q1uCewqo28JYE7Kwqp";
    let expected_token_b_vault = "2WuPMhneDhuGoapi5DtDec9ry3V9q1YPB45ij1NAQGyP";

    println!("Searching for token_a_mint: {}", expected_token_a);
    if let Ok(token_a_pubkey) = Pubkey::from_str(expected_token_a) {
        let token_a_bytes = token_a_pubkey.to_bytes();
        if let Some(pos) = find_bytes_in_data(data, &token_a_bytes) {
            println!("✅ Found token_a_mint at offset: {}", pos);
        } else {
            println!("❌ token_a_mint not found in data");
        }
    }

    println!("Searching for token_b_mint: {}", expected_token_b);
    if let Ok(token_b_pubkey) = Pubkey::from_str(expected_token_b) {
        let token_b_bytes = token_b_pubkey.to_bytes();
        if let Some(pos) = find_bytes_in_data(data, &token_b_bytes) {
            println!("✅ Found token_b_mint at offset: {}", pos);
        } else {
            println!("❌ token_b_mint not found in data");
        }
    }

    println!("Searching for token_a_vault: {}", expected_token_a_vault);
    if let Ok(token_a_vault_pubkey) = Pubkey::from_str(expected_token_a_vault) {
        let token_a_vault_bytes = token_a_vault_pubkey.to_bytes();
        if let Some(pos) = find_bytes_in_data(data, &token_a_vault_bytes) {
            println!("✅ Found token_a_vault at offset: {}", pos);
        } else {
            println!("❌ token_a_vault not found in data");
        }
    }

    println!("Searching for token_b_vault: {}", expected_token_b_vault);
    if let Ok(token_b_vault_pubkey) = Pubkey::from_str(expected_token_b_vault) {
        let token_b_vault_bytes = token_b_vault_pubkey.to_bytes();
        if let Some(pos) = find_bytes_in_data(data, &token_b_vault_bytes) {
            println!("✅ Found token_b_vault at offset: {}", pos);
        } else {
            println!("❌ token_b_vault not found in data");
        }
    }

    // Let's also search for the liquidity value: 2247605956342671431615596450
    println!("\nSearching for liquidity value: 2247605956342671431615596450");
    let liquidity_value: u128 = 2247605956342671431615596450;
    let liquidity_bytes = liquidity_value.to_le_bytes();
    if let Some(pos) = find_bytes_in_data(data, &liquidity_bytes) {
        println!("✅ Found liquidity at offset: {}", pos);
    } else {
        println!("❌ liquidity value not found in data");
    }

    // And the sqrt_price: 128431947757712715
    println!("Searching for sqrt_price value: 128431947757712715");
    let sqrt_price_value: u128 = 128431947757712715;
    let sqrt_price_bytes = sqrt_price_value.to_le_bytes();
    if let Some(pos) = find_bytes_in_data(data, &sqrt_price_bytes) {
        println!("✅ Found sqrt_price at offset: {}", pos);
    } else {
        println!("❌ sqrt_price value not found in data");
    }

    // Let's examine the structure by showing hex dump of the first 1000 bytes
    println!("\n🔍 Hex Dump Analysis (first 1000 bytes):");
    println!("==========================================");
    hex_dump(&data[0..std::cmp::min(1000, data.len())]);

    Ok(())
}

fn find_bytes_in_data(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len()).position(|window| window == pattern)
}

fn hex_dump(data: &[u8]) {
    const BYTES_PER_LINE: usize = 16;

    for (i, chunk) in data.chunks(BYTES_PER_LINE).enumerate() {
        let offset = i * BYTES_PER_LINE;
        print!("{:08X}: ", offset);

        // Print hex bytes
        for (j, byte) in chunk.iter().enumerate() {
            print!("{:02X} ", byte);
            if j == 7 {
                print!(" "); // Extra space in the middle
            }
        }

        // Pad if line is incomplete
        for _ in chunk.len()..BYTES_PER_LINE {
            print!("   ");
            if chunk.len() <= 8 {
                print!(" ");
            }
        }

        // Print ASCII representation
        print!(" |");
        for byte in chunk {
            let c = if byte.is_ascii_graphic() || *byte == b' ' { *byte as char } else { '.' };
            print!("{}", c);
        }
        println!("|");
    }
}
