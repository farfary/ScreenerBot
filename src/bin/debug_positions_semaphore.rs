use screenerbot::{
    arguments,
    logger::{self as logger, LogTag},
    positions::get_open_positions_count,
};

#[tokio::main]
async fn main() -> Result<(), String> {
    // Arguments are auto-captured from env via CMD_ARGS; initialize file logging
    logger::init();

    let open = get_open_positions_count().await;
    // Access semaphore via internal module (unsafe to expose globally) using reflection-like hack not desired; instead we report open vs configured.
    let max = screenerbot::config::with_config(|cfg| cfg.trader.max_open_positions);
    logger::info(
        LogTag::Positions,
        &format!(
            "Open positions: {} / {} (semaphore reconciliation implicit)",
            open, max
        ),
    );
    if arguments::is_debug_positions_enabled() {
        logger::info(
            LogTag::Positions,
            &format!(
                "Enabled debug modes: {:?}",
                arguments::get_enabled_debug_modes()
            ),
        );
    }
    Ok(())
}
