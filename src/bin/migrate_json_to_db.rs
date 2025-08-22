/// JSON to Database Migration Utility
///
/// Migrates existing JSON transaction files to the new SQLite database system.
/// This tool provides a safe way to transition from file-based caching to database-backed storage
/// with comprehensive validation, error handling, and progress reporting.
///
/// Usage: cargo run --bin migrate_json_to_db

use std::fs;
use std::path::Path;
use serde_json;
use tokio::time::Instant;

use screenerbot::{
    transactions::Transaction,
    transactions_db::TransactionDatabase,
    global::get_transactions_cache_dir,
};

#[derive(Default)]
struct MigrationStats {
    total_files: usize,
    successful_migrations: usize,
    failed_migrations: usize,
    skipped_files: usize,
    errors: Vec<String>,
}

impl MigrationStats {
    fn report(&self, elapsed: tokio::time::Duration) {
        println!("\n🔄 MIGRATION COMPLETE");
        println!("╭─────────────────────────────────╮");
        println!("│           SUMMARY               │");
        println!("├─────────────────────────────────┤");
        println!("│ Total JSON files: {:>13} │", self.total_files);
        println!("│ Successfully migrated: {:>8} │", self.successful_migrations);
        println!("│ Failed migrations: {:>12} │", self.failed_migrations);
        println!("│ Skipped files: {:>16} │", self.skipped_files);
        println!("│ Elapsed time: {:>15.2}s │", elapsed.as_secs_f64());
        println!("╰─────────────────────────────────╯");

        if !self.errors.is_empty() {
            println!("\n⚠️  ERRORS ENCOUNTERED:");
            for (i, error) in self.errors.iter().enumerate() {
                if i < 5 {
                    // Show first 5 errors
                    println!("   • {}", error);
                } else if i == 5 {
                    println!("   • ... and {} more errors", self.errors.len() - 5);
                    break;
                }
            }
        }

        let success_rate = if self.total_files > 0 {
            ((self.successful_migrations as f64) / (self.total_files as f64)) * 100.0
        } else {
            0.0
        };

        println!("\n📊 Success Rate: {:.1}%", success_rate);

        if self.failed_migrations == 0 {
            println!("✅ All migrations completed successfully!");
        } else if success_rate >= 95.0 {
            println!("✅ Migration mostly successful with minor issues");
        } else if success_rate >= 80.0 {
            println!("⚠️  Migration completed with some issues");
        } else {
            println!("❌ Migration completed with significant issues");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 JSON to Database Migration Utility");
    println!("=====================================\n");

    // Initialize database
    println!("📅 Initializing SQLite database...");
    let database = match TransactionDatabase::new().await {
        Ok(db) => {
            println!("✅ Database initialized successfully");
            db
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return Err(e.into());
        }
    };

    // Get transaction cache directory
    let cache_dir = get_transactions_cache_dir();
    println!("📁 Scanning cache directory: {}", cache_dir.display());

    if !Path::new(&cache_dir).exists() {
        println!("⚠️  Cache directory does not exist. Nothing to migrate.");
        return Ok(());
    }

    let start_time = Instant::now();
    let mut stats = MigrationStats::default();

    // Read all JSON files
    let entries = fs::read_dir(&cache_dir)?;
    let json_files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map_or(false, |name| name.ends_with(".json"))
        })
        .collect();

    stats.total_files = json_files.len();

    if stats.total_files == 0 {
        println!("⚠️  No JSON transaction files found. Nothing to migrate.");
        return Ok(());
    }

    println!("📝 Found {} JSON transaction files to migrate", stats.total_files);
    println!("🔄 Starting migration...\n");

