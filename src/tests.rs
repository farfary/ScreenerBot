use crate::performance::*;

#[tokio::test]
async fn test_performance_tracking() {
    // Test recording a trade entry
    record_trade_entry(
        "test_mint",
        "TEST",
        0.001,
        0.001,
        vec!["whale_activity".to_string()],
        0.8,
        0.2
    ).await;

    // Test recording a trade exit
    record_trade_exit("test_mint", 0.0011, 0.0011, "profit_taking", 0, false).await;

    // Test getting metrics
    let metrics = get_performance_metrics().await;
    assert_eq!(metrics.total_trades, 1);
    assert_eq!(metrics.winning_trades, 1);

    println!("✅ Performance tracking test passed!");
}
