// Config schema submodule - splits the monolithic schemas.rs into manageable files

use crate::config_struct;

mod connectivity;
mod events;
mod filtering;
mod monitoring;
mod ohlcv;
mod pools;
mod positions;
mod rpc;
mod services;
mod sol_price;
mod strategies;
mod swaps;
mod tokens;
mod trader;
mod wallet;

pub use connectivity::*;
pub use events::*;
pub use filtering::*;
pub use monitoring::*;
pub use ohlcv::*;
pub use pools::*;
pub use positions::*;
pub use rpc::*;
pub use services::*;
pub use sol_price::*;
pub use strategies::*;
pub use swaps::*;
pub use tokens::*;
pub use trader::*;
pub use wallet::*;

// ============================================================================
// ROOT CONFIGURATION
// ============================================================================

config_struct! {
    /// Root configuration structure containing all sub-configurations
    pub struct Config {
        /// Main wallet private key (base58 or array format)
        main_wallet_private: String = String::new(),

        /// RPC configuration
        rpc: RpcConfig = RpcConfig::default(),

        /// Trader configuration
        trader: TraderConfig = TraderConfig::default(),

        /// Positions configuration
        positions: PositionsConfig = PositionsConfig::default(),

        /// Filtering configuration
        filtering: FilteringConfig = FilteringConfig::default(),

        /// Swaps configuration
        swaps: SwapsConfig = SwapsConfig::default(),

        /// Tokens configuration
        tokens: TokensConfig = TokensConfig::default(),

        /// Pools configuration
        pools: PoolsConfig = PoolsConfig::default(),

        /// SOL price service configuration
        sol_price: SolPriceConfig = SolPriceConfig::default(),

        /// Events system configuration
        events: EventsConfig = EventsConfig::default(),

        /// Services configuration
        services: ServicesConfig = ServicesConfig::default(),

        /// Monitoring configuration
        monitoring: MonitoringConfig = MonitoringConfig::default(),

        /// Connectivity monitoring configuration
        connectivity: ConnectivityMonitoringConfig = ConnectivityMonitoringConfig::default(),

        /// OHLCV data configuration
        ohlcv: OhlcvConfig = OhlcvConfig::default(),

        /// Wallet configuration
        wallet: WalletConfig = WalletConfig::default(),

        /// Strategies configuration
        strategies: StrategiesConfig = StrategiesConfig::default(),
    }
}
