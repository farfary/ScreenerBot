use chrono::{DateTime, Duration as ChronoDuration, Utc};
use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
/// Smart Positions Analysis Tool
///
/// This tool provides comprehensive analysis of trading positions including:
/// - Position performance metrics and statistics
/// - Profitable token pattern analysis
/// - Similar token discovery based on characteristics
/// - Trading strategy insights and recommendations
///
/// Features:
/// - P&L analysis with detailed breakdowns
/// - Token similarity scoring based on multiple attributes
/// - Pattern recognition for profitable trades
/// - Risk assessment and recommendation generation
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Notify;

use screenerbot::{
    logger::init_file_logging,
    positions::{get_closed_positions, get_open_positions},
    positions_db::{get_token_snapshots, initialize_positions_database},
    positions_lib::calculate_position_pnl,
    positions_types::Position,
    tokens::{
        cache::TokenDatabase, get_global_rugcheck_service, initialize_global_rugcheck_service,
        rugcheck::RugcheckResponse,
    },
};

/// Safe wrapper for rugcheck data retrieval
async fn get_token_rugcheck_data_safe(mint: &str) -> Option<RugcheckResponse> {
    if let Some(service) = get_global_rugcheck_service() {
        match service.get_rugcheck_data(mint).await {
            Ok(data) => data,
            Err(_) => None,
        }
    } else {
        None
    }
}

/// Calculate transaction activity score based on transaction patterns
/// Returns a score from 0-100 where higher scores indicate more active/healthy trading
fn calculate_transaction_activity_score(
    txns_h24_total: Option<i64>,
    txns_h6_total: Option<i64>,
    txns_h1_total: Option<i64>,
    buy_sell_ratio_24h: Option<f64>,
) -> Option<f64> {
    let mut score = 0.0;
    let mut factors = 0;

    // Factor 1: 24h transaction volume (0-40 points)
    if let Some(txns_24h) = txns_h24_total {
        let activity_score = match txns_24h {
            0..=10 => 0.0,      // Very low activity
            11..=50 => 10.0,    // Low activity
            51..=200 => 25.0,   // Moderate activity
            201..=1000 => 35.0, // High activity
            _ => 40.0,          // Very high activity
        };
        score += activity_score;
        factors += 1;
    }

    // Factor 2: Buy/sell ratio balance (0-30 points)
    if let Some(ratio) = buy_sell_ratio_24h {
        let balance_score = if ratio.is_infinite() || ratio == 0.0 {
            0.0 // All buys or all sells - not balanced
        } else if ratio >= 0.3 && ratio <= 3.0 {
            30.0 // Good balance between buys and sells
        } else if ratio >= 0.1 && ratio <= 10.0 {
            20.0 // Reasonable balance
        } else {
            10.0 // Poor balance
        };
        score += balance_score;
        factors += 1;
    }

    // Factor 3: Recent activity trend (0-20 points)
    if let (Some(txns_1h), Some(_txns_6h), Some(txns_24h)) =
        (txns_h1_total, txns_h6_total, txns_h24_total)
    {
        let trend_score = if txns_24h == 0 {
            0.0
        } else {
            let recent_ratio = (txns_1h as f64) / ((txns_24h as f64) / 24.0); // Normalized hourly rate
            if recent_ratio >= 1.5 {
                20.0 // Increasing activity
            } else if recent_ratio >= 0.8 {
                15.0 // Stable activity
            } else if recent_ratio >= 0.5 {
                10.0 // Decreasing activity
            } else {
                5.0 // Low recent activity
            }
        };
        score += trend_score;
        factors += 1;
    }

    // Factor 4: Minimum viable activity threshold (0-10 points)
    if let Some(txns_24h) = txns_h24_total {
        if txns_24h >= 20 {
            score += 10.0; // Bonus for having meaningful activity
        }
        factors += 1;
    }

    if factors > 0 {
        Some(((score / (factors as f64)) * 100.0) / 100.0) // Normalize to 0-100
    } else {
        None
    }
}

