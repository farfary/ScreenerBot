/// Simple test tool for new filtering system
use screenerbot::filtering::get_filtered_tokens;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing new filtering system");

    screenerbot::logger::init_file_logging();

    let filtered_tokens = get_filtered_tokens().await?;

    println!("✅ Filtering complete: {} tokens returned", filtered_tokens.len());

    // Show first 10 tokens
    for (i, mint) in filtered_tokens.iter().take(10).enumerate() {
        println!("  {}: {}", i + 1, &mint);
    }

    if filtered_tokens.len() > 10 {
        println!("  ... and {} more tokens", filtered_tokens.len() - 10);
    }

    Ok(())
}
