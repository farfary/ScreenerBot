use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use chrono::{ DateTime, Utc };
use solana_sdk::signature::Keypair;
use std::env;

pub static CMD_ARGS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| { Mutex::new(env::args().collect()) });

// Startup timestamp to track when the bot started for trading logic
pub static STARTUP_TIME: Lazy<DateTime<Utc>> = Lazy::new(|| Utc::now());

/// Set command arguments (used for tools and testing)
pub fn set_cmd_args(args: Vec<String>) {
    if let Ok(mut cmd_args) = CMD_ARGS.lock() {
        *cmd_args = args;
    }
}

/// Check if debug filtering mode is enabled via command line args
pub fn is_debug_filtering_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-filtering".to_string())
    } else {
        false
    }
}

/// Check if debug profit mode is enabled via command line args
pub fn is_debug_profit_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-profit".to_string())
    } else {
        false
    }
}

/// Check if debug pool prices mode is enabled via command line args
pub fn is_debug_pool_prices_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-pool-prices".to_string())
    } else {
        false
    }
}

/// Check if debug trader mode is enabled via command line args
pub fn is_debug_trader_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-trader".to_string())
    } else {
        false
    }
}

/// Check if debug API mode is enabled via command line args
pub fn is_debug_api_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() { args.contains(&"--debug-api".to_string()) } else { false }
}

/// Check if debug monitor mode is enabled via command line args
pub fn is_debug_monitor_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-monitor".to_string())
    } else {
        false
    }
}

/// Check if debug discovery mode is enabled via command line args
pub fn is_debug_discovery_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-discovery".to_string())
    } else {
        false
    }
}

/// Check if debug price service mode is enabled via command line args
pub fn is_debug_price_service_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-price-service".to_string())
    } else {
        false
    }
}

/// Check if debug rugcheck mode is enabled via command line args
pub fn is_debug_rugcheck_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-rugcheck".to_string())
    } else {
        false
    }
}

/// Check if debug entry mode is enabled via command line args
pub fn is_debug_entry_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() { args.contains(&"--debug-entry".to_string()) } else { false }
}

/// Check if debug RL learning mode is enabled via command line args
pub fn is_debug_rl_learn_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-rl-learn".to_string())
    } else {
        false
    }
}

/// Check if debug OHLCV mode is enabled via command line args
pub fn is_debug_ohlcv_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() { args.contains(&"--debug-ohlcv".to_string()) } else { false }
}

/// Check if debug wallet mode is enabled via command line args
pub fn is_debug_wallet_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-wallet".to_string())
    } else {
        false
    }
}

/// Check if debug swap mode is enabled via command line args
pub fn is_debug_swap_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() { args.contains(&"--debug-swap".to_string()) } else { false }
}

/// Check if debug decimals mode is enabled via command line args
pub fn is_debug_decimals_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-decimals".to_string())
    } else {
        false
    }
}

/// Represents the runtime configuration loaded from configs.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    pub main_wallet_private: String,
    pub rpc_url: String,
    pub rpc_url_premium: String,
    pub rpc_fallbacks: Vec<String>,
}

/// Reads the configs.json file from the project root and returns a Configs object
pub fn read_configs<P: AsRef<Path>>(path: P) -> Result<Configs, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let configs: Configs = serde_json::from_str(&data)?;
    Ok(configs)
}

/// Load the main wallet keypair from the configs
pub fn load_wallet_from_config(configs: &Configs) -> Result<Keypair, Box<dyn std::error::Error>> {
    // Parse the private key from base58 string
    let keypair = if
        configs.main_wallet_private.starts_with('[') &&
        configs.main_wallet_private.ends_with(']')
    {
        // Handle array format like [1,2,3,4,...]
        let private_key_str = configs.main_wallet_private
            .trim_start_matches('[')
            .trim_end_matches(']');
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
                    format!("Failed to create keypair: {}", e)
                )?
            }
            Err(e) => {
                return Err(format!("Failed to parse private key array: {}", e).into());
            }
        }
    } else {
        // Handle base58 format
        let decoded = bs58::decode(&configs.main_wallet_private).into_vec()?;
        if decoded.len() != 64 {
            return Err(
                format!(
                    "Invalid private key length: expected 64 bytes, got {}",
                    decoded.len()
                ).into()
            );
        }
        Keypair::try_from(&decoded[..]).map_err(|e| format!("Failed to create keypair: {}", e))?
    };

    Ok(keypair)
}
