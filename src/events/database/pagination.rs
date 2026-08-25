//! Keyset pagination queries for the events database.
//!
//! Provides cursor-based pagination (forward, backward, head) and filtered counting
//! for efficient event browsing without OFFSET.

use crate::errors::DatabaseError;
use crate::events::types::{Event, EventCategory, Severity};
use crate::events::{Error, Result};
use chrono::{DateTime, Utc};

use super::EventsDatabase;

impl EventsDatabase {
    /// Get events with ID greater than cursor (keyset forward)
    pub async fn get_events_since(
        &self,
        after_id: i64,
        limit: usize,
        category: Option<EventCategory>,
        severity: Option<Severity>,
        mint: Option<&str>,
        reference_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<Event>> {
        let conn = self.get_read_connection()?;
        let mut query = String::from(
            "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at FROM events WHERE id > ?1"
        );
        let mut bind: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(after_id)];
        let mut idx = 2;
        if let Some(cat) = category {
            query.push_str(&format!(" AND category = ?{idx}"));
            bind.push(Box::new(cat.to_string()));
            idx += 1;
        }
        if let Some(sev) = severity {
            query.push_str(&format!(" AND severity = ?{idx}"));
            bind.push(Box::new(sev.to_string()));
            idx += 1;
        }
        if let Some(m) = mint {
            query.push_str(&format!(" AND mint = ?{idx}"));
            bind.push(Box::new(m.to_string()));
            idx += 1;
        }
        if let Some(r) = reference_id {
            query.push_str(&format!(" AND reference_id = ?{idx}"));
            bind.push(Box::new(r.to_string()));
            idx += 1;
        }
        if let Some(search_term) = search {
            let wildcard = format!("%{}%", search_term.to_lowercase());
            query.push_str(&format!(" AND LOWER(json_payload) LIKE ?{idx}"));
            bind.push(Box::new(wildcard));
            idx += 1;
        }
        query.push_str(" ORDER BY id ASC LIMIT ?");
        bind.push(Box::new(limit as i64));

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "prepare events since query".to_owned(),
                message: e.to_string(),
            })
        })?;
        let rows = stmt
            .query_map(
                bind.iter()
                    .map(|b| b.as_ref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                |row| parse_event_row(row),
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "execute events since query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut events = Vec::new();
        for r in rows {
            events.push(r.map_err(|e| Error::RowDecode {
                column: "event row",
                detail: e.to_string(),
            })?);
        }
        Ok(events)
    }

    /// Get events with ID less than cursor (keyset backward)
    pub async fn get_events_before(
        &self,
        before_id: i64,
        limit: usize,
        category: Option<EventCategory>,
        severity: Option<Severity>,
        mint: Option<&str>,
        reference_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<Event>> {
        let conn = self.get_read_connection()?;
        let mut query = String::from(
            "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at FROM events WHERE id < ?1"
        );
        let mut bind: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(before_id)];
        let mut idx = 2;
        if let Some(cat) = category {
            query.push_str(&format!(" AND category = ?{idx}"));
            bind.push(Box::new(cat.to_string()));
            idx += 1;
        }
        if let Some(sev) = severity {
            query.push_str(&format!(" AND severity = ?{idx}"));
            bind.push(Box::new(sev.to_string()));
            idx += 1;
        }
        if let Some(m) = mint {
            query.push_str(&format!(" AND mint = ?{idx}"));
            bind.push(Box::new(m.to_string()));
            idx += 1;
        }
        if let Some(r) = reference_id {
            query.push_str(&format!(" AND reference_id = ?{idx}"));
            bind.push(Box::new(r.to_string()));
            idx += 1;
        }
        if let Some(search_term) = search {
            let wildcard = format!("%{}%", search_term.to_lowercase());
            query.push_str(&format!(" AND LOWER(json_payload) LIKE ?{idx}"));
            bind.push(Box::new(wildcard));
            idx += 1;
        }
        query.push_str(" ORDER BY id DESC LIMIT ?");
        bind.push(Box::new(limit as i64));

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "prepare events before query".to_owned(),
                message: e.to_string(),
            })
        })?;
        let rows = stmt
            .query_map(
                bind.iter()
                    .map(|b| b.as_ref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                |row| parse_event_row(row),
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "execute events before query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut events = Vec::new();
        for r in rows {
            events.push(r.map_err(|e| Error::RowDecode {
                column: "event row",
                detail: e.to_string(),
            })?);
        }
        Ok(events)
    }

    /// Get latest N events and return also the max id
    pub async fn get_events_head(
        &self,
        limit: usize,
        category: Option<EventCategory>,
        severity: Option<Severity>,
        mint: Option<&str>,
        reference_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<(Vec<Event>, i64)> {
        let conn = self.get_read_connection()?;
        let mut query = String::from(
            "SELECT id, event_time, category, subtype, severity, mint, reference_id, json_payload, created_at FROM events"
        );
        let mut where_added = false;
        let mut bind: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1;
        if let Some(cat) = category {
            query.push_str(&format!(
                "{} category = ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(cat.to_string()));
            idx += 1;
        }
        if let Some(sev) = severity {
            query.push_str(&format!(
                "{} severity = ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(sev.to_string()));
            idx += 1;
        }
        if let Some(m) = mint {
            query.push_str(&format!(
                "{} mint = ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(m.to_string()));
            idx += 1;
        }
        if let Some(r) = reference_id {
            query.push_str(&format!(
                "{} reference_id = ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(r.to_string()));
            idx += 1;
        }
        if let Some(search_term) = search {
            let wildcard = format!("%{}%", search_term.to_lowercase());
            query.push_str(&format!(
                "{} LOWER(json_payload) LIKE ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(wildcard));
            idx += 1;
        }
        query.push_str(" ORDER BY id DESC LIMIT ?");
        bind.push(Box::new(limit as i64));

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "prepare events head query".to_owned(),
                message: e.to_string(),
            })
        })?;
        let rows = stmt
            .query_map(
                bind.iter()
                    .map(|b| b.as_ref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                |row| parse_event_row(row),
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "execute events head query".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut events = Vec::new();
        let mut max_id: i64 = 0;
        for r in rows {
            let e = r.map_err(|e| Error::RowDecode {
                column: "event row",
                detail: e.to_string(),
            })?;
            if let Some(id) = e.id {
                if id > max_id {
                    max_id = id;
                }
            }
            events.push(e);
        }
        Ok((events, max_id))
    }

    /// Count total events matching filters
    pub async fn count_events_filtered(
        &self,
        category: Option<EventCategory>,
        severity: Option<Severity>,
        mint: Option<&str>,
        reference_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<i64> {
        let conn = self.get_read_connection()?;
        let mut query = "SELECT COUNT(*) FROM events".to_owned();
        let mut where_added = false;
        let mut bind: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(cat) = category {
            query.push_str(&format!(
                "{} category = ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(cat.to_string()));
            idx += 1;
        }
        if let Some(sev) = severity {
            query.push_str(&format!(
                "{} severity = ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(sev.to_string()));
            idx += 1;
        }
        if let Some(m) = mint {
            query.push_str(&format!(
                "{} mint = ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(m.to_string()));
            idx += 1;
        }
        if let Some(r) = reference_id {
            query.push_str(&format!(
                "{} reference_id = ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(r.to_string()));
            idx += 1;
        }
        if let Some(search_term) = search {
            let wildcard = format!("%{}%", search_term.to_lowercase());
            query.push_str(&format!(
                "{} LOWER(json_payload) LIKE ?{}",
                if where_added {
                    " AND"
                } else {
                    where_added = true;
                    " WHERE"
                },
                idx
            ));
            bind.push(Box::new(wildcard));
        }

        let count: i64 = conn
            .query_row(
                &query,
                bind.iter()
                    .map(|b| b.as_ref())
                    .collect::<Vec<_>>()
                    .as_slice(),
                |row| row.get(0),
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "count filtered events".to_owned(),
                    message: e.to_string(),
                })
            })?;

        Ok(count)
    }
}

/// Parse a single event row from SQLite query results
fn parse_event_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    Ok(Event {
        id: Some(row.get(0)?),
        event_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    1,
                    "event_time".to_owned(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc),
        category: EventCategory::from_string(&row.get::<_, String>(2)?),
        subtype: row.get(3)?,
        severity: Severity::from_string(&row.get::<_, String>(4)?),
        mint: row.get(5)?,
        reference_id: row.get(6)?,
        payload: serde_json::from_str(&row.get::<_, String>(7)?).map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                7,
                "json_payload".to_owned(),
                rusqlite::types::Type::Text,
            )
        })?,
        created_at: row
            .get::<_, Option<String>>(8)?
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
    })
}