// Analysis result structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionAnalysis {
    pub total_positions: usize,
    pub open_positions: usize,
    pub closed_positions: usize,
    pub profitable_positions: usize,
    pub losing_positions: usize,
    pub total_pnl: f64,
    pub total_fees: f64,
    pub win_rate: f64,
    pub average_profit: f64,
    pub average_loss: f64,
    pub best_position: Option<PositionSummary>,
    pub worst_position: Option<PositionSummary>,
    pub position_duration_stats: DurationStats,
    pub performance_by_time: Vec<TimeBasedPerformance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSummary {
    pub mint: String,
    pub symbol: String,
    pub pnl: f64,
    pub pnl_percentage: f64,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub duration_hours: Option<f64>,
    pub entry_time: DateTime<Utc>,
    pub exit_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStats {
    pub average_duration_hours: f64,
    pub median_duration_hours: f64,
    pub shortest_duration_hours: f64,
    pub longest_duration_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeBasedPerformance {
    pub period: String, // "last_24h", "last_7d", "last_30d"
    pub positions_count: usize,
    pub total_pnl: f64,
    pub win_rate: f64,
    pub average_duration_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSimilarity {
    pub mint1: String,
    pub symbol1: String,
    pub mint2: String,
    pub symbol2: String,
    pub similarity_score: f64,
    pub matching_attributes: Vec<String>,
    pub both_profitable: bool,
    pub pnl1: f64,
    pub pnl2: f64,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitablePattern {
    pub pattern_name: String,
    pub description: String,
    pub positions_count: usize,
    pub average_pnl: f64,
    pub win_rate: f64,
    pub confidence_score: f64,
    pub example_tokens: Vec<String>,
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCharacteristics {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub pnl: f64,
    pub pnl_percentage: f64,
    pub is_profitable: bool,

    // Market data
    pub liquidity_usd: Option<f64>,
    pub market_cap: Option<f64>,
    pub volume_24h: Option<f64>,
    pub price_change_24h: Option<f64>,

    // Transaction counts
    pub txns_h24_buys: Option<i64>,
    pub txns_h24_sells: Option<i64>,
    pub txns_h24_total: Option<i64>,
    pub txns_h6_buys: Option<i64>,
    pub txns_h6_sells: Option<i64>,
    pub txns_h6_total: Option<i64>,
    pub txns_h1_buys: Option<i64>,
    pub txns_h1_sells: Option<i64>,
    pub txns_h1_total: Option<i64>,
    pub buy_sell_ratio_24h: Option<f64>,
    pub transaction_activity_score: Option<f64>, // Calculated score based on transaction patterns

    // Rugcheck data
    pub rugcheck_score: Option<i32>,
    pub rugcheck_score_normalized: Option<i32>, // Added normalized rugcheck score
    pub rugcheck_rugged: Option<bool>,
    pub lp_locked_pct: Option<f64>,
    pub total_holders: Option<i32>,
    pub creator_balance_pct: Option<f64>,

    // Trading metrics
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub duration_hours: Option<f64>,
    pub max_price: f64,
    pub min_price: f64,

    // Verification
    pub jup_verified: Option<bool>,
    pub jup_strict: Option<bool>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (this function returns () so we don't use ?)
    init_file_logging();

    // Initialize positions database
    if let Err(e) = initialize_positions_database().await {
        eprintln!("❌ Failed to initialize positions database: {}", e);
        eprintln!(
            "   This is required for position analysis. Please ensure the database is accessible."
        );
        std::process::exit(1);
    }

    // Initialize rugcheck service for accessing rugcheck data
    if get_global_rugcheck_service().is_none() {
        println!("🔧 Initializing rugcheck service for token analysis...");
        let database = TokenDatabase::new()
            .map_err(|e| format!("Failed to initialize token database: {}", e))?;
        let shutdown_notify = Arc::new(Notify::new());

        if let Err(e) = initialize_global_rugcheck_service(database, shutdown_notify).await {
            eprintln!("⚠️  Warning: Failed to initialize rugcheck service: {}", e);
            eprintln!("   Rugcheck data will not be available for analysis.");
        } else {
            println!("✅ Rugcheck service initialized successfully");
        }
    }

    let matches = Command::new("ScreenerBot Positions Analyzer")
        .version("1.0")
        .author("ScreenerBot Team")
        .about("Smart analysis of trading positions and profitable patterns")
        .arg(
            Arg::new("detailed")
                .long("detailed")
                .help("Show detailed analysis including individual position breakdowns")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("similarity-threshold")
                .long("similarity-threshold")
                .help("Minimum similarity score for token comparison (0.0-1.0)")
                .value_name("THRESHOLD")
                .default_value("0.7"),
        )
        .arg(
            Arg::new("min-positions")
                .long("min-positions")
                .help("Minimum positions required for pattern analysis")
                .value_name("COUNT")
                .default_value("3"),
        )
        .arg(
            Arg::new("export-json")
                .long("export-json")
                .help("Export analysis results to JSON file")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("top-similar")
                .long("top-similar")
                .help("Number of top similar token pairs to show")
                .value_name("COUNT")
                .default_value("10"),
        )
        .arg(
            Arg::new("profitable-only")
                .long("profitable-only")
                .help("Only analyze profitable positions")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let detailed = matches.get_flag("detailed");
    let similarity_threshold: f64 = matches
        .get_one::<String>("similarity-threshold")
        .unwrap()
        .parse()?;
    let min_positions: usize = matches
        .get_one::<String>("min-positions")
        .unwrap()
        .parse()?;
    let export_json = matches.get_one::<String>("export-json");
    let top_similar: usize = matches.get_one::<String>("top-similar").unwrap().parse()?;
    let profitable_only = matches.get_flag("profitable-only");

    println!("🔍 ScreenerBot Smart Positions Analyzer");
    println!("=========================================\n");

    // Analyze positions
    println!("📊 Loading and analyzing positions...");
    let analysis = analyze_positions(profitable_only).await?;
    print_position_analysis(&analysis, detailed);

    // Analyze token characteristics and find patterns
    println!("\n🧬 Analyzing token characteristics and patterns...");
    let token_characteristics = collect_token_characteristics(profitable_only).await?;

    if token_characteristics.len() < 2 {
        println!("⚠️  Not enough positions for similarity analysis (need at least 2)");
        return Ok(());
    }

    // Find similar tokens
    println!("\n🔗 Finding similar profitable tokens...");
    let similarities =
        find_similar_tokens(&token_characteristics, similarity_threshold, top_similar);
    print_token_similarities(&similarities);

    // Identify profitable patterns
    println!("\n📈 Identifying profitable patterns...");
    let patterns = identify_profitable_patterns(&token_characteristics, min_positions);
    print_profitable_patterns(&patterns);

    // Generate recommendations
    println!("\n💡 Generating trading recommendations...");
    let recommendations = generate_recommendations(&analysis, &similarities, &patterns);
    print_recommendations(&recommendations);

    // Export to JSON if requested
    if let Some(export_file) = export_json {
        export_analysis_to_json(
            &analysis,
            &similarities,
            &patterns,
            &recommendations,
            export_file,
        )?;
        println!("\n📁 Analysis exported to: {}", export_file);
    }

    Ok(())
}

/// Analyze all positions and generate comprehensive statistics
async fn analyze_positions(
    profitable_only: bool,
) -> Result<PositionAnalysis, Box<dyn std::error::Error>> {
    let open_positions = get_open_positions().await;
    let closed_positions = get_closed_positions().await;

    let mut all_positions = Vec::new();
    all_positions.extend(open_positions.clone());
    all_positions.extend(closed_positions.clone());

    if profitable_only {
        all_positions.retain(|pos| {
            let (pnl, _) =
                futures::executor::block_on(async { calculate_position_pnl(pos, None).await });
            pnl > 0.0
        });
    }

    let mut profitable_count = 0;
    let mut losing_count = 0;
    let mut total_pnl = 0.0;
    let mut total_fees = 0.0;
    let mut profits = Vec::new();
    let mut losses = Vec::new();
    let mut durations = Vec::new();
    let mut best_position: Option<PositionSummary> = None;
    let mut worst_position: Option<PositionSummary> = None;
    let mut best_pnl = f64::NEG_INFINITY;
    let mut worst_pnl = f64::INFINITY;

    for position in &all_positions {
        let (pnl, pnl_percentage) = calculate_position_pnl(position, None).await;
        total_pnl += pnl;

        // Calculate fees
        let entry_fee = (position.entry_fee_lamports.unwrap_or(0) as f64) / 1_000_000_000.0;
        let exit_fee = (position.exit_fee_lamports.unwrap_or(0) as f64) / 1_000_000_000.0;
        total_fees += entry_fee + exit_fee;

        let duration_hours = if let Some(exit_time) = position.exit_time {
            Some(
                (exit_time
                    .signed_duration_since(position.entry_time)
                    .num_minutes() as f64)
                    / 60.0,
            )
        } else {
            None
        };

        if let Some(duration) = duration_hours {
            durations.push(duration);
        }

        let summary = PositionSummary {
            mint: position.mint.clone(),
            symbol: position.symbol.clone(),
            pnl,
            pnl_percentage,
            entry_price: position.entry_price,
            exit_price: position.exit_price,
            duration_hours,
            entry_time: position.entry_time,
            exit_time: position.exit_time,
        };

        if pnl > 0.0 {
            profitable_count += 1;
            profits.push(pnl);
        } else {
            losing_count += 1;
            losses.push(pnl);
        }

        if pnl > best_pnl {
            best_pnl = pnl;
            best_position = Some(summary.clone());
        }

        if pnl < worst_pnl {
            worst_pnl = pnl;
            worst_position = Some(summary);
        }
    }

    let win_rate = if all_positions.len() > 0 {
        ((profitable_count as f64) / (all_positions.len() as f64)) * 100.0
    } else {
        0.0
    };

    let average_profit = if !profits.is_empty() {
        profits.iter().sum::<f64>() / (profits.len() as f64)
    } else {
        0.0
    };

    let average_loss = if !losses.is_empty() {
        losses.iter().sum::<f64>() / (losses.len() as f64)
    } else {
        0.0
    };

    // Calculate duration statistics
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let duration_stats = if !durations.is_empty() {
        let avg = durations.iter().sum::<f64>() / (durations.len() as f64);
        let median = if durations.len() % 2 == 0 {
            (durations[durations.len() / 2 - 1] + durations[durations.len() / 2]) / 2.0
        } else {
            durations[durations.len() / 2]
        };

        DurationStats {
            average_duration_hours: avg,
            median_duration_hours: median,
            shortest_duration_hours: durations[0],
            longest_duration_hours: durations[durations.len() - 1],
        }
    } else {
        DurationStats {
            average_duration_hours: 0.0,
            median_duration_hours: 0.0,
            shortest_duration_hours: 0.0,
            longest_duration_hours: 0.0,
        }
    };

    // Calculate time-based performance
    let performance_by_time = calculate_time_based_performance(&all_positions).await;

    Ok(PositionAnalysis {
        total_positions: all_positions.len(),
        open_positions: open_positions.len(),
        closed_positions: closed_positions.len(),
        profitable_positions: profitable_count,
        losing_positions: losing_count,
        total_pnl,
        total_fees,
        win_rate,
        average_profit,
        average_loss,
        best_position,
        worst_position,
        position_duration_stats: duration_stats,
        performance_by_time,
    })
}

/// Calculate performance statistics for different time periods
async fn calculate_time_based_performance(positions: &[Position]) -> Vec<TimeBasedPerformance> {
    let now = Utc::now();
    let periods = vec![
        ("last_24h", ChronoDuration::hours(24)),
        ("last_7d", ChronoDuration::days(7)),
        ("last_30d", ChronoDuration::days(30)),
    ];

    let mut results = Vec::new();

    for (period_name, duration) in periods {
        let cutoff = now - duration;
        let period_positions: Vec<_> = positions
            .iter()
            .filter(|pos| pos.entry_time >= cutoff)
            .collect();

        if period_positions.is_empty() {
            continue;
        }

        let mut total_pnl = 0.0;
        let mut profitable_count = 0;
        let mut durations_sum = 0.0;
        let mut duration_count = 0;

        for position in &period_positions {
            let (pnl, _) = calculate_position_pnl(position, None).await;
            total_pnl += pnl;

            if pnl > 0.0 {
                profitable_count += 1;
            }

            if let Some(exit_time) = position.exit_time {
                let duration = (exit_time
                    .signed_duration_since(position.entry_time)
                    .num_minutes() as f64)
                    / 60.0;
                durations_sum += duration;
                duration_count += 1;
            }
        }

        let win_rate = ((profitable_count as f64) / (period_positions.len() as f64)) * 100.0;
        let avg_duration = if duration_count > 0 {
            durations_sum / (duration_count as f64)
        } else {
            0.0
        };

        results.push(TimeBasedPerformance {
            period: period_name.to_string(),
            positions_count: period_positions.len(),
            total_pnl,
            win_rate,
            average_duration_hours: avg_duration,
        });
    }

    results
}

/// Collect comprehensive characteristics for all tokens
async fn collect_token_characteristics(
    profitable_only: bool,
) -> Result<Vec<TokenCharacteristics>, Box<dyn std::error::Error>> {
    let open_positions = get_open_positions().await;
    let closed_positions = get_closed_positions().await;

    let mut all_positions = Vec::new();
    all_positions.extend(open_positions);
    all_positions.extend(closed_positions);

    let mut characteristics = Vec::new();

    for position in all_positions {
        let (pnl, pnl_percentage) = calculate_position_pnl(&position, None).await;

        if profitable_only && pnl <= 0.0 {
            continue;
        }

        // Get token snapshots
        let snapshots = if let Some(id) = position.id {
            get_token_snapshots(id).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        let opening_snapshot = snapshots.iter().find(|s| s.snapshot_type == "opening");

        // Get rugcheck data
        let rugcheck_data = get_token_rugcheck_data_safe(&position.mint).await;

        let duration_hours = if let Some(exit_time) = position.exit_time {
            Some(
                (exit_time
                    .signed_duration_since(position.entry_time)
                    .num_minutes() as f64)
                    / 60.0,
            )
        } else {
            None
        };

        let creator_balance_pct = if let Some(rugcheck) = &rugcheck_data {
            if let (Some(creator_balance_str), Some(total_holders)) =
                (&rugcheck.creator_balance, rugcheck.total_holders)
            {
                if let Ok(creator_balance) = creator_balance_str.parse::<f64>() {
                    Some((creator_balance / (total_holders as f64)) * 100.0)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Calculate transaction metrics from snapshots
        let (txns_h24_buys, txns_h24_sells, txns_h24_total) =
            if let Some(snapshot) = opening_snapshot {
                let buys = snapshot.txns_h24_buys.unwrap_or(0);
                let sells = snapshot.txns_h24_sells.unwrap_or(0);
                let total = buys + sells;
                (Some(buys), Some(sells), Some(total))
            } else {
                (None, None, None)
            };

        let (txns_h6_buys, txns_h6_sells, txns_h6_total) = if let Some(snapshot) = opening_snapshot
        {
            let buys = snapshot.txns_h6_buys.unwrap_or(0);
            let sells = snapshot.txns_h6_sells.unwrap_or(0);
            let total = buys + sells;
            (Some(buys), Some(sells), Some(total))
        } else {
            (None, None, None)
        };

        let (txns_h1_buys, txns_h1_sells, txns_h1_total) = if let Some(snapshot) = opening_snapshot
        {
            let buys = snapshot.txns_h1_buys.unwrap_or(0);
            let sells = snapshot.txns_h1_sells.unwrap_or(0);
            let total = buys + sells;
            (Some(buys), Some(sells), Some(total))
        } else {
            (None, None, None)
        };

        // Calculate buy/sell ratio for 24h
        let buy_sell_ratio_24h = if let (Some(buys), Some(sells)) = (txns_h24_buys, txns_h24_sells)
        {
            if sells > 0 {
                Some((buys as f64) / (sells as f64))
            } else if buys > 0 {
                Some(f64::INFINITY) // All buys, no sells
            } else {
                Some(0.0) // No transactions
            }
        } else {
            None
        };

        // Calculate transaction activity score (0-100)
        let transaction_activity_score = calculate_transaction_activity_score(
            txns_h24_total,
            txns_h6_total,
            txns_h1_total,
            buy_sell_ratio_24h,
        );

        let characteristics_entry = TokenCharacteristics {
            mint: position.mint.clone(),
            symbol: position.symbol.clone(),
            name: position.name.clone(),
            pnl,
            pnl_percentage,
            is_profitable: pnl > 0.0,

            // Market data from snapshots
            liquidity_usd: opening_snapshot.and_then(|s| s.liquidity_usd),
            market_cap: opening_snapshot.and_then(|s| s.market_cap),
            volume_24h: opening_snapshot.and_then(|s| s.volume_h24),
            price_change_24h: opening_snapshot.and_then(|s| s.price_change_h24),

            // Transaction counts
            txns_h24_buys,
            txns_h24_sells,
            txns_h24_total,
            txns_h6_buys,
            txns_h6_sells,
            txns_h6_total,
            txns_h1_buys,
            txns_h1_sells,
            txns_h1_total,
            buy_sell_ratio_24h,
            transaction_activity_score,

            // Rugcheck data
            rugcheck_score: rugcheck_data.as_ref().and_then(|r| r.score),
            rugcheck_score_normalized: rugcheck_data.as_ref().and_then(|r| r.score_normalised),
            rugcheck_rugged: rugcheck_data.as_ref().and_then(|r| r.rugged),
            lp_locked_pct: rugcheck_data
                .as_ref()
                .and_then(|r| r.markets.as_ref())
                .and_then(|markets| markets.first())
                .and_then(|market| market.lp.as_ref())
                .and_then(|lp| lp.lp_locked_pct),
            total_holders: rugcheck_data.as_ref().and_then(|r| r.total_holders),
            creator_balance_pct,

            // Trading metrics
            entry_price: position.entry_price,
            exit_price: position.exit_price,
            duration_hours,
            max_price: position.price_highest,
            min_price: position.price_lowest,

            // Verification
            jup_verified: rugcheck_data
                .as_ref()
                .and_then(|r| r.verification.as_ref())
                .and_then(|v| v.jup_verified),
            jup_strict: rugcheck_data
                .as_ref()
                .and_then(|r| r.verification.as_ref())
                .and_then(|v| v.jup_strict),
        };

        characteristics.push(characteristics_entry);
    }

    Ok(characteristics)
}

/// Find similar tokens based on multiple characteristics
fn find_similar_tokens(
    characteristics: &[TokenCharacteristics],
    threshold: f64,
    top_count: usize,
) -> Vec<TokenSimilarity> {
    let mut similarities = Vec::new();

    for i in 0..characteristics.len() {
        for j in i + 1..characteristics.len() {
            let char1 = &characteristics[i];
            let char2 = &characteristics[j];

            let similarity_score = calculate_similarity_score(char1, char2);

            if similarity_score >= threshold {
                let matching_attributes = get_matching_attributes(char1, char2);
                let both_profitable = char1.is_profitable && char2.is_profitable;

                let recommendation = if both_profitable {
                    format!(
                        "✅ Both tokens profitable - Strong pattern match ({})",
                        matching_attributes.join(", ")
                    )
                } else if char1.is_profitable || char2.is_profitable {
                    format!(
                        "⚠️  Mixed results - Investigate differences in {}",
                        get_differing_attributes(char1, char2).join(", ")
                    )
                } else {
                    "❌ Both unprofitable - Avoid similar patterns".to_string()
                };

                similarities.push(TokenSimilarity {
                    mint1: char1.mint.clone(),
                    symbol1: char1.symbol.clone(),
                    mint2: char2.mint.clone(),
                    symbol2: char2.symbol.clone(),
                    similarity_score,
                    matching_attributes,
                    both_profitable,
                    pnl1: char1.pnl,
                    pnl2: char2.pnl,
                    recommendation,
                });
            }
        }
    }

    // Sort by similarity score and take top results
    similarities.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
    similarities.into_iter().take(top_count).collect()
}

/// Calculate similarity score between two tokens (0.0 to 1.0)
fn calculate_similarity_score(char1: &TokenCharacteristics, char2: &TokenCharacteristics) -> f64 {
    let mut matches = 0;
    let mut total_comparisons = 0;

    // Market cap similarity
    if let (Some(mc1), Some(mc2)) = (char1.market_cap, char2.market_cap) {
        total_comparisons += 1;
        if (mc1 - mc2).abs() / mc1.max(mc2) < 0.5 {
            // Within 50%
            matches += 1;
        }
    }

    // Liquidity similarity
    if let (Some(liq1), Some(liq2)) = (char1.liquidity_usd, char2.liquidity_usd) {
        total_comparisons += 1;
        if (liq1 - liq2).abs() / liq1.max(liq2) < 0.5 {
            matches += 1;
        }
    }

    // Transaction activity similarity
    if let (Some(txns1), Some(txns2)) = (char1.txns_h24_total, char2.txns_h24_total) {
        total_comparisons += 1;
        if txns1 == 0 && txns2 == 0 {
            matches += 1; // Both have no activity
        } else if txns1 > 0 && txns2 > 0 {
            let ratio = (txns1 as f64) / (txns2 as f64);
            if ratio >= 0.5 && ratio <= 2.0 {
                matches += 1; // Similar transaction volume
            }
        }
    }

    // Buy/sell ratio similarity
    if let (Some(ratio1), Some(ratio2)) = (char1.buy_sell_ratio_24h, char2.buy_sell_ratio_24h) {
        total_comparisons += 1;
        if ratio1.is_infinite() && ratio2.is_infinite() {
            matches += 1; // Both all-buy tokens
        } else if !ratio1.is_infinite() && !ratio2.is_infinite() && ratio1 > 0.0 && ratio2 > 0.0 {
            let ratio_diff = (ratio1 - ratio2).abs() / ratio1.max(ratio2);
            if ratio_diff < 0.5 {
                matches += 1; // Similar buy/sell patterns
            }
        }
    }

    // Transaction activity score similarity
    if let (Some(score1), Some(score2)) = (
        char1.transaction_activity_score,
        char2.transaction_activity_score,
    ) {
        total_comparisons += 1;
        if (score1 - score2).abs() < 20.0 {
            matches += 1; // Within 20 points
        }
    }

    // Rugcheck score similarity (original)
    if let (Some(score1), Some(score2)) = (char1.rugcheck_score, char2.rugcheck_score) {
        total_comparisons += 1;
        if (score1 - score2).abs() <= 2 {
            // Within 2 points
            matches += 1;
        }
    }

    // Rugcheck normalized score similarity
    if let (Some(norm1), Some(norm2)) = (
        char1.rugcheck_score_normalized,
        char2.rugcheck_score_normalized,
    ) {
        total_comparisons += 1;
        if (norm1 - norm2).abs() <= 2 {
            // Within 2 points
            matches += 1;
        }
    }

    // LP locked percentage similarity
    if let (Some(lp1), Some(lp2)) = (char1.lp_locked_pct, char2.lp_locked_pct) {
        total_comparisons += 1;
        if (lp1 - lp2).abs() < 20.0 {
            // Within 20%
            matches += 1;
        }
    }

    // Verification status
    total_comparisons += 1;
    if char1.jup_verified == char2.jup_verified {
        matches += 1;
    }

    // Duration similarity (for closed positions)
    if let (Some(dur1), Some(dur2)) = (char1.duration_hours, char2.duration_hours) {
        total_comparisons += 1;
        let ratio = dur1.min(dur2) / dur1.max(dur2);
        if ratio > 0.7 {
            // Within similar time range
            matches += 1;
        }
    }

    if total_comparisons == 0 {
        0.0
    } else {
        (matches as f64) / (total_comparisons as f64)
    }
}

/// Get list of matching attributes between two tokens
fn get_matching_attributes(
    char1: &TokenCharacteristics,
    char2: &TokenCharacteristics,
) -> Vec<String> {
    let mut attributes = Vec::new();

    if let (Some(mc1), Some(mc2)) = (char1.market_cap, char2.market_cap) {
        if (mc1 - mc2).abs() / mc1.max(mc2) < 0.5 {
            attributes.push("Similar Market Cap".to_string());
        }
    }

    if let (Some(liq1), Some(liq2)) = (char1.liquidity_usd, char2.liquidity_usd) {
        if (liq1 - liq2).abs() / liq1.max(liq2) < 0.5 {
            attributes.push("Similar Liquidity".to_string());
        }
    }

    // Transaction activity matching
    if let (Some(txns1), Some(txns2)) = (char1.txns_h24_total, char2.txns_h24_total) {
        if txns1 == 0 && txns2 == 0 {
            attributes.push("Both Low Activity".to_string());
        } else if txns1 > 0 && txns2 > 0 {
            let ratio = (txns1 as f64) / (txns2 as f64);
            if ratio >= 0.5 && ratio <= 2.0 {
                attributes.push("Similar Transaction Volume".to_string());
            }
        }
    }

    // Buy/sell ratio matching
    if let (Some(ratio1), Some(ratio2)) = (char1.buy_sell_ratio_24h, char2.buy_sell_ratio_24h) {
        if ratio1.is_infinite() && ratio2.is_infinite() {
            attributes.push("Both All-Buy Tokens".to_string());
        } else if !ratio1.is_infinite() && !ratio2.is_infinite() && ratio1 > 0.0 && ratio2 > 0.0 {
            let ratio_diff = (ratio1 - ratio2).abs() / ratio1.max(ratio2);
            if ratio_diff < 0.5 {
                attributes.push("Similar Buy/Sell Patterns".to_string());
            }
        }
    }

    // Transaction activity score matching
    if let (Some(score1), Some(score2)) = (
        char1.transaction_activity_score,
        char2.transaction_activity_score,
    ) {
        if (score1 - score2).abs() < 20.0 {
            if score1 >= 70.0 && score2 >= 70.0 {
                attributes.push("Both High Activity".to_string());
            } else if score1 >= 40.0 && score2 >= 40.0 {
                attributes.push("Both Moderate Activity".to_string());
            } else {
                attributes.push("Similar Activity Level".to_string());
            }
        }
    }

    if let (Some(score1), Some(score2)) = (char1.rugcheck_score, char2.rugcheck_score) {
        if (score1 - score2).abs() <= 2 {
            attributes.push("Similar Rugcheck Score".to_string());
        }
    }

    if let (Some(norm1), Some(norm2)) = (
        char1.rugcheck_score_normalized,
        char2.rugcheck_score_normalized,
    ) {
        if (norm1 - norm2).abs() <= 2 {
            attributes.push("Similar Normalized Rugcheck Score".to_string());
        }
    }

    if char1.jup_verified == char2.jup_verified && char1.jup_verified.is_some() {
        attributes.push("Same Verification Status".to_string());
    }

    if let (Some(dur1), Some(dur2)) = (char1.duration_hours, char2.duration_hours) {
        let ratio = dur1.min(dur2) / dur1.max(dur2);
        if ratio > 0.7 {
            attributes.push("Similar Hold Duration".to_string());
        }
    }

    attributes
}

/// Get list of differing attributes between two tokens
fn get_differing_attributes(
    char1: &TokenCharacteristics,
    char2: &TokenCharacteristics,
) -> Vec<String> {
    let mut attributes = Vec::new();

    if char1.is_profitable != char2.is_profitable {
        attributes.push("Profitability".to_string());
    }

    if let (Some(score1), Some(score2)) = (char1.rugcheck_score, char2.rugcheck_score) {
        if (score1 - score2).abs() > 2 {
            attributes.push("Rugcheck Score".to_string());
        }
    }

    if let (Some(norm1), Some(norm2)) = (
        char1.rugcheck_score_normalized,
        char2.rugcheck_score_normalized,
    ) {
        if (norm1 - norm2).abs() > 2 {
            attributes.push("Normalized Rugcheck Score".to_string());
        }
    }

    if char1.jup_verified != char2.jup_verified {
        attributes.push("Verification Status".to_string());
    }

    attributes
}

/// Identify profitable patterns across multiple positions
fn identify_profitable_patterns(
    characteristics: &[TokenCharacteristics],
    min_positions: usize,
) -> Vec<ProfitablePattern> {
    let mut patterns = Vec::new();

    // Pattern 1: High transaction activity profitable tokens
    let high_activity_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| c.is_profitable && c.transaction_activity_score.unwrap_or(0.0) >= 70.0)
        .collect();

    if high_activity_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "High Activity Winners",
            "Tokens with high transaction activity (score >= 70) that were profitable",
            &high_activity_profitable,
            vec![("min_activity_score".to_string(), "70".to_string())],
        ));
    }

    // Pattern 2: Balanced buy/sell ratio profitable tokens
    let balanced_trading_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| {
            c.is_profitable
                && c.buy_sell_ratio_24h.map_or(false, |ratio| {
                    !ratio.is_infinite() && ratio >= 0.3 && ratio <= 3.0
                })
        })
        .collect();

    if balanced_trading_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "Balanced Trading Winners",
            "Tokens with balanced buy/sell ratios (0.3-3.0) that were profitable",
            &balanced_trading_profitable,
            vec![("balanced_trading".to_string(), "true".to_string())],
        ));
    }

    // Pattern 3: High volume profitable tokens (24h transactions)
    let high_volume_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| c.is_profitable && c.txns_h24_total.unwrap_or(0) >= 200)
        .collect();

    if high_volume_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "High Volume Winners",
            "Tokens with high 24h transaction volume (>=200) that were profitable",
            &high_volume_profitable,
            vec![("min_txns_24h".to_string(), "200".to_string())],
        ));
    }

    // Pattern 4: High rugcheck score profitable tokens
    let high_rugcheck_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| c.is_profitable && c.rugcheck_score.unwrap_or(0) >= 7)
        .collect();

    if high_rugcheck_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "High Rugcheck Score Winners",
            "Tokens with rugcheck score >= 7 that were profitable",
            &high_rugcheck_profitable,
            vec![("min_rugcheck_score".to_string(), "7".to_string())],
        ));
    }

    // Pattern 4b: High normalized rugcheck score profitable tokens
    let high_normalized_rugcheck_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| c.is_profitable && c.rugcheck_score_normalized.unwrap_or(0) >= 7)
        .collect();

    if high_normalized_rugcheck_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "High Normalized Rugcheck Score Winners",
            "Tokens with normalized rugcheck score >= 7 that were profitable",
            &high_normalized_rugcheck_profitable,
            vec![("min_normalized_rugcheck_score".to_string(), "7".to_string())],
        ));
    }

    // Pattern 4c: Excellent rugcheck scores (both metrics high)
    let excellent_rugcheck_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| {
            c.is_profitable
                && c.rugcheck_score.unwrap_or(0) >= 8
                && c.rugcheck_score_normalized.unwrap_or(0) >= 8
        })
        .collect();

    if excellent_rugcheck_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "Excellent Rugcheck Winners",
            "Tokens with both original (>=8) and normalized (>=8) rugcheck scores high",
            &excellent_rugcheck_profitable,
            vec![
                ("min_rugcheck_score".to_string(), "8".to_string()),
                ("min_normalized_rugcheck_score".to_string(), "8".to_string()),
            ],
        ));
    }

    // Pattern 5: Jupiter verified profitable tokens
    let jup_verified_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| c.is_profitable && c.jup_verified == Some(true))
        .collect();

    if jup_verified_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "Jupiter Verified Winners",
            "Jupiter verified tokens that were profitable",
            &jup_verified_profitable,
            vec![("jupiter_verified".to_string(), "true".to_string())],
        ));
    }

    // Pattern 6: High liquidity profitable tokens
    let high_liquidity_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| c.is_profitable && c.liquidity_usd.unwrap_or(0.0) >= 50000.0)
        .collect();

    if high_liquidity_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "High Liquidity Winners",
            "Tokens with liquidity >= $50K that were profitable",
            &high_liquidity_profitable,
            vec![("min_liquidity_usd".to_string(), "50000".to_string())],
        ));
    }

    // Pattern 7: Quick profit pattern (< 2 hours)
    let quick_profit: Vec<_> = characteristics
        .iter()
        .filter(|c| c.is_profitable && c.duration_hours.unwrap_or(f64::INFINITY) < 2.0)
        .collect();

    if quick_profit.len() >= min_positions {
        patterns.push(create_pattern(
            "Quick Profit Pattern",
            "Profitable positions closed within 2 hours",
            &quick_profit,
            vec![("max_duration_hours".to_string(), "2".to_string())],
        ));
    }

    // Pattern 8: LP locked tokens
    let lp_locked_profitable: Vec<_> = characteristics
        .iter()
        .filter(|c| c.is_profitable && c.lp_locked_pct.unwrap_or(0.0) >= 50.0)
        .collect();

    if lp_locked_profitable.len() >= min_positions {
        patterns.push(create_pattern(
            "LP Locked Winners",
            "Tokens with LP locked >= 50% that were profitable",
            &lp_locked_profitable,
            vec![("min_lp_locked_pct".to_string(), "50".to_string())],
        ));
    }

    patterns
}

