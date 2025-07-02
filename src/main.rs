mod configs;
mod helpers;
mod swap_gmgn;
mod dexscreener;
mod trader;

#[tokio::main]
async fn main() {

    dexscreener::start_dexscreener_loop().await;
    trader::start_trader_loop().await;
    
    // Keep alive forever
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(600)).await;
    }


    // let token_mint = "GtfNvPGEZEgFyJR8AP7ckvFBdSTnvP4Ses4ZNaZDpump";

    // // ──────────────── BUY ────────────────
    // let amount_in = 10_000_000; // e.g., 0.01 SOL in lamports
    // println!("🚀 Start BUY via GMGN");

    // match swap_gmgn::buy_gmgn(token_mint, amount_in).await {
    //     Ok(sig) => println!("✅ BUY Tx Done: {sig}"),
    //     Err(e) => eprintln!("❌ BUY Error: {e:?}"),
    // }

    // // ──────────────── SELL ────────────────
    // println!("\n🚀 Start SELL ALL via GMGN");

    // match swap_gmgn::sell_all_gmgn(token_mint).await {
    //     Ok(sig) => println!("✅ SELL Tx Done: {sig}"),
    //     Err(e) => eprintln!("❌ SELL Error: {e:?}"),
    // }
}
