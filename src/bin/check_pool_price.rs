// src/bin/check_pool_price.rs
//! Usage:
//!   cargo run --bin check_pool_price <POOL_PUBKEY> [<POOL_PUBKEY> …]

use anyhow::{bail, Result};
use screenerbot::pools::decoder::decode_any_pool_price;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{env, str::FromStr};

fn main() -> Result<()> {
    // collect every CLI argument after the binary name
    let pool_keys: Vec<String> = env::args().skip(1).collect();
    if pool_keys.is_empty() {
        bail!("Provide at least one pool pubkey.");
    }

    // choose the RPC endpoint you prefer
    let rpc = RpcClient::new("https://api.mainnet-beta.solana.com");

    for arg in pool_keys {
        match Pubkey::from_str(&arg) {
            Ok(pk) => match decode_any_pool_price(&rpc, &pk) {
                Ok((base, quote, price)) => println!(
                    "✅ {arg}\n  base:  {base}\n  quote: {quote}\n  price: {price}\n"
                ),
                Err(e) => eprintln!("❌ {arg} – {e}"),
            },
            Err(e) => eprintln!("❌ invalid pubkey {arg}: {e}"),
        }
    }
    Ok(())
}
