use std::fs::{ File, OpenOptions };
use std::io::{ Read, Write };
use std::sync::{ Arc, Mutex };
use std::time::Duration;
use serde::{ Deserialize, Serialize };
use crate::global::{ is_shutdown, update_config };
use crate::logger::{ log, LogLevel };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub main_wallet_private: String,
    pub rpc_url: String,
    pub rpc_fallbacks: Vec<String>,
    pub task_delays: TaskDelayConfig,
    pub trading: TradingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDelayConfig {
    pub monitor_delay: u64,
    pub wallet_delay: u64,
    pub trader_delay: u64,
    pub pools_delay: u64,
    pub logger_delay: u64,
    pub rpc_delay: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    pub enabled: bool,
    pub max_position_size: f64,
    pub stop_loss_percent: f64,
    pub take_profit_percent: f64,
    pub min_confidence: f64,
}

impl Default for TaskDelayConfig {
    fn default() -> Self {
        Self {
            monitor_delay: 5,
            wallet_delay: 10,
            trader_delay: 3,
            pools_delay: 15,
            logger_delay: 1,
            rpc_delay: 1,
        }
    }
}

impl Default for TradingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_position_size: 0.1,
            stop_loss_percent: 0.05,
            take_profit_percent: 0.15,
            min_confidence: 0.8,
        }
    }
}

pub type SharedConfig = Arc<Mutex<Config>>;

impl Config {
    pub fn load_from_file(path: &str) -> anyhow::Result<SharedConfig> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // Try to parse with new structure, fallback to old structure
        let config: Config = match serde_json::from_str(&contents) {
            Ok(config) => config,
            Err(_) => {
                // Try old structure and migrate
                let old_config: OldConfig = serde_json::from_str(&contents)?;
                Config {
                    main_wallet_private: old_config.main_wallet_private,
                    rpc_url: old_config.rpc_url,
                    rpc_fallbacks: old_config.rpc_fallbacks,
                    task_delays: TaskDelayConfig::default(),
                    trading: TradingConfig::default(),
                }
            }
        };

        Ok(Arc::new(Mutex::new(config)))
    }

    pub fn save_to_file(shared: &SharedConfig, path: &str) -> anyhow::Result<()> {
        let config = shared.lock().unwrap();
        let json = serde_json::to_string_pretty(&*config)?;
        let mut file = OpenOptions::new().write(true).truncate(true).create(true).open(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}

// Old config structure for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldConfig {
    pub main_wallet_private: String,
    pub rpc_url: String,
    pub rpc_fallbacks: Vec<String>,
}

// Global config manager
use once_cell::sync::Lazy;
pub static CONFIG_MANAGER: Lazy<Arc<Mutex<Option<SharedConfig>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

pub async fn initialize_config_manager() -> anyhow::Result<()> {
    let config = Config::load_from_file("configs.json")?;
    update_config(config.clone());

    let mut global_manager = CONFIG_MANAGER.lock().unwrap();
    *global_manager = Some(config);

    Ok(())
}

pub fn start_config_manager() {
    tokio::task::spawn(async move {
        if let Err(e) = initialize_config_manager().await {
            log("CONFIG", LogLevel::Error, &format!("Failed to initialize config manager: {}", e));
            return;
        }

        log("CONFIG", LogLevel::Info, "Config Manager initialized successfully");

        let delays = crate::global::get_task_delays();

        loop {
            if is_shutdown() {
                log("CONFIG", LogLevel::Info, "Config Manager shutting down...");

                // Save config on shutdown
                if let Ok(config_guard) = CONFIG_MANAGER.lock() {
                    if let Some(config) = config_guard.as_ref() {
                        if let Err(e) = Config::save_to_file(config, "configs.json") {
                            log(
                                "CONFIG",
                                LogLevel::Error,
                                &format!("Failed to save config on shutdown: {}", e)
                            );
                        } else {
                            log("CONFIG", LogLevel::Info, "Config saved successfully on shutdown");
                        }
                    }
                }
                break;
            }

            // Periodic config validation and auto-save
            if let Ok(config_guard) = CONFIG_MANAGER.lock() {
                if let Some(config) = config_guard.as_ref() {
                    // Auto-save config every 5 minutes
                    if let Err(e) = Config::save_to_file(config, "configs.json") {
                        log(
                            "CONFIG",
                            LogLevel::Warn,
                            &format!("Failed to auto-save config: {}", e)
                        );
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(delays.logger_delay * 300)).await; // 5 minutes
        }
    });
}
