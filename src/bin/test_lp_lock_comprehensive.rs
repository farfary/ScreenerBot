/// Comprehensive LP Lock Verification Test Suite
/// Tests various token types, DEX platforms, and edge cases
/// Usage: cargo run --bin test_lp_lock_comprehensive

use screenerbot::tokens::lp_lock::{ check_lp_lock_status, LpLockStatus, LpLockAnalysis };
use screenerbot::tokens::dexscreener::init_dexscreener_api;
use std::time::{ Duration, Instant };

#[derive(Debug, Clone)]
struct TestCase {
    name: String,
    token: String,
    expected_category: ExpectedCategory,
    description: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ExpectedCategory {
    Burned, // LP tokens burned
    ProgramLocked, // LP tokens locked in program
    NotLocked, // LP tokens not locked
    BondingCurve, // PumpFun bonding curve (no LP tokens)
    Unknown, // Unable to determine
    NoPool, // No pools found
}

impl ExpectedCategory {
    fn matches_status(&self, status: &LpLockStatus) -> bool {
        match (self, status) {
            (ExpectedCategory::Burned, LpLockStatus::Burned) => true,
            (ExpectedCategory::ProgramLocked, LpLockStatus::ProgramLocked { .. }) => true,
            (ExpectedCategory::ProgramLocked, LpLockStatus::TimeLocked { .. }) => true,
            (ExpectedCategory::NotLocked, LpLockStatus::NotLocked { .. }) => true,
            (ExpectedCategory::NotLocked, LpLockStatus::CreatorHeld) => true,
            (ExpectedCategory::BondingCurve, LpLockStatus::NotLocked { .. }) => true,
            (ExpectedCategory::Unknown, LpLockStatus::Unknown) => true,
            (ExpectedCategory::NoPool, LpLockStatus::NoPool) => true,
            _ => false,
        }
    }
}

fn get_test_cases() -> Vec<TestCase> {
    vec![
        // PumpFun Bonding Curves
        TestCase {
            name: "PumpFun Bonding Curve #1".to_string(),
            token: "4CjyetevoeK2u4aUjuVbccbYmx6VqV3Qgqw88VqRPGgk".to_string(),
            expected_category: ExpectedCategory::BondingCurve,
            description: "PumpFun token with bonding curve - no LP tokens".to_string(),
        },

        // Well-known tokens with different LP states
        TestCase {
            name: "SOL (Native)".to_string(),
            token: "So11111111111111111111111111111111111111112".to_string(),
            expected_category: ExpectedCategory::NotLocked, // SOL doesn't have LP locks
            description: "Native SOL token - multiple pools".to_string(),
        },

        TestCase {
            name: "USDC (Circle)".to_string(),
            token: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            expected_category: ExpectedCategory::NotLocked, // USDC is centralized
            description: "USDC stablecoin - multiple major pools".to_string(),
        },

        TestCase {
            name: "BONK (Popular Meme)".to_string(),
            token: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
            expected_category: ExpectedCategory::Burned, // BONK LP likely burned
            description: "BONK meme token - should have burned LP".to_string(),
        },

        TestCase {
            name: "WIF (Meme Token)".to_string(),
            token: "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm".to_string(),
            expected_category: ExpectedCategory::Unknown, // Need to check
            description: "dogwifhat meme token - check LP status".to_string(),
        },

        TestCase {
            name: "JUP (Jupiter Token)".to_string(),
            token: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".to_string(),
            expected_category: ExpectedCategory::Unknown, // Need to check
            description: "Jupiter governance token".to_string(),
        },

        // Edge Cases
        TestCase {
            name: "Invalid Token".to_string(),
            token: "InvalidTokenAddress123".to_string(),
            expected_category: ExpectedCategory::NoPool,
            description: "Invalid token address should return NoPool".to_string(),
        },

        TestCase {
            name: "Non-existent Token".to_string(),
            token: "11111111111111111111111111111111111111113".to_string(), // Valid format but non-existent
            expected_category: ExpectedCategory::NoPool,
            description: "Valid format but non-existent token".to_string(),
        },

        // Recently created tokens (might be risky)
        TestCase {
            name: "Random Recent Token #1".to_string(),
            token: "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump".to_string(),
            expected_category: ExpectedCategory::Unknown, // Recent token
            description: "Recent token - check if pools exist".to_string(),
        }
    ]
}

struct TestResult {
    test_case: TestCase,
    actual_status: Result<LpLockAnalysis, String>,
    execution_time: Duration,
    matches_expectation: bool,
    notes: Vec<String>,
}

impl TestResult {
    fn new(
        test_case: TestCase,
        actual_status: Result<LpLockAnalysis, String>,
        execution_time: Duration
    ) -> Self {
        let matches_expectation = match &actual_status {
            Ok(analysis) => test_case.expected_category.matches_status(&analysis.status),
            Err(_) => test_case.expected_category == ExpectedCategory::NoPool,
        };

        TestResult {
            test_case,
            actual_status,
            execution_time,
            matches_expectation,
            notes: Vec::new(),
        }
    }

