use screenerbot::{
    arguments,
    logger::{init_file_logging, log, LogTag},
    positions::get_open_positions_count,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    // Arguments are auto-captured from env via CMD_ARGS; initialize file logging
    init_file_logging();

    let open = get_open_positions_count().await;
    // Access semaphore via internal module (unsafe to expose globally) using reflection-like hack not desired; instead we report open vs configured.
    let max = screenerbot::trader::MAX_OPEN_POSITIONS;
    log(
        LogTag::Positions,
        "INFO",
        &format!(
            "Open positions: {} / {} (semaphore reconciliation implicit)",
            open, max
        ),
    );
    if arguments::is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "Enabled debug modes: {:?}",
                arguments::get_enabled_debug_modes()
            ),
        );
    }
    Ok(())
}
