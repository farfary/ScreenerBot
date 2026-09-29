//! Guards on the category vocabulary.

use super::category::*;

/// Visibility of every category, pinned from the mapping the name-based
/// implementation produced.
#[test]
fn visibility_mapping_is_pinned() {
    let pinned: &[(ConfigCategory, &str)] = &[
        (ConfigCategory::Activity, "secondary"),
        (ConfigCategory::Age, "primary"),
        (ConfigCategory::Api, "secondary"),
        (ConfigCategory::Authentication, "secondary"),
        (ConfigCategory::Authorities, "secondary"),
        (ConfigCategory::AuthorityAnalysis, "secondary"),
        (ConfigCategory::AutoBlacklist, "secondary"),
        (ConfigCategory::Automation, "secondary"),
        (ConfigCategory::BackgroundCheck, "secondary"),
        (ConfigCategory::Cache, "technical"),
        (ConfigCategory::CategoryRecording, "secondary"),
        (ConfigCategory::Chat, "secondary"),
        (ConfigCategory::Checking, "secondary"),
        (ConfigCategory::CircuitBreaker, "technical"),
        (ConfigCategory::Connection, "primary"),
        (ConfigCategory::ConnectionPooling, "technical"),
        (ConfigCategory::CopyTrading, "secondary"),
        (ConfigCategory::CoreTrading, "primary"),
        (ConfigCategory::CreatorChecks, "secondary"),
        (ConfigCategory::DataSources, "secondary"),
        (ConfigCategory::Database, "secondary"),
        (ConfigCategory::Dca, "primary"),
        (ConfigCategory::Debug, "technical"),
        (ConfigCategory::Discovery, "secondary"),
        (ConfigCategory::Endpoints, "primary"),
        (ConfigCategory::Exit, "secondary"),
        (ConfigCategory::Fallback, "secondary"),
        (ConfigCategory::Fdv, "secondary"),
        (ConfigCategory::Features, "primary"),
        (ConfigCategory::Fees, "secondary"),
        (ConfigCategory::Fetcher, "secondary"),
        (ConfigCategory::Filtering, "secondary"),
        (ConfigCategory::General, "primary"),
        (ConfigCategory::GlobalControl, "primary"),
        (ConfigCategory::HolderDistribution, "secondary"),
        (ConfigCategory::InsiderDetection, "secondary"),
        (ConfigCategory::Installing, "secondary"),
        (ConfigCategory::Limits, "secondary"),
        (ConfigCategory::Liquidity, "primary"),
        (ConfigCategory::LossDetection, "primary"),
        (ConfigCategory::LossLimit, "secondary"),
        (ConfigCategory::LpLock, "secondary"),
        (ConfigCategory::LpProviders, "secondary"),
        (ConfigCategory::MarketCap, "primary"),
        (ConfigCategory::MasterControl, "secondary"),
        (ConfigCategory::MetaRequirements, "secondary"),
        (ConfigCategory::MonitorControl, "secondary"),
        (ConfigCategory::Monitoring, "secondary"),
        (ConfigCategory::Notifications, "primary"),
        (ConfigCategory::OllamaSettings, "secondary"),
        (ConfigCategory::Optimization, "secondary"),
        (ConfigCategory::PartialExit, "primary"),
        (ConfigCategory::Performance, "secondary"),
        (ConfigCategory::PoolMetrics, "secondary"),
        (ConfigCategory::PriceChange, "secondary"),
        (ConfigCategory::Profit, "primary"),
        (ConfigCategory::ProviderSelection, "technical"),
        (ConfigCategory::ProviderSettings, "secondary"),
        (ConfigCategory::Providers, "secondary"),
        (ConfigCategory::Quote, "secondary"),
        (ConfigCategory::RateLimiting, "technical"),
        (ConfigCategory::RateLimits, "secondary"),
        (ConfigCategory::Retention, "technical"),
        (ConfigCategory::Retries, "technical"),
        (ConfigCategory::Risk, "secondary"),
        (ConfigCategory::RiskLevel, "secondary"),
        (ConfigCategory::RiskScore, "secondary"),
        (ConfigCategory::RiskScoring, "secondary"),
        (ConfigCategory::RoiExit, "primary"),
        (ConfigCategory::Router, "primary"),
        (ConfigCategory::Routers, "secondary"),
        (ConfigCategory::Routing, "secondary"),
        (ConfigCategory::Safety, "secondary"),
        (ConfigCategory::Scheduling, "secondary"),
        (ConfigCategory::SecurityFlags, "secondary"),
        (ConfigCategory::Slippage, "primary"),
        (ConfigCategory::SourceControl, "primary"),
        (ConfigCategory::Sources, "secondary"),
        (ConfigCategory::Statistics, "technical"),
        (ConfigCategory::StopLoss, "secondary"),
        (ConfigCategory::SymbolAnalysis, "secondary"),
        (ConfigCategory::Thresholds, "secondary"),
        (ConfigCategory::TimeOverride, "secondary"),
        (ConfigCategory::Timeouts, "technical"),
        (ConfigCategory::Timing, "secondary"),
        (ConfigCategory::TokenInfo, "secondary"),
        (ConfigCategory::ToolPermissions, "secondary"),
        (ConfigCategory::Trading, "secondary"),
        (ConfigCategory::TrailingStop, "primary"),
        (ConfigCategory::TransferFees, "secondary"),
        (ConfigCategory::Updates, "secondary"),
        (ConfigCategory::Volume, "secondary"),
        (ConfigCategory::Wallet, "secondary"),
        (ConfigCategory::Watch, "secondary"),
    ];
    assert_eq!(pinned.len(), ConfigCategory::ALL.len());
    for (category, expected) in pinned {
        assert_eq!(category.visibility_str(), *expected, "{category:?}");
    }
}

#[test]
fn category_ids_are_unique_kebab_case() {
    let mut seen = std::collections::BTreeSet::new();
    for category in ConfigCategory::ALL {
        let id = category.as_str();
        assert!(
            !id.is_empty()
                && id
                    .split('-')
                    .all(|part| !part.is_empty()
                        && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())),
            "uid=501(farhad) gid=20(staff) groups=20(staff),12(everyone),61(localaccounts),79(_appserverusr),80(admin),81(_appserveradm),98(_lpadmin),701(com.apple.sharepoint.group.1),704(com.apple.sharepoint.group.4),33(_appstore),100(_lpoperator),204(_developer),250(_analyticsusers),395(com.apple.access_ftp),398(com.apple.access_screensharing),399(com.apple.access_ssh),400(com.apple.access_remote_ae),703(com.apple.sharepoint.group.3) is not kebab-case"
        );
        assert!(seen.insert(id), "duplicate category id uid=501(farhad) gid=20(staff) groups=20(staff),12(everyone),61(localaccounts),79(_appserverusr),80(admin),81(_appserveradm),98(_lpadmin),701(com.apple.sharepoint.group.1),704(com.apple.sharepoint.group.4),33(_appstore),100(_lpoperator),204(_developer),250(_analyticsusers),395(com.apple.access_ftp),398(com.apple.access_screensharing),399(com.apple.access_ssh),400(com.apple.access_remote_ae),703(com.apple.sharepoint.group.3)");
        let serialized = serde_json::to_value(category).unwrap();
        assert_eq!(serialized, serde_json::Value::String(id.to_string()));
    }
}
