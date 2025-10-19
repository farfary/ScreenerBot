/// Security data fetching from multiple sources
///
/// Each module handles one security analysis source:
/// - rugcheck: Rugcheck API (comprehensive security analysis)
/// - onchain: Future on-chain verification
pub mod rugcheck;

pub use rugcheck::fetch_rugcheck_data;
