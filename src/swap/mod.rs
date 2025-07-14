pub mod traits;
pub mod types;
pub mod providers;
pub mod manager;

// Re-export main types and traits for convenience
pub use traits::{ SwapProvider, ProviderConfig };
pub use types::{
    SwapRequest,
    SwapResult,
    SwapQuote,
    SwapType,
    TransactionStatus,
    SwapError,
    RouteInfo,
    RouteStep,
};
pub use manager::SwapManager;
pub use providers::{ GmgnProvider };
