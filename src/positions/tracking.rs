use super::{
    apply::apply_transition, state::update_position_state, transitions::PositionTransition,
};
use crate::{arguments::is_debug_positions_enabled, pools::PriceResult};
use tokio::time::Duration;

/// Update position price tracking
pub async fn update_position_tracking(
    mint: &str,
    current_price: f64,
    _price_result: &PriceResult,
) -> bool {
    if current_price <= 0.0 || !current_price.is_finite() {
        return false;
    }

    // Try to acquire lock with timeout to avoid blocking
    let lock_result = tokio::time::timeout(
        Duration::from_millis(100),
        super::state::acquire_position_lock(mint),
    )
    .await;

    let _lock = match lock_result {
        Ok(lock) => lock,
        Err(_) => {
            return false;
        } // Don't block tracking updates
    };

    let mut needs_update = false;
    let mut new_highest = None;
    let mut new_lowest = None;

    // Check if position exists and needs price updates
    let position_exists = super::state::update_position_state(mint, |pos| {
        let entry_price = pos.effective_entry_price.unwrap_or(pos.entry_price);

        // Initialize if not set
        if pos.price_highest == 0.0 {
            pos.price_highest = entry_price;
            pos.price_lowest = entry_price;
        }

        // Check for new highs/lows
        if current_price > pos.price_highest {
            pos.price_highest = current_price;
            new_highest = Some(current_price);
            needs_update = true;
        }
        if current_price < pos.price_lowest {
            pos.price_lowest = current_price;
            new_lowest = Some(current_price);
            needs_update = true;
        }

        // Always update current price
        pos.current_price = Some(current_price);
        pos.current_price_updated = Some(chrono::Utc::now());
        needs_update = true;
    })
    .await;

    if position_exists && needs_update {
        // Apply price tracking transition (doesn't require DB update)
        let transition = PositionTransition::UpdatePriceTracking {
            mint: mint.to_string(),
            current_price,
            highest: new_highest,
            lowest: new_lowest,
        };

        // Apply transition (this won't hit DB for price tracking)
        let _ = apply_transition(transition).await;

        if is_debug_positions_enabled() && (new_highest.is_some() || new_lowest.is_some()) {
            crate::logger::log(
                crate::logger::LogTag::Positions,
                "DEBUG",
                &format!(
                    "📊 Price update for {}: current={:.8}, high={}, low={}",
                    mint,
                    current_price,
                    new_highest.map_or("unchanged".to_string(), |h| format!("{:.8}", h)),
                    new_lowest.map_or("unchanged".to_string(), |l| format!("{:.8}", l))
                ),
            );
        }

        true
    } else {
        false
    }
}
