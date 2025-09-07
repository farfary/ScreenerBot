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
use screenerbot::pools::types::{ ProgramKind };
use screenerbot::pools::utils::{ is_sol_mint, is_stablecoin_mint };
use screenerbot::rpc::get_rpc_client;
use screenerbot::tokens::cache::TokenDatabase;
use tokio::time::{ sleep, Duration };

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    log(LogTag::System, "INIT", &format!(
        "Starting unsupported pools scan (limit={}, min_liq={})", args.limit, args.min_liquidity
    ));

    // Open token database
    let db = TokenDatabase::new()?;
    let mut tokens = db.get_all_tokens().await.map_err(|e| format!("DB error: {e}"))?;

    // Filter by liquidity threshold early
    tokens.retain(|t| t.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0) >= args.min_liquidity);

    // Already sorted by liquidity desc per query; still ensure
    tokens.sort_by(|a, b| b.liquidity.as_ref().and_then(|l| l.usd).partial_cmp(&a.liquidity.as_ref().and_then(|l| l.usd)).unwrap_or(std::cmp::Ordering::Equal));

    let tokens = tokens.into_iter().take(args.limit).collect::<Vec<_>>();

    log(LogTag::System, "INFO", &format!("Scanning {} tokens", tokens.len()));

    let discovery = PoolDiscovery::new();
    let rpc = get_rpc_client();

    let mut unsupported: Vec<PoolCheckResult> = Vec::new();
    let mut supported: Vec<PoolCheckResult> = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if idx % 25 == 0 && idx > 0 { // light pacing to not spam external APIs
            sleep(Duration::from_millis(150)).await;
        }

        let mint = &token.mint;

        if is_stablecoin_mint(mint) {
            continue; // skip stablecoins
        }

        // Discover pools (discovery already filters stablecoins inside too)
        let pool_descriptors = discovery.discover_pools_for_token(mint).await;

        if pool_descriptors.is_empty() { continue; }

        // Limit pools per token for speed
        let mut considered = 0usize;
        for descriptor in pool_descriptors {
            if considered >= args.max_pools { break; }
            considered += 1;

            let pool_pubkey = descriptor.pool_id;
            let pool_address = pool_pubkey.to_string();

            // Fetch account to get owner program id
            let program_id = match rpc.get_account(&pool_pubkey).await {
                Ok(acc) => acc.owner.to_string(),
                Err(e) => {
                    unsupported.push(PoolCheckResult {
                        token_mint: mint.clone(),
                        pool_address,
                        program_kind: ProgramKind::Unknown,
                        program_id: String::from("<fetch_error>"),
                        liquidity_usd: descriptor.liquidity_usd,
                        supported: false,
                        sol_pair: false,
                        notes: format!("Account fetch failed: {e}"),
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

            // Skip non-SOL pairs entirely (we only care SOL pricing domain)
            if !sol_pair { continue; }

            // Program name filter (applied to both supported/unsupported for narrower scans)
            if let Some(ref pfilter) = args.program {
                if !program_kind.display_name().to_lowercase().contains(&pfilter.to_lowercase()) {
                    continue;
                }
            }

            let notes = if supported_decoder { String::from("decoder available") } else { String::from("no decoder") };

            let result = PoolCheckResult {
                token_mint: mint.clone(),
                pool_address,
                program_kind,
                program_id: program_id.clone(),
                liquidity_usd: descriptor.liquidity_usd,
                supported: supported_decoder,
                sol_pair,
                notes,
            };

            if supported_decoder {
                if args.show_supported { supported.push(result); }
            } else {
                unsupported.push(result);
            }
        }
    }

    // Sort unsupported by liquidity desc to prioritize
    unsupported.sort_by(|a, b| b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));

    log(LogTag::System, "RESULT", &format!("Unsupported SOL pools: {}", unsupported.len()));
    for item in &unsupported { 
        log(LogTag::System, "UNSUPPORTED_POOL", &format!(
            "mint={} pool={} program={} ({}) liq_usd={:.2} notes={}",
            &item.token_mint[..std::cmp::min(8, item.token_mint.len())],
            &item.pool_address[..std::cmp::min(8, item.pool_address.len())],
            item.program_kind.display_name(),
            item.program_id,
            item.liquidity_usd,
            item.notes
        ));
    }

    if args.show_supported {
        supported.sort_by(|a, b| b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));
        log(LogTag::System, "RESULT", &format!("Supported (sampled) SOL pools: {}", supported.len()));
        for item in &supported { 
            log(LogTag::System, "SUPPORTED_POOL", &format!(
                "mint={} pool={} program={} liq_usd={:.2}",
                &item.token_mint[..std::cmp::min(8, item.token_mint.len())],
                &item.pool_address[..std::cmp::min(8, item.pool_address.len())],
                item.program_kind.display_name(),
                item.liquidity_usd,
            ));
        }
    }

    // Summary counts per program for unsupported
    use std::collections::HashMap;
    let mut counts: HashMap<ProgramKind, usize> = HashMap::new();
    for u in &unsupported { *counts.entry(u.program_kind).or_insert(0) += 1; }

    log(LogTag::System, "SUMMARY", "Unsupported pool counts per program kind (Unknown grouped)");
    for (kind, count) in counts { 
        log(LogTag::System, "SUMMARY", &format!("{} => {} pools", kind.display_name(), count));
    }

    Ok(())
}
