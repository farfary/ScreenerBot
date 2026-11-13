// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! In-memory transaction history derived from the promotional position book.

use std::sync::LazyLock;

use chrono::DateTime;

use crate::transactions::database::{
    TransactionCursor, TransactionListFilters, TransactionListRow,
};
use crate::transactions::{Transaction, TransactionDirection, TransactionStatus, TransactionType};
use crate::webserver::routes::positions::types::PositionResponse;
use crate::webserver::routes::transactions::types::{
    BootstrapStateInfo, ListTransactionsRequest, ListTransactionsResponse,
    TransactionDetailResponse, TransactionSummaryResponse,
};

use super::{get_promo_positions, get_promo_wallet_address};

/// Keep timestamps fixed across requests so a page cursor never shifts its boundary.
static PROMO_TRANSACTIONS: LazyLock<Vec<Transaction>> = LazyLock::new(|| {
    let mut transactions = Vec::new();
    for position in get_promo_positions(Some("all")) {
        transactions.push(position_transaction(&position, false));
        if position.exit_transaction_signature.is_some() {
            transactions.push(position_transaction(&position, true));
        }
    }
    transactions.sort_by(|a, b| {
        b.timestamp
            .cmp(&a.timestamp)
            .then_with(|| b.signature.cmp(&a.signature))
    });
    transactions
});

fn position_transaction(position: &PositionResponse, sell: bool) -> Transaction {
    let (signature, timestamp, native_amount, fee) = if sell {
        (
            position.exit_transaction_signature.as_ref(),
            position.exit_time.unwrap_or(position.entry_time),
            position.native_received.unwrap_or_default(),
            position.exit_fee_raw,
        )
    } else {
        (
            position.entry_transaction_signature.as_ref(),
            position.entry_time,
            position.entry_size_native,
            position.entry_fee_raw,
        )
    };
    let mut transaction = Transaction::new(signature.expect("sample position signature").clone());
    transaction.timestamp = DateTime::from_timestamp(timestamp, 0).expect("sample position time");
    transaction.block_time = Some(timestamp);
    transaction.last_updated = transaction.timestamp;
    transaction.status = TransactionStatus::Finalized;
    transaction.success = true;
    transaction.wallet_signed = true;
    transaction.fee_raw = fee;
    transaction.fee_native = fee.unwrap_or_default() as f64 / 1e9;
    transaction.native_balance_change = if sell { native_amount } else { -native_amount };
    transaction.token_symbol = Some(position.symbol.clone());
    transaction.token_decimals = position.token_decimals;
    let token_amount = position.entry_size_native / position.entry_price;
    transaction.direction = if sell {
        TransactionDirection::TokensOut
    } else {
        TransactionDirection::TokensIn
    };
    transaction.transaction_type = if sell {
        TransactionType::SwapTokenToSol {
            token_mint: position.mint.clone(),
            token_amount,
            sol_amount: native_amount,
            router: "jupiter".to_owned(),
        }
    } else {
        TransactionType::SwapSolToToken {
            token_mint: position.mint.clone(),
            token_amount,
            sol_amount: native_amount,
            router: "jupiter".to_owned(),
        }
    };
    transaction
}

/// Only the sample trading wallet has sample history; no other subject falls through
/// to a local wallet, watch registry or database.
fn sample_subject(subject: Option<&str>) -> bool {
    subject.is_none_or(|value| {
        let value = value.trim();
        value.is_empty() || value == get_promo_wallet_address()
    })
}

