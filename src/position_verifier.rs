/// Position Verification Monitor
/// Background task that continuously monitors unverified positions and automatically verifies them
/// when transaction signatures are present. This ensures all positions are properly verified
/// without manual intervention.

use crate::{
    logger::{log, LogTag},
    positions::{SAVED_POSITIONS, Position},
    transactions_manager::verify_swap_transaction_global,
    transactions_tools::analyze_post_swap_transaction_simple,
    utils::{get_wallet_address, save_positions_to_file},
    global::is_debug_transactions_enabled,
};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};
use chrono::Utc;

/// Position verification monitoring interval (every 60 seconds)
const VERIFICATION_CHECK_INTERVAL_SECONDS: u64 = 60;

/// Maximum number of positions to verify per cycle to avoid overwhelming the system
const MAX_VERIFICATIONS_PER_CYCLE: usize = 5;

/// Position Verification Monitor service
pub struct PositionVerifier {
    wallet_address: String,
    verification_active: bool,
}

impl PositionVerifier {
    /// Create new position verifier
    pub fn new() -> Result<Self, String> {
        let wallet_address = get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address: {}", e))?;

        Ok(Self {
            wallet_address,
            verification_active: false,
        })
    }

    /// Start position verification monitoring service
    pub async fn start_monitoring(&mut self, shutdown: Arc<Notify>) {
        self.verification_active = true;
        log(LogTag::System, "START", "Position verification monitor started");

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::System, "SHUTDOWN", "Position verification monitor stopping");
                    break;
                }
                
                _ = sleep(Duration::from_secs(VERIFICATION_CHECK_INTERVAL_SECONDS)) => {
                    if let Err(e) = self.check_and_verify_positions().await {
                        log(LogTag::System, "ERROR", &format!("Position verification check failed: {}", e));
                    }
                }
            }
        }

        self.verification_active = false;
        log(LogTag::System, "STOP", "Position verification monitor stopped");
    }

    /// Check all unverified positions and attempt to verify them
    async fn check_and_verify_positions(&self) -> Result<(), String> {
        let unverified_positions = self.get_unverified_positions();
        
        if unverified_positions.is_empty() {
            if is_debug_transactions_enabled() {
                log(LogTag::System, "DEBUG", "No unverified positions found");
            }
            return Ok(());
        }

        log(
            LogTag::System, 
            "VERIFICATION", 
            &format!("Found {} unverified positions to check", unverified_positions.len())
        );

        let mut verification_count = 0;
        let mut verified_positions = Vec::new();

        for (index, position) in unverified_positions {
            // Limit verifications per cycle to avoid overwhelming the system
            if verification_count >= MAX_VERIFICATIONS_PER_CYCLE {
                log(
                    LogTag::System,
                    "LIMIT",
                    &format!("Reached verification limit ({}) for this cycle, remaining positions will be checked next cycle", MAX_VERIFICATIONS_PER_CYCLE)
                );
                break;
            }

            // Verify entry transaction if unverified
            if !position.transaction_entry_verified {
                if let Some(ref entry_signature) = position.entry_transaction_signature {
                    log(
                        LogTag::System,
                        "VERIFYING_ENTRY",
                        &format!("Verifying entry transaction for {} ({})", position.symbol, &entry_signature[..8])
                    );

                    match self.verify_position_entry(&position, entry_signature).await {
                        Ok(updated_position) => {
                            log(
                                LogTag::System,
                                "ENTRY_VERIFIED",
                                &format!("✅ Entry verified for {}: {:.12} SOL/token", position.symbol, updated_position.effective_entry_price.unwrap_or(0.0))
                            );
                            verified_positions.push((index, updated_position));
                            verification_count += 1;
                        }
                        Err(e) => {
                            log(
                                LogTag::System,
                                "ENTRY_FAILED",
                                &format!("❌ Entry verification failed for {}: {}", position.symbol, e)
                            );
                        }
                    }
                }
            }

            // Verify exit transaction if unverified
            if !position.transaction_exit_verified {
                if let Some(ref exit_signature) = position.exit_transaction_signature {
                    log(
                        LogTag::System,
                        "VERIFYING_EXIT",
                        &format!("Verifying exit transaction for {} ({})", position.symbol, &exit_signature[..8])
                    );

                    // For exit verification, we need the already updated position if entry was just verified
                    let position_to_verify = verified_positions.iter()
                        .find(|(idx, _)| *idx == index)
                        .map(|(_, pos)| pos)
                        .unwrap_or(&position);

                    match self.verify_position_exit(position_to_verify, exit_signature).await {
                        Ok(updated_position) => {
                            log(
                                LogTag::System,
                                "EXIT_VERIFIED",
                                &format!("✅ Exit verified for {}: {:.12} SOL/token", position.symbol, updated_position.effective_exit_price.unwrap_or(0.0))
                            );
                            // Update or add the position to verified positions
                            if let Some((_, existing)) = verified_positions.iter_mut().find(|(idx, _)| *idx == index) {
                                *existing = updated_position;
                            } else {
                                verified_positions.push((index, updated_position));
                            }
                            verification_count += 1;
                        }
                        Err(e) => {
                            log(
                                LogTag::System,
                                "EXIT_FAILED",
                                &format!("❌ Exit verification failed for {}: {}", position.symbol, e)
                            );
                        }
                    }
                }
            }
        }

        // Update all verified positions in the global positions array
        if !verified_positions.is_empty() {
            self.update_verified_positions(verified_positions).await?;
            log(
                LogTag::System,
                "VERIFICATION_COMPLETE",
                &format!("✅ Successfully verified {} positions this cycle", verification_count)
            );
        }

        Ok(())
    }

    /// Get all unverified positions with their indices
    fn get_unverified_positions(&self) -> Vec<(usize, Position)> {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            positions
                .iter()
                .enumerate()
                .filter(|(_, pos)| {
                    // Position is unverified if:
                    // 1. Entry transaction exists but not verified
                    // 2. Exit transaction exists but not verified
                    (!pos.transaction_entry_verified && pos.entry_transaction_signature.is_some()) ||
                    (!pos.transaction_exit_verified && pos.exit_transaction_signature.is_some())
                })
                .map(|(idx, pos)| (idx, pos.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Verify a position's entry transaction
    async fn verify_position_entry(&self, position: &Position, signature: &str) -> Result<Position, String> {
        match analyze_post_swap_transaction_simple(signature, &self.wallet_address).await {
            Ok(analysis) => {
                let mut updated_position = position.clone();
                
                // Update position with verified entry data
                updated_position.effective_entry_price = Some(analysis.effective_price);
                updated_position.token_amount = Some(analysis.token_amount as u64);
                updated_position.total_size_sol = analysis.sol_amount;
                updated_position.entry_fee_lamports = Some(analysis.transaction_fee.unwrap_or(0));
                updated_position.transaction_entry_verified = true;

                Ok(updated_position)
            }
            Err(e) => Err(format!("Entry transaction analysis failed: {}", e))
        }
    }

    /// Verify a position's exit transaction
    async fn verify_position_exit(&self, position: &Position, signature: &str) -> Result<Position, String> {
        match analyze_post_swap_transaction_simple(signature, &self.wallet_address).await {
            Ok(analysis) => {
                let mut updated_position = position.clone();
                
                // Update position with verified exit data
                updated_position.effective_exit_price = Some(analysis.effective_price);
                updated_position.sol_received = Some(analysis.sol_amount);
                updated_position.exit_fee_lamports = Some(analysis.transaction_fee.unwrap_or(0));
                updated_position.transaction_exit_verified = true;
                
                // Update exit time if not already set
                if updated_position.exit_time.is_none() {
                    updated_position.exit_time = Some(Utc::now());
                }

                Ok(updated_position)
            }
            Err(e) => Err(format!("Exit transaction analysis failed: {}", e))
        }
    }

    /// Update the global positions array with verified positions
    async fn update_verified_positions(&self, verified_positions: Vec<(usize, Position)>) -> Result<(), String> {
        if let Ok(mut positions) = SAVED_POSITIONS.lock() {
            // Update positions at their original indices
            for (index, updated_position) in verified_positions {
                if index < positions.len() {
                    positions[index] = updated_position;
                }
            }

            // Save to file
            save_positions_to_file(&positions);

            log(
                LogTag::System,
                "POSITIONS_UPDATED",
                &format!("Updated and saved {} verified positions to file", positions.len())
            );

            Ok(())
        } else {
            Err("Failed to acquire positions lock".to_string())
        }
    }

    /// Get verification statistics for display
    pub fn get_verification_stats(&self) -> (usize, usize, usize, usize) {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            let total_positions = positions.len();
            let verified_entries = positions.iter().filter(|p| p.transaction_entry_verified).count();
            let verified_exits = positions.iter().filter(|p| p.transaction_exit_verified).count();
            let unverified_count = positions.iter().filter(|p| {
                (!p.transaction_entry_verified && p.entry_transaction_signature.is_some()) ||
                (!p.transaction_exit_verified && p.exit_transaction_signature.is_some())
            }).count();

            (total_positions, verified_entries, verified_exits, unverified_count)
        } else {
            (0, 0, 0, 0)
        }
    }
}

// =============================================================================
// GLOBAL POSITION VERIFIER
// =============================================================================

use std::sync::Mutex;
use once_cell::sync::Lazy;

static GLOBAL_POSITION_VERIFIER: Lazy<Arc<Mutex<Option<PositionVerifier>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize global position verifier
pub async fn init_position_verifier() -> Result<(), String> {
    let verifier = PositionVerifier::new()?;
    let mut global_verifier = GLOBAL_POSITION_VERIFIER.lock().unwrap();
    *global_verifier = Some(verifier);
    
    log(LogTag::System, "INIT", "Position verifier initialized");
    Ok(())
}

/// Start position verification monitoring service
pub async fn start_position_verification(shutdown: Arc<Notify>) -> Result<tokio::task::JoinHandle<()>, String> {
    log(LogTag::System, "START", "Starting position verification service");

    let handle = tokio::spawn(async move {
        // Start verification monitoring loop
        start_position_verification_loop(shutdown).await;
    });

    Ok(handle)
}

/// Position verification monitoring loop
async fn start_position_verification_loop(shutdown: Arc<Notify>) {
    log(LogTag::System, "START", "Position verification monitor started");

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::System, "SHUTDOWN", "Position verification monitor stopping");
                break;
            }
            
            _ = sleep(Duration::from_secs(VERIFICATION_CHECK_INTERVAL_SECONDS)) => {
                // Acquire lock only for the duration of the verification check
                // Don't hold it across the await
                let verification_task = {
                    let verifier_guard = GLOBAL_POSITION_VERIFIER.lock().unwrap();
                    if let Some(ref verifier) = *verifier_guard {
                        // Extract the information we need without holding the lock
                        let wallet_address = verifier.wallet_address.clone();
                        drop(verifier_guard); // Release lock immediately
                        Some(wallet_address)
                    } else {
                        log(LogTag::System, "ERROR", "Position verifier not initialized");
                        None
                    }
                };

                if let Some(wallet_address) = verification_task {
                    // Perform verification outside the lock
                    if let Err(e) = perform_verification_check(&wallet_address).await {
                        log(LogTag::System, "ERROR", &format!("Position verification check failed: {}", e));
                    }
                }
            }
        }
    }

    log(LogTag::System, "STOP", "Position verification monitor stopped");
}

