use screenerbot::global::is_debug_api_enabled;
use screenerbot::logger::{log, LogTag};
use screenerbot::pools::types::ProgramKind;
use screenerbot::rpc::get_rpc_client;
use screenerbot::tokens::{get_global_dexscreener_api, init_dexscreener_api, TokenDatabase};
use solana_sdk::pubkey::Pubkey;
use std::env;
use std::str::FromStr;
use std::time::Instant;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone)]
struct PoolInfo {
  pub pool_address: String,
  pub program_kind: ProgramKind,
  pub liquidity_usd: f64,
  pub is_sol_pair: bool,
  pub pair_url: Option<String>,
}

#[derive(Debug)]
struct TokenPoolAnalysis {
  pub mint: String,
  pub symbol: String,
  pub name: String,
  pub total_liquidity: f64,
  pub pools: Vec<PoolInfo>,
  pub biggest_pool: Option<PoolInfo>,
  pub target_program_pool: Option<PoolInfo>,
  pub is_target_program_biggest: bool,
}

async fn get_token_pools_analysis(
  mint: &str,
  target_program_kind: ProgramKind,
) -> Result<Option<TokenPoolAnalysis>, String> {
  let dex_api = get_global_dexscreener_api().await?;
  let mut api_lock = dex_api.lock().await;

  // Get all pools for this token from DexScreener
  let pools_result = api_lock.get_solana_token_pairs(mint).await;
  drop(api_lock);

  match pools_result {
    Ok(pairs) => {
      if pairs.is_empty() {
        return Ok(None);
      }

      let mut pools = Vec::new();
      let mut total_liquidity = 0.0;
      let rpc_client = get_rpc_client();
      let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

      // Analyze each pool to get real program kind from on-chain data
      for pair in &pairs {
        let liquidity_usd = pair.liquidity.as_ref().map(|l| l.usd).unwrap_or(0.0);

        // Parse pool address
        let pool_pubkey = match Pubkey::from_str(&pair.pair_address) {
          Ok(pubkey) => pubkey,
          Err(_) => {
            logger::info(
    LogTag::Api,
                &format!("Invalid pool address: {}", pair.pair_address),
              );
            continue;
          }
        };

        // Check if this is a SOL pair (base=token, quote=SOL or base=SOL, quote=token)
        let base_mint = match Pubkey::from_str(&pair.base_token.address) {
          Ok(pubkey) => pubkey,
          Err(_) => {
            continue;
          }
        };
        let quote_mint = match Pubkey::from_str(&pair.quote_token.address) {
          Ok(pubkey) => pubkey,
          Err(_) => {
            continue;
          }
        };

        let is_sol_pair = base_mint == sol_mint || quote_mint == sol_mint;
        if !is_sol_pair {
          // Skip non-SOL pairs
          continue;
        }

        // Get pool account data to determine real program owner
        let account_info = match rpc_client.get_account(&pool_pubkey).await {
          Ok(account) => account,
          Err(e) => {
            logger::info(
    LogTag::Api,
                &format!(
                  "Failed to fetch pool account {}: {}",
                  pair.pair_address, e
                ),
              );
            continue;
          }
        };

        // Determine program kind from actual owner
        let program_kind = ProgramKind::from_program_id(&account_info.owner.to_string());

        if program_kind == ProgramKind::Unknown {
          logger::info(
    LogTag::Api,
              &format!(
                "Unknown program kind for pool {} owned by {}",
                pair.pair_address, account_info.owner
              ),
            );
          continue;
        }

        total_liquidity += liquidity_usd;

        pools.push(PoolInfo {
          pool_address: pair.pair_address.clone(),
          program_kind,
          liquidity_usd,
          is_sol_pair,
          pair_url: Some(pair.url.clone()),
        });
      }

      // Filter to only SOL pairs
      pools.retain(|p| p.is_sol_pair);

      if pools.is_empty() {
        return Ok(None);
      }

      // Sort pools by liquidity (descending)
      pools.sort_by(|a, b| {
        b.liquidity_usd
          .partial_cmp(&a.liquidity_usd)
          .unwrap_or(std::cmp::Ordering::Equal)
      });

      // Find biggest pool overall
      let biggest_pool = pools.first().cloned();

      // Find biggest pool for target program kind
      let target_program_pool = pools
        .iter()
        .find(|p| p.program_kind == target_program_kind)
        .cloned();

      // Check if target program has the biggest pool
      let is_target_program_biggest = biggest_pool
        .as_ref()
        .and_then(|bp| {
          target_program_pool
            .as_ref()
            .map(|tp| bp.pool_address == tp.pool_address)
        })
        .unwrap_or(false);

      let token_info = &pairs[0];
      let symbol = token_info.base_token.symbol.clone();
      let name = token_info.base_token.name.clone();

      Ok(Some(TokenPoolAnalysis {
        mint: mint.to_string(),
        symbol,
        name,
        total_liquidity,
        pools,
        biggest_pool,
        target_program_pool,
        is_target_program_biggest,
      }))
    }
    Err(e) => {
      logger::info(
    LogTag::Api,
          &format!("Failed to get pools for token {}: {}", &mint[..8], e),
        );
      Err(e)
    }
  }
}

