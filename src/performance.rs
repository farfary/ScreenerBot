use crate::prelude::*;
use serde::{ Serialize, Deserialize };
use std::collections::{ HashMap, VecDeque };
use tokio::fs;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

const PERFORMANCE_FILE: &str = "performance_history.json";
const MAX_HISTORY_SIZE: usize = 200; // Keep last 200 trades

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeRecord {
    pub mint: String,
    pub symbol: String,
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub sol_spent: f64,
    pub sol_received: Option<f64>,
    pub profit_pct: Option<f64>,
    pub profit_sol: Option<f64>,
    pub hold_duration_minutes: Option<i64>,
    pub dca_count: u8,
    pub exit_reason: Option<String>,
    pub entry_signals: Vec<String>,
    pub is_rug: bool,
    pub whale_score: f64,
    pub bot_score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub rug_count: usize,
    pub win_rate: f64,
    pub avg_win_pct: f64,
    pub avg_loss_pct: f64,
    pub avg_hold_time_minutes: f64,
    pub total_profit_sol: f64,
    pub best_trade_pct: f64,
    pub worst_trade_pct: f64,
    pub recent_performance_7d: f64, // Last 7 days profit
    pub recent_win_rate_7d: f64,
}

pub static TRADE_HISTORY: Lazy<RwLock<VecDeque<TradeRecord>>> = Lazy::new(||
    RwLock::new(VecDeque::new())
);
pub static PERFORMANCE_METRICS: Lazy<RwLock<PerformanceMetrics>> = Lazy::new(||
    RwLock::new(PerformanceMetrics::default())
);

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            rug_count: 0,
            win_rate: 0.0,
            avg_win_pct: 0.0,
            avg_loss_pct: 0.0,
            avg_hold_time_minutes: 0.0,
            total_profit_sol: 0.0,
            best_trade_pct: 0.0,
            worst_trade_pct: 0.0,
            recent_performance_7d: 0.0,
            recent_win_rate_7d: 0.0,
        }
    }
}

/// Load performance history from disk
pub async fn load_performance_history() -> anyhow::Result<()> {
    if let Ok(data) = fs::read(PERFORMANCE_FILE).await {
        let history: VecDeque<TradeRecord> = serde_json::from_slice(&data)?;
        *TRADE_HISTORY.write().await = history;
        update_performance_metrics().await;
        println!("📊 [PERFORMANCE] Loaded {} trade records", TRADE_HISTORY.read().await.len());
    }
    Ok(())
}

/// Save performance history to disk
pub async fn save_performance_history() -> anyhow::Result<()> {
    let history = TRADE_HISTORY.read().await;
    let data = serde_json::to_string_pretty(&*history)?;
    fs::write(PERFORMANCE_FILE, data).await?;
    Ok(())
}

/// Record a new trade entry
pub async fn record_trade_entry(
    mint: &str,
    symbol: &str,
    entry_price: f64,
    sol_spent: f64,
    entry_signals: Vec<String>,
    whale_score: f64,
    bot_score: f64
) {
    let record = TradeRecord {
        mint: mint.to_string(),
        symbol: symbol.to_string(),
        entry_time: Utc::now(),
        exit_time: None,
        entry_price,
        exit_price: None,
        sol_spent,
        sol_received: None,
        profit_pct: None,
        profit_sol: None,
        hold_duration_minutes: None,
        dca_count: 0,
        exit_reason: None,
        entry_signals,
        is_rug: false,
        whale_score,
        bot_score,
    };

    let mut history = TRADE_HISTORY.write().await;
    history.push_back(record);

    // Keep only the most recent records
    if history.len() > MAX_HISTORY_SIZE {
        history.pop_front();
    }

    drop(history);
    let _ = save_performance_history().await;

    println!("📝 [PERFORMANCE] Recorded entry: {} @ ${:.8}", symbol, entry_price);
}