/// Perform verification check without holding any locks
async fn perform_verification_check(wallet_address: &str) -> Result<(), String> {
    let unverified_positions = get_unverified_positions_for_check();
    
    if unverified_positions.is_empty() {
        if is_debug_transactions_enabled() {
            log(LogTag::System, "DEBUG", "No unverified positions found");
        }
        return Ok(());
    }

    log(
        LogTag::System, 
        "VERIFICATION", 
        &format!("Found {} unverified positions to check", unverified_positions.len())
    );

    let mut verification_count = 0;
    let mut verified_positions = Vec::new();

    for (index, position) in unverified_positions {
        // Limit verifications per cycle to avoid overwhelming the system
        if verification_count >= MAX_VERIFICATIONS_PER_CYCLE {
            log(
                LogTag::System,
                "LIMIT",
                &format!("Reached verification limit ({}) for this cycle, remaining positions will be checked next cycle", MAX_VERIFICATIONS_PER_CYCLE)
            );
            break;
        }

        // Verify entry transaction if unverified
        if !position.transaction_entry_verified {
            if let Some(ref entry_signature) = position.entry_transaction_signature {
                log(
                    LogTag::System,
                    "VERIFYING_ENTRY",
                    &format!("Verifying entry transaction for {} ({})", position.symbol, &entry_signature[..8])
                );

                match verify_position_entry_standalone(&position, entry_signature, wallet_address).await {
                    Ok(updated_position) => {
                        log(
                            LogTag::System,
                            "ENTRY_VERIFIED",
                            &format!("✅ Entry verified for {}: {:.12} SOL/token", position.symbol, updated_position.effective_entry_price.unwrap_or(0.0))
                        );
                        verified_positions.push((index, updated_position));
                        verification_count += 1;
                    }
                    Err(e) => {
                        log(
                            LogTag::System,
                            "ENTRY_FAILED",
                            &format!("❌ Entry verification failed for {}: {}", position.symbol, e)
                        );
                    }
                }
            }
        }

        // Verify exit transaction if unverified
        if !position.transaction_exit_verified {
            if let Some(ref exit_signature) = position.exit_transaction_signature {
                log(
                    LogTag::System,
                    "VERIFYING_EXIT",
                    &format!("Verifying exit transaction for {} ({})", position.symbol, &exit_signature[..8])
                );

                // For exit verification, we need the already updated position if entry was just verified
                let position_to_verify = verified_positions.iter()
                    .find(|(idx, _)| *idx == index)
                    .map(|(_, pos)| pos)
                    .unwrap_or(&position);

                match verify_position_exit_standalone(position_to_verify, exit_signature, wallet_address).await {
                    Ok(updated_position) => {
                        log(
                            LogTag::System,
                            "EXIT_VERIFIED",
                            &format!("✅ Exit verified for {}: {:.12} SOL/token", position.symbol, updated_position.effective_exit_price.unwrap_or(0.0))
                        );
                        // Update or add the position to verified positions
                        if let Some((_, existing)) = verified_positions.iter_mut().find(|(idx, _)| *idx == index) {
                            *existing = updated_position;
                        } else {
                            verified_positions.push((index, updated_position));
                        }
                        verification_count += 1;
                    }
                    Err(e) => {
                        log(
                            LogTag::System,
                            "EXIT_FAILED",
                            &format!("❌ Exit verification failed for {}: {}", position.symbol, e)
                        );
                    }
                }
            }
        }
    }

    // Update all verified positions in the global positions array
    if !verified_positions.is_empty() {
        update_verified_positions_standalone(verified_positions).await?;
        log(
            LogTag::System,
            "VERIFICATION_COMPLETE",
            &format!("✅ Successfully verified {} positions this cycle", verification_count)
        );
    }

    Ok(())
}

