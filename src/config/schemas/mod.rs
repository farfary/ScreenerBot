//! Configuration schema type definitions for all bot subsystems.
// Config schema submodule - splits the monolithic schemas.rs into manageable files

use crate::config_struct;

mod account;
mod agent_control;
mod assistant;
mod connectivity;
mod copy_trading;
mod events;
mod filtering;
mod gui;
mod holder_watch;
mod llm;
mod llm_analysis;
mod maintenance;
mod monitoring;
mod network;
mod ohlcv;
mod performance;
mod pools;
mod positions;
mod referral;
mod rpc;
mod services;
mod sol_price;
mod strategies;
mod swaps;
mod telegram;
mod tokens;
mod trader;
mod updates;
mod wallet;
mod webserver;

pub use account::*;
pub use agent_control::*;
pub use assistant::*;
pub use connectivity::*;
pub use copy_trading::*;
pub use events::*;
pub use filtering::*;
pub use gui::*;
pub use holder_watch::*;
pub use llm::*;
pub use llm_analysis::*;
pub use maintenance::*;
pub use monitoring::*;
pub use network::*;
pub use ohlcv::*;
pub use performance::*;
pub use pools::*;
pub use positions::*;
pub use referral::*;
pub use rpc::*;
pub use services::*;
pub use sol_price::*;
pub use strategies::*;
pub use swaps::*;
pub use telegram::*;
pub use tokens::*;
pub use trader::*;
pub use updates::*;
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

        /// Global copy-trading policy (per-task limits live in copy_trading.db)
        copy_trading: CopyTradingConfig = CopyTradingConfig::default(),

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

        /// Holder watch tool configuration
        holder_watch: HolderWatchConfig = HolderWatchConfig::default(),

        /// Outbound LLM provider clients (credentials, models, rate limits)
        llm: LlmConfig = LlmConfig::default(),

        /// Model-scored token/trading analysis settings
        llm_analysis: LlmAnalysisConfig = LlmAnalysisConfig::default(),

        /// Dashboard assistant: chat plus scheduled conversations
        assistant: AssistantConfig = AssistantConfig::default(),

        /// Performance tuning (memory profile, cache sizing)
        performance: PerformanceConfig = PerformanceConfig::default(),

        /// Automatic maintenance and data retention
        maintenance: MaintenanceConfig = MaintenanceConfig::default(),

        /// Automatic update checking, downloading and installation
        updates: UpdatesConfig = UpdatesConfig::default(),

        /// Network proxy configuration
        network: NetworkConfig = NetworkConfig::default(),

        /// Referral attribution (opt-in; inert until a code is set)
        referral: ReferralConfig = ReferralConfig::default(),

        /// ScreenerBot account — optional sign-in. Holds no secrets; tokens
        /// live in the encrypted store, and the server address is a constant.
        account: AccountConfig = AccountConfig::default(),

        /// Shared agent-control capability boundary: availability plus the
        /// per-category tool permission policy.
        agent_control: AgentControlConfig = AgentControlConfig::default(),
    }
}
