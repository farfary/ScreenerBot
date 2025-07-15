use anyhow::Result;
use screenerbot::{
    config::{ Config, TransactionManagerConfig },
    database::Database,
    wallet::WalletTracker,
    rpc_manager::RpcManager,
    trading::transaction_manager::TransactionManager,
    swap::{
        SwapManager,
        types::{ SwapConfig, SwapRequest, DexType, JupiterConfig, RaydiumConfig, GmgnConfig },
    },
};
use solana_sdk::signature::Keypair;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::time::sleep;

/// Test binary for comprehensive swap functionality testing
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 ScreenerBot Swap Testing Suite");
    println!("═══════════════════════════════════════");
    println!();

    // Setup core components
    println!("🔧 Setting up components...");
    let database = Arc::new(Database::new("test_swap.db")?);
    
    let rpc_manager = Arc::new(RpcManager::new(
        "https://api.mainnet-beta.solana.com".to_string(),
        vec![
            "https://solana.api.xen.network".to_string(),
            "https://api.mainnet-beta.solana.com".to_string(),
        ]
    ));

    // Create test wallet
    let test_keypair = Keypair::new();
    let mut config = Config::default();
    config.main_wallet_private = bs58::encode(&test_keypair.to_bytes()).into_string();

    let wallet_tracker = Arc::new(WalletTracker::new(config.clone(), database.clone())?);
    
    let transaction_manager = Arc::new(TransactionManager::new(
        TransactionManagerConfig {
            cache_transactions: true,
            cache_duration_hours: 24,
            track_pnl: true,
            auto_calculate_profits: true,
        },
        database.clone(),
        wallet_tracker.clone()
    ));

    // Configure swap settings
    let swap_config = SwapConfig {
        enabled: true,
        default_dex: "jupiter".to_string(),
        is_anti_mev: false,
        max_slippage: 0.01, // 1%
        timeout_seconds: 30,
        retry_attempts: 3,
        dex_preferences: vec!["jupiter".to_string(), "raydium".to_string(), "gmgn".to_string()],
        jupiter: JupiterConfig {
            enabled: true,
            base_url: "https://quote-api.jup.ag/v6".to_string(),
            timeout_seconds: 10,
            max_accounts: 64,
            only_direct_routes: false,
            as_legacy_transaction: false,
        },
        raydium: RaydiumConfig {
            enabled: true,
            base_url: "https://api.raydium.io/v2".to_string(),
            timeout_seconds: 10,
            pool_type: "all".to_string(),
        },
        gmgn: GmgnConfig {
            enabled: false, // Disable for testing
            base_url: "https://gmgn.ai/defi/quoterv1".to_string(),
            timeout_seconds: 15,
            api_key: String::new(),
            referral_account: String::new(),
            referral_fee_bps: 0,
        },
    };

    let swap_manager = SwapManager::new(
        swap_config,
        rpc_manager.clone(),
        transaction_manager.clone()
    );

    println!("✅ Components initialized successfully");
    println!();

    // Run comprehensive tests
    test_dex_availability(&swap_manager).await?;
    println!();
    
    test_quote_generation(&swap_manager).await?;
    println!();
    
    test_multiple_tokens(&swap_manager).await?;
    println!();
    
    test_different_amounts(&swap_manager).await?;
    println!();
    
    test_slippage_scenarios(&swap_manager).await?;
    println!();

    println!("🎉 All swap tests completed successfully!");

    Ok(())
}

async fn test_dex_availability(swap_manager: &SwapManager) -> Result<()> {
    println!("🔍 Testing DEX Availability");
    println!("───────────────────────────");

    // Since we don't have check_dex_availability, we'll test by making a simple quote request
    let test_request = SwapRequest {
        input_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
        output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
        amount: 1_000_000, // 0.001 SOL
        slippage_bps: 100, // 1%
        user_public_key: "11111111111111111111111111111111".to_string(),
        dex_preference: None,
        is_anti_mev: false,
    };

    match swap_manager.get_best_quote(&test_request).await {
        Ok(_) => println!("✅ Swap system is online and functional"),
        Err(e) => println!("❌ Swap system error: {}", e),
    }

    Ok(())
}

async fn test_quote_generation(swap_manager: &SwapManager) -> Result<()> {
    println!("💱 Testing Quote Generation");
    println!("────────────────────────────");

    let test_cases = vec![
        (1_000_000, "0.001 SOL to USDC"),
        (10_000_000, "0.01 SOL to USDC"),
        (100_000_000, "0.1 SOL to USDC"),
    ];

    for (amount, description) in test_cases {
        println!("🔄 Testing: {}", description);

        let request = SwapRequest {
            input_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
            output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
            amount,
            slippage_bps: 50, // 0.5%
            user_public_key: "11111111111111111111111111111111".to_string(),
            dex_preference: None,
            is_anti_mev: false,
        };

        let start_time = Instant::now();
        
        match swap_manager.get_best_quote(&request).await {
            Ok(route) => {
                let duration = start_time.elapsed();
                println!("   ✅ Quote received in {:?}", duration);
                println!("   📈 Best DEX: {}", route.dex);
                println!("   💰 Input: {} lamports", route.in_amount);
                println!("   💰 Output: {} tokens", route.out_amount);
                
                // Parse price impact
                if let Ok(impact) = route.price_impact_pct.parse::<f64>() {
                    println!("   📊 Price Impact: {}%", impact);
                }
                
                println!("   🛣️  Route steps: {}", route.route_plan.len());
            }
            Err(e) => {
                println!("   ❌ Quote failed: {}", e);
            }
        }
        
        println!();
        sleep(Duration::from_millis(1000)).await; // Rate limiting
    }

    Ok(())
}

