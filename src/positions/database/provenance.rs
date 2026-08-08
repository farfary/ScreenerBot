//! Idempotent migration from legacy position ownership to typed provenance.

use rusqlite::Connection;

fn has_column(conn: &Connection, name: &str) -> Result<bool, String> {
    let mut statement = conn
        .prepare("PRAGMA table_info(positions)")
        .map_err(|e| format!("Failed to inspect positions schema: {e}"))?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Failed to read positions schema: {e}"))?;
    for column in names {
        if column.map_err(|e| format!("Failed to decode positions schema: {e}"))? == name {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn migrate_position_provenance(conn: &Connection) -> Result<(), String> {
    let legacy_manual = has_column(conn, "manual_management")?;
    for (column, sql) in [
        (
            "origin_kind",
            "ALTER TABLE positions ADD COLUMN origin_kind TEXT NOT NULL DEFAULT 'auto'",
        ),
        (
            "origin_ref",
            "ALTER TABLE positions ADD COLUMN origin_ref TEXT",
        ),
        (
            "management",
            "ALTER TABLE positions ADD COLUMN management TEXT NOT NULL DEFAULT 'auto_trader'",
        ),
    ] {
        if !has_column(conn, column)? {
            conn.execute(sql, [])
                .map_err(|e| format!("Failed to add positions.{column}: {e}"))?;
        }
    }

    if legacy_manual {
        conn.execute(
            "UPDATE positions SET origin_kind = CASE WHEN manual_management THEN 'manual' ELSE 'auto' END, origin_ref = NULL, management = CASE WHEN manual_management THEN 'user_only' ELSE 'auto_trader' END WHERE origin_kind = 'auto' AND origin_ref IS NULL AND management = 'auto_trader'",
            [],
        )
        .map_err(|e| format!("Failed to backfill position provenance: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_backfill_is_typed_and_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE positions (id INTEGER PRIMARY KEY, manual_management BOOLEAN NOT NULL DEFAULT 0); INSERT INTO positions (id, manual_management) VALUES (1, 0), (2, 1);",
        )
        .unwrap();

        migrate_position_provenance(&conn).unwrap();
        migrate_position_provenance(&conn).unwrap();

        let values = conn
            .prepare("SELECT id, origin_kind, origin_ref, management FROM positions ORDER BY id")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(
            values,
            vec![
                (1, "auto".to_owned(), None, "auto_trader".to_owned()),
                (2, "manual".to_owned(), None, "user_only".to_owned()),
            ]
        );
    }

    #[test]
    fn fresh_schema_without_legacy_flag_gets_safe_defaults() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE positions (id INTEGER PRIMARY KEY); INSERT INTO positions (id) VALUES (1);")
            .unwrap();
        migrate_position_provenance(&conn).unwrap();
        let values: (String, Option<String>, String) = conn
            .query_row(
                "SELECT origin_kind, origin_ref, management FROM positions WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(values, ("auto".to_owned(), None, "auto_trader".to_owned()));
    }
}
