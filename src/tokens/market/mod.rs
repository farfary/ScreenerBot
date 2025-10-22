/// Market data fetching from multiple sources
///
/// Each module handles one API source:
/// - dexscreener: DexScreener API (30s updates, batch endpoint)
/// - geckoterminal: GeckoTerminal API (60s updates, batch endpoint)
pub mod dexscreener;
pub mod geckoterminal;

pub use dexscreener::{fetch_dexscreener_data, fetch_dexscreener_data_batch};
pub use geckoterminal::{fetch_geckoterminal_data, fetch_geckoterminal_data_batch};
