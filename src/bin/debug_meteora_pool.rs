use base64::{ Engine, engine::general_purpose };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool_address = "EFEsU5nqEQVnjX3YT4fFggmu7PnT9vVam4uhM7sqXoCk";
    let token_mint = "1Z2N6pyhJzkzhWwCn6CfzW2siW8eoGrJEAwMnkTDMmp";

    println!("🔍 Analyzing Meteora DAMM v2 Pool");
    println!("Pool Address: {}", pool_address);
    println!("Token Mint: {}", token_mint);
    println!("Owner: cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
    println!("Space: 1112 bytes");
    println!("");

    // Base64 data from the account
    let base64_data =
        "8ZptBBGxbbxAQg8AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFAAUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACQ1h73mzvlsL6BFMqwzp1wL8lB9fmfCr6HCmVlNF5cGm4hX/quBhPtof2NGGMA12sQ53BrrO1WYoPAAAAAAAWEDAs0NBmh9UWVeFa98AqKmQPRtoWb/Q6L/FCIhlVM9kfG0+EeR9vfQER6UkErM5myNfn1TjRG9ZXQqgJJWhWIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAANpjaB9yhrzIBnGeLCtQogFXJDv8lmixFSC8U4Q+3NsIRhckNNWhBtcqWIt1CAAAAAAAAAAAAAAAAAAAAAAAAABXnxsMAgAAAKr9BwAAAAAAAAAAAAAAAAAAAAAAAAAAAFA7AQABAAAAAAAAAAAAAACbV2lOqRpchLHE/v8AAAAAH0sr55SwEgAAAAAAAAAAAEFtwGgAAAAAAQAAAAABAAB+P7pxVPybrAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAOfPk4PEAwEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAARhckNNWhBtcqWIt1CAAAALJ9bjAIAAAAAAAAAAAAAAAs9x8AAAAAAAAAAAAAAAAAV58bDAIAAACq/QcAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAA2mNoH3KGvMgGcZ4sK1CiAVckO/yWaLEVILxThD7c2wgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    let data = general_purpose::STANDARD.decode(base64_data)?;
    println!("📊 Account Data Analysis (1112 bytes)");
    println!("Raw data length: {} bytes", data.len());
    println!("");

    // Look for token mint in the data
    let token_pubkey = Pubkey::from_str(token_mint)?;
    let token_bytes = token_pubkey.to_bytes();

    println!("🔍 Searching for token mint {} in account data...", token_mint);
    println!("Token bytes: {}", bytes_to_hex(&token_bytes));

    for i in 0..data.len().saturating_sub(32) {
        if &data[i..i + 32] == token_bytes {
            println!("✅ Found token mint at offset: {}", i);
        }
    }

    // Let's also look for common token addresses
    let sol = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
    let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;

    for i in 0..data.len().saturating_sub(32) {
        let chunk = &data[i..i + 32];
        if chunk == sol.to_bytes() {
            println!("✅ Found SOL at offset: {}", i);
        }
        if chunk == usdc.to_bytes() {
            println!("✅ Found USDC at offset: {}", i);
        }
    }

    // Print first 200 bytes in hex for analysis
    println!("");
    println!("📋 First 200 bytes in hex:");
    for i in (0..std::cmp::min(200, data.len())).step_by(32) {
        let end = std::cmp::min(i + 32, data.len());
        let chunk = &data[i..end];
        println!("Offset {}: {}", i, bytes_to_hex(chunk));

        // Try to interpret as pubkey
        if chunk.len() == 32 {
            if let Ok(pubkey) = Pubkey::try_from(chunk) {
                println!("         -> Pubkey: {}", pubkey);
            }
        }
    }

    Ok(())
}
