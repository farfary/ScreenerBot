use std::sync::Arc;

use chrono::{DateTime, SecondsFormat, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};

use screenerbot::config::{load_config, with_config};
use screenerbot::paths;
use screenerbot::tokens::database::{get_global_database, init_global_database, TokenDatabase};
use screenerbot::tokens::market::{dexscreener, geckoterminal};
use screenerbot::tokens::security::rugcheck;
use screenerbot::tokens::store;
use screenerbot::tokens::updates::{self, RateLimitCoordinator};
use screenerbot::tokens::{decimals, CacheMetrics, Token, UpdateTrackingInfo};

#[derive(Parser)]
#[command(
    name = "debug_tokens",
    about = "Inspect tokens module state and data flows"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show global tokens module status
    Status,
    /// List tracked tokens ordered by priority and freshness
    List(ListArgs),
    /// Inspect a single token snapshot and optionally refresh it
    Inspect(InspectArgs),
    /// Show update tracking metrics
    Tracking(TrackingArgs),
    /// Trigger data refresh for a token using selected sources
    Update {
        #[arg(value_name = "MINT")]
        mint: String,
        #[arg(long, value_enum, default_value_t = UpdateScope::All)]
        scope: UpdateScope,
    },
    /// Inspect or clear caches
    Cache {
        #[command(subcommand)]
        action: CacheCommand,
    },
    /// Inspect decimals cache and optionally fetch on-chain decimals
    Decimals(DecimalsArgs),
}

#[derive(Args)]
struct ListArgs {
    #[arg(short, long, default_value_t = 20)]
    limit: usize,
    #[arg(long)]
    priority: Option<i32>,
}

#[derive(Args)]
struct InspectArgs {
    #[arg(value_name = "MINT")]
    mint: String,
    #[arg(long)]
    refresh: bool,
    #[arg(long, default_value_t = true)]
    show_market: bool,
    #[arg(long, default_value_t = true)]
    show_security: bool,
    #[arg(long, default_value_t = true)]
    show_tracking: bool,
    #[arg(long)]
    show_caches: bool,
}

#[derive(Args)]
struct TrackingArgs {
    #[arg(long)]
    mint: Option<String>,
    #[arg(short, long, default_value_t = 10)]
    limit: usize,
    #[arg(long)]
    priority: Option<i32>,
}

#[derive(Subcommand)]
enum CacheCommand {
    /// Show cache metrics for market, security, and decimals layers
    Status,
    /// Clear caches for a given scope
    Clear {
        #[arg(value_enum, default_value_t = CacheScope::All)]
        scope: CacheScope,
    },
}

