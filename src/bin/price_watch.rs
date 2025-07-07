use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use std::env;
use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<()> {
    let token_mint = env::args()
        .nth(1)
        .expect("usage: price_watch <TOKEN_MINT>");

    let rpc = RpcClient::new(
        "https://lb.drpc.org/ogrpc?network=solana&dkey=Av7uqDf0ZEvytvOyAG1UTCcCtbSoJigR8IKSEjfP07KJ",
    );

    let mut last_price = screenerbot::pool_price::price_from_biggest_pool(&rpc, &token_mint)?;

    println!("⏳ Start price watch for {token_mint}, initial price: {last_price:.9}");

    loop {
        sleep(Duration::from_secs(1));

        let price = match screenerbot::pool_price::price_from_biggest_pool(&rpc, &token_mint) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("❌ Error fetching price: {e}");
                continue;
            }
        };

        let change = (price - last_price).abs() / last_price;
        if change >= 0.01 {
            println!(
                "🔔 Price changed >1%: {last_price:.9} → {price:.9} ({:+.2}%)",
                (price - last_price) / last_price * 100.0
            );
            last_price = price;
        }
    }
}
