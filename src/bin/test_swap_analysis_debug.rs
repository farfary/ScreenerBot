// bin/test_swap_analysis_debug.rs - Deep swap analysis and debugging tools
use screenerbot::transactions::*;
use screenerbot::global::read_configs;
use screenerbot::wallet::get_wallet_address;
use screenerbot::logger::{ log, LogTag };
use tokio::time::{ Duration, sleep, Instant };
use tabled::{ Tabled, Table, settings::{ Style, Modify, Alignment, object::Rows } };
use std::collections::HashMap;

/// Confidence analysis display
#[derive(Debug, Clone, Tabled)]
pub struct ConfidenceAnalysisDisplay {
    #[tabled(rename = "📝 Signature")]
    signature: String,
    #[tabled(rename = "🎯 Is Swap")]
    is_swap: String,
    #[tabled(rename = "📊 Confidence")]
    confidence_score: String,
    #[tabled(rename = "🔍 Detection Method")]
    detection_method: String,
    #[tabled(rename = "⏰ Block Time")]
    block_time: String,
}

/// Detailed swap breakdown
#[derive(Debug, Clone, Tabled)]
pub struct SwapBreakdownDisplay {
    #[tabled(rename = "🔢 Step")]
    step: String,
    #[tabled(rename = "📝 Description")]
    description: String,
    #[tabled(rename = "✅ Detected")]
    detected: String,
    #[tabled(rename = "📊 Score")]
    score: String,
}

