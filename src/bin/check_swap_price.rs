use screenerbot::prelude::*;

use anyhow::{Context, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 {
        eprintln!(
            "USAGE:\n  cargo run --bin check_swap_price <TX_SIG> <WALLET_PK> <TOKEN_MINT> <LAMPORTS_IN>\n\
             example:\n  cargo run --bin check_swap_price 5Yp… myWalletPk GtfNv… 10000000"
        );
        std::process::exit(1);
    }

    let tx_sig      = &args[0];
    let wallet      = Pubkey::from_str(&args[1]).context("bad wallet pk")?;
    let token_mint  = Pubkey::from_str(&args[2]).context("bad mint")?;
    let lamports_in = args[3].parse::<u64>().context("LAMPORTS_IN must be integer")?;

    let rpc = RpcClient::new("https://api.mainnet-beta.solana.com");

    let price = effective_swap_price(&rpc, tx_sig, &wallet, &token_mint, lamports_in)?;
    println!("✅ FINAL effective price: {price:.12} SOL");

    Ok(())
}
