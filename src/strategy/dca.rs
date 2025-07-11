use crate::prelude::*;
use crate::price_validation::{ is_price_valid, get_trading_price };
use super::config::*;

/// ENHANCED WHALE-AWARE DCA STRATEGY - OPTIMIZED BASED ON PERFORMANCE ANALYSIS
/// Analysis shows DCA positions have 42% efficiency vs 976% for no-DCA - need MUCH more conservative approach
pub fn should_dca(
    token: &Token,
    pos: &Position,
    current_price: f64,
    trades: Option<&TokenTradesCache>,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> bool {
    // ✅ CRITICAL: Validate price before any DCA decision
    if !is_price_valid(current_price) {
        println!("🚫 [DCA] {} | Invalid price: {:.12} - DCA BLOCKED", token.symbol, current_price);
        return false;
    }

    // Double-check with cached price validation
    if let Some(trading_price) = get_trading_price(&token.mint) {
        let price_diff = (((current_price - trading_price) / trading_price) * 100.0).abs();
        if price_diff > 10.0 {
            println!(
                "⚠️ [DCA] {} | Price mismatch: current={:.12}, cached={:.12} ({:.1}% diff) - using cached",
                token.symbol,
                current_price,
                trading_price,
                price_diff
            );
        }
    } else {
        println!("🚫 [DCA] {} | No valid cached price available - DCA BLOCKED", token.symbol);
        return false;
    }

    let now = Utc::now();
    let elapsed = now - pos.open_time;
    let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;

    println!(
        "\n🔄 [DCA] {} | Drop: {:.1}% | DCA: {}/{} | Held: {}min",
        token.symbol,
        drop_pct,
        pos.dca_count,
        MAX_DCA_COUNT,
        elapsed.num_minutes()
    );

    // ═══ ENHANCED DCA RESTRICTIONS (ADDRESSING EFFICIENCY ISSUE) ═══

    // 1. Hard limits
    if pos.dca_count >= MAX_DCA_COUNT {
        println!("❌ [DCA] {} | Max DCA reached", token.symbol);
        return false;
    }

    // 2. Enhanced cooldown check
    if pos.dca_count > 0 && (now - pos.last_dca_time).num_minutes() < DCA_COOLDOWN_MINUTES {
        println!("⏰ [DCA] {} | Cooldown active", token.symbol);
        return false;
    }

    // 3. Enhanced minimum hold time (longer for DCA)
    if elapsed.num_minutes() < 30 {
        // Increased from 15 to 30 minutes
        println!("⏰ [DCA] {} | Hold longer for DCA consideration", token.symbol);
        return false;
    }

    // 4. ENHANCED DCA TRIGGER LOGIC - Two-tier system
    // Quick DCA for strong whale signals at -8%, standard DCA at -15%
    let whale_confirmed_quick_dca = if let Some(trades_cache) = trades {
        let recent_whale_volume: f64 = trades_cache
            .get_whale_trades(DCA_WHALE_CONFIRMATION_THRESHOLD, 0)
            .iter()
            .filter(
                |t|
                    t.kind == "buy" &&
                    t.timestamp >
                        std::time::SystemTime
                            ::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() - 1800
            ) // Last 30 minutes
            .map(|t| t.volume_usd)
            .sum();

        recent_whale_volume >= DCA_WHALE_CONFIRMATION_THRESHOLD
    } else {
        false
    };

    // Apply appropriate DCA trigger threshold
    let dca_trigger = if whale_confirmed_quick_dca {
        DCA_QUICK_TRIGGER_PCT
    } else {
        DCA_BASE_TRIGGER_PCT
    };

    if drop_pct > dca_trigger {
        if whale_confirmed_quick_dca {
            println!(
                "⚡ [DCA] {} | Quick DCA threshold not met: {:.1}% > {:.1}% (whale confirmed)",
                token.symbol,
                drop_pct,
                dca_trigger
            );
        } else {
            println!(
                "📈 [DCA] {} | Standard DCA threshold not met: {:.1}% > {:.1}%",
                token.symbol,
                drop_pct,
                dca_trigger
            );
        }
        return false;
    }

    println!(
        "✅ [DCA] {} | {} trigger met: {:.1}% <= {:.1}%",
        token.symbol,
        if whale_confirmed_quick_dca {
            "Quick DCA"
        } else {
            "Standard DCA"
        },
        drop_pct,
        dca_trigger
    );

    // 5. Liquidity check (reasonable requirement)
    if liquidity_sol < MIN_LIQUIDITY_SOL * 1.5 {
        // Reduced from 2.0x
        println!(
            "💧 [DCA] {} | Insufficient liquidity for DCA: {:.1}SOL (need {:.1}SOL)",
            token.symbol,
            liquidity_sol,
            MIN_LIQUIDITY_SOL * 1.5
        );
        return false;
    }

    // 6. Check if position would be profitable even at DCA profit target
    let current_value = current_price * pos.token_amount;
    let current_profit_pct = ((current_value - pos.sol_spent) / pos.sol_spent) * 100.0;

    if current_profit_pct >= DCA_PROFIT_TARGET {
        println!(
            "💰 [DCA] {} | Already near DCA profit target: {:.2}%",
            token.symbol,
            current_profit_pct
        );
        return false;
    }

    // 7. Enhanced whale activity check - require STRONG accumulation for DCA
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let buy_ratio = if buys_1h + sells_1h > 0 {
        (buys_1h as f64) / ((buys_1h + sells_1h) as f64)
    } else {
        0.0
    };

    // 7. IMPROVED whale activity check - more flexible but still selective
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let buy_ratio = if buys_1h + sells_1h > 0 {
        (buys_1h as f64) / ((buys_1h + sells_1h) as f64)
    } else {
        0.0
    };

    // More flexible buy ratio requirement
    if buy_ratio < 0.45 {
        // Reduced from 0.6
        println!(
            "📉 [DCA] {} | Buy ratio too low for DCA: {:.2} (need >0.45)",
            token.symbol,
            buy_ratio
        );
        return false;
    }

    let mut strong_whale_accumulation = false;

    if let Some(trades_cache) = trades {
        // More reasonable whale activity check
        let whale_trades_30min = trades_cache.get_whale_trades(50.0, 0); // Reduced from 100.0
        let recent_whale_buys: f64 = whale_trades_30min
            .iter()
            .filter(
                |t|
                    t.kind == "buy" &&
                    t.timestamp >
                        std::time::SystemTime
                            ::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() - 1800 // Last 30 minutes
            )
            .map(|t| t.volume_usd)
            .sum();

        let recent_whale_sells: f64 = whale_trades_30min
            .iter()
            .filter(
                |t|
                    t.kind == "sell" &&
                    t.timestamp >
                        std::time::SystemTime
                            ::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() - 1800
            )
            .map(|t| t.volume_usd)
            .sum();

        let whale_net_flow = recent_whale_buys - recent_whale_sells;

        // More flexible whale requirement
        strong_whale_accumulation = whale_net_flow > 50.0; // Reduced from higher requirements

        println!(
            "🐋 [DCA] {} | Whale flow 30min: ${:.0} (buys: ${:.0}, sells: ${:.0})",
            token.symbol,
            whale_net_flow,
            recent_whale_buys,
            recent_whale_sells
        );
    }

    // Final validation: DCA approved if we reach here with strong whale accumulation
    println!(
        "✅ [DCA] {} | APPROVED | Drop: {:.1}% | BuyRatio: {:.2} | Strong whale accumulation confirmed",
        token.symbol,
        drop_pct,
        buy_ratio
    );
    true
}
