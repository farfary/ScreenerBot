use screenerbot::ohlcv::*;
use screenerbot::prelude::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing OHLCV Cache System...");

    // Test pool address (same as before)
    let pool_address = "4CHY5cahkqpiV2x35hZUdgw7q1qvgdfQywJ3cPWMbB6U";
    let token_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK

    // Test adding token to OHLCV cache
    add_token_to_ohlcv_cache(token_mint, pool_address).await;
    add_priority_token(token_mint).await;

    println!("✅ Added token to OHLCV monitoring");

    // Force update to get some data immediately
    force_update_token_ohlcv(token_mint, pool_address).await;

    println!("✅ Forced OHLCV update");

    // Check if data is available
    let has_data = has_ohlcv_data(token_mint).await;
    println!("📊 Has OHLCV data: {}", has_data);

    // Get dataframe if available
    if let Some(dataframe) = get_token_ohlcv_dataframe(token_mint).await {
        println!("🎯 Retrieved OHLCV dataframe for token {}", &token_mint[..8]);

        let primary_tf = dataframe.get_primary_timeframe();
        println!(
            "📈 Primary timeframe: {} ({} candles)",
            primary_tf.timeframe,
            primary_tf.candles.len()
        );

        if let Some(current_price) = primary_tf.current_price() {
            println!("💰 Current price: ${:.8}", current_price);
        }

        if let Some(avg_volume) = primary_tf.average_volume(10) {
            println!("📊 Average volume (10 periods): ${:.2}", avg_volume);
        }

        if let Some(volatility) = primary_tf.volatility(20) {
            println!("📈 Volatility (20 periods): {:.2}%", volatility);
        }

        if let Some(vwap) = primary_tf.vwap(20) {
            println!("📊 VWAP (20 periods): ${:.8}", vwap);
        }

        // Test timeframe-specific data
        println!("\n📊 Timeframe Data Summary:");
        println!("  • 1m candles: {}", dataframe.minute_1.candles.len());
        println!("  • 5m candles: {}", dataframe.minute_5.candles.len());
        println!("  • 15m candles: {}", dataframe.minute_15.candles.len());
        println!("  • 1h candles: {}", dataframe.hour_1.candles.len());
        println!("  • 4h candles: {}", dataframe.hour_4.candles.len());
        println!("  • 1d candles: {}", dataframe.day_1.candles.len());

        // Show last few candles from 5-minute timeframe
        println!("\n🕐 Recent 5m Candles:");
        for (i, candle) in dataframe.minute_5.candles.iter().take(3).enumerate() {
            println!(
                "  {}. O:{:.8} H:{:.8} L:{:.8} C:{:.8} V:{:.2}",
                i + 1,
                candle.open,
                candle.high,
                candle.low,
                candle.close,
                candle.volume
            );
        }
    } else {
        println!("❌ No OHLCV dataframe available");
    }

    // Test cache summary
    let (total_tokens, total_candles, fresh_tokens) = get_ohlcv_cache_summary().await;
    println!("\n📋 Cache Summary:");
    println!("  • Total tokens cached: {}", total_tokens);
    println!("  • Total candles: {}", total_candles);
    println!("  • Fresh tokens: {}", fresh_tokens);

    // Test data status check
    let status = get_ohlcv_data_status(&vec![token_mint.to_string()]).await;
    println!("\n📊 Data Status: {:?}", status);

    println!("\n✅ OHLCV Cache System test completed!");

    Ok(())
}
