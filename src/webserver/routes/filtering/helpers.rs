/// Reason label to human-readable display mapping
pub fn get_rejection_display_label(reason: &str) -> String {
    match reason {
        "no_decimals" => "No decimals in database",
        "token_too_new" => "Token too new",
        "cooldown_filtered" => "Cooldown filtered",
        "dex_data_missing" => "DexScreener data missing",
        "gecko_data_missing" => "GeckoTerminal data missing",
        "rug_data_missing" => "Rugcheck data missing",
        "dex_empty_name" => "Empty name",
        "dex_empty_symbol" => "Empty symbol",
        "dex_empty_logo" => "Empty logo URL",
        "dex_empty_website" => "Empty website URL",
        "dex_txn_5m" => "Low 5m transactions",
        "dex_txn_1h" => "Low 1h transactions",
        "dex_zero_liq" => "Zero liquidity",
        "dex_liq_low" => "Liquidity too low",
        "dex_liq_high" => "Liquidity too high",
        "dex_mcap_low" => "Market cap too low",
        "dex_mcap_high" => "Market cap too high",
        "dex_vol_low" => "Volume too low",
        "dex_vol_missing" => "Volume missing",
        "dex_fdv_low" => "FDV too low",
        "dex_fdv_high" => "FDV too high",
        "dex_fdv_missing" => "FDV missing",
        "dex_vol5m_low" => "5m volume too low",
        "dex_vol5m_missing" => "5m volume missing",
        "dex_vol1h_low" => "1h volume too low",
        "dex_vol1h_missing" => "1h volume missing",
        "dex_vol6h_low" => "6h volume too low",
        "dex_vol6h_missing" => "6h volume missing",
        "dex_price_change_5m_low" => "5m price change too low",
        "dex_price_change_5m_high" => "5m price change too high",
        "dex_price_change_5m_missing" => "5m price change missing",
        "dex_price_change_low" => "Price change too low",
        "dex_price_change_high" => "Price change too high",
        "dex_price_change_missing" => "Price change missing",
        "dex_price_change_6h_low" => "6h price change too low",
        "dex_price_change_6h_high" => "6h price change too high",
        "dex_price_change_6h_missing" => "6h price change missing",
        "dex_price_change_24h_low" => "24h price change too low",
        "dex_price_change_24h_high" => "24h price change too high",
        "dex_price_change_24h_missing" => "24h price change missing",
        "gecko_liq_missing" => "Liquidity missing",
        "gecko_liq_low" => "Liquidity too low",
        "gecko_liq_high" => "Liquidity too high",
        "gecko_mcap_missing" => "Market cap missing",
        "gecko_mcap_low" => "Market cap too low",
        "gecko_mcap_high" => "Market cap too high",
        "gecko_vol5m_low" => "5m volume too low",
        "gecko_vol5m_missing" => "5m volume missing",
        "gecko_vol1h_low" => "1h volume too low",
        "gecko_vol1h_missing" => "1h volume missing",
        "gecko_vol24h_low" => "24h volume too low",
        "gecko_vol24h_missing" => "24h volume missing",
        "gecko_price_change_5m_low" => "5m price change too low",
        "gecko_price_change_5m_high" => "5m price change too high",
        "gecko_price_change_5m_missing" => "5m price change missing",
        "gecko_price_change_1h_low" => "1h price change too low",
        "gecko_price_change_1h_high" => "1h price change too high",
        "gecko_price_change_1h_missing" => "1h price change missing",
        "gecko_price_change_24h_low" => "24h price change too low",
        "gecko_price_change_24h_high" => "24h price change too high",
        "gecko_price_change_24h_missing" => "24h price change missing",
        "gecko_pool_count_low" => "Pool count too low",
        "gecko_pool_count_high" => "Pool count too high",
        "gecko_pool_count_missing" => "Pool count missing",
        "gecko_reserve_low" => "Reserve too low",
        "gecko_reserve_missing" => "Reserve missing",
        "rug_rugged" => "Rugged token",
        "rug_score" => "Risk score too high",
        "rug_level_danger" => "Danger risk level",
        "rug_mint_authority" => "Mint authority present",
        "rug_freeze_authority" => "Freeze authority present",
        "rug_top_holder" => "Top holder % too high",
        "rug_top3_holders" => "Top 3 holders % too high",
        "rug_min_holders" => "Not enough holders",
        "rug_insider_count" => "Too many insider holders",
        "rug_insider_pct" => "Insider % too high",
        "rug_creator_pct" => "Creator balance too high",
        "rug_transfer_fee_present" => "Transfer fee present",
        "rug_transfer_fee_high" => "Transfer fee too high",
        "rug_transfer_fee_missing" => "Transfer fee data missing",
        "rug_graph_insiders" => "Graph insiders too high",
        "rug_lp_providers_low" => "LP providers too low",
        "rug_lp_providers_missing" => "LP providers missing",
        "rug_lp_lock_low" => "LP lock too low",
        "rug_lp_lock_missing" => "LP lock missing",
        _ => reason, // Return original if not mapped
    }
    .to_string()
}

/// Categorize rejection reason into high-level category
pub fn get_rejection_category(reason: &str) -> &'static str {
    if reason.starts_with("rug_") {
        if reason.contains("authority")
            || reason.contains("rugged")
            || reason.contains("level_danger")
        {
            "security"
        } else if reason.contains("holder")
            || reason.contains("insider")
            || reason.contains("creator")
        {
            "distribution"
        } else if reason.contains("lp_") {
            "liquidity_lock"
        } else if reason.contains("transfer_fee") {
            "fees"
        } else {
            "security"
        }
    } else if reason.starts_with("dex_") || reason.starts_with("gecko_") {
        if reason.contains("liq") || reason.contains("reserve") {
            "liquidity"
        } else if reason.contains("vol") {
            "volume"
        } else if reason.contains("mcap") || reason.contains("fdv") {
            "market_cap"
        } else if reason.contains("price_change") {
            "price_action"
        } else if reason.contains("txn") {
            "activity"
        } else if reason.contains("empty") || reason.contains("missing") {
            "data_quality"
        } else {
            "market"
        }
    } else if reason.contains("decimals") || reason.contains("data_missing") {
        "data_quality"
    } else if reason.contains("cooldown") || reason.contains("new") {
        "timing"
    } else {
        "other"
    }
}

/// Get human-readable category label
pub fn get_category_label(category: &str) -> &'static str {
    match category {
        "security" => "Security Issues",
        "distribution" => "Holder Distribution",
        "liquidity_lock" => "LP Lock Issues",
        "fees" => "Transfer Fees",
        "liquidity" => "Liquidity",
        "volume" => "Trading Volume",
        "market_cap" => "Market Cap/FDV",
        "price_action" => "Price Movement",
        "activity" => "Trading Activity",
        "data_quality" => "Missing Data",
        "timing" => "Timing Filters",
        "market" => "Market Data",
        _ => "Other",
    }
}

/// Get icon for category
pub fn get_category_icon(category: &str) -> &'static str {
    match category {
        "security" => "shield",
        "distribution" => "users",
        "liquidity_lock" => "lock",
        "fees" => "percent",
        "liquidity" => "droplet",
        "volume" => "chart-bar",
        "market_cap" => "dollar-sign",
        "price_action" => "trending-up",
        "activity" => "activity",
        "data_quality" => "circle-alert",
        "timing" => "clock",
        "market" => "trending-up",
        _ => "info",
    }
}
