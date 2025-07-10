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

    // 4. MUCH more conservative drop requirement (-20% instead of -15%)
    if drop_pct > DCA_BASE_TRIGGER_PCT {
        println!(
            "📈 [DCA] {} | Drop insufficient for enhanced threshold: {:.1}% > {:.1}%",
            token.symbol,
            drop_pct,
            DCA_BASE_TRIGGER_PCT
        );
        return false;
    }

    // 5. Enhanced liquidity check (higher requirement for DCA)
    if liquidity_sol < MIN_LIQUIDITY_SOL * 2.0 {
        // Require 2x the normal liquidity
        println!(
            "💧 [DCA] {} | Insufficient liquidity for DCA: {:.1}SOL",
            token.symbol,
            liquidity_sol
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

    // Require higher buy ratio for DCA
    if buy_ratio < 0.6 {
        // Increased from 0.4
        println!("📉 [DCA] {} | Insufficient buy ratio for DCA: {:.2}", token.symbol, buy_ratio);
        return false;
    }

    let mut strong_whale_accumulation = false;

    if let Some(trades_cache) = trades {
        // Check for STRONG whale accumulation (2x normal requirement)
        let whale_trades_30min = trades_cache.get_whale_trades(100.0, 0); // Higher threshold
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
                            .as_secs() - 1800
            ) // Last 30 minutes
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
            ) // Last 30 minutes
            .map(|t| t.volume_usd)
            .sum();

        let whale_net_flow = recent_whale_buys - recent_whale_sells;

        // Require 2x the normal whale accumulation for DCA
        if whale_net_flow > MODERATE_WHALE_ACCUMULATION_USD * 2.0 {
            strong_whale_accumulation = true;
            println!(
                "🐋 [DCA] {} | STRONG whale accumulation detected: ${:.0} net flow",
                token.symbol,
                whale_net_flow
            );
        } else {
            println!(
                "📉 [DCA] {} | Insufficient whale accumulation for DCA: ${:.0} net flow (need ${:.0}+)",
                token.symbol,
                whale_net_flow,
                MODERATE_WHALE_ACCUMULATION_USD * 2.0
            );
        }
    }

    if !strong_whale_accumulation {
        println!("🐋 [DCA] {} | Strong whale accumulation required for DCA", token.symbol);
        return false;
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
