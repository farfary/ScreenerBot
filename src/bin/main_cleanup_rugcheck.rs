/// Cleanup tool to remove orphaned rugcheck data for tokens that no longer exist
/// This tool removes rugcheck entries for tokens that have been deleted from the tokens table

use screenerbot::logger::{ init_file_logging, log, LogTag };
use screenerbot::global::TOKENS_DATABASE;
use rusqlite::{ Connection, params };
use std::sync::{ Arc, Mutex };

struct RugcheckCleaner {
    connection: Arc<Mutex<Connection>>,
}

impl RugcheckCleaner {
    /// Create new rugcheck cleaner instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::open(TOKENS_DATABASE)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Get count of orphaned rugcheck entries
    pub fn get_orphaned_rugcheck_count(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let connection = self.connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "SELECT COUNT(*) FROM rugcheck_data r WHERE NOT EXISTS (SELECT 1 FROM tokens t WHERE t.mint = r.mint)"
        )?;

        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    /// Get some examples of orphaned rugcheck entries
    pub fn get_orphaned_rugcheck_examples(
        &self,
        limit: i64
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let connection = self.connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "SELECT r.mint FROM rugcheck_data r WHERE NOT EXISTS (SELECT 1 FROM tokens t WHERE t.mint = r.mint) LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit], |row| { Ok(row.get::<usize, String>(0)?) })?;

        let mut examples = Vec::new();
        for row in rows {
            examples.push(row?);
        }

        Ok(examples)
    }

    /// Remove all orphaned rugcheck entries
    pub fn cleanup_orphaned_rugcheck_data(
        &self,
        dry_run: bool
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let connection = self.connection.lock().unwrap();

        if dry_run {
            log(
                LogTag::System,
                "DRY_RUN",
                "Would execute: DELETE FROM rugcheck_data WHERE NOT EXISTS (SELECT 1 FROM tokens WHERE tokens.mint = rugcheck_data.mint)"
            );
            // Just return the count for dry run
            let mut stmt = connection.prepare(
                "SELECT COUNT(*) FROM rugcheck_data r WHERE NOT EXISTS (SELECT 1 FROM tokens t WHERE t.mint = r.mint)"
            )?;
            let count: i64 = stmt.query_row([], |row| row.get(0))?;
            return Ok(count);
        }

        let affected_rows = connection.execute(
            "DELETE FROM rugcheck_data WHERE NOT EXISTS (SELECT 1 FROM tokens WHERE tokens.mint = rugcheck_data.mint)",
            []
        )?;

        Ok(affected_rows as i64)
    }

    /// Get database statistics after cleanup
    pub fn get_cleanup_stats(&self) -> Result<(i64, i64), Box<dyn std::error::Error>> {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection.prepare("SELECT COUNT(*) FROM tokens")?;
        let tokens_count: i64 = stmt.query_row([], |row| row.get(0))?;

        let mut stmt = connection.prepare("SELECT COUNT(*) FROM rugcheck_data")?;
        let rugcheck_count: i64 = stmt.query_row([], |row| row.get(0))?;

        Ok((tokens_count, rugcheck_count))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let _ = init_file_logging();

    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string());
    let show_examples = args.contains(&"--show-examples".to_string());
    let force_cleanup = args.contains(&"--force-cleanup".to_string());

    if args.contains(&"--help".to_string()) {
        println!("Rugcheck Cleanup Tool");
        println!();
        println!("Usage: {} [OPTIONS]", args[0]);
        println!();
        println!("Options:");
        println!("  --dry-run         Show what would be deleted without actually deleting");
        println!("  --show-examples   Show examples of orphaned rugcheck entries");
        println!("  --force-cleanup   Actually perform the cleanup (required for real execution)");
        println!("  --help           Show this help message");
        println!();
        println!("Examples:");
        println!("  {} --dry-run", args[0]);
        println!("  {} --show-examples", args[0]);
        println!("  {} --force-cleanup", args[0]);
        return Ok(());
    }

    log(LogTag::System, "START", "Starting rugcheck cleanup tool");

    let cleaner = RugcheckCleaner::new()?;

    // Get initial stats
    let (tokens_count, rugcheck_count) = cleaner.get_cleanup_stats()?;
    let orphaned_count = cleaner.get_orphaned_rugcheck_count()?;

    log(
        LogTag::System,
        "STATS",
        &format!(
            "Database stats: {} tokens, {} rugcheck entries, {} orphaned rugcheck entries",
            tokens_count,
            rugcheck_count,
            orphaned_count
        )
    );

    if orphaned_count == 0 {
        log(LogTag::System, "COMPLETE", "No orphaned rugcheck entries found, nothing to clean up");
        return Ok(());
    }

    if show_examples {
        log(LogTag::System, "EXAMPLES", "Fetching examples of orphaned rugcheck entries...");
        let examples = cleaner.get_orphaned_rugcheck_examples(10)?;
        log(
            LogTag::System,
            "EXAMPLES",
            &format!("First 10 orphaned rugcheck entries: {:?}", examples)
        );
    }

    if dry_run {
        let would_delete = cleaner.cleanup_orphaned_rugcheck_data(true)?;
        log(
            LogTag::System,
            "DRY_RUN",
            &format!("Would delete {} orphaned rugcheck entries", would_delete)
        );
        log(LogTag::System, "DRY_RUN", "Use --force-cleanup to actually perform the deletion");
        return Ok(());
    }

    if force_cleanup {
        log(
            LogTag::System,
            "CLEANUP",
            &format!("Starting cleanup of {} orphaned rugcheck entries...", orphaned_count)
        );

        let start_time = std::time::Instant::now();
        let deleted_count = cleaner.cleanup_orphaned_rugcheck_data(false)?;
        let duration = start_time.elapsed();

        log(
            LogTag::System,
            "CLEANUP",
            &format!(
                "Deleted {} orphaned rugcheck entries in {:.2} seconds",
                deleted_count,
                duration.as_secs_f64()
            )
        );

        // Get final stats
        let (final_tokens_count, final_rugcheck_count) = cleaner.get_cleanup_stats()?;
        log(
            LogTag::System,
            "FINAL_STATS",
            &format!(
                "Final database stats: {} tokens, {} rugcheck entries",
                final_tokens_count,
                final_rugcheck_count
            )
        );

        log(LogTag::System, "COMPLETE", "Rugcheck cleanup completed successfully");
    } else {
        log(LogTag::System, "INFO", "Use --dry-run to see what would be deleted");
        log(LogTag::System, "INFO", "Use --force-cleanup to actually perform the deletion");
        log(LogTag::System, "INFO", "Use --show-examples to see examples of orphaned entries");
    }

    Ok(())
}
