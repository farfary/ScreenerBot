/// Direct swap module for pools
///
/// This module provides direct swap functionality that integrates with the centralized
/// decoder system. It supports multiple DEX programs through a modular architecture.
///
/// Features:
/// - Modular program-specific swap implementations
/// - Integration with centralized decoders
/// - Automatic token account management
/// - WSOL wrapping/unwrapping
/// - Slippage protection
/// - Real-time pool state fetching

pub mod builder;
pub mod executor;
pub mod programs;
pub mod types;

pub use builder::SwapBuilder;
pub use executor::SwapExecutor;
pub use programs::raydium_clmm::RaydiumClmmSwap;
pub use programs::raydium_cpmm::RaydiumCpmmSwap;
pub use types::{SwapDirection, SwapError, SwapRequest, SwapResult};
