#![allow(warnings)]
use crate::prelude::*;

pub const TRADE_SIZE_SOL: f64 = 0.002;
pub const MAX_OPEN_POSITIONS: usize = 5;
pub const MAX_DCA_COUNT: u8 = 2;
pub const TRANSACTION_FEE_SOL: f64 = 0.00003;
pub const POSITIONS_CHECK_TIME: u64 = 15;
pub const POSITIONS_PRINT_TIME: u64 = 15;
pub const PRICE_HISTORY_CAP: usize = 60;
pub const SLIPPAGE_BPS: f64 = 0.5;
pub const FEE_RATE: f64 = 0.00002;
pub const DCA_SIZE_FACTOR: f64 = 0.25;


/// Determine if we should buy based on market data and token information
pub fn should_buy(dataframe: &MarketDataFrame, token: &Token, can_buy: bool) -> bool {
    if !can_buy {
        return false;
    }

    // Simple buy logic: Look for any meaningful dip with basic token health checks
    let hist = &dataframe.prices;
    
    if hist.len() < 5 {
        return false;
    }

    let current_price = *hist.back().unwrap();
    let prev_price = *hist.get(hist.len() - 2).unwrap();
    
    // Look for recent high in last 10 candles
    let lookback = 10.min(hist.len());
    let recent_high = hist.iter().rev().take(lookback).cloned().fold(0.0, f64::max);
    
    // Calculate drops
    let current_drop = ((current_price - prev_price) / prev_price) * 100.0;
    let drop_from_high = ((current_price - recent_high) / recent_high) * 100.0;
    
    // Basic token health filters (more lenient)
    let volume_24h = token.volume.h24;
    let price_change_24h = token.price_change.h24;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let buys_24h = token.txns.h24.buys;
    
    let has_min_liquidity = liquidity_sol >= 1.0; // At least 1 SOL
    let has_some_activity = buys_24h >= 1; // At least 1 buy in 24h
    let not_completely_rugged = price_change_24h > -90.0; // Not completely dead
    let has_some_volume = volume_24h >= 100.0; // At least $100 volume
    
    // Simple entry conditions - buy on any of these:
    let small_dip = current_drop <= -2.0; // 2% drop in current candle
    let bigger_dip = drop_from_high <= -5.0; // 5% drop from recent high
    let oversold = drop_from_high <= -10.0; // 10% drop from recent high (more aggressive)
    
    let basic_health_ok = has_min_liquidity && has_some_activity && not_completely_rugged && has_some_volume;
    
    if basic_health_ok && (small_dip || bigger_dip || oversold) {
        println!(
            "🟢 SIMPLE BUY SIGNAL {}: current_drop={:.2}%, drop_from_high={:.2}% | Vol24h=${:.0} | Liq={:.1}SOL | Buys24h={}",
            token.symbol,
            current_drop,
            drop_from_high,
            volume_24h,
            liquidity_sol,
            buys_24h
        );
        return true;
    }
    
    // Optional: Less frequent debug logging
    if hist.len() % 10 == 0 { // Only log every 10th check to reduce spam
        println!("🔍 {} - curr_drop={:.2}%, high_drop={:.2}%, vol=${:.0}, liq={:.1}SOL, buys={}", 
            token.symbol, current_drop, drop_from_high, volume_24h, liquidity_sol, buys_24h);
    }
    
    false
}

