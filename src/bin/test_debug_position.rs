use screenerbot::positions::debug_position_token_mismatch;
use screenerbot::logger::{log, LogTag, init_file_logging};

#[tokio::main]
async fn main() {
    init_file_logging();
    
    log(LogTag::System, "INFO", "Testing position debug function");
    
    // Test with the PUMP token that has the mismatch
    let pump_mint = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";
    
    debug_position_token_mismatch(pump_mint).await;
    
    log(LogTag::System, "INFO", "Debug test completed");
}
