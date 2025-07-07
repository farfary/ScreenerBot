//! Update token info for a given SPL token mint:
//! – finds every Solana pool via DexScreener
//! – picks the pool with the largest combined reserves
//! – prints its price (quote / base)
//
//! Usage:
//!     cargo run --bin token_info_update <TOKEN_MINT>

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;


fn main() -> Result<()> {
    let token_mint = std::env::args()
        .nth(1)
        .expect("usage: token_info_update <TOKEN_MINT>");

    // Preferred RPC
    let rpc = RpcClient::new(
        "https://lb.drpc.org/ogrpc?network=solana&dkey=Av7uqDf0ZEvytvOyAG1UTCcCtbSoJigR8IKSEjfP07KJ",
    );

    // One-liner call requested
    let price = screenerbot::pool_price::price_from_biggest_pool(&rpc, &token_mint)?;

    println!("✅ Price for {token_mint}: {price}");

    Ok(())
}
