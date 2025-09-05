use clap::Parser;
use screenerbot::logger::{ log, LogTag };
use screenerbot::pool_service;
use screenerbot::pool_discovery;

/// Pool Service Tool - Initialize and manage pool services
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Show help information
    #[arg(long, short)]
    help: bool,

    /// Token address to initialize from
    #[arg(long)]
    from_token_address: Option<String>,

    /// Pool address to initialize from
    #[arg(long)]
    from_pool_address: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Show help if requested
    if args.help {
        println!("Pool Service Tool");
        println!();
        println!("This tool initializes the pool service and related components.");
        println!();
        println!("Usage:");
        println!("  main_pool_service [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -h, --help                    Show this help message");
        println!("      --from-token-address      Token address to initialize from");
        println!("      --from-pool-address       Pool address to initialize from");
        println!();
        println!("The tool will:");
        println!("  1. Initialize the file logging system");
        println!("  2. Initialize the pool service");
        println!("  3. Log initialization progress");
        println!("  4. Process token/pool addresses if provided");
        return;
    }

    // Initialize logger
    screenerbot::logger::init_file_logging();

    log(LogTag::System, "INIT", "🚀 Starting Pool Service Tool");

    // Initialize pool service
    pool_service::init_pool_service();
    // Initialize pool DB service for persistence used by discovery
    if let Err(e) = screenerbot::pool_db::init_pool_db_service() {
        log(LogTag::System, "DB_INIT_WARN", &format!("Pool DB init warning: {}", e));
    }
    log(LogTag::System, "INIT", "🏊 Pool Service initialized");

    // Process token address if provided
    if let Some(token_address) = &args.from_token_address {
        log(
            LogTag::System,
            "TOKEN_ADDRESS",
            &format!("🎯 Processing token address: {}", token_address)
        );

        // Initialize discovery and run triple-API discovery for this token
        let token = token_address.clone();
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async move {
            let discovery = pool_discovery::init_pool_discovery();

            // Fetch token from local database and print full info before discovery
            match screenerbot::tokens::TokenDatabase::new() {
                Ok(db) =>
                    match db.get_token_by_mint(&token) {
                        Ok(Some(api)) => {
                            let short_mint = screenerbot::utils::safe_truncate(&token, 8);
                            let liq = api.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0);
                            let liq_base = api.liquidity
                                .as_ref()
                                .and_then(|l| l.base)
                                .unwrap_or(0.0);
                            let liq_quote = api.liquidity
                                .as_ref()
                                .and_then(|l| l.quote)
                                .unwrap_or(0.0);
                            let vols = api.volume.as_ref();
                            let txns = api.txns.as_ref();
                            let changes = api.price_change.as_ref();
                            let labels = api.labels.clone().unwrap_or_default().join(", ");
                            let pair_addr_short = if api.pair_address.is_empty() {
                                "--------".to_string()
                            } else {
                                screenerbot::utils::safe_truncate(&api.pair_address, 8).to_string()
                            };
                            let pair_url = api.pair_url.clone().unwrap_or_else(|| "-".to_string());
                            let created_at = api.pair_created_at
                                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| "-".to_string());
                            let updated_at = api.last_updated.to_rfc3339();
                            let image_url = api.info
                                .as_ref()
                                .and_then(|i| i.image_url.clone())
                                .unwrap_or_else(|| "-".to_string());

                            log(
                                LogTag::System,
                                "TOKEN_DB",
                                &format!(
                                    "{} ({}) mint={} chain={} dex={} pair={} url={}",
                                    api.symbol,
                                    api.name,
                                    short_mint,
                                    api.chain_id,
                                    api.dex_id,
                                    pair_addr_short,
                                    pair_url
                                )
                            );
                            log(
                                LogTag::System,
                                "TOKEN_PRICES",
                                &format!(
                                    "price_native={:.9} price_usd=${:.6} price_sol={}",
                                    api.price_native,
                                    api.price_usd,
                                    api.price_sol
                                        .map(|v| format!("{:.9}", v))
                                        .unwrap_or_else(|| "-".to_string())
                                )
                            );
                            log(
                                LogTag::System,
                                "TOKEN_LIQUIDITY",
                                &format!(
                                    "liquidity_usd=${:.2} base={:.2} quote={:.2}",
                                    liq,
                                    liq_base,
                                    liq_quote
                                )
                            );
                            log(
                                LogTag::System,
                                "TOKEN_VOLUME",
                                &format!(
                                    "vol m5={:.2} h1={:.2} h6={:.2} h24={:.2}",
                                    vols.and_then(|v| v.m5).unwrap_or(0.0),
                                    vols.and_then(|v| v.h1).unwrap_or(0.0),
                                    vols.and_then(|v| v.h6).unwrap_or(0.0),
                                    vols.and_then(|v| v.h24).unwrap_or(0.0)
                                )
                            );
                            log(
                                LogTag::System,
                                "TOKEN_TXNS",
                                &format!(
                                    "m5={{buys:{},sells:{}}} h1={{buys:{},sells:{}}} h6={{buys:{},sells:{}}} h24={{buys:{},sells:{}}}",
                                    txns
                                        .and_then(|t| t.m5.as_ref())
                                        .and_then(|p| p.buys)
                                        .unwrap_or(0),
                                    txns
                                        .and_then(|t| t.m5.as_ref())
                                        .and_then(|p| p.sells)
                                        .unwrap_or(0),
                                    txns
                                        .and_then(|t| t.h1.as_ref())
                                        .and_then(|p| p.buys)
                                        .unwrap_or(0),
                                    txns
                                        .and_then(|t| t.h1.as_ref())
                                        .and_then(|p| p.sells)
                                        .unwrap_or(0),
                                    txns
                                        .and_then(|t| t.h6.as_ref())
                                        .and_then(|p| p.buys)
                                        .unwrap_or(0),
                                    txns
                                        .and_then(|t| t.h6.as_ref())
                                        .and_then(|p| p.sells)
                                        .unwrap_or(0),
                                    txns
                                        .and_then(|t| t.h24.as_ref())
                                        .and_then(|p| p.buys)
                                        .unwrap_or(0),
                                    txns
                                        .and_then(|t| t.h24.as_ref())
                                        .and_then(|p| p.sells)
                                        .unwrap_or(0)
                                )
                            );
                            log(
                                LogTag::System,
                                "TOKEN_CHANGE",
                                &format!(
                                    "Δ m5={:+.2}% h1={:+.2}% h6={:+.2}% h24={:+.2}%",
                                    changes.and_then(|c| c.m5).unwrap_or(0.0),
                                    changes.and_then(|c| c.h1).unwrap_or(0.0),
                                    changes.and_then(|c| c.h6).unwrap_or(0.0),
                                    changes.and_then(|c| c.h24).unwrap_or(0.0)
                                )
                            );
                            log(
                                LogTag::System,
                                "TOKEN_CAPS",
                                &format!(
                                    "fdv=${:.2} mcap=${:.2} created_at={} updated_at={} image={} labels=[{}]",
                                    api.fdv.unwrap_or(0.0),
                                    api.market_cap.unwrap_or(0.0),
                                    created_at,
                                    updated_at,
                                    image_url,
                                    labels
                                )
                            );
                        }
                        Ok(None) | Err(_) => {
                            // Fallback: fetch from API, store to DB, then print
                            if let Err(e) = screenerbot::tokens::init_dexscreener_api().await {
                                log(
                                    LogTag::System,
                                    "TOKEN_INFO",
                                    &format!(
                                        "Failed to init DexScreener API for {}: {}",
                                        screenerbot::utils::safe_truncate(&token, 8),
                                        e
                                    )
                                );
                            } else {
                                match
                                    screenerbot::tokens::get_token_from_mint_global_api(
                                        &token
                                    ).await
                                {
                                    Ok(Some(tok)) => {
                                        let mut api: screenerbot::tokens::types::ApiToken = tok.into();
                                        // Ensure chain_id and last_updated are set sensibly
                                        if api.chain_id.is_empty() {
                                            api.chain_id = "solana".to_string();
                                        }
                                        // Persist
                                        let _ = db.add_tokens(&[api.clone()]).await;

                                        // Print same detailed info
                                        let short_mint = screenerbot::utils::safe_truncate(
                                            &token,
                                            8
                                        );
                                        let liq = api.liquidity
                                            .as_ref()
                                            .and_then(|l| l.usd)
                                            .unwrap_or(0.0);
                                        let liq_base = api.liquidity
                                            .as_ref()
                                            .and_then(|l| l.base)
                                            .unwrap_or(0.0);
                                        let liq_quote = api.liquidity
                                            .as_ref()
                                            .and_then(|l| l.quote)
                                            .unwrap_or(0.0);
                                        let vols = api.volume.as_ref();
                                        let txns = api.txns.as_ref();
                                        let changes = api.price_change.as_ref();
                                        let labels = api.labels
                                            .clone()
                                            .unwrap_or_default()
                                            .join(", ");
                                        let pair_addr_short = if api.pair_address.is_empty() {
                                            "--------".to_string()
                                        } else {
                                            screenerbot::utils
                                                ::safe_truncate(&api.pair_address, 8)
                                                .to_string()
                                        };
                                        let pair_url = api.pair_url
                                            .clone()
                                            .unwrap_or_else(|| "-".to_string());
                                        let created_at = api.pair_created_at
                                            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                                            .map(|dt| dt.to_rfc3339())
                                            .unwrap_or_else(|| "-".to_string());
                                        let updated_at = api.last_updated.to_rfc3339();
                                        let image_url = api.info
                                            .as_ref()
                                            .and_then(|i| i.image_url.clone())
                                            .unwrap_or_else(|| "-".to_string());

                                        log(
                                            LogTag::System,
                                            "TOKEN_DB",
                                            &format!(
                                                "{} ({}) mint={} chain={} dex={} pair={} url={}",
                                                api.symbol,
                                                api.name,
                                                short_mint,
                                                api.chain_id,
                                                api.dex_id,
                                                pair_addr_short,
                                                pair_url
                                            )
                                        );
                                        log(
                                            LogTag::System,
                                            "TOKEN_PRICES",
                                            &format!(
                                                "price_native={:.9} price_usd=${:.6} price_sol={}",
                                                api.price_native,
                                                api.price_usd,
                                                api.price_sol
                                                    .map(|v| format!("{:.9}", v))
                                                    .unwrap_or_else(|| "-".to_string())
                                            )
                                        );
                                        log(
                                            LogTag::System,
                                            "TOKEN_LIQUIDITY",
                                            &format!(
                                                "liquidity_usd=${:.2} base={:.2} quote={:.2}",
                                                liq,
                                                liq_base,
                                                liq_quote
                                            )
                                        );
                                        log(
                                            LogTag::System,
                                            "TOKEN_VOLUME",
                                            &format!(
                                                "vol m5={:.2} h1={:.2} h6={:.2} h24={:.2}",
                                                vols.and_then(|v| v.m5).unwrap_or(0.0),
                                                vols.and_then(|v| v.h1).unwrap_or(0.0),
                                                vols.and_then(|v| v.h6).unwrap_or(0.0),
                                                vols.and_then(|v| v.h24).unwrap_or(0.0)
                                            )
                                        );
                                        log(
                                            LogTag::System,
                                            "TOKEN_TXNS",
                                            &format!(
                                                "m5={{buys:{},sells:{}}} h1={{buys:{},sells:{}}} h6={{buys:{},sells:{}}} h24={{buys:{},sells:{}}}",
                                                txns
                                                    .and_then(|t| t.m5.as_ref())
                                                    .and_then(|p| p.buys)
                                                    .unwrap_or(0),
                                                txns
                                                    .and_then(|t| t.m5.as_ref())
                                                    .and_then(|p| p.sells)
                                                    .unwrap_or(0),
                                                txns
                                                    .and_then(|t| t.h1.as_ref())
                                                    .and_then(|p| p.buys)
                                                    .unwrap_or(0),
                                                txns
                                                    .and_then(|t| t.h1.as_ref())
                                                    .and_then(|p| p.sells)
                                                    .unwrap_or(0),
                                                txns
                                                    .and_then(|t| t.h6.as_ref())
                                                    .and_then(|p| p.buys)
                                                    .unwrap_or(0),
                                                txns
                                                    .and_then(|t| t.h6.as_ref())
                                                    .and_then(|p| p.sells)
                                                    .unwrap_or(0),
                                                txns
                                                    .and_then(|t| t.h24.as_ref())
                                                    .and_then(|p| p.buys)
                                                    .unwrap_or(0),
                                                txns
                                                    .and_then(|t| t.h24.as_ref())
                                                    .and_then(|p| p.sells)
                                                    .unwrap_or(0)
                                            )
                                        );
                                        log(
                                            LogTag::System,
                                            "TOKEN_CHANGE",
                                            &format!(
                                                "Δ m5={:+.2}% h1={:+.2}% h6={:+.2}% h24={:+.2}%",
                                                changes.and_then(|c| c.m5).unwrap_or(0.0),
                                                changes.and_then(|c| c.h1).unwrap_or(0.0),
                                                changes.and_then(|c| c.h6).unwrap_or(0.0),
                                                changes.and_then(|c| c.h24).unwrap_or(0.0)
                                            )
                                        );
                                        log(
                                            LogTag::System,
                                            "TOKEN_CAPS",
                                            &format!(
                                                "fdv=${:.2} mcap=${:.2} created_at={} updated_at={} image={} labels=[{}]",
                                                api.fdv.unwrap_or(0.0),
                                                api.market_cap.unwrap_or(0.0),
                                                created_at,
                                                updated_at,
                                                image_url,
                                                labels
                                            )
                                        );
                                    }
                                    Ok(None) | Err(_) =>
                                        log(
                                            LogTag::System,
                                            "TOKEN_INFO",
                                            &format!(
                                                "Token {} not found in local DB (data/tokens.db)",
                                                screenerbot::utils::safe_truncate(&token, 8)
                                            )
                                        ),
                                }
                            }
                        }
                    }
                Err(_) =>
                    log(
                        LogTag::System,
                        "TOKEN_INFO",
                        &format!(
                            "Token {} not found in local DB (data/tokens.db)",
                            screenerbot::utils::safe_truncate(&token, 8)
                        )
                    ),
            }

            match discovery.discover_pools_batch(&[token.clone()]).await {
                Ok(map) => {
                    if let Some(pools) = map.get(&token) {
                        log(
                            LogTag::System,
                            "DISCOVERY_RESULT",
                            &format!(
                                "✅ Discovered {} TOKEN/SOL pools for {}",
                                pools.len(),
                                screenerbot::utils::safe_truncate(&token, 8)
                            )
                        );
                        // Log top 3 by liquidity
                        let mut pools_sorted = pools.clone();
                        pools_sorted.sort_by(|a, b|
                            b.liquidity_usd
                                .partial_cmp(&a.liquidity_usd)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        );
                        for (i, p) in pools_sorted.iter().take(3).enumerate() {
                            let pair_short = screenerbot::utils::safe_truncate(&p.pair_address, 8);
                            log(
                                LogTag::System,
                                "POOL",
                                &format!(
                                    "#{} {} liq=${:.2} vol24h=${:.2} dex={}",
                                    i + 1,
                                    pair_short,
                                    p.liquidity_usd,
                                    p.volume_24h,
                                    p.dex_id
                                )
                            );
                        }
                    } else {
                        log(
                            LogTag::System,
                            "DISCOVERY_EMPTY",
                            &format!(
                                "⚪ No pools discovered for {}",
                                screenerbot::utils::safe_truncate(&token, 8)
                            )
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "DISCOVERY_ERROR",
                        &format!(
                            "❌ Discovery error for {}: {}",
                            screenerbot::utils::safe_truncate(&token, 8),
                            e
                        )
                    );
                }
            }
        });
    }

    // Process pool address if provided
    if let Some(pool_address) = &args.from_pool_address {
        log(
            LogTag::System,
            "POOL_ADDRESS",
            &format!("🏊 Processing pool address: {}", pool_address)
        );
        // TODO: Add pool address processing logic here
    }

    log(LogTag::System, "SUCCESS", "✅ Pool Service initialized successfully");
    log(LogTag::System, "INFO", "Pool Service Tool is ready");
}
