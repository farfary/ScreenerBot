#![allow(warnings)]
use crate::prelude::*;

/// Determine if we should buy based on market data and token information
pub fn should_buy(dataframe: &MarketDataFrame, token: &Token, can_buy: bool) -> bool {
    if !can_buy {
        return false;
    }

    // SMART ENTRY BUY: Only on Extreme/Abnormal Drops (No Comeback Buys)
    const DROP_LOOKBACK: usize = 32; // Candles to look back
    const MIN_TOTAL_DROP: f64 = 15.0; // Require total drop at least -15%
    const MIN_DROP_SCALE: f64 = 2.5; // Last drop must be >2.5x avg drop
    const MIN_DROP_ABS: f64 = 8.0; // Or last drop at least -8%
    const MIN_BOUNCE: f64 = 0.5; // Optional: Require at least +0.5% bounce (set 0 for instant)

    let hist = &dataframe.prices;

    if hist.len() <= DROP_LOOKBACK + 2 {
        return false;
    }

    let window: Vec<f64> = hist.iter().rev().take(DROP_LOOKBACK).cloned().collect();
    let high = window.iter().cloned().fold(f64::MIN, f64::max);
    let last = *window.first().unwrap();
    let prev = *hist.get(hist.len() - 2).unwrap();
    let two_back = *hist.get(hist.len() - 3).unwrap();

    let total_drop = if high > 0.0 { ((last - high) / high) * 100.0 } else { 0.0 };
    let curr_drop = ((last - prev) / prev) * 100.0;
    let prev_drop = ((prev - two_back) / two_back) * 100.0;

    let mut drops = Vec::new();
    for i in 1..=DROP_LOOKBACK {
        let now = hist[hist.len() - i];
        let prev = hist[hist.len() - i - 1];
        let drop = ((now - prev) / prev) * 100.0;
        if drop < 0.0 {
            drops.push(drop.abs());
        }
    }
    let avg_drop = if !drops.is_empty() {
        drops.iter().sum::<f64>() / (drops.len() as f64)
    } else {
        0.0
    };

    // Additional token-based filtering using the Token object
    let volume_24h = token.volume.h24;
    let price_change_24h = token.price_change.h24;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let buys_24h = token.txns.h24.buys;

    // Enhanced filtering using token metadata
    let has_sufficient_liquidity = liquidity_sol >= 5.0; // At least 5 SOL liquidity
    let has_activity = buys_24h >= 5; // At least 5 buys in 24h
    let not_rugged = price_change_24h > -70.0; // Not dumped more than 70% in 24h
    let has_volume = volume_24h >= 1000.0; // At least $1k volume in 24h

    if
        total_drop <= -MIN_TOTAL_DROP &&
        (curr_drop <= -MIN_DROP_ABS ||
            (curr_drop < -0.5 && curr_drop.abs() > MIN_DROP_SCALE * avg_drop)) &&
        prev_drop < 0.0 &&
        has_sufficient_liquidity &&
        has_activity &&
        not_rugged &&
        has_volume
    {
        // Rug-prevention: skip ultra dumps
        if curr_drop > -60.0 {
            let bounced = ((last - prev) / prev) * 100.0 > MIN_BOUNCE;
            if bounced || MIN_BOUNCE <= 0.0 {
                println!(
                    "🟢 SMART BUY SIGNAL {}: total_drop={:.2}%, last_drop={:.2}% (avg={:.2}%) | Vol24h=${:.0} | Liq={:.1}SOL | Buys24h={}",
                    token.symbol,
                    total_drop,
                    curr_drop,
                    avg_drop,
                    volume_24h,
                    liquidity_sol,
                    buys_24h
                );
                return true;
            }
        } else {
            println!("⚠️ [SKIP] {}: Dump >60% in 1 candle, likely rug, skip!", token.symbol);
        }
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