fn row(transaction: &Transaction) -> TransactionListRow {
    let (mint, amount, router) = match &transaction.transaction_type {
        TransactionType::SwapSolToToken {
            token_mint,
            token_amount,
            router,
            ..
        }
        | TransactionType::SwapTokenToSol {
            token_mint,
            token_amount,
            router,
            ..
        } => (token_mint.clone(), *token_amount, router.clone()),
        _ => unreachable!("the sample position book contains swaps only"),
    };
    TransactionListRow {
        signature: transaction.signature.clone(),
        timestamp: transaction.timestamp,
        slot: transaction.slot,
        status: "Finalized".to_owned(),
        success: transaction.success,
        direction: Some(transaction.direction.as_str().to_owned()),
        transaction_type: Some(transaction.transaction_type.kind().to_owned()),
        token_mint: Some(mint),
        token_symbol: transaction.token_symbol.clone(),
        router: Some(router),
        native_delta: transaction.native_balance_change,
        token_amount: Some(amount),
        fee_native: transaction.fee_native,
        fee_raw: transaction.fee_raw,
        ata_rents: 0.0,
        instructions_count: transaction.instructions_count,
    }
}

fn contains(value: Option<&str>, needle: Option<&str>) -> bool {
    needle.is_none_or(|needle| {
        value.is_some_and(|value| value.to_lowercase().contains(&needle.to_lowercase()))
    })
}

fn matches(row: &TransactionListRow, filters: &TransactionListFilters) -> bool {
    (filters.types.is_empty()
        || filters
            .types
            .iter()
            .any(|kind| row.transaction_type.as_ref() == Some(kind)))
        && contains(row.token_mint.as_deref(), filters.mint.as_deref())
        && contains(Some(&row.signature), filters.signature.as_deref())
        && contains(row.router.as_deref(), filters.router.as_deref())
        && filters
            .direction
            .as_ref()
            .is_none_or(|direction| row.direction.as_ref() == Some(direction))
        && filters
            .status
            .as_ref()
            .is_none_or(|status| row.status == *status)
        && filters.time_from.is_none_or(|from| row.timestamp >= from)
        && filters.time_to.is_none_or(|to| row.timestamp <= to)
        && filters.min_native.is_none_or(|min| row.native_delta >= min)
        && filters.max_native.is_none_or(|max| row.native_delta <= max)
}

pub fn get_promo_transactions(request: &ListTransactionsRequest) -> ListTransactionsResponse {
    let rows: Vec<_> = PROMO_TRANSACTIONS
        .iter()
        .filter(|_| sample_subject(request.subject.as_deref()))
        .map(row)
        .filter(|row| matches(row, &request.filters))
        .collect();
    let total = rows.len() as u64;
    let mut remaining = rows.into_iter().filter(|row| {
        request.pagination.cursor.as_ref().is_none_or(|cursor| {
            (row.timestamp.to_rfc3339(), &row.signature)
                < (cursor.timestamp.clone(), &cursor.signature)
        })
    });
    let items: Vec<_> = remaining.by_ref().take(request.pagination.limit).collect();
    let next_cursor = remaining.next().and_then(|_| {
        items.last().map(|last| TransactionCursor {
            timestamp: last.timestamp.to_rfc3339(),
            signature: last.signature.clone(),
        })
    });
    ListTransactionsResponse {
        items,
        next_cursor,
        total_estimate: Some(total),
    }
}

pub fn get_promo_transaction_detail(
    subject: Option<&str>,
    signature: &str,
) -> Option<TransactionDetailResponse> {
    sample_subject(subject)
        .then(|| {
            PROMO_TRANSACTIONS
                .iter()
                .find(|tx| tx.signature == signature)
        })
        .flatten()
        .cloned()
        .map(TransactionDetailResponse::from)
}

