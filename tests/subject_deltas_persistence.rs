//! `subject_asset_deltas`: the rows every wallet-derived position is built from, and
//! the two paths that must both write them.
//!
//! The ledger can only be as complete as this table. The bug this suite exists for was
//! exactly that: deltas were extracted on the BOOTSTRAP path only, so a swap made while
//! the bot was running got stored in `raw_transactions`, marked known — and never
//! reduced. Bootstrap skips known signatures, and the historical backfill runs once per
//! version, so the transaction stayed outside the ledger permanently: the position it
//! should have opened never appeared, and the one it should have closed stayed open.
//!
//! Deliberately ONE test function: `paths::get_data_directory()` memoises its base
//! directory in a process-wide `LazyLock` and `cargo test` runs a file's tests
//! concurrently, so a second `isolated_env()` here would race this one.
//!
//! `#[tokio::test(flavor = "multi_thread")]` is required: database init resolves the own
//! wallet through `block_in_place`, which panics on the current-thread runtime.

mod common;

use serde_json::json;

use screenerbot::transactions::{Subject, Transaction, TransactionDatabase};

const MINT: &str = "MintA111111111111111111111111111111111111";
const POOL: &str = "Poo1Addre55111111111111111111111111111111";
/// Raydium CPMM — a known router, so the extracted deltas classify as a trade.
const ROUTER: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// A confirmed swap: the subject spends `sol_lamports` and receives `tokens` of [`MINT`].
fn swap(signature: &str, subject: &str, slot: u64, sol_lamports: u64, tokens: u64) -> Transaction {
    let raw = json!({
        "slot": slot,
        "blockTime": 1_700_000_000 + slot as i64,
        "transactionIndex": 0,
        "transaction": {
            "signatures": [signature],
            "message": {
                "accountKeys": [subject, POOL],
                "instructions": [{ "programId": ROUTER, "accounts": [], "data": "" }]
            }
        },
        "meta": {
            "err": null,
            "fee": 5000,
            "preBalances": [10_000_000_000u64, 0u64],
            "postBalances": [10_000_000_000u64 - sol_lamports - 5000, 0u64],
            "preTokenBalances": [{
                "accountIndex": 0,
                "mint": MINT,
                "owner": subject,
                "uiTokenAmount": { "amount": "0", "decimals": 6, "uiAmount": null, "uiAmountString": "0" }
            }],
            "postTokenBalances": [{
                "accountIndex": 0,
                "mint": MINT,
                "owner": subject,
                "uiTokenAmount": {
                    "amount": tokens.to_string(),
                    "decimals": 6,
                    "uiAmount": null,
                    "uiAmountString": tokens.to_string()
                }
            }],
        }
    });

    let mut transaction = Transaction::new(signature.to_owned());
    transaction.success = true;
    transaction.slot = Some(slot);
    transaction.block_time = Some(1_700_000_000 + slot as i64);
    transaction.raw_transaction_data = Some(raw);
    transaction
}

