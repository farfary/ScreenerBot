/// Comprehensive Swap Transaction Discovery Tool (Binary Wrapper)
/// 
/// This binary provides a command-line interface to the swap analysis functionality
/// in the wallet_transactions module.
///
/// Usage:
///   cargo run --bin swap_analysis [--wallet WALLET] [--limit LIMIT] [--export] [--detailed] [--table]

use screenerbot::{
    logger::init_file_logging,
    transactions_tools::Args,
    wallet_transactions::run_swap_analysis,
};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();
    
    let args = Args::parse();
    
    // Call the main swap analysis function from wallet_transactions module
    run_swap_analysis(args).await
}
