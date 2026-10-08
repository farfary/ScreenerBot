// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The one open position of a mint per chain and wallet: its lookup in storage and the unique index that refuses a second.

use rusqlite::{params, Connection, OptionalExtension};

use crate::errors::DatabaseError;
use crate::positions::{Error, Result};

/// Name of the partial unique index that allows one open row per chain, wallet and mint.
pub(super) const OPEN_ROUND_INDEX_NAME: &str = "idx_positions_open_round";

/// The index itself. A row is open while it has no exit time, archived or not, which is the
/// same set [`super::booking::query_other_open_held`] reads.
const OPEN_ROUND_INDEX: &str = "CREATE UNIQUE INDEX IF NOT EXISTS idx_positions_open_round ON positions(chain_id, wallet_address, mint) WHERE exit_time IS NULL";

/// Open rows of one mint that a store already held before the index existed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateOpenRound {
    pub chain_id: String,
    pub wallet_address: String,
    pub mint: String,
    /// Every open row of the mint, in ascending id order.
    pub position_ids: Vec<i64>,
}

/// Installs the open-round index, or reports why it cannot be installed yet.
///
/// A store written before the index can hold several open rows of one mint. Their books are
/// left exactly as they are: merging them would rewrite cost bases and realized P&L from
/// rows whose entries may still be unverified, and closing all but one would book closes
/// that never happened. Every such group is returned instead, and the index is installed at
/// the first initialization after only one open row of each mint remains. Until then the
/// code paths that create open rows still refuse a second one through
/// [`query_open_round_id`].
pub(super) fn install_open_round_index(conn: &Connection) -> Result<Vec<DuplicateOpenRound>> {
    let duplicates = duplicate_open_rounds(conn)?;
    if duplicates.is_empty() {
        conn.execute(OPEN_ROUND_INDEX, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create the open-round index: {e}"),
            })?;
    }
    Ok(duplicates)
}

