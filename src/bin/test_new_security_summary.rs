use screenerbot::{
    logger::{ init_file_logging, log, LogTag },
    tokens::security::{ initialize_security_analyzer, get_security_analyzer },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    log(LogTag::System, "START", "Testing new simplified security summary");

    // Initialize security analyzer
    initialize_security_analyzer()?;

    if let Some(analyzer) = get_security_analyzer() {
        let summary = analyzer.get_security_summary().await;

        println!("\n=== NEW SIMPLIFIED SECURITY SUMMARY ===");
        println!(
            "API calls: {} total, {} success, {} failed",
            summary.api_calls_total,
            summary.api_calls_success,
            summary.api_calls_failed
        );
        println!("Safe tokens: {}", summary.db_safe_tokens);
        println!("Warning tokens: {}", summary.db_warning_tokens);
        println!("Danger tokens: {}", summary.db_danger_tokens);
        println!("Missing security data: {}", summary.db_missing_tokens);
        println!("Pump.fun tokens: {}", summary.db_pump_fun_tokens);

        println!("\n=== FORMATTED OUTPUT ===");
        // This will show the actual formatted output that would appear in logs
        screenerbot::tokens::security::start_security_summary_task();

        // Wait a moment for the summary to be logged
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    Ok(())
}
