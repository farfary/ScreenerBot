/// Updates orchestrator - State-based priority updates
///
/// Split into focused submodules:
/// - helpers.rs: Constants, statics, and utility functions
/// - rate_limiter.rs: RateLimitCoordinator for API rate limiting
/// - core.rs: Core update functions (update_token, update_tokens_batch, UpdateResult)
/// - loops.rs: Main loop orchestrator and priority-specific update loops
///
/// Architecture:
/// - Market data updates from DexScreener (primary source)
/// - Security data from Rugcheck (one-time fetch)
/// - Rate limiting with separate semaphores per API endpoint
/// - Priority-based scheduling for different token states
mod core;
mod helpers;
mod loops;
mod rate_limiter;

// Re-export public API
pub use core::{force_update_token, update_token, update_tokens_batch, UpdateResult};
pub use loops::start_update_loop;
pub use rate_limiter::RateLimitCoordinator;
