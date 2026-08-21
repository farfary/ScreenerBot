//! SQLite row decoding shared by copy-task and activity queries.

use chrono::Utc;

use crate::trader::copy::types::CopyTask;

pub(super) fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<CopyTask> {
    let parse_json = |index| -> rusqlite::Result<String> { row.get(index) };
    let created: String = row.get(16)?;
    let updated: String = row.get(17)?;
    Ok(CopyTask {
        id: row.get(0)?,
        chain: row.get::<_, String>(1)?.parse().map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                1,
                "chain_id".to_owned(),
                rusqlite::types::Type::Text,
            )
        })?,
        target_address: row.get(2)?,
        label: row.get(3)?,
        enabled: row.get(4)?,
        mode: serde_json::from_str(&parse_json(5)?).map_err(json_error)?,
        sizing: serde_json::from_str(&parse_json(6)?).map_err(json_error)?,
        exit_mode: serde_json::from_str(&parse_json(7)?).map_err(json_error)?,
        exit_policy_overrides: serde_json::from_str(&parse_json(8)?).map_err(json_error)?,
        max_sol_per_trade: row.get(9)?,
        max_sol_per_token: row.get(10)?,
        total_budget_sol: row.get(11)?,
        min_target_trade_sol: row.get(12)?,
        max_target_trade_sol: row.get(13)?,
        buy_once_per_token: row.get(14)?,
        slippage_pct: row.get(15)?,
        created_at: parse_datetime(&created, 16)?,
        updated_at: parse_datetime(&updated, 17)?,
    })
}

pub(super) fn parse_datetime(value: &str, index: usize) -> rusqlite::Result<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

pub(super) fn json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
