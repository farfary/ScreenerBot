//! The agent-control audit log and the redaction policy that guards it.
//!
//! Every pairing lifecycle change, bridge authentication outcome, authorization
//! decision, approval transition and tool execution is recorded here. Nothing
//! written to this table may contain a pairing secret, a private key or raw
//! configuration/tool payloads that could carry one — `sanitize` is the single
//! chokepoint that enforces that.

use serde::Serialize;
use serde_json::Value;

use crate::agent_control::error::Result;
use crate::agent_control::store::{self, now_unix};
use crate::logger::{self, LogTag};

/// Longest single string kept verbatim in a redacted summary.
const MAX_STR: usize = 256;
/// Longest serialized summary persisted or returned (display text, not parsed).
const MAX_SUMMARY_BYTES: usize = 2_048;
/// Longest serialized size for a stored approval result, after redaction. The
/// result is parsed back on retry/poll, so this path must stay valid JSON.
const MAX_RESULT_JSON_BYTES: usize = 4_096;
/// Longest client id / tool / correlation id / outcome kept in an audit row.
const MAX_CONTEXT: usize = 128;
const MAX_ARRAY: usize = 20;
const MAX_OBJECT_KEYS: usize = 40;
const MAX_DEPTH: usize = 6;

/// A kind of audit event. Kept as a small closed vocabulary so the read API can
/// filter on it without free-text matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditKind {
    PairingCreated,
    PairingRevoked,
    BridgeAuth,
    ToolRequest,
    AuthzDecision,
    ApprovalCreated,
    ApprovalDecided,
    ApprovalExpired,
    Execution,
}

impl AuditKind {
    fn as_str(self) -> &'static str {
        match self {
            AuditKind::PairingCreated => "pairing_created",
            AuditKind::PairingRevoked => "pairing_revoked",
            AuditKind::BridgeAuth => "bridge_auth",
            AuditKind::ToolRequest => "tool_request",
            AuditKind::AuthzDecision => "authz_decision",
            AuditKind::ApprovalCreated => "approval_created",
            AuditKind::ApprovalDecided => "approval_decided",
            AuditKind::ApprovalExpired => "approval_expired",
            AuditKind::Execution => "execution",
        }
    }
}

/// Correlation fields shared by most audit events.
#[derive(Debug, Default, Clone)]
pub struct AuditContext {
    pub client_id: Option<String>,
    pub tool: Option<String>,
    pub correlation_id: Option<String>,
}

/// One row of the audit log as returned by the read API.
#[derive(Debug, Clone, Serialize)]
pub struct AuditRecord {
    pub id: i64,
    pub ts: i64,
    pub kind: String,
    pub client_id: Option<String>,
    pub tool: Option<String>,
    pub correlation_id: Option<String>,
    pub outcome: String,
    pub detail: Option<String>,
}

/// Append one audit row. A persistence failure is logged and swallowed — the
/// audit log must never be able to abort a security decision or a tool call.
pub fn record(kind: AuditKind, ctx: &AuditContext, outcome: &str, detail: Option<&str>) {
    if let Err(e) = try_record(kind, ctx, outcome, detail) {
        logger::debug(
            LogTag::Security,
            &format!(
                "agent-control: failed to write audit row ({}): {e}",
                kind.as_str()
            ),
        );
    }
}

fn try_record(
    kind: AuditKind,
    ctx: &AuditContext,
    outcome: &str,
    detail: Option<&str>,
) -> Result<()> {
    let connection = store::conn()?;
    connection.execute(
        "INSERT INTO audit (ts, kind, client_id, tool, correlation_id, outcome, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            now_unix(),
            kind.as_str(),
            ctx.client_id.as_deref().map(|s| clamp(s, MAX_CONTEXT)),
            ctx.tool.as_deref().map(|s| clamp(s, MAX_CONTEXT)),
            ctx.correlation_id.as_deref().map(|s| clamp(s, MAX_CONTEXT)),
            clamp(outcome, MAX_CONTEXT),
            detail.map(|d| clamp(d, MAX_SUMMARY_BYTES)),
        ],
    )?;
    // Enforce the retention window and the hard row cap on the same connection,
    // immediately — the table never exceeds `AUDIT_MAX_ROWS` between the hourly
    // sweeps. `store::prune_audit` runs only DELETEs, so this cannot recurse.
    store::prune_audit(&connection)?;
    Ok(())
}

