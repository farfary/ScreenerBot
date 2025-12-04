/// Real Swap Testing Tool for Jupiter Router
///
/// Tests actual on-chain swaps: SOL → BONK → SOL
/// Uses the new swaps module and config system exactly like main bot flow.
///
/// WARNING: This executes REAL transactions on mainnet!
/// Use small amounts for testing.
///
/// Usage:
/// cargo run -p screenerbot-debug-tools --bin debug_swap_test -- --amount 0.01
/// cargo run -p screenerbot-debug-tools --bin debug_swap_test -- --amount 0.01 --skip-reverse
/// cargo run -p screenerbot-debug-tools --bin debug_swap_test -- --quote-only --amount 0.01
use chrono::Utc;
use clap::Parser;
use screenerbot::{
  config::{load_config, with_config},
  constants::SOL_MINT,
  logger::{self, LogTag},
  swaps::{execute_swap_with_fallback, get_best_quote, QuoteRequest, SwapMode},
  tokens::{decimals, types::DataSource, priorities::Priority, Token},
  utils::{get_sol_balance, get_token_balance, get_wallet_address, lamports_to_sol, sol_to_lamports},
};

/// BONK token mint address (Solana's most popular memecoin for testing)
const BONK_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
const BONK_SYMBOL: &str = "BONK";
const BONK_DECIMALS: u8 = 5;

#[derive(Parser, Debug)]
#[command(name = "debug_swap_test")]
#[command(about = "Test real on-chain swaps via Jupiter (SOL → BONK → SOL)")]
struct Args {
  /// Amount of SOL to swap (default: 0.001 SOL = ~$0.20)
  #[arg(long, default_value = "0.001")]
  amount: f64,

  /// Only get quotes, don't execute swaps
  #[arg(long)]
  quote_only: bool,

  /// Skip the reverse swap (BONK → SOL)
  #[arg(long)]
  skip_reverse: bool,

  /// Custom slippage percentage (overrides config)
  #[arg(long)]
  slippage: Option<f64>,

  /// Custom token mint to test instead of BONK
  #[arg(long)]
  mint: Option<String>,

  /// Custom token symbol (required if --mint is set)
  #[arg(long)]
  symbol: Option<String>,

  /// Custom token decimals (required if --mint is set)
  #[arg(long)]
  decimals: Option<u8>,
}

fn print_separator() {
  println!("\n{}", "=".repeat(70));
}

fn print_header(title: &str) {
  print_separator();
 println!("{}", title);
  print_separator();
}

#[tokio::main]
async fn main() {
  let args = Args::parse();

  // Initialize logger
  logger::init();

  println!("\n Real Swap Test Tool");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("WARNING: This executes REAL transactions on Solana mainnet!");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

  if let Err(e) = run_test(args).await {
 logger::error(LogTag::Swap, &format!("Test failed: {}", e));
    eprintln!("\n Test failed: {}", e);
    std::process::exit(1);
  }
}