async fn test_multiple_tokens(swap_manager: &SwapManager) -> Result<()> {
    println!("🪙 Testing Multiple Token Pairs");
    println!("────────────────────────────────");

    let token_pairs = vec![
        ("SOL", "So11111111111111111111111111111111111111112", "USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        ("SOL", "So11111111111111111111111111111111111111112", "USDT", "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"),
        ("USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "SOL", "So11111111111111111111111111111111111111112"),
    ];

    for (input_symbol, input_mint, output_symbol, output_mint) in token_pairs {
        println!("🔄 Testing: {} → {}", input_symbol, output_symbol);
        
        let amount = if input_symbol == "SOL" { 1_000_000 } else { 1_000_000 }; // Adjust for token decimals
        
        let request = SwapRequest {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            amount,
            slippage_bps: 100, // 1%
            user_public_key: "11111111111111111111111111111111".to_string(),
            dex_preference: None,
            is_anti_mev: false,
        };

        match swap_manager.get_best_quote(&request).await {
            Ok(route) => {
                println!("   ✅ {} via {}: {} → {}",
                    format!("{} → {}", input_symbol, output_symbol),
                    route.dex,
                    route.in_amount,
                    route.out_amount
                );
            }
            Err(e) => {
                println!("   ❌ Failed: {}", e);
            }
        }
        
        sleep(Duration::from_millis(800)).await;
    }

    Ok(())
}

async fn test_different_amounts(swap_manager: &SwapManager) -> Result<()> {
    println!("💰 Testing Different Trade Amounts");
    println!("───────────────────────────────────");

    let amounts = vec![
        (100_000, "0.0001 SOL (Micro)"),
        (1_000_000, "0.001 SOL (Small)"), 
        (10_000_000, "0.01 SOL (Medium)"),
        (100_000_000, "0.1 SOL (Large)"),
        (1_000_000_000, "1.0 SOL (X-Large)"),
    ];

    for (lamports, description) in amounts {
        println!("🔄 Testing: {}", description);
        
        let request = SwapRequest {
            input_mint: "So11111111111111111111111111111111111111112".to_string(),
            output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            amount: lamports,
            slippage_bps: 100, // 1%
            user_public_key: "11111111111111111111111111111111".to_string(),
            dex_preference: None,
            is_anti_mev: false,
        };

        match swap_manager.get_best_quote(&request).await {
            Ok(route) => {
                let price_impact = route.price_impact_pct.parse::<f64>().unwrap_or(0.0);
                let impact_status = if price_impact < 0.1 {
                    "🟢 Low"
                } else if price_impact < 1.0 {
                    "🟡 Medium" 
                } else {
                    "🔴 High"
                };
                
                println!("   ✅ Output: {} USDC", route.out_amount);
                println!("   📊 Impact: {}% {}", price_impact, impact_status);
            }
            Err(e) => {
                println!("   ❌ Failed: {}", e);
            }
        }
        
        sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}

async fn test_slippage_scenarios(swap_manager: &SwapManager) -> Result<()> {
    println!("⚡ Testing Slippage Scenarios");
    println!("─────────────────────────────");

    let slippage_tests = vec![
        (10, "0.1% (Very Low)"),
        (50, "0.5% (Low)"),
        (100, "1.0% (Normal)"),
        (200, "2.0% (High)"),
        (500, "5.0% (Very High)"),
    ];

    for (slippage_bps, description) in slippage_tests {
        println!("🔄 Testing slippage: {}", description);
        
        let request = SwapRequest {
            input_mint: "So11111111111111111111111111111111111111112".to_string(),
            output_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            amount: 10_000_000, // 0.01 SOL
            slippage_bps,
            user_public_key: "11111111111111111111111111111111".to_string(),
            dex_preference: None,
            is_anti_mev: false,
        };

        let start_time = Instant::now();
        
        match swap_manager.get_best_quote(&request).await {
            Ok(route) => {
                let duration = start_time.elapsed();
                println!("   ✅ Quote time: {:?}", duration);
                println!("   💰 Expected output: {} USDC", route.out_amount);
                
                if let Ok(price_impact) = route.price_impact_pct.parse::<f64>() {
                    println!("   📊 Price impact: {}%", price_impact);
                }
            }
            Err(e) => {
                println!("   ❌ Failed: {}", e);
            }
        }
        
        sleep(Duration::from_millis(300)).await;
    }

    Ok(())
}
