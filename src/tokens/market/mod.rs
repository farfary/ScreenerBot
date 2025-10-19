/// Market data fetching from multiple sources
/// 
/// Each module handles one API source:
/// - dexscreener: DexScreener API (30s updates)
/// - geckoterminal: GeckoTerminal API (60s updates)

pub mod dexscreener;
pub mod geckoterminal;

pub use dexscreener::fetch_dexscreener_data;
pub use geckoterminal::fetch_geckoterminal_data;