/// Create a profitable pattern from a group of characteristics
fn create_pattern(
    name: &str,
    description: &str,
    characteristics: &[&TokenCharacteristics],
    attributes: Vec<(String, String)>,
) -> ProfitablePattern {
    let positions_count = characteristics.len();
    let total_pnl: f64 = characteristics.iter().map(|c| c.pnl).sum();
    let average_pnl = total_pnl / (positions_count as f64);
    let profitable_count = characteristics.iter().filter(|c| c.is_profitable).count();
    let win_rate = ((profitable_count as f64) / (positions_count as f64)) * 100.0;

    // Confidence score based on win rate and sample size
    let confidence_score = (win_rate / 100.0) * (1.0 - 1.0 / ((positions_count as f64) + 1.0));

    let example_tokens: Vec<String> = characteristics
        .iter()
        .take(5)
        .map(|c| format!("{} ({:.3} SOL)", c.symbol, c.pnl))
        .collect();

    let attributes_map: HashMap<String, String> = attributes.into_iter().collect();

    ProfitablePattern {
        pattern_name: name.to_string(),
        description: description.to_string(),
        positions_count,
        average_pnl,
        win_rate,
        confidence_score,
        example_tokens,
        attributes: attributes_map,
    }
}

/// Generate trading recommendations based on analysis
fn generate_recommendations(
    analysis: &PositionAnalysis,
    similarities: &[TokenSimilarity],
    patterns: &[ProfitablePattern],
) -> Vec<String> {
    let mut recommendations = Vec::new();

    // Win rate recommendations
    if analysis.win_rate < 50.0 {
        recommendations.push(
            format!(
                "⚠️  Low win rate ({:.1}%) - Consider tightening entry criteria or improving exit strategy",
                analysis.win_rate
            )
        );
    } else if analysis.win_rate > 70.0 {
        recommendations.push(format!(
            "✅ Excellent win rate ({:.1}%) - Current strategy is working well",
            analysis.win_rate
        ));
    }

    // Average duration recommendations
    if analysis.position_duration_stats.average_duration_hours > 24.0 {
        recommendations.push(
            "📅 Long average hold time - Consider implementing profit-taking at key levels"
                .to_string(),
        );
    } else if analysis.position_duration_stats.average_duration_hours < 1.0 {
        recommendations.push(
            "⚡ Very short hold times - Ensure you're not over-trading or missing bigger moves"
                .to_string(),
        );
    }

    // Pattern-based recommendations
    let high_confidence_patterns: Vec<_> = patterns
        .iter()
        .filter(|p| p.confidence_score > 0.7 && p.win_rate > 70.0)
        .collect();

    if !high_confidence_patterns.is_empty() {
        recommendations.push(format!(
            "🎯 {} high-confidence profitable patterns identified - Focus on these characteristics",
            high_confidence_patterns.len()
        ));

        for pattern in high_confidence_patterns {
            recommendations.push(format!(
                "  • {}: {:.1}% win rate with {:.3} SOL average profit",
                pattern.pattern_name, pattern.win_rate, pattern.average_pnl
            ));
        }
    }

    // Similarity recommendations
    let profitable_similarities = similarities.iter().filter(|s| s.both_profitable).count();

    if profitable_similarities > 0 {
        recommendations.push(
            format!("🔗 {} profitable token pairs show similar characteristics - Consider these patterns for future trades", profitable_similarities)
        );
    }

    // Risk management recommendations
    if let Some(worst) = &analysis.worst_position {
        if worst.pnl < -0.1 {
            // More than 0.1 SOL loss
            recommendations.push(format!(
                "🛡️  Largest loss: {:.3} SOL on {} - Review stop-loss strategy",
                worst.pnl, worst.symbol
            ));
        }
    }

    // Fees analysis
    let fee_ratio = analysis.total_fees / analysis.total_pnl.abs();
    if fee_ratio > 0.2 {
        recommendations.push(format!(
            "💰 Transaction fees are {:.1}% of P&L - Consider optimizing for larger position sizes",
            fee_ratio * 100.0
        ));
    }

    recommendations
}

