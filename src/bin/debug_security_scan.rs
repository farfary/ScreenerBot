// Debug security scan functionality
use screenerbot::logger::{ init_file_logging, log, LogTag };
use screenerbot::tokens::security::{ get_security_analyzer, check_api_status };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    println!("🔍 Testing Security System Debug");

    // 1. Check API status
    println!("\n1. Checking Rugcheck API status...");
    match check_api_status().await {
        Ok(true) => println!("✅ API is operational"),
        Ok(false) => println!("❌ API is not operational"),
        Err(e) => println!("⚠️ API check failed: {}", e),
    }

    // 2. Initialize security analyzer
    println!("\n2. Initializing security analyzer...");
    let analyzer = get_security_analyzer();
    println!("✅ Security analyzer initialized");

    // 3. Check count of tokens without security
    println!("\n3. Checking tokens without security info...");
    match analyzer.database.count_tokens_without_security() {
        Ok(count) => {
            println!("✅ Found {} tokens without security info", count);

            if count > 0 {
                // 4. Get actual list of tokens (first 10)
                println!("\n4. Getting list of tokens without security...");
                match analyzer.database.get_tokens_without_security() {
                    Ok(tokens) => {
                        println!("✅ Retrieved {} tokens without security", tokens.len());
                        if !tokens.is_empty() {
                            println!("First 10 tokens without security:");
                            for (i, token) in tokens.iter().take(10).enumerate() {
                                println!("  {}. {}", i + 1, token);
                            }

                            // 5. Test analyzing one token
                            println!("\n5. Testing security analysis on first token...");
                            let test_mint = &tokens[0];
                            println!("Testing with mint: {}", test_mint);

                            match analyzer.analyze_token_security(test_mint).await {
                                Ok(info) => {
                                    println!("✅ Security analysis successful!");
                                    println!(
                                        "   Mint authority disabled: {}",
                                        info.mint_authority_disabled
                                    );
                                    println!(
                                        "   Freeze authority disabled: {}",
                                        info.freeze_authority_disabled
                                    );
                                    println!("   LP is safe: {}", info.lp_is_safe);
                                    println!("   Holder count: {}", info.holder_count);
                                    println!("   Overall safe: {}", info.is_safe);
                                }
                                Err(e) => {
                                    println!("❌ Security analysis failed: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => println!("❌ Failed to get tokens: {}", e),
                }
            }
        }
        Err(e) => println!("❌ Failed to count tokens: {}", e),
    }

    // 6. Check database table existence
    println!("\n6. Checking database table existence...");

    // Check tokens.db
    let tokens_conn = rusqlite::Connection::open("data/tokens.db")?;
    let token_count: i64 = tokens_conn.query_row("SELECT COUNT(*) FROM tokens", [], |row|
        row.get(0)
    )?;
    println!("✅ tokens.db has {} tokens", token_count);

    // Check security.db
    match rusqlite::Connection::open("data/security.db") {
        Ok(security_conn) => {
            match
                security_conn.query_row("SELECT COUNT(*) FROM security", [], |row|
                    row.get::<_, i64>(0)
                )
            {
                Ok(security_count) =>
                    println!("✅ security.db has {} security records", security_count),
                Err(e) => println!("❌ Failed to query security table: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to open security.db: {}", e),
    }

    println!("\n🏁 Debug complete");
    Ok(())
}
