/// Debug tool: find pools that are currently unsupported or failing in calculation
///
/// This tool does NOT start the full pool service. It:
/// - Iterates tokens from the token database (optionally limited by CLI)
/// - Discovers pools (DexScreener + GeckoTerminal + Raydium discovery logic re-used via PoolDiscovery)
/// - For each discovered pool: fetches the on-chain account to get owner (program id)
/// - Classifies program kind (using ProgramKind mapping)
/// - Marks pool as:
///     * supported (decoder implemented) OR
///     * unsupported (unknown program) OR
///     * supported-but-decode-failed (attempt lightweight decode path when possible)
/// - Prints summary tables so we can prioritize decoder work
///
/// IMPORTANT RULES (per project guidelines):
/// - SOL-only focus: we only consider pools where at least one mint is SOL; stablecoin pairs skipped
/// - No duplicate structs/enums; we reuse ProgramKind & existing utils
/// - We do NOT calculate prices here (avoid duplicating calculator logic)
/// - We do NOT mutate global state / start service
/// - We rely on `PoolDiscovery` filtering for stablecoins & SOL pairing
///
/// Usage:
///   cargo run --bin debug_unsupported_pools -- --limit 200 --min-liquidity 1000
///
/// Optional flags:
///   --limit <n>           : Max tokens to scan (default 100)
///   --min-liquidity <usd> : Minimum token liquidity_usd to include (default 0)
///   --program <name>      : Filter results to only pools of a specific program name (case-insensitive substring)
///   --show-supported      : Also list supported pools (default false)
///   --max-pools <n>       : Max pools per token to inspect (default 25)
///
use clap::Parser;
use screenerbot::logger::{ log, LogTag };
use screenerbot::pools::discovery::PoolDiscovery;
use screenerbot::pools::types::ProgramKind;
use screenerbot::pools::utils::{ is_sol_mint, is_stablecoin_mint };
use screenerbot::pools::AccountData;
use screenerbot::rpc::get_rpc_client;
use screenerbot::tokens::cache::TokenDatabase;
use screenerbot::tokens::dexscreener::{ init_dexscreener_api, get_global_dexscreener_api };
use screenerbot::tokens::get_token_decimals_sync;
use tokio::time::{ sleep, Duration };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "debug_unsupported_pools", about = "Find unsupported or failing pools")]
struct Args {
    /// Max tokens to scan (sorted by liquidity desc)
    #[arg(long, default_value = "100")]
    limit: usize,

    /// Minimum token liquidity_usd
    #[arg(long, default_value = "0")]
    min_liquidity: f64,

    /// Filter program display name substring (e.g. "raydium")
    #[arg(long)]
    program: Option<String>,

    /// Show supported pools as well
    #[arg(long, default_value_t = false)]
    show_supported: bool,

    /// Max pools per token to inspect
    #[arg(long, default_value = "25")]
    max_pools: usize,

    /// Test price calculations and compare with API prices
    #[arg(long, default_value_t = false)]
    test_prices: bool,

    /// Price difference threshold (%) to consider calculation wrong
    #[arg(long, default_value = "100.0")]
    price_diff_threshold: f64,
}

#[derive(Debug, Clone)]
struct PoolCheckResult {
    token_mint: String,
    pool_address: String,
    program_kind: ProgramKind,
    program_id: String,
    liquidity_usd: f64,
    supported: bool,
    sol_pair: bool,
    notes: String,
    // Price calculation fields
    calculated_price: Option<f64>,
    api_price: Option<f64>,
    price_diff_percent: Option<f64>,
    calculation_error: Option<String>,
}

#[derive(Debug, Clone)]
struct ProgramErrorStats {
    program_kind: ProgramKind,
    total_pools: usize,
    calculation_errors: usize,
    price_validation_errors: usize,
    error_messages: Vec<String>,
}