/// Print comprehensive position analysis
fn print_position_analysis(analysis: &PositionAnalysis, _detailed: bool) {
    println!("📊 Position Analysis Summary");
    println!("──────────────────────────");
    println!("Total Positions: {}", analysis.total_positions);
    println!("  • Open: {}", analysis.open_positions);
    println!("  • Closed: {}", analysis.closed_positions);
    println!(
        "  • Profitable: {} ({:.1}%)",
        analysis.profitable_positions,
        if analysis.total_positions > 0 {
            ((analysis.profitable_positions as f64) / (analysis.total_positions as f64)) * 100.0
        } else {
            0.0
        }
    );
    println!("  • Losing: {}", analysis.losing_positions);
    println!();

    println!("💰 Financial Performance");
    println!("──────────────────────");
    println!("Total P&L: {:.6} SOL", analysis.total_pnl);
    println!("Total Fees: {:.6} SOL", analysis.total_fees);
    println!(
        "Net P&L: {:.6} SOL",
        analysis.total_pnl - analysis.total_fees
    );
    println!("Win Rate: {:.1}%", analysis.win_rate);
    println!("Average Profit: {:.6} SOL", analysis.average_profit);
    println!("Average Loss: {:.6} SOL", analysis.average_loss);
    println!();

    if let Some(best) = &analysis.best_position {
        println!(
            "🏆 Best Position: {} ({:.6} SOL, {:.1}%)",
            best.symbol, best.pnl, best.pnl_percentage
        );
    }

    if let Some(worst) = &analysis.worst_position {
        println!(
            "📉 Worst Position: {} ({:.6} SOL, {:.1}%)",
            worst.symbol, worst.pnl, worst.pnl_percentage
        );
    }
    println!();

    println!("⏰ Duration Statistics");
    println!("────────────────────");
    println!(
        "Average Duration: {:.1} hours",
        analysis.position_duration_stats.average_duration_hours
    );
    println!(
        "Median Duration: {:.1} hours",
        analysis.position_duration_stats.median_duration_hours
    );
    println!(
        "Shortest: {:.1} hours",
        analysis.position_duration_stats.shortest_duration_hours
    );
    println!(
        "Longest: {:.1} hours",
        analysis.position_duration_stats.longest_duration_hours
    );
    println!();

    if !analysis.performance_by_time.is_empty() {
        println!("📅 Performance by Time Period");
        println!("───────────────────────────");
        for perf in &analysis.performance_by_time {
            println!(
                "{}: {} positions, {:.6} SOL P&L, {:.1}% win rate",
                perf.period, perf.positions_count, perf.total_pnl, perf.win_rate
            );
        }
        println!();
    }
}

