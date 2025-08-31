/// Token Authority Debug Tool
///
/// This binary allows testing and debugging of token authority checking functionality.
/// It can check authorities for individual tokens or batch process multiple tokens.

use screenerbot::errors::ScreenerBotError;
use screenerbot::logger::{ log, LogTag };
use screenerbot::rpc::init_rpc_client;
use screenerbot::tokens::authority::{
    get_authority_summary,
    get_multiple_token_authorities,
    get_token_authorities,
    is_token_safe,
    TokenAuthorities,
    TokenRiskLevel,
};
use std::env;
use std::io::{ self, Write };

/// Test tokens for authority checking (mix of known safe and risky tokens)
const TEST_TOKENS: &[(&str, &str)] = &[
    ("So11111111111111111111111111111111111111112", "SOL (Wrapped)"),
    ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "USDC"),
    ("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", "USDT"),
    ("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", "Bonk"),
    ("jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v", "jupSOL"),
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    println!("🔍 Token Authority Debug Tool");
    println!("=============================");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return run_interactive_mode().await;
    }

    let command = &args[1];

    match command.as_str() {
        "check" => {
            if args.len() < 3 {
                eprintln!("Error: Token mint address required for 'check' command");
                print_usage();
                return Ok(());
            }
            check_single_token(&args[2]).await?;
        }
        "batch" => {
            if args.len() < 3 {
                eprintln!("Error: File path or token list required for 'batch' command");
                print_usage();
                return Ok(());
            }
            batch_check_tokens(&args[2..]).await?;
        }
        "test" => {
            test_known_tokens().await?;
        }
        "interactive" | "i" => {
            return run_interactive_mode().await;
        }
        "safe" => {
            if args.len() < 3 {
                eprintln!("Error: Token mint address required for 'safe' command");
                print_usage();
                return Ok(());
            }
            check_token_safety(&args[2]).await?;
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", command);
            print_usage();
            return Ok(());
        }
    }

    Ok(())
}