/// Calculate pool price and compare with API price
async fn validate_price_availability(
    token_mint: &str,
    pool_address: &str,
    program_kind: ProgramKind,
    test_prices: bool,
    price_diff_threshold: f64
) -> (Option<f64>, Option<f64>, Option<f64>, Option<String>) {
    if !test_prices {
        return (None, None, None, None);
    }

    let mut calculated_price = None;
    let mut api_price = None;
    let mut price_diff_percent = None;
    let mut calculation_error = None;

    // Get token decimals first (required for price calculation)
    let token_decimals = match get_token_decimals_sync(token_mint) {
        Some(decimals) => decimals,
        None => {
            calculation_error = Some("Missing token decimals".to_string());
            log(LogTag::System, "DECIMALS_MISS", &format!("No decimals for {}", &token_mint[..8]));
            return (calculated_price, api_price, price_diff_percent, calculation_error);
        }
    };

    // Try to calculate pool price using the pools API (single pool mode)
    if program_kind != ProgramKind::Unknown {
        match
            calculate_single_pool_price(
                pool_address,
                token_mint,
                program_kind,
                token_decimals
            ).await
        {
            Ok(price) => {
                calculated_price = Some(price);
                log(
                    LogTag::System,
                    "POOL_PRICE_CALC",
                    &format!("Calculated pool price for {}: {:.8} SOL", &token_mint[..8], price)
                );
            }
            Err(e) => {
                calculation_error = Some(e.clone());
                log(
                    LogTag::System,
                    "POOL_PRICE_ERROR",
                    &format!("Pool price calculation failed for {}: {}", &token_mint[..8], e)
                );
            }
        }
    }

    // Get API price for comparison
    {
        let price_opt = if let Ok(api) = get_global_dexscreener_api().await {
            if
                let Ok(mut guard) = tokio::time::timeout(
                    std::time::Duration::from_secs(8),
                    api.lock()
                ).await
            {
                guard.get_price(token_mint).await
            } else {
                None
            }
        } else {
            None
        };

        match price_opt {
            Some(price) => {
                api_price = Some(price);
                log(
                    LogTag::System,
                    "API_PRICE",
                    &format!("API price for {}: {:.8} SOL", &token_mint[..8], price)
                );
            }
            None => {
                log(
                    LogTag::System,
                    "API_PRICE_MISS",
                    &format!("No API price available for {}", &token_mint[..8])
                );
            }
        }
    }

    // Compare prices if both available
    if let (Some(calc), Some(api)) = (calculated_price, api_price) {
        if api > 0.0 {
            let diff_percent = (((calc - api) / api) * 100.0).abs();
            price_diff_percent = Some(diff_percent);

            if diff_percent > price_diff_threshold {
                log(
                    LogTag::System,
                    "PRICE_DIFF_HIGH",
                    &format!(
                        "HIGH PRICE DIFF for {}: calc={:.8} vs api={:.8} ({:.1}%)",
                        &token_mint[..8],
                        calc,
                        api,
                        diff_percent
                    )
                );
            } else {
                log(
                    LogTag::System,
                    "PRICE_MATCH",
                    &format!(
                        "Price match for {}: calc={:.8} vs api={:.8} ({:.1}%)",
                        &token_mint[..8],
                        calc,
                        api,
                        diff_percent
                    )
                );
            }
        }
    }

    (calculated_price, api_price, price_diff_percent, calculation_error)
}

/// Calculate price for a single pool using direct components (no pool service)
async fn calculate_single_pool_price(
    pool_address: &str,
    token_mint: &str,
    program_kind: ProgramKind,
    token_decimals: u8
) -> Result<f64, String> {
    let pool_pubkey = Pubkey::from_str(pool_address).map_err(|e|
        format!("Invalid pool address: {}", e)
    )?;

    let rpc = get_rpc_client();

    // Step 1: Fetch all accounts needed for the pool calculation
    let accounts = fetch_pool_accounts(&pool_pubkey, &program_kind, &rpc).await?;

    // Step 2: Use the appropriate decoder to calculate price
    calculate_price_from_accounts(
        &pool_pubkey,
        token_mint,
        &program_kind,
        &accounts,
        token_decimals
    ).await
}