/// Get all unverified positions with their indices (standalone function)
fn get_unverified_positions_for_check() -> Vec<(usize, Position)> {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .enumerate()
            .filter(|(_, pos)| {
                // Position is unverified if:
                // 1. Entry transaction exists but not verified
                // 2. Exit transaction exists but not verified
                (!pos.transaction_entry_verified && pos.entry_transaction_signature.is_some()) ||
                (!pos.transaction_exit_verified && pos.exit_transaction_signature.is_some())
            })
            .map(|(idx, pos)| (idx, pos.clone()))
            .collect()
    } else {
        Vec::new()
    }
}

/// Verify a position's entry transaction (standalone function)
async fn verify_position_entry_standalone(position: &Position, signature: &str, wallet_address: &str) -> Result<Position, String> {
    match analyze_post_swap_transaction_simple(signature, wallet_address).await {
        Ok(analysis) => {
            let mut updated_position = position.clone();
            
            // Update position with verified entry data
            updated_position.effective_entry_price = Some(analysis.effective_price);
            updated_position.token_amount = Some(analysis.token_amount as u64);
            updated_position.total_size_sol = analysis.sol_amount;
            updated_position.entry_fee_lamports = Some(analysis.transaction_fee.unwrap_or(0));
            updated_position.transaction_entry_verified = true;

            Ok(updated_position)
        }
        Err(e) => Err(format!("Entry transaction analysis failed: {}", e))
    }
}

