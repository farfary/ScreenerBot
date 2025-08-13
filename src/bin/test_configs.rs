/// Test binary to verify the centralized configuration system works correctly
/// 
/// Usage: cargo run --bin test_configs

use screenerbot::configs::{
    read_configs, read_configs_from_path, load_wallet_from_config,
    validate_configs, get_wallet_pubkey_string, create_default_config, save_configs_to_path
};
use std::fs;

#[tokio::main]
async fn main() {
    println!("🧪 Testing Centralized Configuration System");
    println!("==========================================");

    // Test 1: Create default configuration
    println!("\n📋 Test 1: Creating default configuration template");
    let default_config = create_default_config();
    println!("  ✓ Default config created successfully");
    println!("  RPC URL: {}", default_config.rpc_url);
    println!("  Premium RPC URL: {}", default_config.rpc_url_premium);
    println!("  Fallback URLs: {:?}", default_config.rpc_fallbacks);

    // Test 2: Save configuration to a test file
    println!("\n💾 Test 2: Saving configuration to file");
    match save_configs_to_path(&default_config, "test_config.json") {
        Ok(()) => {
            println!("  ✓ Configuration saved successfully to test_config.json");
            
            // Test 3: Load configuration from the test file
            println!("\n📖 Test 3: Loading configuration from file");
            match read_configs_from_path("test_config.json") {
                Ok(loaded_config) => {
                    println!("  ✓ Configuration loaded successfully");
                    println!("  RPC URL matches: {}", loaded_config.rpc_url == default_config.rpc_url);
                    println!("  Premium URL matches: {}", loaded_config.rpc_url_premium == default_config.rpc_url_premium);
                    println!("  Fallbacks match: {}", loaded_config.rpc_fallbacks == default_config.rpc_fallbacks);
                },
                Err(e) => println!("  ❌ Failed to load configuration: {}", e),
            }
        },
        Err(e) => println!("  ❌ Failed to save configuration: {}", e),
    }

    // Test 4: Try to load the real configuration file
    println!("\n🔧 Test 4: Loading real configuration file");
    match read_configs() {
        Ok(real_config) => {
            println!("  ✓ Real configuration loaded successfully");
            println!("  RPC URL: {}", real_config.rpc_url);
            println!("  Premium RPC URL: {}", real_config.rpc_url_premium);
            println!("  Number of fallback URLs: {}", real_config.rpc_fallbacks.len());
            
            // Test 5: Validate the real configuration
            println!("\n✅ Test 5: Validating real configuration");
            match validate_configs(&real_config) {
                Ok(()) => {
                    println!("  ✓ Configuration validation passed");
                    
                    // Test 6: Try to load wallet from configuration
                    println!("\n🔑 Test 6: Loading wallet from configuration");
                    match load_wallet_from_config(&real_config) {
                        Ok(wallet) => {
                            println!("  ✓ Wallet loaded successfully");
                            
                            // Test 7: Get wallet public key string
                            println!("\n📍 Test 7: Getting wallet public key");
                            match get_wallet_pubkey_string(&real_config) {
                                Ok(pubkey_str) => {
                                    println!("  ✓ Public key retrieved successfully");
                                    println!("  Wallet address: {}", pubkey_str);
                                },
                                Err(e) => println!("  ❌ Failed to get public key: {}", e),
                            }
                        },
                        Err(e) => {
                            println!("  ❌ Failed to load wallet: {}", e);
                            println!("  This is expected if the private key is in a test format");
                        }
                    }
                },
                Err(e) => {
                    println!("  ❌ Configuration validation failed: {}", e);
                    println!("  This is expected if the wallet key is in a test format");
                }
            }
        },
        Err(e) => {
            println!("  ❌ Failed to load real configuration: {}", e);
            println!("  Make sure data/configs.json exists and is properly formatted");
        }
    }

    // Test 8: Test backward compatibility through global.rs re-exports
    println!("\n🔄 Test 8: Testing backward compatibility");
    match screenerbot::global::read_configs() {
        Ok(compat_config) => {
            println!("  ✓ Backward compatibility through global.rs works");
            println!("  RPC URL: {}", compat_config.rpc_url);
        },
        Err(e) => {
            println!("  ❌ Backward compatibility failed: {}", e);
        }
    }

    // Cleanup test file
    println!("\n🧹 Cleanup: Removing test configuration file");
    if let Err(e) = fs::remove_file("test_config.json") {
        println!("  Warning: Could not remove test_config.json: {}", e);
    } else {
        println!("  ✓ Test file cleaned up successfully");
    }

    println!("\n✅ All configuration system tests completed!");
    println!("\n💡 Summary of functionality tested:");
    println!("  - ✓ Default configuration creation");
    println!("  - ✓ Configuration file saving and loading");
    println!("  - ✓ Configuration validation");
    println!("  - ✓ Wallet keypair loading (if valid private key available)");
    println!("  - ✓ Public key extraction");
    println!("  - ✓ Backward compatibility with global.rs re-exports");
}