fn print_usage() {
    println!("\nUsage: main_authority_debug <command> [options]");
    println!("\nCommands:");
    println!("  check <mint>        Check authorities for a single token");
    println!("  batch <mint1> <mint2> ...  Check authorities for multiple tokens");
    println!("  test               Check authorities for known test tokens");
    println!("  safe <mint>        Quick safety check for a token");
    println!("  interactive, i     Run in interactive mode");
    println!("  help, --help, -h   Show this help message");
    println!("\nExamples:");
    println!(
        "  cargo run --bin main_authority_debug check So11111111111111111111111111111111111111112"
    );
    println!("  cargo run --bin main_authority_debug test");
    println!(
        "  cargo run --bin main_authority_debug safe EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    println!("  cargo run --bin main_authority_debug interactive");
}

async fn initialize_system() -> Result<(), ScreenerBotError> {
    log(LogTag::System, "INIT", "Initializing RPC client...");

    // Initialize RPC client
    init_rpc_client().map_err(|e| {
        ScreenerBotError::Configuration(screenerbot::errors::ConfigurationError::Generic {
            message: format!("Failed to initialize RPC client: {}", e),
        })
    })?;

    log(LogTag::System, "SUCCESS", "System initialized successfully");
    Ok(())
}

async fn check_single_token(mint: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Checking authorities for token: {}", mint);
    println!("{}", "=".repeat(60));

    // Initialize system
    initialize_system().await?;

    // Check authorities
    match get_token_authorities(mint).await {
        Ok(authorities) => {
            print_token_authorities(&authorities);
            print_risk_analysis(&authorities);
        }
        Err(e) => {
            eprintln!("❌ Error checking token authorities: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn batch_check_tokens(mints: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Batch checking authorities for {} tokens", mints.len());
    println!("{}", "=".repeat(60));

    // Initialize system
    initialize_system().await?;

    // Check authorities
    match get_multiple_token_authorities(mints).await {
        Ok(authorities_list) => {
            println!("\n📊 Batch Results:");
            println!("{}", "-".repeat(60));

            for authorities in &authorities_list {
                print_compact_token_info(authorities);
            }

            // Print summary
            print_batch_summary(&authorities_list);
        }
        Err(e) => {
            eprintln!("❌ Error in batch checking: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn test_known_tokens() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧪 Testing known tokens for authority checking");
    println!("{}", "=".repeat(60));

    // Initialize system
    initialize_system().await?;

    let mints: Vec<String> = TEST_TOKENS.iter()
        .map(|(mint, _)| mint.to_string())
        .collect();

    match get_multiple_token_authorities(&mints).await {
        Ok(authorities_list) => {
            for (i, authorities) in authorities_list.iter().enumerate() {
                let (_, name) = TEST_TOKENS[i];
                println!("\n{} {}", authorities.get_risk_level().get_color_code(), name);
                println!("Mint: {}", authorities.mint);
                println!("Summary: {}", authorities.get_authority_summary());
                println!("Risk Level: {}", authorities.get_risk_level().as_str());
                println!("Rug Safe: {}", if authorities.is_rug_safe() {
                    "✅ Yes"
                } else {
                    "⚠️ No"
                });
                println!("{}", "-".repeat(40));
            }

            print_batch_summary(&authorities_list);
        }
        Err(e) => {
            eprintln!("❌ Error testing known tokens: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn check_token_safety(mint: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🛡️ Safety check for token: {}", mint);
    println!("{}", "=".repeat(60));

    // Initialize system
    initialize_system().await?;

    match is_token_safe(mint).await {
        Ok(is_safe) => {
            if is_safe {
                println!("✅ Token is considered SAFE (rug-safe)");
                println!("   - Mint authority is disabled");
                println!("   - Freeze authority is disabled");
            } else {
                println!("⚠️ Token has potential RISKS");
                println!("   - One or more dangerous authorities are active");
            }

            // Get detailed info
            if let Ok(summary) = get_authority_summary(mint).await {
                println!("\nDetailed Summary: {}", summary);
            }
        }
        Err(e) => {
            eprintln!("❌ Error checking token safety: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn run_interactive_mode() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎮 Interactive Authority Checker");
    println!("{}", "=".repeat(60));
    println!("Commands:");
    println!("  check <mint>     - Check a single token");
    println!("  safe <mint>      - Quick safety check");
    println!("  test             - Test known tokens");
    println!("  help             - Show commands");
    println!("  quit, exit       - Exit the program");
    println!();

    // Initialize system once
    initialize_system().await?;

    loop {
        print!("authority> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0];

        match command {
            "check" => {
                if parts.len() < 2 {
                    println!("Usage: check <mint>");
                    continue;
                }
                if let Err(e) = check_single_token_interactive(parts[1]).await {
                    eprintln!("Error: {}", e);
                }
            }
            "safe" => {
                if parts.len() < 2 {
                    println!("Usage: safe <mint>");
                    continue;
                }
                if let Err(e) = check_token_safety_interactive(parts[1]).await {
                    eprintln!("Error: {}", e);
                }
            }
            "test" => {
                if let Err(e) = test_known_tokens_interactive().await {
                    eprintln!("Error: {}", e);
                }
            }
            "help" => {
                println!("Commands:");
                println!("  check <mint>     - Check a single token");
                println!("  safe <mint>      - Quick safety check");
                println!("  test             - Test known tokens");
                println!("  help             - Show commands");
                println!("  quit, exit       - Exit the program");
            }
            "quit" | "exit" => {
                println!("👋 Goodbye!");
                break;
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for available commands.", command);
            }
        }
    }

    Ok(())
}

async fn check_single_token_interactive(mint: &str) -> Result<(), ScreenerBotError> {
    println!("\n🔍 Checking: {}", mint);

    match get_token_authorities(mint).await {
        Ok(authorities) => {
            print_token_authorities(&authorities);
            print_risk_analysis(&authorities);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn check_token_safety_interactive(mint: &str) -> Result<(), ScreenerBotError> {
    println!("\n🛡️ Safety check: {}", mint);

    match is_token_safe(mint).await {
        Ok(is_safe) => {
            let status = if is_safe { "✅ SAFE" } else { "⚠️ RISKY" };
            println!("Result: {}", status);

            if let Ok(summary) = get_authority_summary(mint).await {
                println!("Summary: {}", summary);
            }
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

async fn test_known_tokens_interactive() -> Result<(), ScreenerBotError> {
    println!("\n🧪 Testing known tokens...");

    let mints: Vec<String> = TEST_TOKENS.iter()
        .map(|(mint, _)| mint.to_string())
        .collect();

    match get_multiple_token_authorities(&mints).await {
        Ok(authorities_list) => {
            for (i, authorities) in authorities_list.iter().enumerate() {
                let (_, name) = TEST_TOKENS[i];
                print_compact_token_result(name, &authorities);
            }
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

fn print_token_authorities(authorities: &TokenAuthorities) {
    println!("\n📋 Token Authority Details:");
    println!("{}", "-".repeat(40));
    println!("Mint Address: {}", authorities.mint);
    println!("Token Type: {}", if authorities.is_token_2022 { "Token-2022" } else { "SPL Token" });
    println!();

    println!("🏭 Mint Authority:");
    match &authorities.mint_authority {
        Some(auth) => println!("  ⚠️ ENABLED: {}", auth),
        None => println!("  ✅ DISABLED (Permanent)"),
    }

    println!("\n🧊 Freeze Authority:");
    match &authorities.freeze_authority {
        Some(auth) => println!("  ⚠️ ENABLED: {}", auth),
        None => println!("  ✅ DISABLED (Permanent)"),
    }

    println!("\n📝 Update Authority:");
    match &authorities.update_authority {
        Some(auth) => println!("  ⚠️ ENABLED: {}", auth),
        None => println!("  ✅ DISABLED (Permanent)"),
    }
}

fn print_risk_analysis(authorities: &TokenAuthorities) {
    println!("\n🎯 Risk Analysis:");
    println!("{}", "-".repeat(40));

    let risk_level = authorities.get_risk_level();
    println!("Risk Level: {} {}", risk_level.get_color_code(), risk_level.as_str());

    println!("\nSafety Checks:");
    println!("  Rug Safe: {}", if authorities.is_rug_safe() { "✅ Yes" } else { "❌ No" });
    println!("  Fully Renounced: {}", if authorities.is_fully_renounced() {
        "✅ Yes"
    } else {
        "❌ No"
    });
    println!("  Has Authorities: {}", if authorities.has_any_authority() {
        "⚠️ Yes"
    } else {
        "✅ No"
    });

    println!("\nExplanation:");
    match risk_level {
        TokenRiskLevel::Safe => println!("  🟢 All authorities disabled - maximum safety"),
        TokenRiskLevel::Low => println!("  🟡 Only metadata updates possible - generally safe"),
        TokenRiskLevel::Medium => println!("  🟠 Some authorities active - moderate risk"),
        TokenRiskLevel::High => println!("  🔴 Mint/freeze authorities active - high risk"),
    }
}

fn print_compact_token_info(authorities: &TokenAuthorities) {
    let risk_icon = authorities.get_risk_level().get_color_code();
    let risk_level = authorities.get_risk_level().as_str();
    let mint_short = &authorities.mint[..8];

    println!(
        "{} {} | {} | {}",
        risk_icon,
        risk_level.to_uppercase(),
        mint_short,
        authorities.get_authority_summary()
    );
}

fn print_compact_token_result(name: &str, authorities: &TokenAuthorities) {
    let risk_icon = authorities.get_risk_level().get_color_code();
    let risk_level = authorities.get_risk_level().as_str();
    let safe_status = if authorities.is_rug_safe() { "SAFE" } else { "RISKY" };

    println!("{} {} | {} | {}", risk_icon, risk_level, safe_status, name);
}

fn print_batch_summary(authorities_list: &[TokenAuthorities]) {
    if authorities_list.is_empty() {
        return;
    }

    println!("\n📊 Batch Summary:");
    println!("{}", "=".repeat(40));

    let total = authorities_list.len();
    let safe_count = authorities_list
        .iter()
        .filter(|a| a.get_risk_level() == TokenRiskLevel::Safe)
        .count();
    let low_count = authorities_list
        .iter()
        .filter(|a| a.get_risk_level() == TokenRiskLevel::Low)
        .count();
    let medium_count = authorities_list
        .iter()
        .filter(|a| a.get_risk_level() == TokenRiskLevel::Medium)
        .count();
    let high_count = authorities_list
        .iter()
        .filter(|a| a.get_risk_level() == TokenRiskLevel::High)
        .count();
    let rug_safe_count = authorities_list
        .iter()
        .filter(|a| a.is_rug_safe())
        .count();
    let token_2022_count = authorities_list
        .iter()
        .filter(|a| a.is_token_2022)
        .count();

    println!("Total Tokens: {}", total);
    println!("Risk Distribution:");
    println!("  🟢 Safe: {} ({:.1}%)", safe_count, ((safe_count as f64) / (total as f64)) * 100.0);
    println!("  🟡 Low: {} ({:.1}%)", low_count, ((low_count as f64) / (total as f64)) * 100.0);
    println!(
        "  🟠 Medium: {} ({:.1}%)",
        medium_count,
        ((medium_count as f64) / (total as f64)) * 100.0
    );
    println!("  🔴 High: {} ({:.1}%)", high_count, ((high_count as f64) / (total as f64)) * 100.0);
    println!();
    println!("Safety Status:");
    println!(
        "  Rug Safe: {} ({:.1}%)",
        rug_safe_count,
        ((rug_safe_count as f64) / (total as f64)) * 100.0
    );
    println!(
        "  Token-2022: {} ({:.1}%)",
        token_2022_count,
        ((token_2022_count as f64) / (total as f64)) * 100.0
    );
}