/// Verify a position's exit transaction (standalone function)
async fn verify_position_exit_standalone(position: &Position, signature: &str, wallet_address: &str) -> Result<Position, String> {
    match analyze_post_swap_transaction_simple(signature, wallet_address).await {
        Ok(analysis) => {
            let mut updated_position = position.clone();
            
            // Update position with verified exit data
            updated_position.effective_exit_price = Some(analysis.effective_price);
            updated_position.sol_received = Some(analysis.sol_amount);
            updated_position.exit_fee_lamports = Some(analysis.transaction_fee.unwrap_or(0));
            updated_position.transaction_exit_verified = true;
            
            // Update exit time if not already set
            if updated_position.exit_time.is_none() {
                updated_position.exit_time = Some(Utc::now());
            }

            Ok(updated_position)
        }
        Err(e) => Err(format!("Exit transaction analysis failed: {}", e))
    }
}

/// Update the global positions array with verified positions (standalone function)
async fn update_verified_positions_standalone(verified_positions: Vec<(usize, Position)>) -> Result<(), String> {
    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
        // Update positions at their original indices
        for (index, updated_position) in verified_positions {
            if index < positions.len() {
                positions[index] = updated_position;
            }
        }

        // Save to file
        save_positions_to_file(&positions);

        log(
            LogTag::System,
            "POSITIONS_UPDATED",
            &format!("Updated and saved {} verified positions to file", positions.len())
        );

        Ok(())
    } else {
        Err("Failed to acquire positions lock".to_string())
    }
}

/// Get position verification statistics for display
pub fn get_position_verification_stats() -> (usize, usize, usize, usize) {
    let verifier_guard = GLOBAL_POSITION_VERIFIER.lock().unwrap();
    if let Some(ref verifier) = *verifier_guard {
        verifier.get_verification_stats()
    } else {
        (0, 0, 0, 0)
    }
}

/// Manually trigger position verification (for testing/debugging)
pub async fn trigger_manual_verification() -> Result<String, String> {
    // Get wallet address first
    let wallet_address = {
        let verifier_guard = GLOBAL_POSITION_VERIFIER.lock().unwrap();
        if let Some(ref verifier) = *verifier_guard {
            verifier.wallet_address.clone()
        } else {
            return Err("Position verifier not initialized".to_string());
        }
    };
    
    // Perform verification without holding the lock
    match perform_verification_check(&wallet_address).await {
        Ok(()) => {
            let (total, entry_verified, exit_verified, unverified) = get_position_verification_stats();
            Ok(format!(
                "Manual verification complete. Stats: {} total, {} entry verified, {} exit verified, {} unverified",
                total, entry_verified, exit_verified, unverified
            ))
        }
        Err(e) => Err(format!("Manual verification failed: {}", e))
    }
}
