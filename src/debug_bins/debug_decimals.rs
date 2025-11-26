use clap::Parser;
use screenerbot::logger::{self as logger, LogTag};
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
    let db_path = screenerbot::paths::get_tokens_db_path();
    let db = Arc::new(TokenDatabase::new(&db_path.to_string_lossy())?);
    if let Err(err) = init_global_database(db.clone()) {
        logger::info(
            LogTag::Tokens,
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
    logger::info(
        LogTag::Tokens,
        &format!("Inspecting decimals for mint={}", mint),
    );

    let cached = get_cached_decimals(mint);
    logger::info(LogTag::Tokens, &format!("Cached decimals: {:?}", cached));

    match get_decimals_async(mint).await {
        Some(value) => {
            logger::info(
                LogTag::Tokens,
                &format!("Resolved decimals for mint={} value={}", mint, value),
            );
        }
        None => {
            logger::info(
                LogTag::Tokens,
                &format!("Unable to resolve decimals for mint={}", mint),
            );
        }
    }
}

async fn inspect_missing_decimals(limit: usize, fetch: bool) {
    logger::info(
        LogTag::Tokens,
        &format!("Scanning up to {} tokens missing decimals", limit),
    );

    let tokens = match list_tokens_async(limit * 5).await {
        Ok(tokens) => tokens,
        Err(err) => {
            logger::info(LogTag::Tokens, &format!("Failed to list tokens: {}", err));
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
        logger::info(LogTag::Tokens, "All recent tokens have decimals populated");
        return;
    }

    logger::info(
        LogTag::Tokens,
        &format!("Found {} tokens without decimals", missing.len()),
    );

    for token in missing {
        let mint = &token.mint;
        let cached = get_cached_decimals(mint);
        logger::info(
            LogTag::Tokens,
            &format!(
                "mint={} symbol={:?} cached={:?}",
                mint, token.symbol, cached
            ),
        );

        if fetch {
            match get_decimals_async(mint).await {
                Some(value) => {
                    logger::info(
                        LogTag::Tokens,
                        &format!("mint={} decimals={} (stored)", mint, value),
                    );
                }
                None => {
                    logger::info(
                        LogTag::Tokens,
                        &format!("mint={} still missing decimals", mint),
                    );
                }
            }
        }
    }

    logger::info(LogTag::Tokens, "Decimals scan finished");
}
