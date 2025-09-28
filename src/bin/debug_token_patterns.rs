use colored::*;
/// Debug tool for token pattern analysis
use screenerbot::logger::init_file_logging;
use screenerbot::tokens::patterns::{initialize_pattern_analyzer, log_pattern_analysis_results};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    println!("{}", "🔍 TOKEN PATTERN ANALYSIS TOOL".bright_cyan().bold());

    println!("🚀 Initializing pattern analyzer...");
    match initialize_pattern_analyzer().await {
        Ok(()) => println!("✅ Pattern analyzer initialized successfully"),
        Err(e) => {
            eprintln!("❌ Failed to initialize: {}", e);
            return Err(e.into());
        }
    }

    log_pattern_analysis_results();
    println!("🎉 Pattern analysis complete!");
    Ok(())
}
