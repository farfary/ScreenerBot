// discovery_manager.rs - Separate background task for mint discovery from DexScreener and RugCheck
use crate::discovery::*;
use crate::logger::{ log, LogTag };
use crate::utils::check_shutdown_or_delay;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::Duration;

/// Discovery manager handles mint finding from external APIs
pub struct DiscoveryManager {
    discovery_cycle: usize,
}

impl DiscoveryManager {
    const DISCOVERY_CYCLE_MINUTES: u64 = 3; // Run discovery every 3 minutes

    /// Create new discovery manager
    pub fn new() -> Self {
        Self {
            discovery_cycle: 0,
        }
    }

    /// Start discovery background task
    pub async fn start_discovery(&mut self, shutdown: Arc<Notify>) {
        log(LogTag::Monitor, "INFO", "Starting mint discovery background task...");

        loop {
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(100)).await {
                log(LogTag::Monitor, "INFO", "Discovery manager shutting down...");
                break;
            }

            self.discovery_cycle += 1;
            log(
                LogTag::Monitor,
                "INFO",
                &format!("Starting discovery cycle #{}", self.discovery_cycle)
            );

            let cycle_start = chrono::Utc::now();

            // Run all discovery tasks concurrently
            let discovery_result = self.run_discovery_tasks().await;

            let cycle_duration = chrono::Utc::now().signed_duration_since(cycle_start);

            match discovery_result {
                Ok(total_new_mints) => {
                    log(
                        LogTag::Monitor,
                        "SUCCESS",
                        &format!(
                            "Discovery cycle #{} completed: {} new mints found in {:.1}s",
                            self.discovery_cycle,
                            total_new_mints,
                            (cycle_duration.num_milliseconds() as f64) / 1000.0
                        )
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Monitor,
                        "ERROR",
                        &format!(
                            "Discovery cycle #{} failed: {} (duration: {:.1}s)",
                            self.discovery_cycle,
                            e,
                            (cycle_duration.num_milliseconds() as f64) / 1000.0
                        )
                    );
                }
            }

            // Wait for next discovery cycle
            self.wait_for_next_discovery_cycle(shutdown.clone()).await;
        }
    }

    /// Run all discovery tasks concurrently
    async fn run_discovery_tasks(&self) -> Result<usize, String> {
        // Get initial mint count
        let initial_count = match crate::global::LIST_MINTS.read() {
            Ok(mints) => mints.len(),
            Err(_) => {
                return Err("Failed to read LIST_MINTS".to_string());
            }
        };

        // Run all discovery tasks with timeout
        let discovery_tasks = async {
            // Run all discovery tasks concurrently using tokio::join!
            let (
                profiles_result,
                boosts_result,
                boosts_top_result,
                rugcheck_verified_result,
                rugcheck_trending_result,
                rugcheck_recent_result,
                rugcheck_new_tokens_result,
            ) = tokio::join!(
                discovery_dexscreener_fetch_token_profiles(),
                discovery_dexscreener_fetch_token_boosts(),
                discovery_dexscreener_fetch_token_boosts_top(),
                discovery_rugcheck_fetch_verified(),
                discovery_rugcheck_fetch_trending(),
                discovery_rugcheck_fetch_recent(),
                discovery_rugcheck_fetch_new_tokens()
            );

            // Collect results and log any errors
            let mut errors = Vec::new();

            if let Err(e) = profiles_result {
                errors.push(format!("Token profiles: {}", e));
            }
            if let Err(e) = boosts_result {
                errors.push(format!("Token boosts: {}", e));
            }
            if let Err(e) = boosts_top_result {
                errors.push(format!("Token boosts top: {}", e));
            }
            if let Err(e) = rugcheck_verified_result {
                errors.push(format!("RugCheck verified: {}", e));
            }
            if let Err(e) = rugcheck_trending_result {
                errors.push(format!("RugCheck trending: {}", e));
            }
            if let Err(e) = rugcheck_recent_result {
                errors.push(format!("RugCheck recent: {}", e));
            }
            if let Err(e) = rugcheck_new_tokens_result {
                errors.push(format!("RugCheck new tokens: {}", e));
            }

            if !errors.is_empty() {
                log(LogTag::Monitor, "WARN", &format!("Discovery errors: {}", errors.join(", ")));
            }

            Ok::<(), String>(())
        };

        // Run with timeout
        match tokio::time::timeout(Duration::from_secs(60), discovery_tasks).await {
            Ok(_) => {
                // Calculate new mints found
                let final_count = match crate::global::LIST_MINTS.read() {
                    Ok(mints) => mints.len(),
                    Err(_) => {
                        return Err("Failed to read LIST_MINTS after discovery".to_string());
                    }
                };

                let new_mints = final_count.saturating_sub(initial_count);
                Ok(new_mints)
            }
            Err(_) => Err("Discovery tasks timed out".to_string()),
        }
    }

    /// Wait for next discovery cycle
    async fn wait_for_next_discovery_cycle(&self, shutdown: Arc<Notify>) {
        let cycle_duration = Duration::from_secs(Self::DISCOVERY_CYCLE_MINUTES * 60);

        log(
            LogTag::Monitor,
            "INFO",
            &format!(
                "Waiting {} minutes for next discovery cycle...",
                Self::DISCOVERY_CYCLE_MINUTES
            )
        );

        if check_shutdown_or_delay(&shutdown, cycle_duration).await {
            return;
        }
    }
}

/// Start the discovery background task
pub async fn start_discovery_task(shutdown: Arc<Notify>) {
    let mut discovery_manager = DiscoveryManager::new();
    discovery_manager.start_discovery(shutdown).await;
}