async fn find_tokens_with_biggest_pools_by_program(
  target_program_kind: ProgramKind,
  max_tokens_to_check: usize,
  target_count: usize,
) -> Result<Vec<TokenPoolAnalysis>, Box<dyn std::error::Error>> {
  logger::info(
    LogTag::System,
    &format!(
 "Finding tokens with biggest pools for program: {}",
      target_program_kind.display_name()
    ),
  );
  logger::info(
    LogTag::System,
    &format!(
 "Checking top {} tokens by liquidity...",
      max_tokens_to_check
    ),
  );

  let start_time = Instant::now();

  // Get top tokens from database by liquidity
  let db = TokenDatabase::new()?;
  let all_tokens = db.get_all_tokens().await?;

  // Sort by liquidity (descending)
  let mut sorted_tokens = all_tokens;
  sorted_tokens.sort_by(|a, b| {
    let a_liq = a.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
    let b_liq = b.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
    b_liq
      .partial_cmp(&a_liq)
      .unwrap_or(std::cmp::Ordering::Equal)
  });

  logger::info(
    LogTag::System,
 &format!("Found {} tokens in database", sorted_tokens.len()),
  );

  let mut found_tokens = Vec::new();
  let mut checked_count = 0;
  let mut error_count = 0;

  // Check tokens one by one
  for (i, token) in sorted_tokens.iter().take(max_tokens_to_check).enumerate() {
    if found_tokens.len() >= target_count {
      break;
    }

    checked_count += 1;

    if i > 0 && i % 10 == 0 {
      logger::info(
    LogTag::System,
        &format!(
 "Checked {} tokens, found {} matches...",
          i,
          found_tokens.len()
        ),
      );
    }

    // Rate limiting - conservative delay
    if i > 0 {
      sleep(Duration::from_millis(250)).await; // 4 requests per second to stay under DexScreener limits
    }

    match get_token_pools_analysis(&token.mint, target_program_kind).await {
      Ok(Some(analysis)) => {
        if analysis.is_target_program_biggest {
          let target_pool = analysis.target_program_pool.as_ref().unwrap();
          logger::info(
    LogTag::System,
            &format!(
 "Found match #{}: {} ({}) - ${:.2} liquidity in {} pool",
              found_tokens.len() + 1,
              analysis.symbol,
              &analysis.mint[..8],
              target_pool.liquidity_usd,
              target_pool.program_kind.display_name()
            ),
          );
          found_tokens.push(analysis);
        }
      }
      Ok(None) => {
        // No pools found for this token
      }
      Err(e) => {
        error_count += 1;
        logger::info(
    LogTag::Api,
            &format!("Error analyzing token {}: {}", &token.mint[..8], e),
          );
      }
    }
  }

  let elapsed = start_time.elapsed();

  logger::info(
    LogTag::System, "\n Analysis Complete:");
  logger::info(
    LogTag::System,
 &format!("Time taken: {:.2}s", elapsed.as_secs_f64()),
  );
  logger::info(
    LogTag::System,
 &format!("Tokens checked: {}", checked_count),
  );
  logger::info(
    LogTag::System,
 &format!("Matches found: {}", found_tokens.len()),
  );
  logger::info(
    LogTag::System,
 &format!("Errors: {}", error_count),
  );

  Ok(found_tokens)
}