/// Record a trade exit
pub async fn record_trade_exit(
    mint: &str,
    exit_price: f64,
    sol_received: f64,
    exit_reason: &str,
    dca_count: u8,
    is_rug: bool
) {
    let mut history = TRADE_HISTORY.write().await;

    // Find the matching trade record
    if
        let Some(record) = history
            .iter_mut()
            .rev()
            .find(|r| r.mint == mint && r.exit_time.is_none())
    {
        let exit_time = Utc::now();
        let hold_duration = (exit_time - record.entry_time).num_minutes();
        let profit_sol = sol_received - record.sol_spent;
        let profit_pct = (profit_sol / record.sol_spent) * 100.0;

        record.exit_time = Some(exit_time);
        record.exit_price = Some(exit_price);
        record.sol_received = Some(sol_received);
        record.profit_pct = Some(profit_pct);
        record.profit_sol = Some(profit_sol);
        record.hold_duration_minutes = Some(hold_duration);
        record.dca_count = dca_count;
        record.exit_reason = Some(exit_reason.to_string());
        record.is_rug = is_rug;

        println!(
            "📝 [PERFORMANCE] Recorded exit: {} | {:.2}% profit | {} reason",
            record.symbol,
            profit_pct,
            exit_reason
        );
    }

    drop(history);
    update_performance_metrics().await;
    let _ = save_performance_history().await;
}

/// Update performance metrics based on trade history
async fn update_performance_metrics() {
    let history = TRADE_HISTORY.read().await;
    let completed_trades: Vec<_> = history
        .iter()
        .filter(|t| t.exit_time.is_some())
        .collect();

    if completed_trades.is_empty() {
        return;
    }

    let total_trades = completed_trades.len();
    let winning_trades = completed_trades
        .iter()
        .filter(|t| t.profit_pct.unwrap_or(0.0) > 0.0)
        .count();
    let losing_trades = completed_trades
        .iter()
        .filter(|t| t.profit_pct.unwrap_or(0.0) <= 0.0)
        .count();
    let rug_count = completed_trades
        .iter()
        .filter(|t| t.is_rug)
        .count();

    let win_rate = if total_trades > 0 {
        (winning_trades as f64) / (total_trades as f64)
    } else {
        0.0
    };

    let wins: Vec<f64> = completed_trades
        .iter()
        .filter_map(|t| t.profit_pct)
        .filter(|&p| p > 0.0)
        .collect();

    let losses: Vec<f64> = completed_trades
        .iter()
        .filter_map(|t| t.profit_pct)
        .filter(|&p| p <= 0.0)
        .collect();

    let avg_win_pct = if !wins.is_empty() {
        wins.iter().sum::<f64>() / (wins.len() as f64)
    } else {
        0.0
    };
    let avg_loss_pct = if !losses.is_empty() {
        losses.iter().sum::<f64>() / (losses.len() as f64)
    } else {
        0.0
    };

    let avg_hold_time_minutes = {
        let durations: Vec<i64> = completed_trades
            .iter()
            .filter_map(|t| t.hold_duration_minutes)
            .collect();
        if !durations.is_empty() {
            (durations.iter().sum::<i64>() as f64) / (durations.len() as f64)
        } else {
            0.0
        }
    };

    let total_profit_sol = completed_trades
        .iter()
        .filter_map(|t| t.profit_sol)
        .sum::<f64>();

    let best_trade_pct = completed_trades
        .iter()
        .filter_map(|t| t.profit_pct)
        .fold(0.0, f64::max);
    let worst_trade_pct = completed_trades
        .iter()
        .filter_map(|t| t.profit_pct)
        .fold(0.0, f64::min);

    // Recent performance (last 7 days)
    let seven_days_ago = Utc::now() - chrono::Duration::days(7);
    let recent_trades: Vec<_> = completed_trades
        .iter()
        .filter(|t| t.exit_time.unwrap_or(seven_days_ago) >= seven_days_ago)
        .collect();

    let recent_performance_7d = recent_trades
        .iter()
        .filter_map(|t| t.profit_sol)
        .sum::<f64>();
    let recent_wins = recent_trades
        .iter()
        .filter(|t| t.profit_pct.unwrap_or(0.0) > 0.0)
        .count();
    let recent_win_rate_7d = if !recent_trades.is_empty() {
        (recent_wins as f64) / (recent_trades.len() as f64)
    } else {
        0.0
    };

    let metrics = PerformanceMetrics {
        total_trades,
        winning_trades,
        losing_trades,
        rug_count,
        win_rate,
        avg_win_pct,
        avg_loss_pct,
        avg_hold_time_minutes,
        total_profit_sol,
        best_trade_pct,
        worst_trade_pct,
        recent_performance_7d,
        recent_win_rate_7d,
    };

    *PERFORMANCE_METRICS.write().await = metrics;
}

