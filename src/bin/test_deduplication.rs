use screenerbot::global::LIST_MINTS;
use std::collections::HashSet;

fn main() {
    println!("Testing LIST_MINTS deduplication...");

    // Add some test mints with potential duplicates
    {
        let mut mints = LIST_MINTS.write().unwrap();
        mints.insert("mint1".to_string());
        mints.insert("mint2".to_string());
        mints.insert("mint3".to_string());
        mints.insert("mint1".to_string()); // Duplicate - should be ignored by HashSet
        mints.insert("mint2".to_string()); // Duplicate - should be ignored by HashSet
        mints.insert("mint4".to_string());
    }

    // Check initial count
    let initial_count = {
        let mints = LIST_MINTS.read().unwrap();
        println!("Initial mints count: {}", mints.len());
        mints.len()
    };

    // Test deduplication function (similar to what we added in monitor.rs)
    let deduplicated_count = deduplicate_list_mints();

    match deduplicated_count {
        Ok(count) => {
            println!("Deduplication successful, final count: {}", count);
            if count == initial_count {
                println!("✅ No duplicates found (expected for HashSet)");
            } else {
                println!("⚠️  Unexpected count change: {} -> {}", initial_count, count);
            }
        }
        Err(e) => {
            println!("❌ Deduplication failed: {}", e);
        }
    }

    // Print final mints
    {
        let mints = LIST_MINTS.read().unwrap();
        println!("Final mints: {:?}", mints);
    }
}

/// Deduplicate the LIST_MINTS HashSet to remove any potential duplicates
fn deduplicate_list_mints() -> Result<usize, String> {
    match LIST_MINTS.write() {
        Ok(mut mints_set) => {
            let original_count = mints_set.len();
            // HashSet automatically deduplicates, but let's ensure consistency
            // by rebuilding the set from its own values
            let deduped_mints: HashSet<String> = mints_set.iter().cloned().collect();
            let deduped_count = deduped_mints.len();

            if original_count != deduped_count {
                println!("Deduplicated LIST_MINTS: {} -> {} mints", original_count, deduped_count);
                *mints_set = deduped_mints;
            }

            Ok(deduped_count)
        }
        Err(e) => { Err(format!("Failed to deduplicate LIST_MINTS: {}", e)) }
    }
}