    fn add_note(&mut self, note: String) {
        self.notes.push(note);
    }
}

#[tokio::main]
async fn main() {
    println!("🧪 Comprehensive LP Lock Verification Test Suite");
    println!("Testing various token types, DEX platforms, and edge cases");
    println!("{}", "=".repeat(80));

    // Initialize DexScreener API
    match init_dexscreener_api().await {
        Ok(_) => println!("✅ DexScreener API initialized successfully"),
        Err(e) => {
            println!("❌ Failed to initialize DexScreener API: {}", e);
            println!("Continuing with tests anyway...");
        }
    }
    println!();

    let test_cases = get_test_cases();
    let mut results = Vec::new();
    let mut passed_tests = 0;
    let mut failed_tests = 0;
    let total_tests = test_cases.len();

    println!("🚀 Running {} test cases...", total_tests);
    println!();

    for (i, test_case) in test_cases.into_iter().enumerate() {
        println!("Test {}/{}: {}", i + 1, total_tests, test_case.name);
        println!("  Token: {}", test_case.token);
        println!("  Expected: {:?}", test_case.expected_category);
        println!("  Description: {}", test_case.description);

        let start_time = Instant::now();
        let result = check_lp_lock_status(&test_case.token).await;
        let execution_time = start_time.elapsed();

        let mut test_result = TestResult::new(test_case, result, execution_time);

        match &test_result.actual_status {
            Ok(analysis) => {
                let status = &analysis.status;
                println!("  ✅ Result: {:?}", status);
                println!("  📊 Description: {}", status.description());
                println!("  🔒 Lock Score: {}/100", analysis.lock_score);
                println!("  ⚡ Execution time: {:?}", execution_time);

                if test_result.matches_expectation {
                    println!("  ✅ PASS - Matches expectation");
                    passed_tests += 1;
                } else {
                    println!("  ❌ FAIL - Does not match expectation");
                    failed_tests += 1;
                    test_result.add_note(
                        format!(
                            "Expected {:?} but got {:?}",
                            test_result.test_case.expected_category,
                            &analysis.status
                        )
                    );
                }
            }
            Err(e) => {
                println!("  ❌ Error: {}", e);
                println!("  ⚡ Execution time: {:?}", execution_time);

                if test_result.matches_expectation {
                    println!("  ✅ PASS - Error was expected for this case");
                    passed_tests += 1;
                } else {
                    println!("  ❌ FAIL - Unexpected error");
                    failed_tests += 1;
                    test_result.add_note(format!("Unexpected error: {}", e));
                }
            }
        }

        results.push(test_result);
        println!();

        // Small delay to avoid rate limiting
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Generate comprehensive report
    generate_test_report(&results, passed_tests, failed_tests, total_tests).await;
}

async fn generate_test_report(
    results: &[TestResult],
    passed_tests: usize,
    failed_tests: usize,
    total_tests: usize
) {
    println!("{}", "=".repeat(80));
    println!("📊 TEST REPORT SUMMARY");
    println!("{}", "=".repeat(80));

    println!("Overall Results:");
    println!(
        "  ✅ Passed: {}/{} ({:.1}%)",
        passed_tests,
        total_tests,
        ((passed_tests as f64) / (total_tests as f64)) * 100.0
    );
    println!(
        "  ❌ Failed: {}/{} ({:.1}%)",
        failed_tests,
        total_tests,
        ((failed_tests as f64) / (total_tests as f64)) * 100.0
    );
    println!();

    // Performance analysis
    let total_time: Duration = results
        .iter()
        .map(|r| r.execution_time)
        .sum();
    let avg_time = total_time / (total_tests as u32);
    let fastest = results
        .iter()
        .min_by_key(|r| r.execution_time)
        .unwrap();
    let slowest = results
        .iter()
        .max_by_key(|r| r.execution_time)
        .unwrap();

    println!("⚡ Performance Analysis:");
    println!("  Total execution time: {:?}", total_time);
    println!("  Average execution time: {:?}", avg_time);
    println!("  Fastest test: {} ({:?})", fastest.test_case.name, fastest.execution_time);
    println!("  Slowest test: {} ({:?})", slowest.test_case.name, slowest.execution_time);
    println!();

    // Status distribution
    let mut status_counts = std::collections::HashMap::new();
    for result in results {
        if let Ok(analysis) = &result.actual_status {
            let status_name = match &analysis.status {
                LpLockStatus::Burned => "Burned",
                LpLockStatus::ProgramLocked { .. } => "ProgramLocked",
                LpLockStatus::TimeLocked { .. } => "TimeLocked",
                LpLockStatus::Locked { .. } => "Locked",
                LpLockStatus::NotLocked { .. } => "NotLocked",
                LpLockStatus::CreatorHeld => "CreatorHeld",
                LpLockStatus::Unknown => "Unknown",
                LpLockStatus::NoPool => "NoPool",
            };
            *status_counts.entry(status_name).or_insert(0) += 1;
        } else {
            *status_counts.entry("Error").or_insert(0) += 1;
        }
    }

    println!("📈 Status Distribution:");
    for (status, count) in status_counts {
        println!("  {}: {} tokens", status, count);
    }
    println!();

    // Failed tests details
    if failed_tests > 0 {
        println!("❌ Failed Tests Details:");
        for result in results {
            if !result.matches_expectation {
                println!("  • {}", result.test_case.name);
                println!("    Token: {}", result.test_case.token);
                for note in &result.notes {
                    println!("    Note: {}", note);
                }
            }
        }
        println!();
    }

    // Recommendations
    println!("💡 Recommendations:");

    let success_rate = (passed_tests as f64) / (total_tests as f64);
    if success_rate < 0.8 {
        println!("  ⚠️  Test success rate is below 80% - review failed cases");
    } else {
        println!("  ✅ Good test success rate");
    }

    if avg_time > Duration::from_secs(2) {
        println!("  ⚠️  Average execution time is high - consider caching improvements");
    } else {
        println!("  ✅ Good performance - average execution time is acceptable");
    }

    println!("  🔍 Consider adding more edge cases for comprehensive coverage");
    println!("  📊 Monitor real-world usage to identify additional test scenarios");

    println!();
    println!("🏁 Test suite completed!");
}