async fn run_test(args: Args) -> Result<(), String> {
  // 1. Load configuration
  print_header("INITIALIZATION");
  println!("Loading configuration...");
  load_config().map_err(|e| format!("Failed to load config: {}", e))?;
 println!("Configuration loaded");

  // 2. Get wallet address
  let wallet_address = get_wallet_address().map_err(|e| format!("Failed to get wallet: {}", e))?;
 println!("Wallet: {}", wallet_address);

  // 3. Get slippage from config or args
  let slippage_pct = args.slippage.unwrap_or_else(|| {
    with_config(|cfg| cfg.swaps.slippage.quote_default_pct)
  });
 println!("Slippage: {}%", slippage_pct);

  // 4. Determine token to test
  let (token_mint, token_symbol, token_decimals) = if let Some(mint) = &args.mint {
    let symbol = args.symbol.as_deref().unwrap_or("TOKEN");
    let decimals = args.decimals.unwrap_or_else(|| {
      // Try to get from cache
      decimals::get_cached(mint).unwrap_or(9)
    });
    (mint.as_str(), symbol, decimals)
  } else {
    (BONK_MINT, BONK_SYMBOL, BONK_DECIMALS)
  };
 println!("Test token: {} ({})", token_symbol, token_mint);
 println!("Token decimals: {}", token_decimals);

  // 5. Check initial balances
  print_header("INITIAL BALANCES");
  let initial_sol = get_sol_balance(&wallet_address)
    .await
    .map_err(|e| format!("Failed to get SOL balance: {}", e))?;
 println!("SOL Balance: {:.6} SOL", initial_sol);

  let initial_token = get_token_balance(&wallet_address, token_mint)
    .await
    .unwrap_or(0);
  let initial_token_display = initial_token as f64 / 10_f64.powi(token_decimals as i32);
 println!("{} Balance: {:.6} {}", token_symbol, initial_token_display, token_symbol);

  // Verify we have enough SOL
  if initial_sol < args.amount + 0.01 {
    return Err(format!(
      "Insufficient SOL balance. Need {} + 0.01 for fees, have {}",
      args.amount, initial_sol
    ));
  }

  // 6. Create minimal Token struct for swap execution
  let token = create_test_token(token_mint, token_symbol, token_decimals);

  // =========================================================================
  // SWAP 1: SOL → TOKEN
  // =========================================================================
  print_header(&format!("SWAP 1: SOL → {}", token_symbol));

  let input_lamports = sol_to_lamports(args.amount);
 println!("Input: {} SOL ({} lamports)", args.amount, input_lamports);

  // Create quote request
  let quote_request = QuoteRequest {
    input_mint: SOL_MINT.to_string(),
    output_mint: token_mint.to_string(),
    input_amount: input_lamports,
    wallet_address: wallet_address.clone(),
    slippage_pct,
    swap_mode: SwapMode::ExactIn,
  };

  println!("\n Fetching quote from routers...");
  let quote1 = get_best_quote(quote_request)
    .await
    .map_err(|e| format!("Quote failed: {}", e))?;

  let expected_tokens = quote1.output_amount as f64 / 10_f64.powi(token_decimals as i32);
  println!("\n Best Quote:");
 println!("Router: {}", quote1.router_name);
 println!("Output: {:.6} {} ({} raw)", expected_tokens, token_symbol, quote1.output_amount);
 println!("Price Impact: {:.4}%", quote1.price_impact_pct);
 println!("Slippage: {} bps", quote1.slippage_bps);
 println!("Route: {}", quote1.route_plan);

  if args.quote_only {
    println!("\n Quote-only mode - skipping execution");
  } else {
    println!("\n Executing swap...");
    let start = std::time::Instant::now();

    let result1 = execute_swap_with_fallback(&token, quote1)
      .await
      .map_err(|e| format!("Swap execution failed: {}", e))?;

    println!("\n Swap 1 Complete!");
 println!("Success: {}", result1.success);
 println!("Router: {}", result1.router_name);
 println!("Signature: {}", result1.transaction_signature);
 println!("Input: {} lamports", result1.input_amount);
 println!("Output: {} raw tokens", result1.output_amount);
 println!("Time: {:.2}s", start.elapsed().as_secs_f64());

    // Wait a moment for balances to update
    println!("\n Waiting for balance update...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Check balances after swap 1
    let post_swap1_sol = get_sol_balance(&wallet_address).await.unwrap_or(0.0);
    let post_swap1_token = get_token_balance(&wallet_address, token_mint).await.unwrap_or(0);
    let post_swap1_token_display = post_swap1_token as f64 / 10_f64.powi(token_decimals as i32);

    println!("\n Balances after Swap 1:");
 println!("SOL: {:.6} (delta: {:.6})", post_swap1_sol, post_swap1_sol - initial_sol);
 println!("{}: {:.6} (delta: {:.6})", token_symbol, post_swap1_token_display, post_swap1_token_display - initial_token_display);

    // =========================================================================
    // SWAP 2: TOKEN → SOL (reverse)
    // =========================================================================
    if !args.skip_reverse && post_swap1_token > 0 {
      print_header(&format!("SWAP 2: {} → SOL", token_symbol));

      // Use actual received amount for reverse swap
      let reverse_amount = post_swap1_token;
 println!("Input: {:.6} {} ({} raw)", 
        reverse_amount as f64 / 10_f64.powi(token_decimals as i32),
        token_symbol,
        reverse_amount
      );

      let quote_request2 = QuoteRequest {
        input_mint: token_mint.to_string(),
        output_mint: SOL_MINT.to_string(),
        input_amount: reverse_amount,
        wallet_address: wallet_address.clone(),
        slippage_pct,
        swap_mode: SwapMode::ExactIn,
      };

      println!("\n Fetching quote from routers...");
      let quote2 = get_best_quote(quote_request2)
        .await
        .map_err(|e| format!("Reverse quote failed: {}", e))?;

      let expected_sol = lamports_to_sol(quote2.output_amount);
      println!("\n Best Quote:");
 println!("Router: {}", quote2.router_name);
 println!("Output: {:.6} SOL ({} lamports)", expected_sol, quote2.output_amount);
 println!("Price Impact: {:.4}%", quote2.price_impact_pct);
 println!("Route: {}", quote2.route_plan);

      println!("\n Executing reverse swap...");
      let start2 = std::time::Instant::now();

      let result2 = execute_swap_with_fallback(&token, quote2)
        .await
        .map_err(|e| format!("Reverse swap failed: {}", e))?;

      println!("\n Swap 2 Complete!");
 println!("Success: {}", result2.success);
 println!("Router: {}", result2.router_name);
 println!("Signature: {}", result2.transaction_signature);
 println!("Output: {} lamports ({:.6} SOL)", result2.output_amount, lamports_to_sol(result2.output_amount));
 println!("Time: {:.2}s", start2.elapsed().as_secs_f64());

      // Wait for final balances
      tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    } else if args.skip_reverse {
      println!("\n Skipping reverse swap (--skip-reverse)");
    }
  }

  // =========================================================================
  // FINAL SUMMARY
  // =========================================================================
  print_header("FINAL SUMMARY");

  let final_sol = get_sol_balance(&wallet_address).await.unwrap_or(0.0);
  let final_token = get_token_balance(&wallet_address, token_mint).await.unwrap_or(0);
  let final_token_display = final_token as f64 / 10_f64.powi(token_decimals as i32);

 println!("Initial SOL: {:.6}", initial_sol);
 println!("Final SOL: {:.6}", final_sol);
 println!("SOL Change: {:.6} ({:.2}%)", 
    final_sol - initial_sol,
    ((final_sol - initial_sol) / initial_sol) * 100.0
  );
  println!();
 println!("Initial {}: {:.6}", token_symbol, initial_token_display);
 println!("Final {}: {:.6}", token_symbol, final_token_display);
 println!("{} Change: {:.6}", token_symbol, final_token_display - initial_token_display);

  if !args.quote_only && !args.skip_reverse {
    let net_cost = initial_sol - final_sol;
    println!();
 println!("Net cost (fees + slippage): {:.6} SOL", net_cost);
 println!("Round-trip cost: {:.4}%", (net_cost / args.amount) * 100.0);
  }

  print_separator();
  println!("\n Test completed successfully!\n");

  Ok(())
}

/// Create a minimal Token struct for swap execution
fn create_test_token(mint: &str, symbol: &str, decimals: u8) -> Token {
  let now = Utc::now();
  Token {
    // Core identity
    mint: mint.to_string(),
    symbol: symbol.to_string(),
    name: symbol.to_string(),
    decimals,

    // Optional metadata
    description: None,
    image_url: None,
    header_image_url: None,
    supply: None,

    // Data source configuration
    data_source: DataSource::DexScreener,
    first_discovered_at: now,
    blockchain_created_at: None,
    metadata_last_fetched_at: now,
    decimals_last_fetched_at: now,
    market_data_last_fetched_at: now,
    security_data_last_fetched_at: None,
    pool_price_last_calculated_at: now,
    pool_price_last_used_pool: None,

    // Price information (zeros for test)
    price_usd: 0.0,
    price_sol: 0.0,
    price_native: "0".to_string(),
    price_change_m5: None,
    price_change_h1: None,
    price_change_h6: None,
    price_change_h24: None,

    // Market metrics
    market_cap: None,
    fdv: None,
    liquidity_usd: None,

    // Volume data
    volume_m5: None,
    volume_h1: None,
    volume_h6: None,
    volume_h24: None,
    pool_count: None,
    reserve_in_usd: None,

    // Transaction activity
    txns_m5_buys: None,
    txns_m5_sells: None,
    txns_h1_buys: None,
    txns_h1_sells: None,
    txns_h6_buys: None,
    txns_h6_sells: None,
    txns_h24_buys: None,
    txns_h24_sells: None,

    // Social & links
    websites: vec![],
    socials: vec![],

    // Security information
    mint_authority: None,
    freeze_authority: None,
    security_score: None,
    is_rugged: false,
    token_type: None,
    graph_insiders_detected: None,
    lp_provider_count: None,
    security_risks: vec![],
    total_holders: None,
    top_holders: vec![],
    creator_balance_pct: None,
    transfer_fee_pct: None,
    transfer_fee_max_amount: None,
    transfer_fee_authority: None,

    // Bot-specific state
    is_blacklisted: false,
    priority: Priority::Background,
  }
}