/// Determine if we should DCA based on position state and market data
pub fn should_dca(
    dataframe: &MarketDataFrame,
    token: &Token,
    pos: &Position,
    current_price: f64
) -> bool {
    const DCA_BASE_TRIGGER_PCT: f64 = -12.0; // first DCA at -12%
    const DCA_STEP_TRIGGER_PCT: f64 = -5.0; // every extra DCA needs another -5%
    const DCA_MIN_COOLDOWN_MIN: i64 = 3; // min minutes between DCAs

    if pos.dca_count >= MAX_DCA_COUNT {
        return false;
    }

    let now = Utc::now();
    let elapsed = now - pos.open_time;
    let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;

    // override 3rd DCA (pos.dca_count == 2) to require ≥70% drop
    let next_trig = if pos.dca_count == 2 {
        -80.0
    } else {
        DCA_BASE_TRIGGER_PCT + (pos.dca_count as f64) * DCA_STEP_TRIGGER_PCT
    };

    let cooldown_ok = elapsed.num_minutes() >= DCA_MIN_COOLDOWN_MIN * ((pos.dca_count as i64) + 1);

    // Additional market data filtering
    let volume_declining = if dataframe.volumes.len() >= 2 {
        let latest_vol = dataframe.volumes.back().unwrap_or(&0.0);
        let prev_vol = dataframe.volumes.get(dataframe.volumes.len() - 2).unwrap_or(&0.0);
        latest_vol < prev_vol // Don't DCA if volume is increasing (might be pump)
    } else {
        true // Default to allowing DCA if insufficient volume data
    };

    // Check recent price action - avoid DCA during rapid recovery
    let price_stabilizing = if dataframe.prices.len() >= 3 {
        let prices = &dataframe.prices;
        let latest = prices.back().unwrap_or(&current_price);
        let prev = prices.get(prices.len() - 2).unwrap_or(&current_price);
        let two_back = prices.get(prices.len() - 3).unwrap_or(&current_price);

        // Avoid DCA if price is rapidly recovering (>5% up in last 2 candles)
        let recent_recovery = ((latest - two_back) / two_back) * 100.0;
        recent_recovery < 5.0
    } else {
        true
    };

    current_price < pos.last_dca_price &&
        drop_pct <= next_trig &&
        cooldown_ok &&
        volume_declining &&
        price_stabilizing
}

/// Determine if we should sell and return the reason
pub fn should_sell(
    dataframe: &MarketDataFrame,
    token: &Token,
    pos: &Position,
    current_price: f64
) -> (bool, String) {
    let profit_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
    let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;

    // Stop-loss check
    const STOP_LOSS_PCT: f64 = -50.0;
    const EARLY_RUG_SL: f64 = -20.0; // -20% early SL
    let held_secs = (Utc::now() - pos.open_time).num_seconds();
    let is_early = held_secs < 900; // 15 min

    let stop_loss = if is_early { EARLY_RUG_SL } else { STOP_LOSS_PCT };

    if profit_pct <= stop_loss {
        return (true, "stop_loss".to_string());
    }

    // Enhanced stop-loss based on token metadata
    // If 24h price change is extremely negative, trigger early exit
    let severe_dump_threshold = -60.0;
    if token.price_change.h24 <= severe_dump_threshold {
        return (true, format!("severe_dump_24h({:.1}%)", token.price_change.h24));
    }

    // If liquidity is draining rapidly, exit
    let liquidity_total = token.liquidity.base + token.liquidity.quote;
    if liquidity_total < 2.0 {
        // Less than 2 SOL liquidity
        return (true, "low_liquidity".to_string());
    }

    // If volume has died (very low recent activity), consider exit
    let volume_dying = token.volume.h1 < 100.0 && token.txns.h1.buys < 2;
    if volume_dying && profit_pct < 10.0 {
        // Only if not in significant profit
        return (true, "volume_died".to_string());
    }

    // Trailing stop check
    const TP_START_PCT: f64 = 4.0; // enable at +4%
    const TP_MIN_DROP: f64 = 0.0; // lock breakeven at +4%
    const TP_MID_PCT: f64 = 25.0; // reach -1% trail by +25%
    const TP_MID_DROP: f64 = -1.0;
    const TP_MAX_PCT: f64 = 1000.0; // theoretic upper bound
    const TP_MAX_DROP: f64 = -40.0; // never wider than -40%

    let trail = if profit_pct < TP_START_PCT {
        f64::NEG_INFINITY // trailing not active yet
    } else if profit_pct <= TP_MID_PCT {
        // linear easing 0 → -1%
        let t = (profit_pct - TP_START_PCT) / (TP_MID_PCT - TP_START_PCT);
        TP_MIN_DROP + t * (TP_MID_DROP - TP_MIN_DROP)
    } else {
        // quadratic easing -1% → -40%
        let norm = ((profit_pct - TP_MID_PCT) / (TP_MAX_PCT - TP_MID_PCT)).min(1.0);
        let widen = norm * norm; // slow at first, faster later
        let drop = TP_MID_DROP + widen * (TP_MAX_DROP - TP_MID_DROP);
        drop.max(TP_MAX_DROP) // clamp
    };

    if trail > f64::NEG_INFINITY && drop_from_peak <= trail {
        return (true, format!("trailing_stop(trail:{:.2}%, drop:{:.2}%)", trail, drop_from_peak));
    }

    (false, "".to_string())
}