/// Page of audit rows, newest first.
pub fn list(page: u32, per_page: u32) -> Result<(Vec<AuditRecord>, i64)> {
    let per_page = per_page.clamp(1, 200) as i64;
    let offset = (page.max(1) as i64 - 1) * per_page;
    let connection = store::conn()?;

    let total: i64 = connection.query_row("SELECT COUNT(*) FROM audit", [], |r| r.get(0))?;

    let mut stmt = connection.prepare(
        "SELECT id, ts, kind, client_id, tool, correlation_id, outcome, detail
               FROM audit ORDER BY id DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![per_page, offset], |row| {
            Ok(AuditRecord {
                id: row.get(0)?,
                ts: row.get(1)?,
                kind: row.get(2)?,
                client_id: row.get(3)?,
                tool: row.get(4)?,
                correlation_id: row.get(5)?,
                outcome: row.get(6)?,
                detail: row.get(7)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok((rows, total))
}

/// Produce a bounded, redacted one-line summary of an arbitrary value — used
/// for `approvals.args_summary` and audit `detail`, both of which are DISPLAY
/// TEXT and never parsed back. Sensitive-looking keys are dropped, strings are
/// truncated, and the whole thing is capped (with an explicit `[truncated]`
/// marker if the structural caps were not enough).
pub fn sanitize(value: &Value) -> String {
    let redacted = redact(value, 0);
    let mut s = redacted.to_string();
    if s.len() > MAX_SUMMARY_BYTES {
        s.truncate(MAX_SUMMARY_BYTES);
        while !s.is_char_boundary(s.len()) {
            s.pop();
        }
        s.push_str(" …[truncated]");
    }
    s
}

/// Redact + structurally bound a value AND guarantee the serialized form is
/// valid JSON under `MAX_RESULT_JSON_BYTES`. Used for `approvals.result_json`,
/// which is parsed back on retry and poll — a mid-string truncation there would
/// make a completed request permanently lose its result.
pub fn sanitize_value(value: &Value) -> Value {
    let redacted = redact(value, 0);
    if json_len(&redacted) <= MAX_RESULT_JSON_BYTES {
        return redacted;
    }
    // Still too large after the structural caps: collapse to a valid
    // placeholder, keeping the small scalar fields a caller reasons about.
    match &redacted {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for key in ["success", "ok", "error", "code", "message", "status"] {
                if let Some(v) = map.get(key) {
                    if json_len(v) <= 512 {
                        out.insert(key.to_owned(), v.clone());
                    }
                }
            }
            out.insert(
                "data".to_owned(),
                Value::String(format!(
                    "[omitted: redacted result exceeded {MAX_RESULT_JSON_BYTES} bytes]"
                )),
            );
            Value::Object(out)
        }
        _ => Value::String(format!(
            "[omitted: redacted result exceeded {MAX_RESULT_JSON_BYTES} bytes]"
        )),
    }
}

fn json_len(value: &Value) -> usize {
    serde_json::to_string(value)
        .map(|s| s.len())
        .unwrap_or(usize::MAX)
}

/// Whole-segment match against known secret-bearing field names. Segments are
/// split on any non-alphanumeric character so `api_key`, `api-key`,
/// `X-ScreenerBot-Pairing-Secret` and `pairingSecret` all match.
fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    if lower.contains("secret") || lower.contains("mnemonic") || lower.contains("passphrase") {
        return true;
    }
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|seg| {
            matches!(
                seg,
                "password" | "passwd" | "pwd" | "seed" | "seedphrase" | "privatekey" | "apikey"
                    | "authorization" | "auth" | "credential" | "credentials" | "token" | "bearer"
                    | "cookie" | "session"
            )
        })
        // `token` on its own is a secret; `token_mint` / `tokenAddress` are not.
        && !lower.contains("token_mint")
        && !lower.contains("tokenmint")
        && !lower.contains("token_address")
        && !lower.contains("tokenaddress")
}

fn redact(value: &Value, depth: usize) -> Value {
    if depth >= MAX_DEPTH {
        return Value::String("[depth-limited]".to_owned());
    }
    match value {
        Value::String(s) => Value::String(clamp(s, MAX_STR)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .take(MAX_ARRAY)
                .map(|v| redact(v, depth + 1))
                .collect(),
        ),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map.iter().take(MAX_OBJECT_KEYS) {
                if is_sensitive_key(k) {
                    out.insert(k.clone(), Value::String("[redacted]".to_owned()));
                } else {
                    out.insert(k.clone(), redact(v, depth + 1));
                }
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn clamp(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_owned();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_secret_bearing_keys_but_keeps_token_mint() {
        let v = json!({
            "pairing_secret": "abc",
            "apiKey": "xyz",
            "X-ScreenerBot-Pairing-Secret": "s",
            "token_mint": "So11111111111111111111111111111111111111112",
            "amount_sol": 1.5
        });
        let out = sanitize(&v);
        assert!(!out.contains("abc") && !out.contains("xyz"));
        assert!(out.contains("[redacted]"));
        assert!(out.contains("So11111111111111111111111111111111111111112"));
        assert!(out.contains("amount_sol"));
    }

    #[test]
    fn caps_total_length() {
        let big = "x".repeat(10_000);
        let out = sanitize(&json!({ "note": big }));
        assert!(out.len() <= MAX_SUMMARY_BYTES + 32);
    }

    #[test]
    fn sanitize_value_stays_valid_json_under_cap_with_no_secret() {
        // Oversized, deeply nested, secret-bearing tool output.
        let mut node = json!({ "api_key": "SUPER-SECRET", "blob": "z".repeat(4_000) });
        for _ in 0..10 {
            node = json!({ "secret": "leak", "child": node, "pad": "y".repeat(2_000) });
        }
        let payload = json!({ "success": false, "error": "boom", "data": node });

        let value = sanitize_value(&payload);
        let serialized = serde_json::to_string(&value).unwrap();
        // Must round-trip as valid JSON.
        let reparsed: Value = serde_json::from_str(&serialized).expect("valid JSON");
        assert!(serialized.len() <= MAX_RESULT_JSON_BYTES);
        assert!(!serialized.contains("SUPER-SECRET"));
        assert!(!serialized.contains("\"leak\""));
        // The scalar decision fields survive the collapse.
        assert_eq!(reparsed.get("success"), Some(&json!(false)));
        assert_eq!(reparsed.get("error"), Some(&json!("boom")));
    }

    #[test]
    fn sanitize_value_passes_small_results_through_redacted() {
        let value = sanitize_value(&json!({ "success": true, "pairing_secret": "x", "n": 1 }));
        assert_eq!(value.get("success"), Some(&json!(true)));
        assert_eq!(value.get("n"), Some(&json!(1)));
        assert_eq!(value.get("pairing_secret"), Some(&json!("[redacted]")));
    }
}