/// Advanced swap comparison
#[derive(Debug, Clone, Tabled)]
pub struct SwapComparisonDisplay {
    #[tabled(rename = "📝 Signature")]
    signature: String,
    #[tabled(rename = "🔄 Traditional")]
    traditional_detection: String,
    #[tabled(rename = "🧠 Advanced")]
    advanced_detection: String,
    #[tabled(rename = "📊 Confidence")]
    confidence: String,
    #[tabled(rename = "🎯 Match")]
    methods_match: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "🔬 Starting Deep Swap Analysis & Debug");
    log(LogTag::System, "INFO", "==========================================");

    // Initialize system
    let configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address().map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>
    })?;

    log(LogTag::System, "INFO", &format!("🎯 Analyzing wallet: {}", wallet_address));

    let db = TransactionDatabase::new()?;
    let fetcher = TransactionFetcher::new(configs.clone(), None)?;
    let analyzer = TransactionAnalyzer::new();

    // Test 1: Fetch and analyze recent transactions
    log(LogTag::System, "INFO", "=== Test 1: Recent Transaction Analysis ===");
    let signatures = fetcher.get_recent_signatures(&wallet_address, 20).await?;
    let transactions = fetcher.batch_fetch_transactions(&signatures, Some(20)).await?;

    log(
        LogTag::System,
        "SUCCESS",
        &format!("📥 Loaded {} transactions for analysis", transactions.len())
    );

    // Test 2: Confidence-based swap detection
    log(LogTag::System, "INFO", "=== Test 2: Confidence-Based Detection ===");
    let mut confidence_results = Vec::new();

    for (sig_info, transaction) in &transactions {
        let (is_swap, confidence, reasons) = analyzer.analyze_swap_confidence(&transaction);

        confidence_results.push(ConfidenceAnalysisDisplay {
            signature: format!("{}...", &sig_info.signature[..12]),
            is_swap: (if is_swap { "✅ YES" } else { "❌ NO" }).to_string(),
            confidence_score: format!("{:.1}%", confidence),
            detection_method: (
                if confidence >= 70.0 {
                    "High Confidence"
                } else if confidence >= 50.0 {
                    "Medium Confidence"
                } else if confidence >= 25.0 {
                    "Low Confidence"
                } else {
                    "No Confidence"
                }
            ).to_string(),
            block_time: sig_info.block_time
                .map(|t| format_timestamp(Some(t)))
                .unwrap_or_else(|| "Unknown".to_string()),
        });
    }

    // Display confidence analysis
    if !confidence_results.is_empty() {
        let mut table = Table::new(confidence_results);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("\n🎯 Confidence-Based Swap Detection:");
        println!("{}", table);
    }

    // Test 3: Method comparison analysis
    log(LogTag::System, "INFO", "=== Test 3: Detection Method Comparison ===");
    let mut comparison_results = Vec::new();

    for (sig_info, transaction) in &transactions {
        let traditional_is_swap = analyzer.is_swap_transaction(&transaction);
        let advanced_swaps = analyzer.detect_swaps_advanced(&transaction);
        let advanced_is_swap = !advanced_swaps.is_empty();
        let (_, confidence, _) = analyzer.analyze_swap_confidence(&transaction);

        comparison_results.push(SwapComparisonDisplay {
            signature: format!("{}...", &sig_info.signature[..12]),
            traditional_detection: (if traditional_is_swap { "✅" } else { "❌" }).to_string(),
            advanced_detection: (if advanced_is_swap { "✅" } else { "❌" }).to_string(),
            confidence: format!("{:.1}%", confidence),
            methods_match: (
                if traditional_is_swap == advanced_is_swap {
                    "✅"
                } else {
                    "⚠️"
                }
            ).to_string(),
        });
    }

    // Display method comparison
    if !comparison_results.is_empty() {
        let mut table = Table::new(comparison_results);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("\n🔄 Detection Method Comparison:");
        println!("{}", table);
    }

    // Test 4: Deep dive into specific swap transactions
    log(LogTag::System, "INFO", "=== Test 4: Deep Dive Analysis ===");

    let swap_candidates: Vec<_> = transactions
        .iter()
        .filter(|(_, tx)| {
            let (is_swap, confidence, _) = analyzer.analyze_swap_confidence(tx);
            is_swap && confidence >= 50.0
        })
        .take(3)
        .collect();

    for (i, (sig_info, transaction)) in swap_candidates.iter().enumerate() {
        log(
            LogTag::System,
            "INFO",
            &format!("🔍 Deep Analysis #{}: {}", i + 1, &sig_info.signature[..12])
        );

        // Show confidence breakdown
        let (is_swap, confidence, reasons) = analyzer.analyze_swap_confidence(transaction);
        log(
            LogTag::System,
            "INFO",
            &format!("  📊 Overall Confidence: {:.1}% ({})", confidence, if is_swap {
                "SWAP"
            } else {
                "NOT SWAP"
            })
        );

        // Create breakdown display
        let mut breakdown = Vec::new();
        for (i, reason) in reasons.iter().enumerate() {
            breakdown.push(SwapBreakdownDisplay {
                step: format!("{}", i + 1),
                description: reason.clone(),
                detected: "✅".to_string(),
                score: "+".to_string(),
            });
        }

        if !breakdown.is_empty() {
            let mut table = Table::new(breakdown);
            table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::left()));
            println!("\n  📋 Confidence Breakdown:");
            println!("{}", table);
        }

        // Show advanced detection results
        let advanced_swaps = analyzer.detect_swaps_advanced(transaction);
        log(
            LogTag::System,
            "INFO",
            &format!("  🧠 Advanced Detection: {} swap(s) found", advanced_swaps.len())
        );

        for (j, swap) in advanced_swaps.iter().enumerate() {
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "    Swap #{}: {} via {}",
                    j + 1,
                    swap.swap_type,
                    swap.dex_name.as_deref().unwrap_or("Unknown")
                )
            );
        }

        // Show detailed transaction analysis
        let debug_info = analyzer.debug_transaction_analysis(transaction);
        println!("  📊 Detailed Analysis:");
        for line in debug_info.lines() {
            println!("    {}", line);
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    // Test 5: Performance benchmarking
    log(LogTag::System, "INFO", "=== Test 5: Performance Benchmarking ===");

    let mut traditional_times = Vec::new();
    let mut advanced_times = Vec::new();
    let mut confidence_times = Vec::new();

    for (_, transaction) in &transactions {
        // Benchmark traditional detection
        let start = Instant::now();
        let _ = analyzer.is_swap_transaction(&transaction);
        traditional_times.push(start.elapsed().as_micros() as f64);

        // Benchmark advanced detection
        let start = Instant::now();
        let _ = analyzer.detect_swaps_advanced(&transaction);
        advanced_times.push(start.elapsed().as_micros() as f64);

        // Benchmark confidence analysis
        let start = Instant::now();
        let _ = analyzer.analyze_swap_confidence(&transaction);
        confidence_times.push(start.elapsed().as_micros() as f64);
    }

    let avg_traditional = traditional_times.iter().sum::<f64>() / (traditional_times.len() as f64);
    let avg_advanced = advanced_times.iter().sum::<f64>() / (advanced_times.len() as f64);
    let avg_confidence = confidence_times.iter().sum::<f64>() / (confidence_times.len() as f64);

    log(LogTag::System, "SUCCESS", "⚡ Performance Results:");
    log(
        LogTag::System,
        "SUCCESS",
        &format!("  Traditional Detection: {:.2}μs avg", avg_traditional)
    );
    log(LogTag::System, "SUCCESS", &format!("  Advanced Detection: {:.2}μs avg", avg_advanced));
    log(LogTag::System, "SUCCESS", &format!("  Confidence Analysis: {:.2}μs avg", avg_confidence));

    // Test 6: Accuracy validation
    log(LogTag::System, "INFO", "=== Test 6: Accuracy Validation ===");

    let mut traditional_swaps = 0;
    let mut advanced_swaps = 0;
    let mut confidence_swaps = 0;
    let mut method_agreements = 0;

    for (_, transaction) in &transactions {
        let traditional = analyzer.is_swap_transaction(&transaction);
        let advanced = !analyzer.detect_swaps_advanced(&transaction).is_empty();
        let (confidence_result, confidence_score, _) = analyzer.analyze_swap_confidence(
            &transaction
        );

        if traditional {
            traditional_swaps += 1;
        }
        if advanced {
            advanced_swaps += 1;
        }
        if confidence_result {
            confidence_swaps += 1;
        }

        if traditional == advanced && advanced == confidence_result {
            method_agreements += 1;
        }
    }

    let total = transactions.len();
    let agreement_rate = ((method_agreements as f64) / (total as f64)) * 100.0;

    log(LogTag::System, "SUCCESS", "🎯 Accuracy Validation:");
    log(LogTag::System, "SUCCESS", &format!("  Traditional Method: {} swaps", traditional_swaps));
    log(LogTag::System, "SUCCESS", &format!("  Advanced Method: {} swaps", advanced_swaps));
    log(LogTag::System, "SUCCESS", &format!("  Confidence Method: {} swaps", confidence_swaps));
    log(
        LogTag::System,
        "SUCCESS",
        &format!("  Method Agreement: {:.1}% ({}/{})", agreement_rate, method_agreements, total)
    );

    log(LogTag::System, "SUCCESS", "🎉 Deep swap analysis completed successfully!");

    Ok(())
}