#[derive(Args)]
struct DecimalsArgs {
    #[arg(value_name = "MINT")]
    mint: String,
    #[arg(long)]
    force_chain: bool,
    #[arg(long)]
    clear_cache: bool,
    #[arg(long)]
    persist: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum UpdateScope {
    All,
    Dexscreener,
    Geckoterminal,
    Rugcheck,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CacheScope {
    Market,
    Security,
    Decimals,
    All,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(err) = execute(cli).await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn execute(cli: Cli) -> Result<(), String> {
    paths::ensure_all_directories().map_err(|e| format!("Failed to prepare data directories: {e}"))?;
    load_config()?;

    // Initialize SOL price cache (needed for price_sol calculations)
    if let Err(e) = screenerbot::sol_price::fetch_and_cache_sol_price().await {
        eprintln!("Warning: Failed to fetch SOL price: {}", e);
        eprintln!("  price_sol calculations may be incorrect for non-SOL-paired tokens");
    }

    let db = ensure_database()?;

    match cli.command {
        Command::Status => handle_status(&db).await,
        Command::List(args) => handle_list(&db, args).await,
        Command::Inspect(args) => handle_inspect(&db, args).await,
        Command::Tracking(args) => handle_tracking(&db, args).await,
        Command::Update { mint, scope } => handle_update(&db, &mint, scope).await,
        Command::Cache { action } => handle_cache(action).await,
        Command::Decimals(args) => handle_decimals(&db, args).await,
    }
}

fn ensure_database() -> Result<Arc<TokenDatabase>, String> {
    if let Some(existing) = get_global_database() {
        return Ok(existing);
    }

    let db_path = screenerbot::paths::get_tokens_db_path();
    let db = Arc::new(TokenDatabase::new(&db_path.to_string_lossy()).map_err(|e| e.to_string())?);
    if let Err(e) = init_global_database(db.clone()) {
        if get_global_database().is_none() {
            return Err(e);
        }
    }

    Ok(db)
}

async fn handle_status(db: &Arc<TokenDatabase>) -> Result<(), String> {
    let total_tokens = db.count_tokens().map_err(|e| e.to_string())?;
    let tracked = db.count_tracked_tokens().map_err(|e| e.to_string())?;
    let blacklisted = db.count_blacklisted().map_err(|e| e.to_string())?;
    let priority_summary = db.summarize_priorities().map_err(|e| e.to_string())?;
    let preferred_source = with_config(|cfg| cfg.tokens.preferred_market_data_source.clone());

    let db_path = screenerbot::paths::get_tokens_db_path();
    println!("Tokens database: {}", db_path.display());
    println!("Total tokens: {total_tokens}");
    println!("Tracked tokens: {tracked}");
    println!("Blacklisted tokens: {blacklisted}");
    println!("Preferred market source: {preferred_source}");
    if priority_summary.is_empty() {
        println!("Priority summary: none");
    } else {
        println!("Priority summary:");
        for (priority, count) in priority_summary {
            println!("  priority={priority} count={count}");
        }
    }

    println!("\nCache metrics:");
    print_cache_metrics(
        "DexScreener",
        screenerbot::tokens::dexscreener_cache_metrics(),
        screenerbot::tokens::dexscreener_cache_size(),
    );
    print_cache_metrics(
        "GeckoTerminal",
        screenerbot::tokens::geckoterminal_cache_metrics(),
        screenerbot::tokens::geckoterminal_cache_size(),
    );
    print_cache_metrics(
        "Rugcheck",
        screenerbot::tokens::rugcheck_cache_metrics(),
        screenerbot::tokens::rugcheck_cache_size(),
    );

    Ok(())
}

async fn handle_list(db: &Arc<TokenDatabase>, args: ListArgs) -> Result<(), String> {
    let entries = db
        .list_update_tracking(args.limit, args.priority)
        .map_err(|e| e.to_string())?;

    if entries.is_empty() {
        println!("No tracked tokens found.");
        return Ok(());
    }

    println!(
        "{:<45} {:<12} {:<8} {:<25} {:<25}",
        "Mint", "Symbol", "Priority", "Last Market Update", "Last Security Update"
    );

    for entry in entries {
        let symbol = db
            .get_token(&entry.mint)
            .map_err(|e| e.to_string())?
            .and_then(|meta| meta.symbol)
            .unwrap_or_else(|| "-".to_string());

        println!(
            "{:<45} {:<12} {:<8} {:<25} {:<25}",
            entry.mint,
            symbol,
            entry.priority,
            fmt_datetime(entry.market_data_last_updated_at),
            fmt_datetime(entry.security_data_last_updated_at),
        );
    }

    Ok(())
}

async fn handle_inspect(db: &Arc<TokenDatabase>, mut args: InspectArgs) -> Result<(), String> {
    if args.refresh {
        handle_update(db, &args.mint, UpdateScope::All).await?;
        // Avoid double-refresh when update command prints data already
        args.refresh = false;
    }

    let token = db.get_full_token(&args.mint).map_err(|e| e.to_string())?;

    match token {
        Some(token) => print_token_snapshot(&token),
        None => println!("Token not found in assembled snapshot: {}", args.mint),
    }

    if args.show_market {
        print_market_data(db, &args.mint)?;
    }

    if args.show_security {
        print_security_data(db, &args.mint)?;
    }

    if args.show_tracking {
        print_tracking(db, &args.mint)?;
    }

    if args.show_caches {
        print_cache_hits(&args.mint);
    }

    Ok(())
}

async fn handle_tracking(db: &Arc<TokenDatabase>, args: TrackingArgs) -> Result<(), String> {
    if let Some(mint) = args.mint {
        print_tracking(db, &mint)?;
        return Ok(());
    }

    let entries = db
        .list_update_tracking(args.limit, args.priority)
        .map_err(|e| e.to_string())?;

    if entries.is_empty() {
        println!("No tracking entries found.");
        return Ok(());
    }

    for entry in entries {
        print_tracking_entry(db, &entry)?;
        println!();
    }

    Ok(())
}

async fn handle_update(
    db: &Arc<TokenDatabase>,
    mint: &str,
    scope: UpdateScope,
) -> Result<(), String> {
    // Ensure token metadata row exists to satisfy FK constraints for market/security tables
    // Attempt to prefill decimals via cache/DB/chain
    let dec_opt = decimals::get(mint).await;
    db.upsert_token(mint, None, None, dec_opt)
        .map_err(|e| e.to_string())?;

    match scope {
        UpdateScope::All => {
            let coordinator = RateLimitCoordinator::new();
            let result = updates::update_token(mint, db.as_ref(), &coordinator)
                .await
                .map_err(|e| e.to_string())?;

            if result.successes.is_empty() {
                println!("Update failed for {mint}: {:?}", result.failures);
            } else if result.failures.is_empty() {
                println!("Update succeeded for {mint}: {:?}", result.successes);
            } else {
                println!(
                    "Partial update for {mint}: successes={:?} failures={:?}",
                    result.successes, result.failures
                );
            }
        }
        UpdateScope::Dexscreener => {
            match dexscreener::fetch_dexscreener_data(mint, db.as_ref())
                .await
                .map_err(|e| e.to_string())?
            {
                Some(data) => println!(
                    "DexScreener → price_sol={} liquidity_usd={:?} last_fetch={}",
                    fmt_price(data.price_sol),
                    data.liquidity_usd,
                    data.market_data_last_fetched_at
                        .to_rfc3339_opts(SecondsFormat::Secs, true)
                ),
                None => println!("DexScreener returned no market data for {mint}"),
            }
        }
        UpdateScope::Geckoterminal => {
            match geckoterminal::fetch_geckoterminal_data(mint, db.as_ref())
                .await
                .map_err(|e| e.to_string())?
            {
                Some(data) => println!(
                    "GeckoTerminal → price_sol={} liquidity_usd={:?} last_fetch={}",
                    fmt_price(data.price_sol),
                    data.liquidity_usd,
                    data.market_data_last_fetched_at
                        .to_rfc3339_opts(SecondsFormat::Secs, true)
                ),
                None => println!("GeckoTerminal returned no market data for {mint}"),
            }
        }
        UpdateScope::Rugcheck => match rugcheck::fetch_rugcheck_data(mint, db.as_ref())
            .await
            .map_err(|e| e.to_string())?
        {
            Some(data) => println!(
                "Rugcheck → score={:?} top_10_pct={:?} last_fetch={}",
                data.score,
                data.top_10_holders_pct,
                data.security_data_last_fetched_at
                    .to_rfc3339_opts(SecondsFormat::Secs, true)
            ),
            None => println!("Rugcheck has no report for {mint}"),
        },
    }

    Ok(())
}

async fn handle_cache(action: CacheCommand) -> Result<(), String> {
    match action {
        CacheCommand::Status => {
            println!("Cache status:");
            print_cache_metrics(
                "DexScreener",
                screenerbot::tokens::dexscreener_cache_metrics(),
                screenerbot::tokens::dexscreener_cache_size(),
            );
            print_cache_metrics(
                "GeckoTerminal",
                screenerbot::tokens::geckoterminal_cache_metrics(),
                screenerbot::tokens::geckoterminal_cache_size(),
            );
            print_cache_metrics(
                "Rugcheck",
                screenerbot::tokens::rugcheck_cache_metrics(),
                screenerbot::tokens::rugcheck_cache_size(),
            );
        }
        CacheCommand::Clear { scope } => match scope {
            CacheScope::Market => {
                store::clear_all_market_caches();
                println!("Cleared market caches (DexScreener + GeckoTerminal).");
            }
            CacheScope::Security => {
                store::clear_security_cache();
                println!("Cleared Rugcheck cache.");
            }
            CacheScope::Decimals => {
                decimals::clear_all_cache();
                println!("Cleared decimals cache.");
            }
            CacheScope::All => {
                store::clear_all_market_caches();
                store::clear_security_cache();
                decimals::clear_all_cache();
                println!("Cleared market, security, and decimals caches.");
            }
        },
    }

    Ok(())
}

async fn handle_decimals(db: &Arc<TokenDatabase>, args: DecimalsArgs) -> Result<(), String> {
    if args.clear_cache {
        decimals::clear_cache(&args.mint);
        println!("Cleared in-memory decimals cache for {}", args.mint);
    }

    let cached_before = decimals::get_cached(&args.mint);
    let metadata_decimals = db
        .get_token(&args.mint)
        .map_err(|e| e.to_string())?
        .and_then(|meta| meta.decimals);

    let mut chain_decimals = None;
    if args.force_chain {
        match decimals::get_token_decimals_from_chain(&args.mint).await {
            Ok(value) => {
                decimals::cache(&args.mint, value);
                chain_decimals = Some(value);
                if args.persist {
                    db.upsert_token(&args.mint, None, None, Some(value))
                        .map_err(|e| e.to_string())?;
                    println!("Persisted decimals={} to database", value);
                }
                println!("Fetched on-chain decimals: {}", value);
            }
            Err(err) => println!("Failed to fetch on-chain decimals: {err}"),
        }
    }

    let resolved = decimals::get(&args.mint).await;

    println!("Decimals summary for {}:", args.mint);
    println!("  cached_before={:?}", cached_before);
    println!("  metadata={:?}", metadata_decimals);
    println!("  chain={:?}", chain_decimals);
    println!("  resolved_now={:?}", resolved);

    Ok(())
}

fn print_token_snapshot(token: &Token) {
    println!("--- Token Snapshot ---");
    println!("Mint: {}", token.mint);
    println!("Symbol: {}", token.symbol);
    println!("Name: {}", token.name);
    println!("Decimals: {}", token.decimals);
    println!("Data source: {}", token.data_source.as_str());
    println!(
        "Price: SOL={} USD={}",
        fmt_price(token.price_sol),
        fmt_price(token.price_usd)
    );
    println!("Liquidity USD: {:?}", token.liquidity_usd);
    println!("FDV USD: {:?}", token.fdv);
    println!("Volume (24h): {:?}", token.volume_h24);
    println!("Security score: {:?}", token.security_score);
    println!("Blacklisted: {}", token.is_blacklisted);
    println!(
        "Priority: {:?} (value={})",
        token.priority,
        token.priority.to_value()
    );
    println!(
        "Market data last fetched: {}",
        token
            .market_data_last_fetched_at
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    );
    println!(
        "Pool price last calculated: {}",
        token
            .pool_price_last_calculated_at
            .to_rfc3339_opts(SecondsFormat::Secs, true)
    );
}

fn print_market_data(db: &Arc<TokenDatabase>, mint: &str) -> Result<(), String> {
    let dex = db.get_dexscreener_data(mint).map_err(|e| e.to_string())?;
    let gecko = db.get_geckoterminal_data(mint).map_err(|e| e.to_string())?;

    println!("--- Market Data (database) ---");
    match dex {
        Some(data) => println!(
            "DexScreener → price_sol={} liquidity_usd={:?} last_fetch={}",
            fmt_price(data.price_sol),
            data.liquidity_usd,
            data.market_data_last_fetched_at
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        ),
        None => println!("DexScreener → no record"),
    }

    match gecko {
        Some(data) => println!(
            "GeckoTerminal → price_sol={} liquidity_usd={:?} last_fetch={}",
            fmt_price(data.price_sol),
            data.liquidity_usd,
            data.market_data_last_fetched_at
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        ),
        None => println!("GeckoTerminal → no record"),
    }

    Ok(())
}

fn print_security_data(db: &Arc<TokenDatabase>, mint: &str) -> Result<(), String> {
    let rug = db.get_rugcheck_data(mint).map_err(|e| e.to_string())?;

    println!("--- Security Data (Rugcheck) ---");
    match rug {
        Some(data) => {
            println!("Score: {:?}", data.score);
            println!("Top 10 holders %: {:?}", data.top_10_holders_pct);
            println!(
                "Fetched at: {}",
                data.security_data_last_fetched_at
                    .to_rfc3339_opts(SecondsFormat::Secs, true)
            );
            println!("Mint authority: {:?}", data.mint_authority);
            println!("Freeze authority: {:?}", data.freeze_authority);
            if !data.risks.is_empty() {
                println!("Risks ({} items)", data.risks.len());
                for risk in data.risks.iter().take(5) {
                    println!("  - {}: {}", risk.name, risk.level);
                }
                if data.risks.len() > 5 {
                    println!("  ... ({} more)", data.risks.len() - 5);
                }
            }
        }
        None => println!("No Rugcheck data in database"),
    }

    Ok(())
}

fn print_tracking(db: &Arc<TokenDatabase>, mint: &str) -> Result<(), String> {
    match db
        .get_update_tracking_info(mint)
        .map_err(|e| e.to_string())?
    {
        Some(info) => print_tracking_entry(db, &info)?,
        None => println!("No update tracking entry for {mint}"),
    }
    Ok(())
}

fn print_tracking_entry(db: &Arc<TokenDatabase>, info: &UpdateTrackingInfo) -> Result<(), String> {
    let symbol = db
        .get_token(&info.mint)
        .map_err(|e| e.to_string())?
        .and_then(|meta| meta.symbol)
        .unwrap_or_else(|| "-".to_string());

    println!("Mint: {}", info.mint);
    println!("Symbol: {}", symbol);
    println!("Priority: {}", info.priority);
    println!("Market updates: {}", info.market_data_update_count);
    println!("Security updates: {}", info.security_data_update_count);
    println!(
        "Last market update: {}",
        fmt_datetime(info.market_data_last_updated_at)
    );
    println!(
        "Last security update: {}",
        fmt_datetime(info.security_data_last_updated_at)
    );
    println!(
        "Last decimals update: {}",
        fmt_datetime(info.decimals_last_updated_at)
    );
    if let Some(err) = &info.last_error {
        println!("Last error: {err}");
    }
    if let Some(err_at) = info.last_error_at {
        println!(
            "Last error at: {}",
            err_at.to_rfc3339_opts(SecondsFormat::Secs, true)
        );
    }

    Ok(())
}

fn print_cache_hits(mint: &str) {
    let market_cached = store::get_cached_dexscreener(mint).is_some()
        || store::get_cached_geckoterminal(mint).is_some();
    let security_cached = store::get_cached_rugcheck(mint).is_some();
    let snapshot_cached = store::get_cached_token(mint).is_some();
    let decimals_cached = decimals::get_cached(mint);

    println!("--- Cache Presence ---");
    println!("Market cache: {}", market_cached);
    println!("Security cache: {}", security_cached);
    println!("Token snapshot cache: {}", snapshot_cached);
    println!("Decimals cache: {:?}", decimals_cached);
}

fn print_cache_metrics(name: &str, metrics: CacheMetrics, size: usize) {
    println!(
        "  {name:<14} size={size:<5} hits={} misses={} evictions={} expirations={} hit_rate={:.3}",
        metrics.hits,
        metrics.misses,
        metrics.evictions,
        metrics.expirations,
        metrics.hit_rate()
    );
}

fn fmt_datetime(dt: Option<DateTime<Utc>>) -> String {
    dt.map(|d| d.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_price(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else if value.abs() < 1e-6 {
        format!("{:.6e}", value)
    } else if value.abs() < 1.0 {
        format!("{:.9}", value)
    } else if value.abs() < 1_000.0 {
        format!("{:.4}", value)
    } else {
        format!("{:.2}", value)
    }
}
