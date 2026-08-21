//! Durable, subject-scoped dedupe for the watch funnel (§6.3).
//!
//! Reuses the exact machinery the own-wallet pipeline already has -- `transactions::
//! utils`'s in-memory moka front plus `TransactionDatabase`'s `known_signatures`
//! table (subject-scoped since the Phase 1a subject plumbing, collision-free across
//! subjects since the §7.1 composite-key migration) -- rather than a second cache.
//!
//! A restart mid-burst must not re-emit: `has_seen` is checked before `decode()`, and
//! `commit` runs after a successful decode but before the activity is dispatched.
//! Committing before decode would mark a signature seen that decode then failed to
//! produce; committing after dispatch would let a crash between the two re-emit on
//! the next run.

use crate::logger::{self, LogTag};
use crate::transactions::database::{get_transaction_database, TransactionDatabase};
use crate::transactions::types::Subject;
use crate::transactions::utils::{add_signature_to_known_globally, is_signature_known_globally};

/// True when `signature` has already been recorded for `subject`. Checks the fast
/// in-memory cache first, falling back to the database -- the source of truth across
/// restarts, when the in-memory cache is cold.
pub(super) async fn has_seen(subject: Subject, signature: &str) -> Result<bool, String> {
    if is_signature_known_globally(subject, signature).await {
        return Ok(true);
    }

    match get_transaction_database().await {
        Some(db) => has_seen_in_database(&db, subject, signature).await,
        None => Err("Transaction database not initialized".to_owned()),
    }
}

/// Mark `signature` as seen for `subject`, in both the in-memory cache and the
/// database. Durability is part of success: callers must not advance the watch
/// cursor or publish activity when the database write fails, otherwise a restart
/// can re-emit something that was only marked in memory.
pub(super) async fn commit(subject: Subject, signature: &str) -> Result<(), String> {
    let db = get_transaction_database()
        .await
        .ok_or_else(|| "Transaction database not initialized".to_owned())?;
    commit_to_database(&db, subject, signature)
        .await
        .map_err(|e| {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Failed to persist known signature {signature} for {subject}: {e}"),
            );
            e
        })?;
    add_signature_to_known_globally(subject, signature.to_owned()).await;
    Ok(())
}

async fn has_seen_in_database(
    db: &TransactionDatabase,
    subject: Subject,
    signature: &str,
) -> Result<bool, String> {
    db.is_signature_known(subject, signature).await
}

async fn commit_to_database(
    db: &TransactionDatabase,
    subject: Subject,
    signature: &str,
) -> Result<(), String> {
    db.add_known_signature(subject, signature).await
}

#[cfg(test)]
mod tests {
    use crate::chains::solana::solana_sdk::pubkey::Pubkey;

    use super::*;
    use crate::transactions::utils::remove_signature_from_known_globally;

    #[tokio::test]
    async fn in_memory_admission_is_subject_scoped() {
        let subject = Subject::solana(Pubkey::new_unique());
        let signature = "dedupe-admission-signature";

        remove_signature_from_known_globally(subject, signature).await;
        // No global database is installed in this co-located unit test, so exercise
        // the fast admission seam directly rather than claiming restart durability.
        assert!(!is_signature_known_globally(subject, signature).await);
        add_signature_to_known_globally(subject, signature.to_owned()).await;
        assert!(is_signature_known_globally(subject, signature).await);
        remove_signature_from_known_globally(subject, signature).await;
    }

    #[tokio::test]
    async fn durable_admission_survives_database_reopen() {
        let dir = tempfile::tempdir().expect("temp database directory");
        let path = dir.path().join("transactions.db");
        let subject = Subject::solana(Pubkey::new_unique());
        let signature = "durable-dedupe-signature";

        let db = TransactionDatabase::new_with_path(&path, crate::chains::ChainId::Solana)
            .await
            .expect("create transaction database");
        commit_to_database(&db, subject, signature)
            .await
            .expect("persist dedupe admission");
        drop(db);

        let reopened = TransactionDatabase::new_with_path(&path, crate::chains::ChainId::Solana)
            .await
            .expect("reopen transaction database");
        assert!(has_seen_in_database(&reopened, subject, signature)
            .await
            .expect("read durable admission"));
    }
}