/// Fetch accounts needed for pool calculation based on program type
async fn fetch_pool_accounts(
    pool_pubkey: &Pubkey,
    program_kind: &ProgramKind,
    rpc: &screenerbot::rpc::RpcClient
) -> Result<Vec<AccountData>, String> {
    let mut accounts = Vec::new();

    // Fetch the main pool account
    match rpc.get_account(pool_pubkey).await {
        Ok(account) => {
            accounts.push(AccountData {
                pubkey: *pool_pubkey,
                data: account.data.clone(),
                slot: 0, // We don't have slot info in this context
                fetched_at: std::time::Instant::now(),
                lamports: account.lamports,
                owner: account.owner,
            });
        }
        Err(e) => {
            return Err(format!("Failed to fetch pool account: {}", e));
        }
    }

    // For now, we'll start with just the main pool account
    // This can be extended to fetch additional accounts based on program type
    log(
        LogTag::System,
        "ACCOUNT_FETCH",
        &format!("Fetched {} accounts for {} pool", accounts.len(), program_kind.display_name())
    );

    Ok(accounts)
}

/// Calculate price from fetched accounts using the appropriate decoder
async fn calculate_price_from_accounts(
    pool_pubkey: &Pubkey,
    token_mint: &str,
    program_kind: &ProgramKind,
    accounts: &[AccountData],
    token_decimals: u8
) -> Result<f64, String> {
    if accounts.is_empty() {
        return Err("No accounts provided for calculation".to_string());
    }

    // For this initial implementation, we'll use a simplified approach
    // that demonstrates the framework but would need full decoder implementation
    match program_kind {
        ProgramKind::OrcaWhirlpool => {
            calculate_orca_whirlpool_price(pool_pubkey, token_mint, accounts, token_decimals).await
        }
        ProgramKind::RaydiumClmm => {
            calculate_raydium_clmm_price(pool_pubkey, token_mint, accounts, token_decimals).await
        }
        ProgramKind::RaydiumCpmm => {
            calculate_raydium_cpmm_price(pool_pubkey, token_mint, accounts, token_decimals).await
        }
        ProgramKind::MeteoraDamm => {
            calculate_meteora_price(pool_pubkey, token_mint, accounts, token_decimals).await
        }
        ProgramKind::PumpFunAmm => {
            calculate_pumpfun_price(pool_pubkey, token_mint, accounts, token_decimals).await
        }
        _ => {
            Err(format!("Price calculation not implemented for {}", program_kind.display_name()))
        }
    }
}

/// Calculate Orca Whirlpool price (simplified version)
async fn calculate_orca_whirlpool_price(
    _pool_pubkey: &Pubkey,
    _token_mint: &str,
    accounts: &[AccountData],
    _token_decimals: u8
) -> Result<f64, String> {
    if accounts.is_empty() {
        return Err("No pool account data".to_string());
    }

    // This is a placeholder for actual Orca Whirlpool decoding
    // The real implementation would decode the account data according to Orca's structure
    log(LogTag::System, "ORCA_DECODE", "Attempting Orca Whirlpool price calculation (placeholder)");

    // For now, return error to show that calculation was attempted but needs implementation
    Err("Orca Whirlpool decoder not fully implemented in debug tool".to_string())
}

/// Calculate Raydium CLMM price (simplified version)
async fn calculate_raydium_clmm_price(
    _pool_pubkey: &Pubkey,
    _token_mint: &str,
    accounts: &[AccountData],
    _token_decimals: u8
) -> Result<f64, String> {
    if accounts.is_empty() {
        return Err("No pool account data".to_string());
    }

    log(
        LogTag::System,
        "RAYDIUM_CLMM_DECODE",
        "Attempting Raydium CLMM price calculation (placeholder)"
    );

    Err("Raydium CLMM decoder not fully implemented in debug tool".to_string())
}

/// Calculate Raydium CPMM price (simplified version)
async fn calculate_raydium_cpmm_price(
    _pool_pubkey: &Pubkey,
    _token_mint: &str,
    accounts: &[AccountData],
    _token_decimals: u8
) -> Result<f64, String> {
    if accounts.is_empty() {
        return Err("No pool account data".to_string());
    }

    log(
        LogTag::System,
        "RAYDIUM_CPMM_DECODE",
        "Attempting Raydium CPMM price calculation (placeholder)"
    );

    Err("Raydium CPMM decoder not fully implemented in debug tool".to_string())
}