fn print_detailed_results(results: &[TokenPoolAnalysis]) {
  if results.is_empty() {
    logger::info(
    LogTag::System,
      "\n No tokens found where the target program has the biggest pool",
    );
    return;
  }

  logger::info(
    LogTag::System, "\n DETAILED RESULTS:");
  logger::info(
    LogTag::System, &"=".repeat(80));

  for (i, analysis) in results.iter().enumerate() {
    logger::info(
    LogTag::System,
      &format!(
        "\n🪙 Token #{}: {} ({})",
        i + 1,
        analysis.symbol,
        analysis.name
      ),
    );
    logger::info(
    LogTag::System,
 &format!("Mint: {}", analysis.mint),
    );
    logger::info(
    LogTag::System,
 &format!("Total Liquidity: ${:.2}", analysis.total_liquidity),
    );

    if let Some(target_pool) = &analysis.target_program_pool {
      logger::info(
 LogTag::System, "Target Program Pool:");
      logger::info(
    LogTag::System,
 &format!("Pool Address: {}", target_pool.pool_address),
      );
      logger::info(
    LogTag::System,
 &format!("Program: {}", target_pool.program_kind.display_name()),
      );
      logger::info(
    LogTag::System,
 &format!("Liquidity: ${:.2}", target_pool.liquidity_usd),
      );
      if let Some(url) = &target_pool.pair_url {
        logger::info(
 LogTag::System, &format!("URL: {}", url));
      }
    }

    logger::info(
 LogTag::System, "All Pools (top 5):");
    for (j, pool) in analysis.pools.iter().take(5).enumerate() {
      let marker = if Some(&pool.pool_address)
        == analysis
          .target_program_pool
          .as_ref()
          .map(|tp| &tp.pool_address)
      {
        ""
      } else {
 ""
      };
      logger::info(
    LogTag::System,
        &format!(
 "{} {}. {} - ${:.2}",
          marker,
          j + 1,
          pool.program_kind.display_name(),
          pool.liquidity_usd
        ),
      );
    }

    if i < results.len() - 1 {
      logger::info(
    LogTag::System, &"-".repeat(60));
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Parse command line arguments
  let args: Vec<String> = env::args().collect();

  if args.len() < 2 {
    logger::info(
    LogTag::System,
      &format!(
        "Usage: {} <program_name> [max_tokens_to_check] [target_count]",
        args[0]
      ),
    );
    logger::info(
    LogTag::System, "\nExamples:");
    logger::info(
    LogTag::System,
      &format!(
 "{} raydium-cpmm # Find 5 tokens where Raydium CPMM has biggest pool",
        args[0]
      ),
    );
    logger::info(
    LogTag::System,
      &format!(
 "{} orca 200 10 # Check 200 tokens, find 10 where Orca has biggest pool",
        args[0]
      ),
    );
    logger::info(
    LogTag::System,
      &format!(
 "{} pumpfun 50 3 # Check 50 tokens, find 3 where PumpFun has biggest pool",
        args[0]
      ),
    );
    logger::info(
    LogTag::System, "\nSupported Program Names:");
    logger::info(
    LogTag::System,
 "- raydium-cpmm: Raydium CPMM pools",
    );
    logger::info(
    LogTag::System,
 "- raydium-legacy: Raydium Legacy AMM pools",
    );
    logger::info(
    LogTag::System,
 "- raydium-clmm: Raydium CLMM pools",
    );
    logger::info(
 LogTag::System, "- orca: Orca Whirlpool pools");
    logger::info(
    LogTag::System,
 "- meteora-damm: Meteora DAMM pools",
    );
    logger::info(
    LogTag::System,
 "- meteora-dlmm: Meteora DLMM pools",
    );
    logger::info(
 LogTag::System, "- pumpfun: PumpFun AMM pools");
    logger::info(
    LogTag::System,
 "- pumpfun-legacy: PumpFun Legacy pools",
    );
    logger::info(
 LogTag::System, "- moonit: Moonit AMM pools");
    logger::info(
 LogTag::System, "- fluxbeam: FluxBeam AMM pools");
    std::process::exit(1);
  }

  let target_program_name = &args[1];
  let target_program_kind = match target_program_name.to_lowercase().as_str() {
 "raydium-cpmm"| "raydium_cpmm"=> ProgramKind::RaydiumCpmm,
 "raydium-legacy"| "raydium_legacy"| "raydium"=> ProgramKind::RaydiumLegacyAmm,
 "raydium-clmm"| "raydium_clmm"=> ProgramKind::RaydiumClmm,
 "orca"| "orca-whirlpool"| "orca_whirlpool"=> ProgramKind::OrcaWhirlpool,
 "meteora-damm"| "meteora_damm"=> ProgramKind::MeteoraDamm,
 "meteora-dlmm"| "meteora_dlmm"=> ProgramKind::MeteoraDlmm,
 "pumpfun"| "pump-fun"| "pump_fun"=> ProgramKind::PumpFunAmm,
 "pumpfun-legacy"| "pump-fun-legacy"| "pump_fun_legacy"=> ProgramKind::PumpFunLegacy,
 "moonit"=> ProgramKind::Moonit,
 "fluxbeam"| "fluxbeam-amm"| "fluxbeam_amm"=> ProgramKind::FluxbeamAmm,
    _ => {
      logger::info(
    LogTag::System,
 &format!("Unknown program name: {}", target_program_name),
      );
      logger::info(
    LogTag::System,
        "Run with no arguments to see supported program names.",
      );
      std::process::exit(1);
    }
  };
  let max_tokens_to_check = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);
  let target_count = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(5);

  logger::info(
 LogTag::System, "Starting Pool Analysis Tool");
  logger::info(
    LogTag::System,
 &format!("Target Program: {}", target_program_kind.display_name()),
  );
  logger::info(
    LogTag::System,
 &format!("Max tokens to check: {}", max_tokens_to_check),
  );
  logger::info(
    LogTag::System,
 &format!("Target matches: {}", target_count),
  );
  logger::info(
    LogTag::System, "");

  // Initialize services
  logger::info(
 LogTag::System, "Initializing services...");

  // Initialize RPC client
  if let Err(e) = screenerbot::rpc::init_rpc_client() {
    logger::info(
    LogTag::System,
      &format!("RPC config initialization failed: {}", e),
    );
  }

  // Initialize DexScreener API
  init_dexscreener_api().await?;

  logger::info(
    LogTag::System,
 "Services initialized successfully",
  );
  logger::info(
    LogTag::System, "");

  // Find tokens
  let results = find_tokens_with_biggest_pools_by_program(
    target_program_kind,
    max_tokens_to_check,
    target_count,
  )
  .await?;

  // Print results
  print_detailed_results(&results);

  // Print summary
  logger::info(
    LogTag::System, &format!("\n{}", "=".repeat(80)));
  logger::info(
 LogTag::System, "SUMMARY:");
  if !results.is_empty() {
    logger::info(
    LogTag::System,
      &format!(
 "Found {} tokens where '{}'has the biggest pool",
        results.len(),
        target_program_kind.display_name()
      ),
    );
    logger::info(
    LogTag::System,
      &format!(
 "Use these mints for trading strategies focused on {} liquidity",
        target_program_kind.display_name()
      ),
    );
  } else {
    logger::info(
    LogTag::System,
      &format!(
 "No tokens found where '{}'has the biggest pool",
        target_program_kind.display_name()
      ),
    );
    logger::info(
    LogTag::System,
 "Try checking more tokens or a different program type",
    );
  }

  Ok(())
}
