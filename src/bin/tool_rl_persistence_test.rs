use chrono::Utc;
use screenerbot::{
    logger::{ log, LogTag },
    global::{ read_configs },
    rl_learning::{ get_trading_learner, LearningRecord },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read configs to initialize systems
    let _configs = read_configs().expect("Failed to read configs");

    log(LogTag::System, "INFO", "🧪 Starting RL persistence test");

    let learner = get_trading_learner();

    // Display current state
    log(
        LogTag::System,
        "INFO",
        &format!("📊 Current record count: {}", learner.get_record_count())
    );
    log(LogTag::System, "INFO", &format!("🤖 Model ready: {}", learner.is_model_ready()));

    if let Some(metrics) = learner.get_model_metrics() {
        log(
            LogTag::System,
            "INFO",
            &format!(
                "📈 Model metrics: {} records, {} features, trained at {}",
                metrics.training_records,
                metrics.feature_count,
                metrics.training_time.format("%Y-%m-%d %H:%M:%S")
            )
        );
    } else {
        log(LogTag::System, "INFO", "📈 No model metrics available");
    }

    // Add a test learning record
    let test_record = LearningRecord {
        timestamp: Utc::now(),
        token_mint: "TestMint123456789".to_string(),
        token_symbol: "TEST".to_string(),
        current_price: 0.001234,
        price_change_5min: -2.5,
        price_change_10min: -5.0,
        price_change_30min: -8.0,
        liquidity_usd: 50000.0,
        volume_24h: 250000.0,
        market_cap: Some(1000000.0),
        rugcheck_score: Some(25.0),
        pool_price: 0.001235,
        price_drop_detected: 8.0,
        confidence_score: 0.75,
        actual_profit_percent: 15.5,
        hold_duration_minutes: 120.0,
        success: true,
    };

    log(LogTag::System, "INFO", "📝 Adding test learning record");
    learner.add_learning_record(test_record);

    log(LogTag::System, "INFO", &format!("📊 New record count: {}", learner.get_record_count()));

    // Test manual save
    log(LogTag::System, "INFO", "💾 Testing manual save to disk");
    match learner.save_to_disk() {
        Ok(_) => log(LogTag::System, "INFO", "✅ Manual save successful"),
        Err(e) => log(LogTag::System, "ERROR", &format!("❌ Manual save failed: {}", e)),
    }

    // Test manual load (simulate restart)
    log(LogTag::System, "INFO", "📂 Testing manual load from disk");
    match learner.load_from_disk() {
        Ok(_) => log(LogTag::System, "INFO", "✅ Manual load successful"),
        Err(e) => log(LogTag::System, "ERROR", &format!("❌ Manual load failed: {}", e)),
    }

    log(LogTag::System, "INFO", &format!("📊 Final record count: {}", learner.get_record_count()));

    // Test training if we have enough records
    if learner.get_record_count() >= 50 {
        log(LogTag::System, "INFO", "🎯 Testing model training");
        match learner.train_model().await {
            Ok(_) => {
                log(LogTag::System, "INFO", "✅ Model training successful");
                log(
                    LogTag::System,
                    "INFO",
                    &format!("🤖 Model ready: {}", learner.is_model_ready())
                );
            }
            Err(e) => log(LogTag::System, "ERROR", &format!("❌ Model training failed: {}", e)),
        }
    } else {
        log(
            LogTag::System,
            "INFO",
            &format!("⏳ Need {} records for training, have {}", 50, learner.get_record_count())
        );
    }

    log(LogTag::System, "INFO", "🏁 RL persistence test completed");

    Ok(())
}