    // Process each JSON file
    for (index, entry) in json_files.iter().enumerate() {
        let file_path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_str().unwrap_or("unknown");

        // Show progress
        if index % 100 == 0 || index == json_files.len() - 1 {
            println!(
                "📈 Progress: {}/{} files processed ({:.1}%)",
                index + 1,
                stats.total_files,
                (((index + 1) as f64) / (stats.total_files as f64)) * 100.0
            );
        }

        // Extract signature from filename
        let signature = file_name_str.replace(".json", "");

        // Check if already exists in database
        if let Ok(true) = database.is_signature_known(&signature).await {
            stats.skipped_files += 1;
            continue;
        }

        // Read and parse JSON file
        let json_content = match fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(e) => {
                let error = format!("Failed to read {}: {}", file_name_str, e);
                stats.errors.push(error);
                stats.failed_migrations += 1;
                continue;
            }
        };

        let transaction: Transaction = match serde_json::from_str(&json_content) {
            Ok(tx) => tx,
            Err(e) => {
                let error = format!("Failed to parse {}: {}", file_name_str, e);
                stats.errors.push(error);
                stats.failed_migrations += 1;
                continue;
            }
        };

        // Migrate raw transaction data to database
        let status_string = match &transaction.status {
            screenerbot::transactions::TransactionStatus::Pending => "Pending",
            screenerbot::transactions::TransactionStatus::Confirmed => "Confirmed",
            screenerbot::transactions::TransactionStatus::Finalized => "Finalized",
            screenerbot::transactions::TransactionStatus::Failed(_) => "Failed",
        };

        let raw_data_string = if let Some(ref raw_data) = transaction.raw_transaction_data {
            match serde_json::to_string(raw_data) {
                Ok(s) => Some(s),
                Err(e) => {
                    let error = format!(
                        "Failed to serialize raw data for {}: {}",
                        file_name_str,
                        e
                    );
                    stats.errors.push(error);
                    None
                }
            }
        } else {
            None
        };

        // Store raw transaction
        if
            let Err(e) = database.store_raw_transaction(
                &transaction.signature,
                transaction.slot,
                transaction.block_time,
                &transaction.timestamp,
                status_string,
                transaction.success,
                transaction.error_message.as_deref(),
                raw_data_string.as_deref()
            ).await
        {
            let error = format!("Failed to store raw transaction {}: {}", file_name_str, e);
            stats.errors.push(error);
            stats.failed_migrations += 1;
            continue;
        }

        // Add to known signatures
        if let Err(e) = database.add_known_signature(&transaction.signature).await {
            let error = format!("Failed to add known signature {}: {}", file_name_str, e);
            stats.errors.push(error);
            // Don't fail the migration for this, continue
        }

        stats.successful_migrations += 1;
    }

    let elapsed = start_time.elapsed();

    // Print detailed migration report
    stats.report(elapsed);

    // Get database statistics
    println!("\n📊 DATABASE STATISTICS:");
    match database.get_database_stats().await {
        Ok(db_stats) => {
            println!("   Raw transactions: {}", db_stats.total_raw_transactions);
            println!("   Known signatures: {}", db_stats.total_known_signatures);
            println!(
                "   Database size: {:.2} MB",
                (db_stats.database_size_bytes as f64) / 1_048_576.0
            );
        }
        Err(e) => {
            println!("   Failed to get database statistics: {}", e);
        }
    }

    // Optimize database after migration
    println!("\n🔧 Optimizing database...");
    if let Err(e) = database.vacuum_database().await {
        println!("⚠️  Failed to vacuum database: {}", e);
    }
    if let Err(e) = database.analyze_database().await {
        println!("⚠️  Failed to analyze database: {}", e);
    } else {
        println!("✅ Database optimization complete");
    }

    // Provide next steps
    println!("\n🎯 NEXT STEPS:");
    if stats.successful_migrations > 0 {
        println!("1. 🧪 Test the database integration with your transaction processing");
        println!("2. 🚀 Once confident, you can remove the old JSON files");
        println!("3. 📈 Monitor performance improvements (expect 10x faster signature lookups)");

        if stats.failed_migrations > 0 {
            println!("4. ⚠️  Review failed migrations and fix any issues");
            println!("5. 🔄 Re-run migration to process failed files");
        }
    } else {
        println!("1. ❌ No files were migrated successfully");
        println!("2. 🔍 Review error messages above and fix issues");
        println!("3. 🔄 Re-run migration after fixing problems");
    }

    Ok(())
}