/// Every chain, wallet and mint with more than one open row.
fn duplicate_open_rounds(conn: &Connection) -> Result<Vec<DuplicateOpenRound>> {
    let schema_error = |e: rusqlite::Error| Error::SchemaMigration {
        detail: format!("failed to read duplicate open positions: {e}"),
    };
    let mut statement = conn
        .prepare(
            "SELECT chain_id, wallet_address, mint, group_concat(id)
             FROM positions
             WHERE exit_time IS NULL
             GROUP BY chain_id, wallet_address, mint
             HAVING COUNT(*) > 1
             ORDER BY chain_id, wallet_address, mint",
        )
        .map_err(schema_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(schema_error)?;
    let mut duplicates = Vec::new();
    for row in rows {
        let (chain_id, wallet_address, mint, ids) = row.map_err(schema_error)?;
        let mut position_ids = ids
            .split(',')
            .map(|id| {
                id.parse::<i64>().map_err(|e| Error::SchemaMigration {
                    detail: format!("failed to decode open position id {id:?}: {e}"),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        position_ids.sort_unstable();
        duplicates.push(DuplicateOpenRound {
            chain_id,
            wallet_address,
            mint,
            position_ids,
        });
    }
    Ok(duplicates)
}

/// The id of the open row of `mint` in this chain and wallet other than `excluded`,
/// archived or not. Where a store still holds several, the choice is the one
/// `positions::state::get_open_round_by_mint` makes in memory: an active row before an
/// archived one, then the earliest by entry time and id.
pub(super) fn query_open_round_id(
    conn: &Connection,
    chain: &str,
    wallet_address: &str,
    mint: &str,
    excluded: Option<i64>,
) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM positions
         WHERE chain_id = ?1 AND wallet_address = ?2 AND mint = ?3
           AND (?4 IS NULL OR id != ?4)
           AND exit_time IS NULL
         ORDER BY archived, entry_time, id
         LIMIT 1",
        params![chain, wallet_address, mint, excluded],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(|e| DatabaseError::classify_sqlite_failure("open_round", e).into())
}

/// The ids of the active (not archived) open rows of `mint` in this chain and wallet other
/// than `excluded`, in ascending id order. Empty unless the store held several open rows of
/// the mint before the open-round index.
pub(super) fn query_active_open_round_ids(
    conn: &Connection,
    chain: &str,
    wallet_address: &str,
    mint: &str,
    excluded: i64,
) -> Result<Vec<i64>> {
    let sqlite = |e| DatabaseError::classify_sqlite_failure("open_round", e);
    let mut statement = conn
        .prepare(
            "SELECT id FROM positions
             WHERE chain_id = ?1 AND wallet_address = ?2 AND mint = ?3
               AND id != ?4 AND exit_time IS NULL AND archived = 0
             ORDER BY id",
        )
        .map_err(sqlite)?;
    let ids = statement
        .query_map(params![chain, wallet_address, mint, excluded], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(sqlite)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(sqlite)?;
    Ok(ids)
}

/// Refuses to make `position_id`, an open row of `mint`, active while another open row of
/// the mint in this chain and wallet is active: the error names every open row involved.
pub(super) fn refuse_second_active_open_round(
    conn: &Connection,
    chain: &str,
    wallet_address: &str,
    mint: &str,
    position_id: i64,
) -> Result<()> {
    let mut position_ids =
        query_active_open_round_ids(conn, chain, wallet_address, mint, position_id)?;
    if position_ids.is_empty() {
        return Ok(());
    }
    position_ids.push(position_id);
    position_ids.sort_unstable();
    Err(Error::DuplicateOpenRound {
        mint: mint.to_owned(),
        position_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE positions (
                id INTEGER PRIMARY KEY,
                chain_id TEXT NOT NULL DEFAULT 'solana',
                wallet_address TEXT NOT NULL,
                mint TEXT NOT NULL,
                entry_time TEXT NOT NULL,
                exit_time TEXT,
                archived INTEGER NOT NULL DEFAULT 0
             );",
        )
        .unwrap();
        conn
    }

    fn insert(conn: &Connection, id: i64, mint: &str, exit_time: Option<&str>) {
        conn.execute(
            "INSERT INTO positions (id, wallet_address, mint, entry_time, exit_time) VALUES (?1, 'wallet', ?2, ?3, ?4)",
            params![id, mint, format!("2026-01-0{id}T00:00:00Z"), exit_time],
        )
        .unwrap();
    }

    fn has_index(conn: &Connection) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [OPEN_ROUND_INDEX_NAME],
            |_| Ok(()),
        )
        .optional()
        .unwrap()
        .is_some()
    }

    #[test]
    fn the_index_refuses_a_second_open_row_and_allows_closed_ones() {
        let conn = store();
        insert(&conn, 1, "mint", Some("2026-01-02T00:00:00Z"));
        insert(&conn, 2, "mint", None);
        assert!(install_open_round_index(&conn).unwrap().is_empty());
        assert!(has_index(&conn));

        insert(&conn, 3, "mint", Some("2026-01-04T00:00:00Z"));
        insert(&conn, 4, "other", None);
        let second_open = conn.execute(
            "INSERT INTO positions (id, wallet_address, mint, entry_time) VALUES (5, 'wallet', 'mint', '2026-01-05T00:00:00Z')",
            [],
        );
        assert!(second_open.is_err(), "a second open row was stored");
        assert_eq!(
            query_open_round_id(&conn, "solana", "wallet", "mint", None).unwrap(),
            Some(2)
        );
        assert_eq!(
            query_open_round_id(&conn, "solana", "wallet", "mint", Some(2)).unwrap(),
            None
        );
    }

    #[test]
    fn duplicate_open_rows_are_reported_untouched_and_the_index_waits_for_them() {
        let conn = store();
        insert(&conn, 1, "mint", None);
        insert(&conn, 2, "other", None);
        insert(&conn, 3, "mint", None);
        insert(&conn, 4, "mint", Some("2026-01-05T00:00:00Z"));
        let before: Vec<(i64, Option<String>)> = conn
            .prepare("SELECT id, exit_time FROM positions ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();

        let reported = install_open_round_index(&conn).unwrap();
        assert_eq!(
            reported,
            vec![DuplicateOpenRound {
                chain_id: "solana".to_owned(),
                wallet_address: "wallet".to_owned(),
                mint: "mint".to_owned(),
                position_ids: vec![1, 3],
            }]
        );
        assert!(!has_index(&conn), "the index was installed over duplicates");
        let after: Vec<(i64, Option<String>)> = conn
            .prepare("SELECT id, exit_time FROM positions ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(after, before, "a reported row was changed");
        assert_eq!(
            query_open_round_id(&conn, "solana", "wallet", "mint", None).unwrap(),
            Some(1),
            "the earliest open row is the open round"
        );

        conn.execute(
            "UPDATE positions SET exit_time = '2026-01-06T00:00:00Z' WHERE id = 3",
            [],
        )
        .unwrap();
        assert!(install_open_round_index(&conn).unwrap().is_empty());
        assert!(has_index(&conn));
    }

    #[test]
    fn an_active_open_row_is_chosen_before_an_archived_one_then_the_earliest() {
        let conn = store();
        insert(&conn, 1, "mint", None);
        insert(&conn, 2, "mint", None);
        insert(&conn, 3, "mint", None);
        conn.execute("UPDATE positions SET archived = 1 WHERE id = 1", [])
            .unwrap();
        assert_eq!(
            query_open_round_id(&conn, "solana", "wallet", "mint", None).unwrap(),
            Some(2),
            "an archived row was chosen beside an active one"
        );
        assert_eq!(
            query_open_round_id(&conn, "solana", "wallet", "mint", Some(2)).unwrap(),
            Some(3)
        );
        conn.execute("UPDATE positions SET archived = 1", [])
            .unwrap();
        assert_eq!(
            query_open_round_id(&conn, "solana", "wallet", "mint", None).unwrap(),
            Some(1),
            "among archived rows the earliest is chosen"
        );
    }

    #[test]
    fn a_second_active_open_row_is_refused_naming_every_one_involved() {
        let conn = store();
        insert(&conn, 1, "mint", None);
        insert(&conn, 2, "mint", None);
        insert(&conn, 3, "mint", Some("2026-01-04T00:00:00Z"));
        conn.execute("UPDATE positions SET archived = 1 WHERE id = 2", [])
            .unwrap();
        match refuse_second_active_open_round(&conn, "solana", "wallet", "mint", 2) {
            Err(Error::DuplicateOpenRound { mint, position_ids }) => {
                assert_eq!(mint, "mint");
                assert_eq!(position_ids, vec![1, 2]);
            }
            other => panic!("a second active open row was allowed: {other:?}"),
        }
        conn.execute("UPDATE positions SET archived = 1 WHERE id = 1", [])
            .unwrap();
        assert!(refuse_second_active_open_round(&conn, "solana", "wallet", "mint", 2).is_ok());
    }
}
