/// Centralized configuration management for ScreenerBot
///
/// This module handles all configuration file operations that were previously
/// scattered across global.rs. It provides a clean interface for loading,
/// parsing, and managing configuration data.
///
/// Features:
/// - Configuration file parsing from configs.json
/// - Wallet keypair loading with multiple format support
/// - Path-based configuration loading for backwards compatibility
/// - Comprehensive error handling and validation
use serde::{ Deserialize, Serialize };
use solana_sdk::signature::{ Keypair, Signer };
use std::fs;
use std::path::Path;

// Import the CONFIG_FILE constant from global.rs for the default path
use crate::global::CONFIG_FILE;

/// Represents the runtime configuration loaded from configs.json
///
/// This struct contains all the essential configuration parameters needed
/// for ScreenerBot operation, including wallet credentials and RPC endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    /// Private key for the main wallet (supports both base58 and array formats)
    pub main_wallet_private: String,
    /// Primary RPC URL for Solana blockchain operations
    pub rpc_url: String,
    /// Premium RPC URL for enhanced performance operations
    pub rpc_url_premium: String,
    /// Premium WebSocket URL for real-time transaction monitoring
    pub rpc_url_ws_premium: String,
    /// List of fallback RPC URLs for redundancy
    pub rpc_fallbacks: Vec<String>,
    /// Solscan API token for external transaction analysis (optional)
    pub solscan_api_token: Option<String>,
}

/// Reads the configs.json file from the default data directory and returns a Configs object
///
/// This is the primary function for loading configuration in normal operation.
/// Uses the CONFIG_FILE constant from global.rs for the file path.
///
/// # Returns
/// - `Ok(Configs)` - Successfully loaded and parsed configuration
/// - `Err(Box<dyn std::error::Error>)` - File read error or JSON parsing error
///
/// # Examples
/// ```rust
/// use screenerbot::configs::read_configs;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let configs = read_configs()?;
///     println!("Loaded RPC URL: {}", configs.rpc_url);
///     Ok(())
/// }
/// ```
pub fn read_configs() -> Result<Configs, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(CONFIG_FILE)?;
    let configs: Configs = serde_json::from_str(&data)?;
    Ok(configs)
}

/// Backward compatibility function - reads configs from a specified path
///
/// This function allows loading configuration from custom paths, which is
/// useful for testing, different environments, or legacy code compatibility.
///
/// # Arguments
/// * `path` - Path to the configuration file (can be &str, String, or PathBuf)
///
/// # Returns
/// - `Ok(Configs)` - Successfully loaded and parsed configuration
/// - `Err(Box<dyn std::error::Error>)` - File read error or JSON parsing error
///
/// # Examples
/// ```rust
/// use screenerbot::configs::read_configs_from_path;
///
/// let configs = read_configs_from_path("custom/path/configs.json")?;
/// ```
pub fn read_configs_from_path<P: AsRef<Path>>(
    path: P
) -> Result<Configs, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let configs: Configs = serde_json::from_str(&data)?;
    Ok(configs)
}

/// Load the main wallet keypair from the configuration
///
/// This function supports multiple private key formats:
/// - Base58 encoded string (standard Solana format)
/// - Array format like [1,2,3,4,...] (byte array representation)
///
/// The function performs comprehensive validation to ensure the private key
/// is exactly 64 bytes and can be successfully converted to a Keypair.
///
/// # Arguments
/// * `configs` - Reference to the Configs struct containing the private key
///
/// # Returns
/// - `Ok(Keypair)` - Successfully created Solana keypair
/// - `Err(Box<dyn std::error::Error>)` - Invalid format, wrong length, or parsing error
///
/// # Examples
/// ```rust
/// use screenerbot::configs::{read_configs, load_wallet_from_config};
///
/// let configs = read_configs()?;
/// let wallet = load_wallet_from_config(&configs)?;
/// println!("Wallet public key: {}", wallet.pubkey());
/// ```
pub fn load_wallet_from_config(configs: &Configs) -> Result<Keypair, Box<dyn std::error::Error>> {
    // Parse the private key from base58 string or array format
    let keypair = if
        configs.main_wallet_private.starts_with('[') &&
        configs.main_wallet_private.ends_with(']')
    {
        // Handle array format like [1,2,3,4,...]
        load_keypair_from_array_format(&configs.main_wallet_private)?
    } else {
        // Handle base58 format
        load_keypair_from_base58_format(&configs.main_wallet_private)?
    };

    Ok(keypair)
}

/// Helper function to load keypair from array format
///
/// Parses private key strings in the format "[1,2,3,4,...]" where each
/// number represents a byte value from 0-255.
///
/// # Arguments
/// * `private_key_str` - String in array format
///
/// # Returns
/// - `Ok(Keypair)` - Successfully parsed keypair
/// - `Err(Box<dyn std::error::Error>)` - Parsing or validation error
fn load_keypair_from_array_format(
    private_key_str: &str
) -> Result<Keypair, Box<dyn std::error::Error>> {
    let private_key_str = private_key_str.trim_start_matches('[').trim_end_matches(']');

    let private_key_bytes: Result<Vec<u8>, _> = private_key_str
        .split(',')
        .map(|s| s.trim().parse::<u8>())
        .collect();

    match private_key_bytes {
        Ok(bytes) => {
            if bytes.len() != 64 {
                return Err(
                    format!(
                        "Invalid private key length: expected 64 bytes, got {}",
                        bytes.len()
                    ).into()
                );
            }
            Keypair::try_from(&bytes[..]).map_err(|e|
                format!("Failed to create keypair from array: {}", e).into()
            )
        }
        Err(e) => Err(format!("Failed to parse private key array: {}", e).into()),
    }
}

