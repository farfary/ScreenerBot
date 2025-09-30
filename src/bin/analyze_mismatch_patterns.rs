use clap::{ Arg, Command };
use std::collections::HashMap;

/// Simplified CSV verification analysis tool
/// Analyzes mismatches in verification results and identifies patterns
/// without requiring complex transaction processing

#[derive(Debug)]
struct MismatchRecord {
    pub signature: String,
    pub router: String,
    pub percentage_diff: f64,
    pub calculated_amount: f64,
    pub verified_amount: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Mismatch Pattern Analyzer")
        .about("Analyzes CSV verification mismatches and identifies patterns")
        .arg(
            Arg::new("csv_file")
                .long("csv-file")
                .value_name("FILE")
                .help("Path to the CSV verification results file")
                .default_value("analysis-exports/verification_results.csv")
        )
        .arg(
            Arg::new("threshold")
                .long("threshold")
                .value_name("PERCENT")
                .help("Minimum percentage difference to consider (default 0.5%)")
                .default_value("0.5")
        )
        .get_matches();

    let csv_file = matches.get_one::<String>("csv_file").unwrap();
    let threshold: f64 = matches.get_one::<String>("threshold").unwrap().parse()?;

    println!("🔍 Analyzing verification mismatches from: {}", csv_file);
    println!("📊 Threshold: {}%", threshold);

    if !std::path::Path::new(csv_file).exists() {
        return Err(format!("CSV file not found: {}", csv_file).into());
    }

    let mismatches = load_mismatches(csv_file, threshold)?;
    analyze_patterns(&mismatches)?;

    Ok(())
}

fn load_mismatches(
    csv_file: &str,
    threshold: f64
) -> Result<Vec<MismatchRecord>, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::{ BufRead, BufReader };

    let file = File::open(csv_file)?;
    let reader = BufReader::new(file);
    let mut mismatches = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;

        // Skip header
        if line_num == 0 {
            continue;
        }

        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 6 {
            continue;
        }

        // Parse CSV fields: signature, calculated_amount, verified_amount, percentage_diff, router, match_status
        let signature = fields[0].to_string();
        let calculated_amount: f64 = fields[1].parse().unwrap_or(0.0);
        let verified_amount: f64 = fields[2].parse().unwrap_or(0.0);
        let percentage_diff: f64 = fields[3].parse().unwrap_or(0.0);
        let router = fields[4].to_string();

        // Only include mismatches above threshold
        if percentage_diff.abs() >= threshold {
            mismatches.push(MismatchRecord {
                signature,
                router,
                percentage_diff,
                calculated_amount,
                verified_amount,
            });
        }
    }

    println!("📋 Found {} mismatches above {}% threshold", mismatches.len(), threshold);
    Ok(mismatches)
}

fn analyze_patterns(mismatches: &[MismatchRecord]) -> Result<(), Box<dyn std::error::Error>> {
    if mismatches.is_empty() {
        println!("✅ No significant mismatches found!");
        return Ok(());
    }

    // Group by router
    let mut router_stats: HashMap<String, Vec<&MismatchRecord>> = HashMap::new();
    for mismatch in mismatches {
        router_stats.entry(mismatch.router.clone()).or_default().push(mismatch);
    }

    println!("\n📊 Mismatch Analysis by Router:");
    println!("================================");

    for (router, records) in &router_stats {
        println!("\n🔗 Router: {}", router);
        println!("   Count: {}", records.len());

        let total_diff: f64 = records
            .iter()
            .map(|r| r.percentage_diff.abs())
            .sum();
        let avg_diff = total_diff / (records.len() as f64);
        let max_diff: f64 = records
            .iter()
            .map(|r| r.percentage_diff.abs())
            .fold(0.0f64, |a, b| a.max(b));

        println!("   Average difference: {:.2}%", avg_diff);
        println!("   Maximum difference: {:.2}%", max_diff);

        // Analyze amount ranges
        let amounts: Vec<f64> = records
            .iter()
            .map(|r| r.calculated_amount)
            .collect();
        if !amounts.is_empty() {
            let min_amount = amounts.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_amount = amounts.iter().fold(0.0f64, |a, &b| a.max(b));
            println!("   Amount range: {:.6} - {:.6} SOL", min_amount, max_amount);
        }

        // Show worst cases
        let mut sorted = records.clone();
        sorted.sort_by(|a, b|
            b.percentage_diff.abs().partial_cmp(&a.percentage_diff.abs()).unwrap()
        );

        println!("   Worst mismatches:");
        for (i, record) in sorted.iter().take(3).enumerate() {
            println!(
                "     {}. {:.2}% diff - {:.6} vs {:.6} SOL ({}...)",
                i + 1,
                record.percentage_diff,
                record.calculated_amount,
                record.verified_amount,
                &record.signature[..8]
            );
        }
    }

    // Overall statistics
    println!("\n📈 Overall Statistics:");
    println!("======================");
    let total_diff: f64 = mismatches
        .iter()
        .map(|r| r.percentage_diff.abs())
        .sum();
    let overall_avg = total_diff / (mismatches.len() as f64);
    let overall_max: f64 = mismatches
        .iter()
        .map(|r| r.percentage_diff.abs())
        .fold(0.0f64, |a, b| a.max(b));

    println!("Total mismatches: {}", mismatches.len());
    println!("Average difference: {:.2}%", overall_avg);
    println!("Maximum difference: {:.2}%", overall_max);

    // Percentage distribution
    let mut ranges = [0; 5]; // 0.5-1%, 1-2%, 2-5%, 5-10%, >10%
    for mismatch in mismatches {
        let abs_diff = mismatch.percentage_diff.abs();
        if abs_diff < 1.0 {
            ranges[0] += 1;
        } else if abs_diff < 2.0 {
            ranges[1] += 1;
        } else if abs_diff < 5.0 {
            ranges[2] += 1;
        } else if abs_diff < 10.0 {
            ranges[3] += 1;
        } else {
            ranges[4] += 1;
        }
    }

    println!("\n📊 Distribution by percentage difference:");
    println!("0.5-1%:   {} mismatches", ranges[0]);
    println!("1-2%:     {} mismatches", ranges[1]);
    println!("2-5%:     {} mismatches", ranges[2]);
    println!("5-10%:    {} mismatches", ranges[3]);
    println!(">10%:     {} mismatches", ranges[4]);

    // Generate intelligent pattern suggestions
    println!("\n💡 Intelligent Correction Suggestions:");
    println!("======================================");

    for (router, records) in &router_stats {
        if records.len() >= 3 {
            let avg_adjustment =
                records
                    .iter()
                    .map(|r| r.percentage_diff)
                    .sum::<f64>() / (records.len() as f64);
            let is_consistent = records
                .iter()
                .all(|r| r.percentage_diff.signum() == avg_adjustment.signum());

            if is_consistent && avg_adjustment.abs() > 0.1 {
                let adjustment_type = if avg_adjustment > 0.0 { "increase" } else { "decrease" };
                let pattern_confidence = if avg_adjustment.abs() < 2.0 { "High" } else { "Medium" };

                println!(
                    "🎯 {}: Apply {:.2}% {} (Confidence: {}, {} samples)",
                    router,
                    avg_adjustment.abs(),
                    adjustment_type,
                    pattern_confidence,
                    records.len()
                );

                // Show implementation suggestion
                println!(
                    "   💻 Suggested implementation: percentage-based adjustment of {:.3}",
                    1.0 + avg_adjustment / 100.0
                );
            }
        }
    }

    Ok(())
}