/// Calculate Meteora price (simplified version)
async fn calculate_meteora_price(
    _pool_pubkey: &Pubkey,
    _token_mint: &str,
    accounts: &[AccountData],
    _token_decimals: u8
) -> Result<f64, String> {
    if accounts.is_empty() {
        return Err("No pool account data".to_string());
    }

    log(LogTag::System, "METEORA_DECODE", "Attempting Meteora price calculation (placeholder)");

    Err("Meteora decoder not fully implemented in debug tool".to_string())
}

/// Calculate PumpFun price (simplified version)
async fn calculate_pumpfun_price(
    _pool_pubkey: &Pubkey,
    _token_mint: &str,
    accounts: &[AccountData],
    _token_decimals: u8
) -> Result<f64, String> {
    if accounts.is_empty() {
        return Err("No pool account data".to_string());
    }

    log(LogTag::System, "PUMPFUN_DECODE", "Attempting PumpFun price calculation (placeholder)");

    Err("PumpFun decoder not fully implemented in debug tool".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    log(
        LogTag::System,
        "INIT",
        &format!(
            "Starting unsupported pools scan (limit={}, min_liq={})",
            args.limit,
            args.min_liquidity
        )
    );

    // Open token database
    log(LogTag::System, "DATABASE", "Opening token database...");
    let db = TokenDatabase::new()?;
    log(LogTag::System, "DATABASE", "Fetching all tokens from database...");
    let mut tokens = db.get_all_tokens().await.map_err(|e| format!("DB error: {e}"))?;
    log(LogTag::System, "DATABASE", &format!("Retrieved {} tokens from database", tokens.len()));

    // Filter by liquidity threshold early
    let initial_count = tokens.len();
    tokens.retain(
        |t|
            t.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0) >= args.min_liquidity
    );
    log(
        LogTag::System,
        "FILTER",
        &format!(
            "Filtered tokens by min_liquidity ${}: {} -> {}",
            args.min_liquidity,
            initial_count,
            tokens.len()
        )
    );

    // Already sorted by liquidity desc per query; still ensure
    tokens.sort_by(|a, b|
        b.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .partial_cmp(&a.liquidity.as_ref().and_then(|l| l.usd))
            .unwrap_or(std::cmp::Ordering::Equal)
    );

    let tokens = tokens.into_iter().take(args.limit).collect::<Vec<_>>();

    log(LogTag::System, "INFO", &format!("Scanning {} tokens", tokens.len()));

    if tokens.is_empty() {
        log(
            LogTag::System,
            "ERROR",
            "No tokens match the criteria! Check min_liquidity setting or database content."
        );
        return Ok(());
    }

    log(LogTag::System, "INIT", "Initializing pool discovery and RPC client...");
    let discovery = PoolDiscovery::new();
    let rpc = get_rpc_client();

    // Initialize DexScreener API if price testing is enabled
    if args.test_prices {
        log(LogTag::System, "INIT", "Initializing DexScreener API for price testing...");
        if let Err(e) = init_dexscreener_api().await {
            log(
                LogTag::System,
                "WARNING",
                &format!("Failed to initialize DexScreener API: {}. Price testing may fail.", e)
            );
        } else {
            log(LogTag::System, "SUCCESS", "DexScreener API initialized successfully");
        }
    }

    log(LogTag::System, "INIT", "Pool discovery and RPC client ready");

    let mut unsupported: Vec<PoolCheckResult> = Vec::new();
    let mut supported: Vec<PoolCheckResult> = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if idx % 25 == 0 && idx > 0 {
            // light pacing to not spam external APIs
            sleep(Duration::from_millis(150)).await;
        }

        let mint = &token.mint;
        let liquidity_usd = token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);

        log(
            LogTag::System,
            "PROGRESS",
            &format!(
                "Scanning token {}/{}: {} (liq: ${:.2})",
                idx + 1,
                tokens.len(),
                &mint[..std::cmp::min(8, mint.len())],
                liquidity_usd
            )
        );

        if is_stablecoin_mint(mint) {
            log(
                LogTag::System,
                "SKIP",
                &format!("Skipping stablecoin: {}", &mint[..std::cmp::min(8, mint.len())])
            );
            continue; // skip stablecoins
        }

        // Discover pools (discovery already filters stablecoins inside too)
        log(
            LogTag::System,
            "DISCOVERY",
            &format!("Discovering pools for token: {}", &mint[..std::cmp::min(8, mint.len())])
        );
        let pool_descriptors = discovery.discover_pools_for_token(mint).await;

        if pool_descriptors.is_empty() {
            log(
                LogTag::System,
                "NO_POOLS",
                &format!("No pools found for token: {}", &mint[..std::cmp::min(8, mint.len())])
            );
            continue;
        }

        log(
            LogTag::System,
            "POOLS_FOUND",
            &format!(
                "Found {} pools for token: {}",
                pool_descriptors.len(),
                &mint[..std::cmp::min(8, mint.len())]
            )
        );

        // Limit pools per token for speed
        let mut considered = 0usize;
        let total_pools = pool_descriptors.len();
        log(
            LogTag::System,
            "ANALYZING",
            &format!(
                "Analyzing {} pools for token: {}",
                total_pools,
                &mint[..std::cmp::min(8, mint.len())]
            )
        );

        for descriptor in pool_descriptors {
            if considered >= args.max_pools {
                log(
                    LogTag::System,
                    "LIMIT_REACHED",
                    &format!(
                        "Reached max pools limit ({}) for token: {}",
                        args.max_pools,
                        &mint[..std::cmp::min(8, mint.len())]
                    )
                );
                break;
            }
            considered += 1;

            let pool_pubkey = descriptor.pool_id;
            let pool_address = pool_pubkey.to_string();

            log(
                LogTag::System,
                "POOL_CHECK",
                &format!(
                    "Checking pool {}/{}: {} (liq: ${:.2})",
                    considered,
                    std::cmp::min(total_pools, args.max_pools),
                    &pool_address[..std::cmp::min(8, pool_address.len())],
                    descriptor.liquidity_usd
                )
            );

            // Fetch account to get owner program id
            let program_id = match rpc.get_account(&pool_pubkey).await {
                Ok(acc) => {
                    log(
                        LogTag::System,
                        "ACCOUNT_FETCHED",
                        &format!(
                            "Pool {} owner: {}",
                            &pool_address[..std::cmp::min(8, pool_address.len())],
                            acc.owner
                        )
                    );
                    acc.owner.to_string()
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "ACCOUNT_ERROR",
                        &format!(
                            "Failed to fetch account for pool {}: {}",
                            &pool_address[..std::cmp::min(8, pool_address.len())],
                            e
                        )
                    );
                    unsupported.push(PoolCheckResult {
                        token_mint: mint.clone(),
                        pool_address,
                        program_kind: ProgramKind::Unknown,
                        program_id: String::from("<fetch_error>"),
                        liquidity_usd: descriptor.liquidity_usd,
                        supported: false,
                        sol_pair: false,
                        notes: format!("Account fetch failed: {e}"),
                        calculated_price: None,
                        api_price: None,
                        price_diff_percent: None,
                        calculation_error: Some(format!("Account fetch failed: {e}")),
                    });
                    continue;
                }
            };

            let program_kind = ProgramKind::from_program_id(&program_id);
            let supported_decoder = program_kind != ProgramKind::Unknown;

            // Basic SOL pair heuristic: either base or quote mint is SOL; use discovery descriptor fields
            let base_is_sol = is_sol_mint(&descriptor.base_mint.to_string());
            let quote_is_sol = is_sol_mint(&descriptor.quote_mint.to_string());
            let sol_pair = base_is_sol || quote_is_sol;

            log(
                LogTag::System,
                "POOL_ANALYSIS",
                &format!(
                    "Pool {}: program={} supported={} sol_pair={} (base={} quote={})",
                    &pool_address[..std::cmp::min(8, pool_address.len())],
                    program_kind.display_name(),
                    supported_decoder,
                    sol_pair,
                    if base_is_sol {
                        "SOL"
                    } else {
                        &descriptor.base_mint.to_string()
                            [..std::cmp::min(8, descriptor.base_mint.to_string().len())]
                    },
                    if quote_is_sol {
                        "SOL"
                    } else {
                        &descriptor.quote_mint.to_string()
                            [..std::cmp::min(8, descriptor.quote_mint.to_string().len())]
                    }
                )
            );

            // Skip non-SOL pairs entirely (we only care SOL pricing domain)
            if !sol_pair {
                log(
                    LogTag::System,
                    "SKIP_NON_SOL",
                    &format!(
                        "Skipping non-SOL pair: {}",
                        &pool_address[..std::cmp::min(8, pool_address.len())]
                    )
                );
                continue;
            }

            // Program name filter (applied to both supported/unsupported for narrower scans)
            if let Some(ref pfilter) = args.program {
                if !program_kind.display_name().to_lowercase().contains(&pfilter.to_lowercase()) {
                    continue;
                }
            }

            // Check price availability and basic data integrity if price testing is enabled
            let (calculated_price, api_price, price_diff_percent, calculation_error) =
                validate_price_availability(
                    mint,
                    &pool_address,
                    program_kind,
                    args.test_prices,
                    args.price_diff_threshold
                ).await;

            let notes = if supported_decoder {
                if let Some(ref err) = calculation_error {
                    format!("decoder available, calc error: {}", err)
                } else if let Some(diff) = price_diff_percent {
                    if diff > args.price_diff_threshold {
                        format!("decoder available, price diff: {:.1}%", diff)
                    } else {
                        format!("decoder available, price OK ({:.1}%)", diff)
                    }
                } else {
                    String::from("decoder available")
                }
            } else {
                String::from("no decoder")
            };

            let result = PoolCheckResult {
                token_mint: mint.clone(),
                pool_address,
                program_kind,
                program_id: program_id.clone(),
                liquidity_usd: descriptor.liquidity_usd,
                supported: supported_decoder,
                sol_pair,
                notes,
                calculated_price,
                api_price,
                price_diff_percent,
                calculation_error,
            };

            if supported_decoder {
                if args.show_supported {
                    supported.push(result);
                }
            } else {
                unsupported.push(result);
            }
        }
    }

    // Sort unsupported by liquidity desc to prioritize
    unsupported.sort_by(|a, b|
        b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
    );

    log(LogTag::System, "RESULT", &format!("=== SCAN COMPLETE ==="));
    log(LogTag::System, "RESULT", &format!("Total tokens scanned: {}", tokens.len()));
    log(LogTag::System, "RESULT", &format!("Unsupported SOL pools found: {}", unsupported.len()));
    if args.show_supported {
        log(LogTag::System, "RESULT", &format!("Supported SOL pools found: {}", supported.len()));
    }
    log(LogTag::System, "RESULT", &format!("=== UNSUPPORTED POOLS ==="));

    if unsupported.is_empty() {
        log(LogTag::System, "RESULT", "No unsupported pools found!");
    } else {
        for item in &unsupported {
            let price_info = if args.test_prices {
                match (&item.calculated_price, &item.api_price, &item.price_diff_percent) {
                    (Some(calc), Some(api), Some(diff)) => {
                        format!(" calc_price={:.8} api_price={:.8} diff={:.1}%", calc, api, diff)
                    }
                    (Some(calc), None, _) => { format!(" calc_price={:.8} api_price=N/A", calc) }
                    (None, Some(api), _) => { format!(" calc_price=FAILED api_price={:.8}", api) }
                    _ => String::from(" price_test=FAILED"),
                }
            } else {
                String::new()
            };

            log(
                LogTag::System,
                "UNSUPPORTED_POOL",
                &format!(
                    "mint={} pool={} program={} ({}) liq_usd={:.2} notes={}{}",
                    &item.token_mint[..std::cmp::min(8, item.token_mint.len())],
                    &item.pool_address[..std::cmp::min(8, item.pool_address.len())],
                    item.program_kind.display_name(),
                    item.program_id,
                    item.liquidity_usd,
                    item.notes,
                    price_info
                )
            );
        }
    }

    if args.show_supported {
        supported.sort_by(|a, b|
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        );
        log(
            LogTag::System,
            "RESULT",
            &format!("Supported (sampled) SOL pools: {}", supported.len())
        );
        for item in &supported {
            let price_info = if args.test_prices {
                match (&item.calculated_price, &item.api_price, &item.price_diff_percent) {
                    (Some(calc), Some(api), Some(diff)) => {
                        format!(" calc_price={:.8} api_price={:.8} diff={:.1}%", calc, api, diff)
                    }
                    (Some(calc), None, _) => { format!(" calc_price={:.8} api_price=N/A", calc) }
                    (None, Some(api), _) => { format!(" calc_price=FAILED api_price={:.8}", api) }
                    _ => String::from(" price_test=FAILED"),
                }
            } else {
                String::new()
            };

            log(
                LogTag::System,
                "SUPPORTED_POOL",
                &format!(
                    "mint={} pool={} program={} liq_usd={:.2}{}",
                    &item.token_mint[..std::cmp::min(8, item.token_mint.len())],
                    &item.pool_address[..std::cmp::min(8, item.pool_address.len())],
                    item.program_kind.display_name(),
                    item.liquidity_usd,
                    price_info
                )
            );
        }
    }

    // Summary counts per program for unsupported
    use std::collections::HashMap;
    let mut counts: HashMap<ProgramKind, usize> = HashMap::new();
    let mut program_error_stats: HashMap<ProgramKind, ProgramErrorStats> = HashMap::new();

    // Process all pools for error statistics (supported and unsupported)
    let all_pools: Vec<&PoolCheckResult> = unsupported.iter().chain(supported.iter()).collect();

    for pool in &all_pools {
        *counts.entry(pool.program_kind).or_insert(0) += 1;

        let stats = program_error_stats
            .entry(pool.program_kind)
            .or_insert_with(|| ProgramErrorStats {
                program_kind: pool.program_kind,
                total_pools: 0,
                calculation_errors: 0,
                price_validation_errors: 0,
                error_messages: Vec::new(),
            });

        stats.total_pools += 1;

        // Track calculation errors and price validation issues
        if let Some(ref error) = pool.calculation_error {
            stats.calculation_errors += 1;
            if !stats.error_messages.contains(error) {
                stats.error_messages.push(error.clone());
            }
        }

        // Count pools with high price differences
        if let Some(diff) = pool.price_diff_percent {
            if diff > args.price_diff_threshold {
                stats.price_validation_errors += 1;
                let diff_msg = format!("Price diff too high: {:.1}%", diff);
                if !stats.error_messages.contains(&diff_msg) {
                    stats.error_messages.push(diff_msg);
                }
            }
        }

        // Count pools that don't have API prices (potential data gaps)
        if args.test_prices && pool.api_price.is_none() {
            stats.price_validation_errors += 1;
            let no_price_msg = "No API price available".to_string();
            if !stats.error_messages.contains(&no_price_msg) {
                stats.error_messages.push(no_price_msg);
            }
        }
    }

    for u in &unsupported {
        *counts.entry(u.program_kind).or_insert(0) += 1;
    }

    log(LogTag::System, "SUMMARY", "Unsupported pool counts per program kind (Unknown grouped)");
    for (kind, count) in counts {
        log(LogTag::System, "SUMMARY", &format!("{} => {} pools", kind.display_name(), count));
    }

    // Print detailed error analysis per program if price testing was enabled
    if args.test_prices {
        log(LogTag::System, "PRICE_ANALYSIS", "=== DATA AVAILABILITY ANALYSIS ===");
        for (program_kind, stats) in program_error_stats {
            if stats.total_pools > 0 {
                log(
                    LogTag::System,
                    "PROGRAM_STATS",
                    &format!(
                        "{}: {} pools, {} data errors, {} API price gaps",
                        program_kind.display_name(),
                        stats.total_pools,
                        stats.calculation_errors,
                        stats.price_validation_errors
                    )
                );

                // Print specific error messages for this program
                for error_msg in &stats.error_messages {
                    log(
                        LogTag::System,
                        "PROGRAM_ERROR",
                        &format!("  {} issue: {}", program_kind.display_name(), error_msg)
                    );
                }
            }
        }
    }

    Ok(())
}
