use clap::Parser;
use screenerbot::global::TOKENS_DATABASE;
use screenerbot::logger::{log, LogTag};
use screenerbot::tokens::database::{init_global_database, TokenDatabase};
use screenerbot::tokens::decimals::{get as get_decimals_async, get_cached as get_cached_decimals};
use screenerbot::tokens::list_tokens_async;
use std::sync::Arc;

/// Inspect token decimals state after the tokens module rewrite.
///
/// Provides two capabilities:
/// 1. Inspect a specific mint and report cached/database decimals.
/// 2. Scan recent tokens missing decimals and optionally fetch them.
#[derive(Parser, Debug)]
#[command(name = "debug_decimals", about = "Inspect token decimals state")]
struct Args {
    /// Inspect a specific mint (skips bulk scan when set)
    #[arg(long)]
    mint: Option<String>,

    /// Number of recently updated tokens to scan for missing decimals
    #[arg(long, default_value_t = 100)]
    limit: usize,

    /// Attempt to fetch decimals for tokens missing them
    #[arg(long, default_value_t = false)]
    fetch: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialise token database so shared helpers work in this standalone binary.
    let db = Arc::new(TokenDatabase::new(TOKENS_DATABASE)?);
    if let Err(err) = init_global_database(db.clone()) {
        log(
            LogTag::Tokens,
            "WARN",
            &format!("Global token database already initialised: {}", err),
        );
    }

    if let Some(mint) = args.mint {
        inspect_single_mint(&mint).await;
        return Ok(());
    }

    inspect_missing_decimals(args.limit, args.fetch).await;
    Ok(())
}

async fn inspect_single_mint(mint: &str) {
    log(
        LogTag::Tokens,
        "DECIMALS_INSPECT",
        &format!("Inspecting decimals for mint={}", mint),
    );

    let cached = get_cached_decimals(mint);
    log(
        LogTag::Tokens,
        "DECIMALS_CACHE_STATE",
        &format!("Cached decimals: {:?}", cached),
    );

    match get_decimals_async(mint).await {
        Some(value) => {
            log(
                LogTag::Tokens,
                "DECIMALS_FOUND",
                &format!("Resolved decimals for mint={} value={}", mint, value),
            );
        }
        None => {
            log(
                LogTag::Tokens,
                "DECIMALS_MISSING",
                &format!("Unable to resolve decimals for mint={}", mint),
            );
        }
    }
}

async fn inspect_missing_decimals(limit: usize, fetch: bool) {
    log(
        LogTag::Tokens,
        "SCAN_START",
        &format!("Scanning up to {} tokens missing decimals", limit),
    );

    let tokens = match list_tokens_async(limit * 5).await {
        Ok(tokens) => tokens,
        Err(err) => {
            log(
                LogTag::Tokens,
                "SCAN_FAILED",
                &format!("Failed to list tokens: {}", err),
            );
            return;
        }
    };

    let mut missing = Vec::new();
    for token in tokens {
        if token.decimals.is_none() {
            missing.push(token);
            if missing.len() >= limit {
                break;
            }
        }
    }

    if missing.is_empty() {
        log(
            LogTag::Tokens,
            "SCAN_COMPLETE",
            "All recent tokens have decimals populated",
        );
        return;
    }

    log(
        LogTag::Tokens,
        "SCAN_RESULTS",
        &format!("Found {} tokens without decimals", missing.len()),
    );

    for token in missing {
        let mint = &token.mint;
        let cached = get_cached_decimals(mint);
        log(
            LogTag::Tokens,
            "DECIMALS_TOKEN",
            &format!(
                "mint={} symbol={:?} cached={:?}",
                mint, token.symbol, cached
            ),
        );

        if fetch {
            match get_decimals_async(mint).await {
                Some(value) => {
                    log(
                        LogTag::Tokens,
                        "DECIMALS_FETCHED",
                        &format!("mint={} decimals={} (stored)", mint, value),
                    );
                }
                None => {
                    log(
                        LogTag::Tokens,
                        "DECIMALS_FETCH_FAILED",
                        &format!("mint={} still missing decimals", mint),
                    );
                }
            }
        }
    }

    log(LogTag::Tokens, "SCAN_COMPLETE", "Decimals scan finished");
}
