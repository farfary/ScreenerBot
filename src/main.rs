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

#[tokio::main]
async fn main() {
    screenerbot::run::boot().await;
}