/// Get current performance metrics
pub async fn get_performance_metrics() -> PerformanceMetrics {
    // Add timeout to prevent hanging
    use std::time::Duration;

    match tokio::time::timeout(Duration::from_secs(5), PERFORMANCE_METRICS.read()).await {
        Ok(metrics) => metrics.clone(),
        Err(_) => {
            eprintln!(
                "⚠️ [PERFORMANCE] Timeout acquiring PERFORMANCE_METRICS lock, returning default"
            );
            // Return default metrics if timeout occurs
            PerformanceMetrics {
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                rug_count: 0,
                win_rate: 0.0,
                avg_win_pct: 0.0,
                avg_loss_pct: 0.0,
                avg_hold_time_minutes: 0.0,
                total_profit_sol: 0.0,
                best_trade_pct: 0.0,
                worst_trade_pct: 0.0,
                recent_performance_7d: 0.0,
                recent_win_rate_7d: 0.0,
            }
        }
    }
}

/// Calculate adaptive position sizing based on recent performance
pub async fn calculate_adaptive_position_size(base_size: f64) -> f64 {
    let metrics = get_performance_metrics().await;

    if metrics.total_trades < 10 {
        return base_size; // Use base size until we have enough data
    }

    let performance_multiplier = if
        metrics.recent_win_rate_7d > 0.7 &&
        metrics.recent_performance_7d > 0.0
    {
        1.2 // Increase size when performing well
    } else if metrics.recent_win_rate_7d < 0.4 || metrics.recent_performance_7d < -0.005 {
        0.7 // Reduce size when performing poorly
    } else {
        1.0 // Normal size
    };

    let final_size = base_size * performance_multiplier;

    if performance_multiplier != 1.0 {
        println!(
            "📈 [ADAPTIVE SIZING] Adjusting position size: {:.6} -> {:.6} SOL ({}x multiplier)",
            base_size,
            final_size,
            performance_multiplier
        );
    }

    final_size
}

/// Print detailed performance report
pub async fn print_performance_report() {
    // Add error handling to prevent crashes
    if let Err(e) = print_performance_report_inner().await {
        eprintln!("💥 [PERFORMANCE REPORT] Error occurred: {:?}", e);
    }
}

/// Internal implementation with error handling
async fn print_performance_report_inner() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::time::Duration;

    let metrics = tokio::time
        ::timeout(Duration::from_secs(10), get_performance_metrics()).await
        .map_err(|_| "Timeout getting performance metrics")?;

    println!("\n═══════════════════════════════════════════════════════════");
    println!("📊 PERFORMANCE REPORT");
    println!("═══════════════════════════════════════════════════════════");
    println!("📈 Total Trades: {}", metrics.total_trades);
    println!("✅ Winning Trades: {} ({:.1}%)", metrics.winning_trades, metrics.win_rate * 100.0);
    println!("❌ Losing Trades: {}", metrics.losing_trades);
    println!("💀 Rug Pulls: {}", metrics.rug_count);
    println!("🎯 Win Rate: {:.1}%", metrics.win_rate * 100.0);
    println!("💰 Avg Win: {:.2}%", metrics.avg_win_pct);
    println!("💸 Avg Loss: {:.2}%", metrics.avg_loss_pct);
    println!("⏱️ Avg Hold Time: {:.1} minutes", metrics.avg_hold_time_minutes);
    println!("💵 Total Profit: {:.6} SOL", metrics.total_profit_sol);
    println!("🚀 Best Trade: {:.2}%", metrics.best_trade_pct);
    println!("📉 Worst Trade: {:.2}%", metrics.worst_trade_pct);
    println!("📅 Recent 7d Profit: {:.6} SOL", metrics.recent_performance_7d);
    println!("📅 Recent 7d Win Rate: {:.1}%", metrics.recent_win_rate_7d * 100.0);
    println!("═══════════════════════════════════════════════════════════\n");

    Ok(())
}

/// Get adaptive entry threshold based on recent performance
pub async fn get_adaptive_entry_threshold() -> f64 {
    let metrics = get_performance_metrics().await;

    if metrics.total_trades < 5 {
        return 0.5; // Default threshold
    }

    // Adjust threshold based on recent performance
    if metrics.recent_win_rate_7d > 0.7 {
        0.4 // Lower threshold when performing well (more aggressive)
    } else if metrics.recent_win_rate_7d < 0.4 {
        0.7 // Higher threshold when performing poorly (more selective)
    } else {
        0.5 // Normal threshold
    }
}

/// Check if we should pause trading based on recent performance
pub async fn should_pause_trading() -> bool {
    let metrics = get_performance_metrics().await;

    // Pause if recent performance is very poor
    metrics.total_trades >= 10 &&
        metrics.recent_win_rate_7d < 0.3 &&
        metrics.recent_performance_7d < -0.01 // Lost more than 0.01 SOL in 7 days
}
