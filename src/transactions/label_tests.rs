//! Label maps of the transaction enums must resolve in the `en` catalog.

use super::types::{AtaOperationType, TransactionDirection, TransactionStatus};

/// Catalog key of the label for each direction. The match is exhaustive, so a
/// new variant fails to compile until it is mapped here and in
/// `DIRECTION_LABELS` (ui/transaction_direction.js).
fn direction_key(direction: &TransactionDirection) -> &'static str {
    match direction {
        TransactionDirection::Incoming => "transactions-direction-incoming",
        TransactionDirection::Outgoing => "transactions-direction-outgoing",
        TransactionDirection::Internal => "transactions-direction-internal",
        TransactionDirection::Unknown => "transactions-direction-unknown",
    }
}

/// Catalog key of each status label, mirrored by `TRANSACTION_STATUS_LABELS`
/// (ui/transaction_status.js).
fn status_key(status: &TransactionStatus) -> &'static str {
    match status {
        TransactionStatus::Pending => "transactions-status-pending",
        TransactionStatus::Confirmed => "transactions-status-confirmed",
        TransactionStatus::Finalized => "transactions-status-finalized",
        TransactionStatus::Failed(_) => "transactions-status-failed",
    }
}

/// Catalog key of each ATA operation label, mirrored by `ATA_OPERATION_LABELS`
/// (ui/transaction_details_dialog.js).
fn ata_operation_key(operation: &AtaOperationType) -> &'static str {
    match operation {
        AtaOperationType::Creation => "transactions-ata-operation-creation",
        AtaOperationType::Closure => "transactions-ata-operation-closure",
    }
}

fn assert_in_catalog(key: &str) {
    assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
}

#[test]
fn direction_labels_exist_in_the_catalog() {
    for direction in [
        TransactionDirection::Incoming,
        TransactionDirection::Outgoing,
        TransactionDirection::Internal,
        TransactionDirection::Unknown,
    ] {
        assert_in_catalog(direction_key(&direction));
    }
}

#[test]
fn status_labels_exist_in_the_catalog() {
    for status in [
        TransactionStatus::Pending,
        TransactionStatus::Confirmed,
        TransactionStatus::Finalized,
        TransactionStatus::Failed(String::new()),
    ] {
        assert_in_catalog(status_key(&status));
    }
}

#[test]
fn ata_operation_labels_exist_in_the_catalog() {
    for operation in [AtaOperationType::Creation, AtaOperationType::Closure] {
        assert_in_catalog(ata_operation_key(&operation));
    }
}
