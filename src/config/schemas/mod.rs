// Config schema submodule - splits the monolithic schemas.rs into manageable files

use crate::config_struct;

mod rpc;
mod trader;
mod wallet;
mod pools;
mod positions;
mod filtering;
mod swaps;
mod tokens;
mod ohlcv;
mod sol_price;
mod events;
mod services;
mod monitoring;
mod strategies;

pub use rpc::*;
pub use trader::*;
pub use wallet::*;
pub use pools::*;
pub use positions::*;
pub use filtering::*;
pub use swaps::*;
pub use tokens::*;
pub use ohlcv::*;
pub use sol_price::*;
pub use events::*;
pub use services::*;
pub use monitoring::*;
pub use strategies::*;

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

        /// OHLCV data configuration
        ohlcv: OhlcvConfig = OhlcvConfig::default(),

        /// Wallet configuration
        wallet: WalletConfig = WalletConfig::default(),

        /// Strategies configuration
        strategies: StrategiesConfig = StrategiesConfig::default(),
    }
}
