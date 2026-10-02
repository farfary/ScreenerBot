// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! UI category a config field is grouped under, and how prominently the
//! dashboard shows it. Serialized as the id its catalog key is built from
//! (`config-category-<id>`).

use serde::Serialize;

/// How prominently the dashboard shows a category by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    /// Essential settings, expanded by default.
    Primary,
    /// Important but less frequently changed, collapsed.
    Secondary,
    /// Power-user settings (timeouts, retries, ...), collapsed and grouped last.
    Technical,
}

impl Visibility {
    const fn as_str(self) -> &'static str {
        match self {
            Visibility::Primary => "primary",
            Visibility::Secondary => "secondary",
            Visibility::Technical => "technical",
        }
    }
}

/// UI category a config field is grouped under.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigCategory {
    #[serde(rename = "activity")]
    Activity,
    #[serde(rename = "age")]
    Age,
    #[serde(rename = "api")]
    Api,
    #[serde(rename = "authentication")]
    Authentication,
    #[serde(rename = "authorities")]
    Authorities,
    #[serde(rename = "authority-analysis")]
    AuthorityAnalysis,
    #[serde(rename = "auto-blacklist")]
    AutoBlacklist,
    #[serde(rename = "automation")]
    Automation,
    #[serde(rename = "background-check")]
    BackgroundCheck,
    #[serde(rename = "cache")]
    Cache,
    #[serde(rename = "category-recording")]
    CategoryRecording,
    #[serde(rename = "chat")]
    Chat,
    #[serde(rename = "checking")]
    Checking,
    #[serde(rename = "circuit-breaker")]
    CircuitBreaker,
    #[serde(rename = "connection")]
    Connection,
    #[serde(rename = "connection-pooling")]
    ConnectionPooling,
    #[serde(rename = "copy-trading")]
    CopyTrading,
    #[serde(rename = "core-trading")]
    CoreTrading,
    #[serde(rename = "creator-checks")]
    CreatorChecks,
    #[serde(rename = "data-sources")]
    DataSources,
    #[serde(rename = "database")]
    Database,
    #[serde(rename = "dca")]
    Dca,
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "discovery")]
    Discovery,
    #[serde(rename = "endpoints")]
    Endpoints,
    #[serde(rename = "exit")]
    Exit,
    #[serde(rename = "fallback")]
    Fallback,
    #[serde(rename = "fdv")]
    Fdv,
    #[serde(rename = "features")]
    Features,
    #[serde(rename = "fees")]
    Fees,
    #[serde(rename = "fetcher")]
    Fetcher,
    #[serde(rename = "filtering")]
    Filtering,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "global-control")]
    GlobalControl,
    #[serde(rename = "holder-distribution")]
    HolderDistribution,
    #[serde(rename = "insider-detection")]
    InsiderDetection,
    #[serde(rename = "installing")]
    Installing,
    #[serde(rename = "limits")]
    Limits,
    #[serde(rename = "liquidity")]
    Liquidity,
    #[serde(rename = "loss-detection")]
    LossDetection,
    #[serde(rename = "loss-limit")]
    LossLimit,
    #[serde(rename = "lp-lock")]
    LpLock,
    #[serde(rename = "lp-providers")]
    LpProviders,
    #[serde(rename = "market-cap")]
    MarketCap,
    #[serde(rename = "master-control")]
    MasterControl,
    #[serde(rename = "meta-requirements")]
    MetaRequirements,
    #[serde(rename = "monitor-control")]
    MonitorControl,
    #[serde(rename = "monitoring")]
    Monitoring,
    #[serde(rename = "notifications")]
    Notifications,
    #[serde(rename = "ollama-settings")]
    OllamaSettings,
    #[serde(rename = "optimization")]
    Optimization,
    #[serde(rename = "partial-exit")]
    PartialExit,
    #[serde(rename = "performance")]
    Performance,
    #[serde(rename = "pool-metrics")]
    PoolMetrics,
    #[serde(rename = "price-change")]
    PriceChange,
    #[serde(rename = "profit")]
    Profit,
    #[serde(rename = "provider-selection")]
    ProviderSelection,
    #[serde(rename = "provider-settings")]
    ProviderSettings,
    #[serde(rename = "providers")]
    Providers,
    #[serde(rename = "quote")]
    Quote,
    #[serde(rename = "rate-limiting")]
    RateLimiting,
    #[serde(rename = "rate-limits")]
    RateLimits,
    #[serde(rename = "retention")]
    Retention,
    #[serde(rename = "retries")]
    Retries,
    #[serde(rename = "risk")]
    Risk,
    #[serde(rename = "risk-level")]
    RiskLevel,
    #[serde(rename = "risk-score")]
    RiskScore,
    #[serde(rename = "risk-scoring")]
    RiskScoring,
    #[serde(rename = "roi-exit")]
    RoiExit,
    #[serde(rename = "router")]
    Router,
    #[serde(rename = "routers")]
    Routers,
    #[serde(rename = "routing")]
    Routing,
    #[serde(rename = "safety")]
    Safety,
    #[serde(rename = "scheduling")]
    Scheduling,
    #[serde(rename = "security-flags")]
    SecurityFlags,
    #[serde(rename = "slippage")]
    Slippage,
    #[serde(rename = "source-control")]
    SourceControl,
    #[serde(rename = "sources")]
    Sources,
    #[serde(rename = "statistics")]
    Statistics,
    #[serde(rename = "stop-loss")]
    StopLoss,
    #[serde(rename = "symbol-analysis")]
    SymbolAnalysis,
    #[serde(rename = "thresholds")]
    Thresholds,
    #[serde(rename = "time-override")]
    TimeOverride,
    #[serde(rename = "timeouts")]
    Timeouts,
    #[serde(rename = "timing")]
    Timing,
    #[serde(rename = "token-info")]
    TokenInfo,
    #[serde(rename = "tool-permissions")]
    ToolPermissions,
    #[serde(rename = "trading")]
    Trading,
    #[serde(rename = "trailing-stop")]
    TrailingStop,
    #[serde(rename = "transfer-fees")]
    TransferFees,
    #[serde(rename = "updates")]
    Updates,
    #[serde(rename = "volume")]
    Volume,
    #[serde(rename = "wallet")]
    Wallet,
    #[serde(rename = "watch")]
    Watch,
}

