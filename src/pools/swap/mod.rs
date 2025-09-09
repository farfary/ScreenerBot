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

// Public modules

pub mod types;
pub mod builder;
pub mod executor;

// Program-specific swap implementations
pub mod programs;

// Re-export main API
pub use builder::SwapBuilder;
pub use executor::SwapExecutor;
pub use types::{ SwapRequest, SwapResult, SwapDirection, SwapError };

// Re-export program implementations for direct access if needed
pub use programs::raydium_cpmm::RaydiumCpmmSwap;
