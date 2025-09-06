use screenerbot::rpc::get_rpc_client;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    let rpc = get_rpc_client();

    let vaults = vec![
        ("Current SOL vault (offset 264)", "2sDMe65gZVeZjGWa872wZWK4XMBMFaxtxqCMeXCPSTw5"),
        ("Alternative vault 1 (offset 250)", "GJdLqXjxU4X2HcVfLTkdkiSpPZw8uTEo4pfUJbg23EkU"),
        ("Alternative vault 2 (offset 282)", "FA6aKDuPum7KXam5JfH3yftv1avCMpKmxgD6JwfdPYdD"),
        ("Current token vault (offset 232)", "HF5MVZdr3gBFWdKNpbbAQrR1zYe9QV6yFoutD5yjLJbN")
    ];

    for (name, addr) in vaults {
        if let Ok(pubkey) = Pubkey::from_str(addr) {
            match rpc.get_account(&pubkey).await {
                Ok(account) => {
                    if account.data.len() >= 64 {
                        // Try to decode as token account
                        let amount_bytes = &account.data[64..72];
                        let amount = u64::from_le_bytes(
                            amount_bytes.try_into().unwrap_or([0u8; 8])
                        );
                        let amount_ui = (amount as f64) / 1e9; // Assume 9 decimals for SOL-like

                        // Try to get mint info
                        let mint_bytes = &account.data[0..32];
                        let mint = Pubkey::new_from_array(
                            mint_bytes.try_into().unwrap_or([0u8; 32])
                        );

                        println!(
                            "{}: {} -> Balance: {} raw ({:.6} UI) | Mint: {}",
                            name,
                            addr,
                            amount,
                            amount_ui,
                            mint
                        );
                    } else {
                        println!(
                            "{}: {} -> Not a token account (len={})",
                            name,
                            addr,
                            account.data.len()
                        );
                    }
                }
                Err(e) => {
                    println!("{}: {} -> Error: {}", name, addr, e);
                }
            }
        }
    }
}