impl ConfigCategory {
    /// Every category, in declaration order.
    pub const ALL: [ConfigCategory; 94] = [
        ConfigCategory::Activity,
        ConfigCategory::Age,
        ConfigCategory::Api,
        ConfigCategory::Authentication,
        ConfigCategory::Authorities,
        ConfigCategory::AuthorityAnalysis,
        ConfigCategory::AutoBlacklist,
        ConfigCategory::Automation,
        ConfigCategory::BackgroundCheck,
        ConfigCategory::Cache,
        ConfigCategory::CategoryRecording,
        ConfigCategory::Chat,
        ConfigCategory::Checking,
        ConfigCategory::CircuitBreaker,
        ConfigCategory::Connection,
        ConfigCategory::ConnectionPooling,
        ConfigCategory::CopyTrading,
        ConfigCategory::CoreTrading,
        ConfigCategory::CreatorChecks,
        ConfigCategory::DataSources,
        ConfigCategory::Database,
        ConfigCategory::Dca,
        ConfigCategory::Debug,
        ConfigCategory::Discovery,
        ConfigCategory::Endpoints,
        ConfigCategory::Exit,
        ConfigCategory::Fallback,
        ConfigCategory::Fdv,
        ConfigCategory::Features,
        ConfigCategory::Fees,
        ConfigCategory::Fetcher,
        ConfigCategory::Filtering,
        ConfigCategory::General,
        ConfigCategory::GlobalControl,
        ConfigCategory::HolderDistribution,
        ConfigCategory::InsiderDetection,
        ConfigCategory::Installing,
        ConfigCategory::Limits,
        ConfigCategory::Liquidity,
        ConfigCategory::LossDetection,
        ConfigCategory::LossLimit,
        ConfigCategory::LpLock,
        ConfigCategory::LpProviders,
        ConfigCategory::MarketCap,
        ConfigCategory::MasterControl,
        ConfigCategory::MetaRequirements,
        ConfigCategory::MonitorControl,
        ConfigCategory::Monitoring,
        ConfigCategory::Notifications,
        ConfigCategory::OllamaSettings,
        ConfigCategory::Optimization,
        ConfigCategory::PartialExit,
        ConfigCategory::Performance,
        ConfigCategory::PoolMetrics,
        ConfigCategory::PriceChange,
        ConfigCategory::Profit,
        ConfigCategory::ProviderSelection,
        ConfigCategory::ProviderSettings,
        ConfigCategory::Providers,
        ConfigCategory::Quote,
        ConfigCategory::RateLimiting,
        ConfigCategory::RateLimits,
        ConfigCategory::Retention,
        ConfigCategory::Retries,
        ConfigCategory::Risk,
        ConfigCategory::RiskLevel,
        ConfigCategory::RiskScore,
        ConfigCategory::RiskScoring,
        ConfigCategory::RoiExit,
        ConfigCategory::Router,
        ConfigCategory::Routers,
        ConfigCategory::Routing,
        ConfigCategory::Safety,
        ConfigCategory::Scheduling,
        ConfigCategory::SecurityFlags,
        ConfigCategory::Slippage,
        ConfigCategory::SourceControl,
        ConfigCategory::Sources,
        ConfigCategory::Statistics,
        ConfigCategory::StopLoss,
        ConfigCategory::SymbolAnalysis,
        ConfigCategory::Thresholds,
        ConfigCategory::TimeOverride,
        ConfigCategory::Timeouts,
        ConfigCategory::Timing,
        ConfigCategory::TokenInfo,
        ConfigCategory::ToolPermissions,
        ConfigCategory::Trading,
        ConfigCategory::TrailingStop,
        ConfigCategory::TransferFees,
        ConfigCategory::Updates,
        ConfigCategory::Volume,
        ConfigCategory::Wallet,
        ConfigCategory::Watch,
    ];

