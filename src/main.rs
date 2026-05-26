//! ScreenerBot - Automated Solana DeFi Trading Bot
//!
//! This is the main entry point for the ScreenerBot application.
//! The bot runs as a headless server with a web-based dashboard.

// jemalloc: better fragmentation behavior than system allocator for long-running processes.
// Tune via MALLOC_CONF env var if needed, e.g.:
//   MALLOC_CONF=dirty_decay_ms:1000,muzzy_decay_ms:2000
// Default dirty_decay_ms=10000 (10s) — lower values return pages to OS faster
// but increase CPU overhead from the decay thread.
#[cfg(all(feature = "jemalloc", not(target_env = "msvc")))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use screenerbot::arguments::{print_help, print_version, set_cmd_args};
use screenerbot::config::utils::load_config;
use screenerbot::config::with_config;
use screenerbot::logger::{error, info, LogTag};
use screenerbot::run::run_bot;
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Global flag to signal shutdown
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// Check if shutdown was requested
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_FLAG.load(Ordering::SeqCst)
}

/// Request application shutdown
pub fn request_shutdown() {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
}

/// Set up panic hook to send Telegram notification when bot crashes
fn setup_panic_hook() {
    // Get Telegram config before setting hook (config is already loaded at this point)
    let (enabled, bot_token, chat_id) = with_config(|cfg| {
        (
            cfg.telegram.enabled,
            cfg.telegram.bot_token.clone(),
            cfg.telegram.chat_id.clone(),
        )
    });

    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Log the panic to stderr
        eprintln!("\n🚨 PANIC: {:?}\n", panic_info);

        // Try to send Telegram notification if configured
        if enabled && !bot_token.is_empty() && !chat_id.is_empty() {
            let location = panic_info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "unknown".to_owned());

            let payload = panic_info.payload();
            let panic_message = if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_owned()
            };

            // Truncate message if too long
            let panic_message = if panic_message.len() > 200 {
                let mut end = 200;
                while end > 0 && !panic_message.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &panic_message[..end])
            } else {
                panic_message
            };

            let message = format!(
                "🚨 <b>Bot Crashed!</b>\n\n\
                 <b>Location:</b> <code>{}</code>\n\
                 <b>Error:</b> {}\n\n\
                 ⚠️ Please restart the bot.",
                location, panic_message
            );

            let bot_token_clone = bot_token.clone();
            let chat_id_clone = chat_id.clone();

            // Spawn a thread for blocking HTTP call (tokio runtime may be unavailable in panic)
            let handle = std::thread::spawn(move || {
                send_telegram_crash_notification(&bot_token_clone, &chat_id_clone, &message);
            });

            // Wait up to 5 seconds for notification to send
            let _ = handle.join();
        }

        // Call default panic handler
        default_panic(panic_info);
    }));
}

/// Send crash notification directly via Telegram API (blocking, for panic context)
fn send_telegram_crash_notification(bot_token: &str, chat_id: &str, message: &str) {
    use std::collections::HashMap;
    use std::time::Duration;

    // Use reqwest blocking client (panic-safe, no async runtime needed)
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "⚠️ Could not create HTTP client for crash notification: {}",
                e
            );
            return;
        }
    };

    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");

    let mut params = HashMap::new();
    params.insert("chat_id", chat_id);
    params.insert("text", message);
    params.insert("parse_mode", "HTML");

    match client.post(&url).form(&params).send() {
        Ok(response) => {
            if response.status().is_success() {
                eprintln!("✅ Crash notification sent to Telegram");
            } else {
                eprintln!(
                    "⚠️ Telegram API returned error: {} - {}",
                    response.status(),
                    response.text().unwrap_or_default()
                );
            }
        }
        Err(e) => {
            eprintln!("⚠️ Failed to send crash notification: {e}");
            eprintln!("📝 Crash message: {message}");
        }
    }
}

#[tokio::main]
async fn main() {
    // Store command line arguments
    set_cmd_args(std::env::args().collect());

    // Handle help flag
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    // Handle version flag
    if std::env::args().any(|arg| arg == "--version" || arg == "-v") {
        print_version();
        return;
    }

    // Print banner
    println!("\x1b[36;1;3m");
    println!(
        r#"
   ███████╗ ██████╗██████╗ ███████╗███████╗███╗   ██╗███████╗██████╗ ██████╗  ██████╗ ████████╗
   ██╔════╝██╔════╝██╔══██╗██╔════╝██╔════╝████╗  ██║██╔════╝██╔══██╗██╔══██╗██╔═══██╗╚══██╔══╝
   ███████╗██║     ██████╔╝█████╗  █████╗  ██╔██╗ ██║█████╗  ██████╔╝██████╔╝██║   ██║   ██║   
   ╚════██║██║     ██╔══██╗██╔══╝  ██╔══╝  ██║╚██╗██║██╔══╝  ██╔══██╗██╔══██╗██║   ██║   ██║   
   ███████║╚██████╗██║  ██║███████╗███████╗██║ ╚████║███████╗██║  ██║██████╔╝╚██████╔╝   ██║   
   ╚══════╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚══════╝╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝╚═════╝  ╚═════╝    ╚═╝   

                                             SCREENERBOT
                                ◆ Automated Solana DeFi Trading Bot ◆

                  Website: screenerbot.io           Channel: t.me/screenerbotio
                  Docs:    screenerbot.io/docs      Group:   t.me/screenerbotio_talk
                  X:       x.com/screenerbotio      Support: t.me/screenerbotio_support
"#
    );
    println!("\x1b[0m");

    // Initialize logger
    screenerbot::logger::init();
    info(
        LogTag::System,
        "Logger initialized, attempting to load config...",
    );

    // Load configuration
    if let Err(e) = load_config() {
        error(
            LogTag::System,
            &format!("Failed to load configuration: {e}"),
        );
        return;
    }
    info(LogTag::System, "Configuration loaded successfully");

    // Set up panic hook for crash notifications (after config is loaded)
    setup_panic_hook();

    info(LogTag::System, "ScreenerBot starting...");

    // Set up shutdown signal handler
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
        info(LogTag::System, "Shutdown signal received");
        shutdown_flag_clone.store(true, Ordering::SeqCst);
        request_shutdown();
    });

    // Run the bot in headless mode
    if let Err(e) = run_bot().await {
        error(LogTag::System, &format!("Bot error: {e}"));
    }

    info(LogTag::System, "ScreenerBot shutdown complete");
}