pub fn get_promo_transaction_summary(subject: Option<&str>) -> TransactionSummaryResponse {
    let transactions: Vec<_> = PROMO_TRANSACTIONS
        .iter()
        .filter(|_| sample_subject(subject))
        .collect();
    let total = transactions.len() as u64;
    TransactionSummaryResponse {
        total,
        success_count: total,
        failed_count: 0,
        pending_global: 0,
        pending_local: 0,
        deferred_count: 0,
        success_rate: if total > 0 { 100.0 } else { 0.0 },
        failure_rate: 0.0,
        newest_known_signature: transactions.first().map(|tx| tx.signature.clone()),
        oldest_known_signature: transactions.last().map(|tx| tx.signature.clone()),
        db_size_mb: 0.0,
        db_schema_version: 0,
        bootstrap_state: BootstrapStateInfo {
            full_history_completed: true,
            older_history_available: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webserver::routes::transactions::types::PaginationParams;
    use std::collections::HashSet;

    fn request() -> ListTransactionsRequest {
        ListTransactionsRequest {
            subject: None,
            filters: TransactionListFilters::default(),
            pagination: PaginationParams {
                cursor: None,
                limit: usize::MAX,
            },
        }
    }

    #[test]
    fn rows_details_and_summary_describe_the_same_sample_book() {
        let all = get_promo_transactions(&request());
        let summary = get_promo_transaction_summary(None);
        let positions = get_promo_positions(Some("all"));
        assert_eq!(
            all.items.len(),
            positions.len() + positions.iter().filter(|p| p.exit_time.is_some()).count()
        );
        assert_eq!(summary.total, all.items.len() as u64);
        assert_eq!(summary.total, summary.success_count + summary.failed_count);
        assert!(all.items.len() > 20);
        for row in all.items {
            let detail = get_promo_transaction_detail(None, &row.signature).expect("sample detail");
            assert_eq!(row.timestamp, detail.timestamp);
            assert_eq!(row.native_delta, detail.native_balance_change);
            assert_eq!(row.fee_raw, detail.fee_raw);
            assert_eq!(
                row.transaction_type.as_deref(),
                Some(detail.transaction_type.kind())
            );
            assert!(detail.raw_transaction_data.is_none());
        }
    }

    #[test]
    fn filtered_cursor_pages_do_not_duplicate_or_lose_samples() {
        let mut request = request();
        request.filters.types = vec!["buy".to_owned()];
        let expected = get_promo_transactions(&request).items.len();
        request.pagination.limit = 3;
        let mut signatures = HashSet::new();
        loop {
            let page = get_promo_transactions(&request);
            for row in page.items {
                assert_eq!(row.transaction_type.as_deref(), Some("buy"));
                assert!(signatures.insert(row.signature));
            }
            request.pagination.cursor = page.next_cursor;
            if request.pagination.cursor.is_none() {
                break;
            }
        }
        assert_eq!(signatures.len(), expected);
    }

    #[test]
    fn every_list_filter_limits_the_same_sample_rows() {
        let all = get_promo_transactions(&request());
        let target = all
            .items
            .iter()
            .find(|row| row.native_delta > 0.0)
            .expect("sample sale");
        let mut request = request();
        request.filters = TransactionListFilters {
            types: vec!["sell".to_owned()],
            mint: target.token_mint.clone(),
            only_confirmed: Some(true),
            direction: target.direction.clone(),
            status: Some("Finalized".to_owned()),
            signature: Some(target.signature.clone()),
            time_from: Some(target.timestamp),
            time_to: Some(target.timestamp),
            router: Some("JUP".to_owned()),
            min_native: Some(target.native_delta),
            max_native: Some(target.native_delta),
        };
        let page = get_promo_transactions(&request);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].signature, target.signature);
        request.filters.status = Some("Pending".to_owned());
        assert!(get_promo_transactions(&request).items.is_empty());
        request.filters.status = None;
        request.pagination.limit = 0;
        let empty = get_promo_transactions(&request);
        assert!(empty.items.is_empty());
        assert!(empty.next_cursor.is_none());
    }

    #[test]
    fn other_subjects_and_unknown_signatures_never_return_real_history() {
        let mut request = request();
        request.subject = Some("not-the-sample-wallet".to_owned());
        assert!(get_promo_transactions(&request).items.is_empty());
        assert_eq!(
            get_promo_transaction_summary(request.subject.as_deref()).total,
            0
        );
        assert!(get_promo_transaction_detail(None, "not-a-sample-signature").is_none());
        assert!(get_promo_transaction_detail(
            request.subject.as_deref(),
            &PROMO_TRANSACTIONS[0].signature
        )
        .is_none());
    }
}