/// Helper function to load keypair from base58 format
///
/// Parses private key strings in base58 format, which is the standard
/// Solana wallet format used by most tools and libraries.
///
/// # Arguments
/// * `private_key_str` - Base58 encoded private key string
///
/// # Returns
/// - `Ok(Keypair)` - Successfully parsed keypair
/// - `Err(Box<dyn std::error::Error>)` - Decoding or validation error
fn load_keypair_from_base58_format(
    private_key_str: &str
) -> Result<Keypair, Box<dyn std::error::Error>> {
    let decoded = bs58::decode(private_key_str).into_vec()?;

    if decoded.len() != 64 {
        return Err(
            format!("Invalid private key length: expected 64 bytes, got {}", decoded.len()).into()
        );
    }

    Keypair::try_from(&decoded[..]).map_err(|e|
        format!("Failed to create keypair from base58: {}", e).into()
    )
}

/// Validates that a Configs struct contains all required fields
///
/// This function performs comprehensive validation of a configuration
/// object to ensure all required fields are present and non-empty.
///
/// # Arguments
/// * `configs` - Reference to the Configs struct to validate
///
/// # Returns
/// - `Ok(())` - Configuration is valid
/// - `Err(Box<dyn std::error::Error>)` - Missing or invalid configuration
pub fn validate_configs(configs: &Configs) -> Result<(), Box<dyn std::error::Error>> {
    if configs.main_wallet_private.is_empty() {
        return Err("Main wallet private key is empty".into());
    }

    if configs.rpc_url.is_empty() {
        return Err("RPC URL is empty".into());
    }

    if configs.rpc_url_premium.is_empty() {
        return Err("Premium RPC URL is empty".into());
    }

    // Validate that we can actually load the wallet
    load_wallet_from_config(configs)?;

    Ok(())
}

/// Gets the public key string from a configuration without loading the full keypair
///
/// This is useful for logging or display purposes where you need to show
/// the wallet address but don't need the private key functionality.
///
/// # Arguments
/// * `configs` - Reference to the Configs struct
///
/// # Returns
/// - `Ok(String)` - Base58 encoded public key
/// - `Err(Box<dyn std::error::Error>)` - Failed to load or parse keypair
pub fn get_wallet_pubkey_string(configs: &Configs) -> Result<String, Box<dyn std::error::Error>> {
    let keypair = load_wallet_from_config(configs)?;
    Ok(keypair.pubkey().to_string())
}

/// Creates a default configuration template
///
/// This function creates a Configs struct with placeholder values,
/// useful for generating configuration file templates or testing.
///
/// # Returns
/// A Configs struct with default/placeholder values
pub fn create_default_config() -> Configs {
    Configs {
        main_wallet_private: "your_base58_private_key_here".to_string(),
        rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
        rpc_url_premium: "https://your-premium-rpc-url.com".to_string(),
        rpc_url_ws_premium: "wss://your-premium-ws-url.com".to_string(),
        rpc_fallbacks: vec![
            "https://fallback1.com".to_string(),
            "https://fallback2.com".to_string()
        ],
        solscan_api_token: None,
    }
}

/// Saves a configuration to a file
///
/// This function serializes a Configs struct to JSON and writes it to
/// the specified file path. Useful for generating configuration files
/// or saving modified configurations.
///
/// # Arguments
/// * `configs` - Reference to the Configs struct to save
/// * `path` - Path where to save the configuration file
///
/// # Returns
/// - `Ok(())` - Successfully saved configuration
/// - `Err(Box<dyn std::error::Error>)` - File write or serialization error
pub fn save_configs_to_path<P: AsRef<Path>>(
    configs: &Configs,
    path: P
) -> Result<(), Box<dyn std::error::Error>> {
    let json_data = serde_json::to_string_pretty(configs)?;
    fs::write(path, json_data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_create_default_config() {
        let config = create_default_config();
        assert!(!config.main_wallet_private.is_empty());
        assert!(!config.rpc_url.is_empty());
        assert!(!config.rpc_url_premium.is_empty());
        assert!(!config.rpc_fallbacks.is_empty());
    }

    #[test]
    fn test_validate_configs() {
        let mut config = create_default_config();

        // Should fail validation with placeholder private key
        assert!(validate_configs(&config).is_err());

        // Test empty fields validation
        config.main_wallet_private = "".to_string();
        assert!(validate_configs(&config).is_err());

        config.main_wallet_private = "placeholder".to_string();
        config.rpc_url = "".to_string();
        assert!(validate_configs(&config).is_err());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        let original_config = create_default_config();

        // Save config
        save_configs_to_path(&original_config, &config_path).unwrap();

        // Load config
        let loaded_config = read_configs_from_path(&config_path).unwrap();

        // Compare
        assert_eq!(original_config.rpc_url, loaded_config.rpc_url);
        assert_eq!(original_config.rpc_url_premium, loaded_config.rpc_url_premium);
        assert_eq!(original_config.rpc_fallbacks, loaded_config.rpc_fallbacks);
    }

    #[test]
    fn test_array_format_parsing() {
        // This would need a valid test keypair to work properly
        let test_config = Configs {
            main_wallet_private: "[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64]".to_string(),
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            rpc_url_premium: "https://premium.com".to_string(),
            rpc_url_ws_premium: "wss://premium-ws.com".to_string(),
            rpc_fallbacks: vec![],
            solscan_api_token: None,
        };

        // This should parse without panicking, though the actual keypair creation might fail
        // with an invalid test key. In a real test, you'd use a valid keypair.
        let result = load_wallet_from_config(&test_config);
        // We expect this to fail with the test data, but it should fail gracefully
        assert!(result.is_err());
    }
}