    /// Id used in serialized metadata and in the catalog key.
    pub const fn as_str(self) -> &'static str {
        match self {
            ConfigCategory::Activity => "activity",
            ConfigCategory::Age => "age",
            ConfigCategory::Api => "api",
            ConfigCategory::Authentication => "authentication",
            ConfigCategory::Authorities => "authorities",
            ConfigCategory::AuthorityAnalysis => "authority-analysis",
            ConfigCategory::AutoBlacklist => "auto-blacklist",
            ConfigCategory::Automation => "automation",
            ConfigCategory::BackgroundCheck => "background-check",
            ConfigCategory::Cache => "cache",
            ConfigCategory::CategoryRecording => "category-recording",
            ConfigCategory::Chat => "chat",
            ConfigCategory::Checking => "checking",
            ConfigCategory::CircuitBreaker => "circuit-breaker",
            ConfigCategory::Connection => "connection",
            ConfigCategory::ConnectionPooling => "connection-pooling",
            ConfigCategory::CopyTrading => "copy-trading",
            ConfigCategory::CoreTrading => "core-trading",
            ConfigCategory::CreatorChecks => "creator-checks",
            ConfigCategory::DataSources => "data-sources",
            ConfigCategory::Database => "database",
            ConfigCategory::Dca => "dca",
            ConfigCategory::Debug => "debug",
            ConfigCategory::Discovery => "discovery",
            ConfigCategory::Endpoints => "endpoints",
            ConfigCategory::Exit => "exit",
            ConfigCategory::Fallback => "fallback",
            ConfigCategory::Fdv => "fdv",
            ConfigCategory::Features => "features",
            ConfigCategory::Fees => "fees",
            ConfigCategory::Fetcher => "fetcher",
            ConfigCategory::Filtering => "filtering",
            ConfigCategory::General => "general",
            ConfigCategory::GlobalControl => "global-control",
            ConfigCategory::HolderDistribution => "holder-distribution",
            ConfigCategory::InsiderDetection => "insider-detection",
            ConfigCategory::Installing => "installing",
            ConfigCategory::Limits => "limits",
            ConfigCategory::Liquidity => "liquidity",
            ConfigCategory::LossDetection => "loss-detection",
            ConfigCategory::LossLimit => "loss-limit",
            ConfigCategory::LpLock => "lp-lock",
            ConfigCategory::LpProviders => "lp-providers",
            ConfigCategory::MarketCap => "market-cap",
            ConfigCategory::MasterControl => "master-control",
            ConfigCategory::MetaRequirements => "meta-requirements",
            ConfigCategory::MonitorControl => "monitor-control",
            ConfigCategory::Monitoring => "monitoring",
            ConfigCategory::Notifications => "notifications",
            ConfigCategory::OllamaSettings => "ollama-settings",
            ConfigCategory::Optimization => "optimization",
            ConfigCategory::PartialExit => "partial-exit",
            ConfigCategory::Performance => "performance",
            ConfigCategory::PoolMetrics => "pool-metrics",
            ConfigCategory::PriceChange => "price-change",
            ConfigCategory::Profit => "profit",
            ConfigCategory::ProviderSelection => "provider-selection",
            ConfigCategory::ProviderSettings => "provider-settings",
            ConfigCategory::Providers => "providers",
            ConfigCategory::Quote => "quote",
            ConfigCategory::RateLimiting => "rate-limiting",
            ConfigCategory::RateLimits => "rate-limits",
            ConfigCategory::Retention => "retention",
            ConfigCategory::Retries => "retries",
            ConfigCategory::Risk => "risk",
            ConfigCategory::RiskLevel => "risk-level",
            ConfigCategory::RiskScore => "risk-score",
            ConfigCategory::RiskScoring => "risk-scoring",
            ConfigCategory::RoiExit => "roi-exit",
            ConfigCategory::Router => "router",
            ConfigCategory::Routers => "routers",
            ConfigCategory::Routing => "routing",
            ConfigCategory::Safety => "safety",
            ConfigCategory::Scheduling => "scheduling",
            ConfigCategory::SecurityFlags => "security-flags",
            ConfigCategory::Slippage => "slippage",
            ConfigCategory::SourceControl => "source-control",
            ConfigCategory::Sources => "sources",
            ConfigCategory::Statistics => "statistics",
            ConfigCategory::StopLoss => "stop-loss",
            ConfigCategory::SymbolAnalysis => "symbol-analysis",
            ConfigCategory::Thresholds => "thresholds",
            ConfigCategory::TimeOverride => "time-override",
            ConfigCategory::Timeouts => "timeouts",
            ConfigCategory::Timing => "timing",
            ConfigCategory::TokenInfo => "token-info",
            ConfigCategory::ToolPermissions => "tool-permissions",
            ConfigCategory::Trading => "trading",
            ConfigCategory::TrailingStop => "trailing-stop",
            ConfigCategory::TransferFees => "transfer-fees",
            ConfigCategory::Updates => "updates",
            ConfigCategory::Volume => "volume",
            ConfigCategory::Wallet => "wallet",
            ConfigCategory::Watch => "watch",
        }
    }

    const fn visibility(self) -> Visibility {
        match self {
            ConfigCategory::Age
            | ConfigCategory::Connection
            | ConfigCategory::CoreTrading
            | ConfigCategory::Dca
            | ConfigCategory::Endpoints
            | ConfigCategory::Features
            | ConfigCategory::General
            | ConfigCategory::GlobalControl
            | ConfigCategory::Liquidity
            | ConfigCategory::LossDetection
            | ConfigCategory::MarketCap
            | ConfigCategory::Notifications
            | ConfigCategory::PartialExit
            | ConfigCategory::Profit
            | ConfigCategory::RoiExit
            | ConfigCategory::Router
            | ConfigCategory::Slippage
            | ConfigCategory::SourceControl
            | ConfigCategory::TrailingStop => Visibility::Primary,
            ConfigCategory::Activity
            | ConfigCategory::Api
            | ConfigCategory::Authentication
            | ConfigCategory::Authorities
            | ConfigCategory::AuthorityAnalysis
            | ConfigCategory::AutoBlacklist
            | ConfigCategory::Automation
            | ConfigCategory::BackgroundCheck
            | ConfigCategory::CategoryRecording
            | ConfigCategory::Chat
            | ConfigCategory::Checking
            | ConfigCategory::CopyTrading
            | ConfigCategory::CreatorChecks
            | ConfigCategory::DataSources
            | ConfigCategory::Database
            | ConfigCategory::Discovery
            | ConfigCategory::Exit
            | ConfigCategory::Fallback
            | ConfigCategory::Fdv
            | ConfigCategory::Fees
            | ConfigCategory::Fetcher
            | ConfigCategory::Filtering
            | ConfigCategory::HolderDistribution
            | ConfigCategory::InsiderDetection
            | ConfigCategory::Installing
            | ConfigCategory::Limits
            | ConfigCategory::LossLimit
            | ConfigCategory::LpLock
            | ConfigCategory::LpProviders
            | ConfigCategory::MasterControl
            | ConfigCategory::MetaRequirements
            | ConfigCategory::MonitorControl
            | ConfigCategory::Monitoring
            | ConfigCategory::OllamaSettings
            | ConfigCategory::Optimization
            | ConfigCategory::Performance
            | ConfigCategory::PoolMetrics
            | ConfigCategory::PriceChange
            | ConfigCategory::ProviderSettings
            | ConfigCategory::Providers
            | ConfigCategory::Quote
            | ConfigCategory::RateLimits
            | ConfigCategory::Risk
            | ConfigCategory::RiskLevel
            | ConfigCategory::RiskScore
            | ConfigCategory::RiskScoring
            | ConfigCategory::Routers
            | ConfigCategory::Routing
            | ConfigCategory::Safety
            | ConfigCategory::Scheduling
            | ConfigCategory::SecurityFlags
            | ConfigCategory::Sources
            | ConfigCategory::StopLoss
            | ConfigCategory::SymbolAnalysis
            | ConfigCategory::Thresholds
            | ConfigCategory::TimeOverride
            | ConfigCategory::Timing
            | ConfigCategory::TokenInfo
            | ConfigCategory::ToolPermissions
            | ConfigCategory::Trading
            | ConfigCategory::TransferFees
            | ConfigCategory::Updates
            | ConfigCategory::Volume
            | ConfigCategory::Wallet
            | ConfigCategory::Watch => Visibility::Secondary,
            ConfigCategory::Cache
            | ConfigCategory::CircuitBreaker
            | ConfigCategory::ConnectionPooling
            | ConfigCategory::Debug
            | ConfigCategory::ProviderSelection
            | ConfigCategory::RateLimiting
            | ConfigCategory::Retention
            | ConfigCategory::Retries
            | ConfigCategory::Statistics
            | ConfigCategory::Timeouts => Visibility::Technical,
        }
    }

    /// Visibility level for UI rendering: "primary", "secondary" or "technical".
    pub const fn visibility_str(self) -> &'static str {
        self.visibility().as_str()
    }
}

/// Category catalog key (`config-category-<id>`).
pub(crate) fn category_key(category: ConfigCategory) -> String {
    format!("config-category-{}", category.as_str())
}