#[tokio::test(flavor = "multi_thread")]
async fn deltas_are_written_live_and_any_gap_is_repaired_on_the_next_boot() {
    let _guard = common::isolated_env();
    let wallet = common::configure_own_wallet();
    let subject = Subject::own().expect("own subject resolves from the test config");

    // The data directory only exists once something writes to it; the pool cannot
    // create it for us.
    if let Some(parent) = screenerbot::paths::get_transactions_db_path().parent() {
        std::fs::create_dir_all(parent).expect("create the temp data directory");
    }

    let db = TransactionDatabase::new(screenerbot::chains::ChainId::Solana)
        .await
        .expect("open the transactions database");

    // ---- 1. The live path writes deltas for a transaction as it is recorded ----
    let live = swap("live-signature", &wallet, 100, 1_000_000_000, 2_000_000);
    let written = db
        .store_transaction_deltas(&wallet, &live)
        .await
        .expect("store the live transaction's deltas");
    assert_eq!(written, 2, "one delta for the token, one for native SOL");

    let deltas = db.get_subject_deltas(&wallet).await.expect("read deltas");
    let token = deltas
        .iter()
        .find(|delta| delta.mint == MINT)
        .expect("the token leg is stored");
    assert_eq!(token.delta_raw, 2_000_000);
    assert_eq!(token.before_raw, Some(0));
    assert_eq!(token.after_raw, Some(2_000_000));

    // Storing the same transaction again is a no-op, not a duplicate: the primary key
    // is (wallet, signature, mint) and a re-record must never double a balance.
    db.store_transaction_deltas(&wallet, &live)
        .await
        .expect("re-storing is idempotent");
    assert_eq!(
        db.count_subject_deltas(&wallet).await.expect("count"),
        2,
        "re-recording a transaction must not add rows"
    );

    // ---- 2. A transaction recorded WITHOUT its deltas is the gap the fill repairs ----
    // This is precisely the state the old code left behind: raw JSON stored, signature
    // known, no ledger rows, and nothing that would ever revisit it.
    let orphan = swap("orphan-signature", &wallet, 200, 500_000_000, 1_500_000);
    db.store_raw_transaction(subject, &orphan)
        .await
        .expect("store the raw transaction without extracting deltas");
    assert_eq!(
        db.count_subject_deltas(&wallet).await.expect("count"),
        2,
        "storing raw data alone writes no ledger rows"
    );

    let repaired = db
        .fill_subject_delta_gaps(&wallet)
        .await
        .expect("gap fill runs");
    assert_eq!(repaired, 1, "the orphaned transaction is reduced");

    let deltas = db.get_subject_deltas(&wallet).await.expect("read deltas");
    assert_eq!(deltas.len(), 4, "both legs of the orphan are now stored");
    let repaired_token = deltas
        .iter()
        .find(|delta| delta.mint == MINT && delta.signature == "orphan-signature")
        .expect("the orphan's token leg is stored");
    assert_eq!(repaired_token.delta_raw, 1_500_000);
    assert_eq!(repaired_token.slot, Some(200));

    // ---- 3. Repairing is idempotent and does not rewrite settled history ----
    let repaired_again = db
        .fill_subject_delta_gaps(&wallet)
        .await
        .expect("gap fill runs again");
    assert_eq!(repaired_again, 0, "nothing is left to repair");
    assert_eq!(db.count_subject_deltas(&wallet).await.expect("count"), 4);

    // ---- 4. The scan is bounded by the ledger's newest slot ----
    // Everything older than the newest delta was reduced when it arrived, so an old
    // transaction is deliberately NOT re-examined. Without that bound this pass would
    // re-read the whole raw history — tens of thousands of JSON blobs — every boot.
    let ancient = swap("ancient-signature", &wallet, 10, 250_000_000, 750_000);
    db.store_raw_transaction(subject, &ancient)
        .await
        .expect("store an old raw transaction");

    assert_eq!(
        db.fill_subject_delta_gaps(&wallet)
            .await
            .expect("gap fill runs"),
        0,
        "history below the watermark is not rescanned"
    );
    assert_eq!(db.count_subject_deltas(&wallet).await.expect("count"), 4);

    // ---- 5. A failed transaction moves nothing and must never reach the ledger ----
    let mut failed = swap("failed-signature", &wallet, 300, 1_000_000_000, 3_000_000);
    failed.success = false;
    assert_eq!(
        db.store_transaction_deltas(&wallet, &failed)
            .await
            .expect("store"),
        0,
        "a failed transaction has no balance movements"
    );
    assert_eq!(db.count_subject_deltas(&wallet).await.expect("count"), 4);

    // ---- 6. A cached blob that hides its timestamp is repaired from the row itself ----
    // Blobs cached before `TransactionDetails` renamed the field spell it `block_time`,
    // and a delta with no timestamp gives its round no open/close moment at all: the
    // position materialised from it is dated "now" on every boot and sorts above trades
    // that really are the wallet's most recent. The row's own column is the authority.
    let mut undated = swap("undated-signature", &wallet, 400, 300_000_000, 900_000);
    let block_time = undated.block_time.expect("the helper stamps a block time");
    if let Some(raw) = undated.raw_transaction_data.as_mut() {
        let object = raw.as_object_mut().expect("the blob is an object");
        object.remove("blockTime");
        object.insert("block_time".to_owned(), json!(block_time));
    }
    db.store_raw_transaction(subject, &undated)
        .await
        .expect("store the undated raw transaction");

    assert_eq!(
        db.fill_subject_delta_gaps(&wallet)
            .await
            .expect("gap fill runs"),
        1,
        "the transaction above the watermark is reduced"
    );

    let deltas = db.get_subject_deltas(&wallet).await.expect("read deltas");
    let undated_legs: Vec<_> = deltas
        .iter()
        .filter(|delta| delta.signature == "undated-signature")
        .collect();
    assert_eq!(undated_legs.len(), 2);
    assert!(
        undated_legs
            .iter()
            .all(|delta| delta.block_time == Some(block_time)),
        "every repaired delta carries the transaction's real block time"
    );
    assert!(undated_legs.iter().all(|delta| delta.slot == Some(400)));
}
