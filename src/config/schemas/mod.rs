// Config schema submodule - splits the monolithic schemas.rs into manageable files

use crate::config_struct;

mod connectivity;
mod events;
mod filtering;
mod gui;
mod monitoring;
mod ohlcv;
mod pools;
mod positions;
mod rpc;
mod services;
mod sol_price;
mod strategies;
mod swaps;
mod telegram;
mod tokens;
mod trader;
mod wallet;
mod webserver;

pub use connectivity::*;
pub use events::*;
pub use filtering::*;
pub use gui::*;
pub use monitoring::*;
pub use ohlcv::*;
pub use pools::*;
pub use positions::*;
pub use rpc::*;
pub use services::*;
pub use sol_price::*;
pub use strategies::*;
pub use swaps::*;
pub use telegram::*;
pub use tokens::*;
pub use trader::*;
pub use wallet::*;
pub use webserver::*;

// ============================================================================
// ROOT CONFIGURATION
// ============================================================================

config_struct! {
    /// Root configuration structure containing all sub-configurations
    pub struct Config {
        /// Encrypted wallet private key (base64-encoded AES-256-GCM ciphertext)
        wallet_encrypted: String = String::new(),

        /// Nonce for wallet encryption (base64-encoded 12-byte nonce)
        wallet_nonce: String = String::new(),

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

        /// GUI/Desktop application configuration
        gui: GuiConfig = GuiConfig::default(),

        /// Webserver configuration (headless/CLI mode only)
        webserver: WebserverConfig = WebserverConfig::default(),

        /// Telegram bot configuration for notifications and commands
        telegram: TelegramConfig = TelegramConfig::default(),
    }
}
