use screenerbot::logger::{ log, LogTag, init_file_logging };

#[tokio::main]
async fn main() {
    // Initialize file logging
    init_file_logging();

    println!("Testing file logging functionality...");

    // Test various log types and tags
    log(LogTag::System, "INFO", "File logging system initialized");
    log(LogTag::Trader, "SUCCESS", "This is a test success message");
    log(LogTag::Wallet, "WARN", "This is a test warning message");
    log(LogTag::Monitor, "ERROR", "This is a test error message");
    log(LogTag::Pool, "DEBUG", "This is a test debug message");

    // Test multi-line messages
    log(
        LogTag::System,
        "INFO",
        "This is a very long message that should be wrapped across multiple lines to test the line wrapping functionality and ensure that both console and file logging handle it correctly"
    );

    // Test different log types
    log(LogTag::Trader, "BUY", "Test buy transaction logged");
    log(LogTag::Trader, "SELL", "Test sell transaction logged");
    log(LogTag::Wallet, "BALANCE", "Current SOL balance: 10.5 SOL");
    log(LogTag::Pool, "PRICE", "Token price: $0.001234");
    log(LogTag::Trader, "PROFIT", "Position closed with 25% profit");
    log(LogTag::Trader, "LOSS", "Position closed with 5% loss");

    println!("Test completed. Check the 'logs' directory for log files.");
    println!("Log files are rotated daily and kept for 24 hours.");
}
