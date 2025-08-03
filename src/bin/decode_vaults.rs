use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() {
    // Expected vault addresses from pool data
    let expected_base_vault = "F6iWqisguZYprVwp916BgGR7d5ahP6Ev5E213k8y3MEb"; // SOL vault
    let expected_quote_vault = "7bxbfwXi1CY7zWUXW35PBMZjhPD27SarVuHaehMzR2Fn"; // Token vault

    println!("Expected vault addresses:");
    println!("Base vault (SOL): {}", expected_base_vault);
    println!("Quote vault (Token): {}", expected_quote_vault);
    println!();

    // Decode to bytes and show hex representation
    if let Ok(base_pubkey) = Pubkey::from_str(expected_base_vault) {
        let bytes = base_pubkey.to_bytes();
        let hex_str: String = bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        println!("Base vault hex: {}", hex_str);
    }

    if let Ok(quote_pubkey) = Pubkey::from_str(expected_quote_vault) {
        let bytes = quote_pubkey.to_bytes();
        let hex_str: String = bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        println!("Quote vault hex: {}", hex_str);
    }

    // Show first 8 bytes for pattern matching
    if let Ok(base_pubkey) = Pubkey::from_str(expected_base_vault) {
        let bytes = base_pubkey.to_bytes();
        let pattern: String = bytes[..8]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        println!("Base vault pattern (first 8 bytes): {}", pattern);
    }

    if let Ok(quote_pubkey) = Pubkey::from_str(expected_quote_vault) {
        let bytes = quote_pubkey.to_bytes();
        let pattern: String = bytes[..8]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        println!("Quote vault pattern (first 8 bytes): {}", pattern);
    }
}