/// Print token similarity analysis
fn print_token_similarities(similarities: &[TokenSimilarity]) {
    if similarities.is_empty() {
        println!("No similar token pairs found above the threshold.");
        return;
    }

    println!("🔗 Similar Token Pairs");
    println!("────────────────────");

    for (i, sim) in similarities.iter().enumerate() {
        println!(
            "{}. {} ↔ {} (Similarity: {:.1}%)",
            i + 1,
            sim.symbol1,
            sim.symbol2,
            sim.similarity_score * 100.0
        );
        println!("   P&L: {:.6} SOL ↔ {:.6} SOL", sim.pnl1, sim.pnl2);
        println!("   Matching: {}", sim.matching_attributes.join(", "));
        println!("   {}", sim.recommendation);
        println!();
    }
}

/// Print profitable patterns analysis
fn print_profitable_patterns(patterns: &[ProfitablePattern]) {
    if patterns.is_empty() {
        println!("No significant profitable patterns found.");
        return;
    }

    println!("📈 Profitable Patterns");
    println!("────────────────────");

    for pattern in patterns {
        println!("🎯 {}", pattern.pattern_name);
        println!("   Description: {}", pattern.description);
        println!(
            "   Positions: {} | Win Rate: {:.1}% | Avg P&L: {:.6} SOL",
            pattern.positions_count, pattern.win_rate, pattern.average_pnl
        );
        println!("   Confidence: {:.1}%", pattern.confidence_score * 100.0);
        println!("   Examples: {}", pattern.example_tokens.join(", "));

        if !pattern.attributes.is_empty() {
            println!(
                "   Criteria: {}",
                pattern
                    .attributes
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!();
    }
}

/// Print trading recommendations
fn print_recommendations(recommendations: &[String]) {
    if recommendations.is_empty() {
        return;
    }

    println!("💡 Trading Recommendations");
    println!("─────────────────────────");

    for rec in recommendations {
        println!("{}", rec);
    }
    println!();
}

/// Export analysis results to JSON file
fn export_analysis_to_json(
    analysis: &PositionAnalysis,
    similarities: &[TokenSimilarity],
    patterns: &[ProfitablePattern],
    recommendations: &[String],
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct ExportData {
        analysis: PositionAnalysis,
        similarities: Vec<TokenSimilarity>,
        patterns: Vec<ProfitablePattern>,
        recommendations: Vec<String>,
        generated_at: DateTime<Utc>,
    }

    let export_data = ExportData {
        analysis: analysis.clone(),
        similarities: similarities.to_vec(),
        patterns: patterns.to_vec(),
        recommendations: recommendations.to_vec(),
        generated_at: Utc::now(),
    };

    let json_data = serde_json::to_string_pretty(&export_data)?;
    std::fs::write(filename, json_data)?;

    Ok(())
}
