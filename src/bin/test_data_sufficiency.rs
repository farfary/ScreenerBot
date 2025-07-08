use screenerbot::trader::MarketDataFrame;

#[tokio::main]
async fn main() {
    println!("🧪 Testing data sufficiency checks");

    // Test 1: Empty dataframe (should fail)
    let empty_df = MarketDataFrame::new();
    let (has_data, status) = empty_df.has_sufficient_data_for_trading();
    println!("📊 Empty dataframe: {}", status);
    assert!(!has_data, "Empty dataframe should not have sufficient data");

    // Test 2: Dataframe with some data but insufficient
    let mut partial_df = MarketDataFrame::new();

    // Add some minute data but not enough
    for i in 0..30 {
        partial_df.minute_data.add_ohlcv(
            i as u64,
            100.0 + (i as f64),
            101.0 + (i as f64),
            99.0 + (i as f64),
            100.5 + (i as f64),
            1000.0
        );
    }

    // Update legacy data
    partial_df.update_legacy_data();

    let (has_data, status) = partial_df.has_sufficient_data_for_trading();
    println!("📊 Partial dataframe (30 minute points): {}", status);
    assert!(!has_data, "Partial dataframe should not have sufficient data");

    // Test 3: Dataframe with sufficient data
    let mut full_df = MarketDataFrame::new();

    // Add sufficient minute data
    for i in 0..60 {
        full_df.minute_data.add_ohlcv(
            i as u64,
            100.0 + (i as f64),
            101.0 + (i as f64),
            99.0 + (i as f64),
            100.5 + (i as f64),
            1000.0
        );
    }

    // Add sufficient hour data
    for i in 0..30 {
        full_df.hour_data.add_ohlcv(
            (i as u64) * 3600,
            100.0 + (i as f64),
            101.0 + (i as f64),
            99.0 + (i as f64),
            100.5 + (i as f64),
            60000.0
        );
    }

    // Add sufficient day data
    for i in 0..10 {
        full_df.day_data.add_ohlcv(
            (i as u64) * 86400,
            100.0 + (i as f64),
            101.0 + (i as f64),
            99.0 + (i as f64),
            100.5 + (i as f64),
            1440000.0
        );
    }

    // Update legacy data
    full_df.update_legacy_data();

    let (has_data, status) = full_df.has_sufficient_data_for_trading();
    println!("📊 Full dataframe: {}", status);
    assert!(has_data, "Full dataframe should have sufficient data");

    println!("✅ All data sufficiency tests passed!");
}
