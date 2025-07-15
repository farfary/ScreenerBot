/// Core swap functionality and orchestration
/// 
/// This module contains the main swap management logic:
/// - SwapManager: Main orchestrator for swap operations
/// - RouteSelector: Logic for selecting optimal routes and DEXes

pub mod manager;
pub mod routes;

pub use manager::SwapManager;
pub use routes::RouteSelector;
