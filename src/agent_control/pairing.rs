//! Durable client pairings: the credential that lets an external agent reach
//! the live-app bridge, and the per-connection policy that credential carries.
//!
//! A pairing secret is 256 bits of randomness shown exactly once at creation.
//! Only its SHA-256 verifier is stored. Because the secret is full-entropy
//! random, a plain hash (no KDF) is sufficient; the comparison is
//! constant-time. Unknown, malformed and revoked credentials are all rejected
//! with one indistinguishable error so the bridge cannot be used as an oracle.

use base64::Engine;
use rand::RngCore;
use rusqlite::OptionalExtension;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::agent_control::audit::{self, AuditContext, AuditKind};
use crate::agent_control::error::{Error, Result};
use crate::agent_control::store::{self, now_unix};
use crate::agent_control::ToolPermissions;

const MAX_LABEL: usize = 64;
const MAX_KIND: usize = 32;
const SECRET_BYTES: usize = 32;

/// Compared against `candidate` when the client id is unknown, so the auth path
/// spends the same work whether or not the client exists.
const DUMMY_VERIFIER: [u8; 32] = [0u8; 32];

/// A pairing as shown to a dashboard operator. Never carries the verifier or
/// the secret. `permissions` is what the settings dialog renders and edits.
#[derive(Debug, Clone, Serialize)]
pub struct PairingSummary {
    pub client_id: String,
    pub label: String,
    pub agent_kind: String,
    pub permissions: ToolPermissions,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked: bool,
}

/// The result of a successful bridge authentication.
#[derive(Debug, Clone)]
pub struct AuthedClient {
    pub client_id: String,
    pub label: String,
    pub permissions: ToolPermissions,
}

/// What `create` returns: the new client id and the one-time secret. The secret
/// is never persisted and never appears again. `Serialize` is retained so the
/// creation endpoint can return it exactly once; `Debug` is implemented by hand
/// so a stray `{:?}` (log line, error context, panic message) can never print
/// the secret.
#[derive(Clone, Serialize)]
pub struct NewPairing {
    pub client_id: String,
    pub pairing_secret: String,
}

impl std::fmt::Debug for NewPairing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewPairing")
            .field("client_id", &self.client_id)
            .field("pairing_secret", &"[redacted]")
            .finish()
    }
}

fn validate_label(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_LABEL {
        return Err(Error::InvalidPairingRequest {
            detail: format!("label must be 1..={MAX_LABEL} characters"),
        });
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(Error::InvalidPairingRequest {
            detail: "label must not contain control characters".to_owned(),
        });
    }
    Ok(trimmed.to_owned())
}

fn validate_kind(raw: &str) -> Result<String> {
    let value = raw.trim();
    let ok = (1..=MAX_KIND).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if !ok {
        return Err(Error::InvalidPairingRequest {
            detail: format!("agent_kind must be a 1..={MAX_KIND} char slug of [a-z0-9_-]"),
        });
    }
    Ok(value.to_owned())
}

fn generate_secret() -> String {
    let mut bytes = [0u8; SECRET_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn verifier_of(secret: &str) -> [u8; 32] {
    let digest = Sha256::digest(secret.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Create a pairing. Validates the label and kind, stores only the verifier,
/// and returns the secret once.
///
/// `permissions` is the connection's own policy. `None` means the default a new
/// connection gets: full access, which the owner then limits per connection in
/// Settings → Agent Connections.
pub fn create(
    label: &str,
    agent_kind: &str,
    permissions: Option<ToolPermissions>,
) -> Result<NewPairing> {
    let label = validate_label(label)?;
    let agent_kind = validate_kind(agent_kind)?;
    let permissions = permissions.unwrap_or_else(ToolPermissions::full_access);

    let client_id = uuid::Uuid::new_v4().to_string();
    let secret = generate_secret();
    let verifier = verifier_of(&secret);

    let connection = store::conn()?;
    connection.execute(
        "INSERT INTO pairings (client_id, label, agent_kind, permissions, verifier, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            client_id,
            label,
            agent_kind,
            permissions.to_json(),
            verifier.to_vec(),
            now_unix(),
        ],
    )?;

    audit::record(
        AuditKind::PairingCreated,
        &AuditContext {
            client_id: Some(client_id.clone()),
            ..Default::default()
        },
        "created",
        Some(&format!(
            "label={label:?} kind={agent_kind} permissions={}",
            permissions.to_json()
        )),
    );

    Ok(NewPairing {
        client_id,
        pairing_secret: secret,
    })
}

/// Replace an active pairing's policy. Returns `false` when there is no such
/// active pairing. The bridge reads the policy on every request, so a change
/// takes effect on the connection's very next call.
pub fn set_permissions(client_id: &str, permissions: ToolPermissions) -> Result<bool> {
    let connection = store::conn()?;
    let changed = connection.execute(
        "UPDATE pairings SET permissions = ?1 WHERE client_id = ?2 AND revoked_at IS NULL",
        rusqlite::params![permissions.to_json(), client_id],
    )?;
    if changed > 0 {
        audit::record(
            AuditKind::PairingCreated,
            &AuditContext {
                client_id: Some(client_id.to_owned()),
                ..Default::default()
            },
            "permissions_updated",
            Some(&permissions.to_json()),
        );
    }
    Ok(changed > 0)
}

/// A stored policy, failing closed: an unreadable value grants nothing.
fn stored_permissions(raw: &str) -> ToolPermissions {
    ToolPermissions::from_json(raw).unwrap_or_else(ToolPermissions::denied)
}

/// All pairings, newest first. Never includes the verifier.
pub fn list() -> Result<Vec<PairingSummary>> {
    let connection = store::conn()?;
    let mut stmt = connection.prepare(
        "SELECT client_id, label, agent_kind, permissions, created_at, last_used_at, revoked_at
               FROM pairings ORDER BY created_at DESC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let revoked_at: Option<i64> = row.get(6)?;
            let permissions: String = row.get(3)?;
            Ok(PairingSummary {
                client_id: row.get(0)?,
                label: row.get(1)?,
                agent_kind: row.get(2)?,
                permissions: stored_permissions(&permissions),
                created_at: row.get(4)?,
                last_used_at: row.get(5)?,
                revoked: revoked_at.is_some(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Revoke a pairing. Returns `true` if a row moved from active to revoked. The
/// bridge resolves the policy from this table on every request, so revocation takes
/// effect on the very next bridge call.
pub fn revoke(client_id: &str) -> Result<bool> {
    let connection = store::conn()?;
    let changed = connection.execute(
        "UPDATE pairings SET revoked_at = ?1 WHERE client_id = ?2 AND revoked_at IS NULL",
        rusqlite::params![now_unix(), client_id],
    )?;
    if changed > 0 {
        audit::record(
            AuditKind::PairingRevoked,
            &AuditContext {
                client_id: Some(client_id.to_owned()),
                ..Default::default()
            },
            "revoked",
            None,
        );
    }
    Ok(changed > 0)
}

/// The current policy of an active (non-revoked) pairing, or `None` if the
/// pairing is unknown or revoked. Used when re-checking policy just before a
/// human-approved execution runs, where no secret is available to re-run
/// `authenticate`.
pub fn active_permissions(client_id: &str) -> Result<Option<ToolPermissions>> {
    let connection = store::conn()?;
    let row: Option<(String, Option<i64>)> = connection
        .query_row(
            "SELECT permissions, revoked_at FROM pairings WHERE client_id = ?1",
            rusqlite::params![client_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    Ok(match row {
        Some((permissions, None)) => Some(stored_permissions(&permissions)),
        _ => None,
    })
}

/// Authenticate a bridge request. Every failure mode — missing/blank fields,
/// unknown client id, wrong secret, revoked pairing — returns
/// `Error::PairingRejected`, and the work done is the same in each case. An
/// unreadable stored policy authenticates but grants nothing.
pub fn authenticate(client_id: &str, secret: &str) -> Result<AuthedClient> {
    let candidate = verifier_of(secret);

    let connection = store::conn()?;
    let row: Option<(Vec<u8>, String, Option<i64>, String)> = connection
        .query_row(
            "SELECT verifier, permissions, revoked_at, label FROM pairings WHERE client_id = ?1",
            rusqlite::params![client_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;

    let Some((verifier, permissions_json, revoked_at, label)) = row else {
        // Equalize timing/branch shape against the "found" path.
        let _ = constant_time_eq::constant_time_eq(&candidate, &DUMMY_VERIFIER);
        return Err(Error::PairingRejected);
    };

    let secret_ok = verifier.len() == candidate.len()
        && constant_time_eq::constant_time_eq(&candidate, &verifier);
    let not_revoked = revoked_at.is_none();

    if !(secret_ok && not_revoked) {
        return Err(Error::PairingRejected);
    }

    connection.execute(
        "UPDATE pairings SET last_used_at = ?1 WHERE client_id = ?2",
        rusqlite::params![now_unix(), client_id],
    )?;

    Ok(AuthedClient {
        client_id: client_id.to_owned(),
        label,
        permissions: stored_permissions(&permissions_json),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_slug_is_bounded() {
        assert!(validate_kind("claude-code").is_ok());
        assert!(validate_kind("Bad Kind").is_err());
        assert!(validate_kind("").is_err());
        assert!(validate_kind(&"x".repeat(33)).is_err());
    }

    #[test]
    fn label_is_bounded_and_trimmed() {
        assert_eq!(validate_label("  Desk agent  ").unwrap(), "Desk agent");
        assert!(validate_label("").is_err());
        assert!(validate_label(&"x".repeat(65)).is_err());
    }

    #[test]
    fn new_pairing_debug_never_prints_the_secret() {
        let created = NewPairing {
            client_id: "client-abc".to_owned(),
            pairing_secret: "SUPER-SECRET-ONE-TIME-VALUE".to_owned(),
        };
        let rendered = format!("{created:?}");
        assert!(!rendered.contains("SUPER-SECRET-ONE-TIME-VALUE"));
        assert!(rendered.contains("[redacted]"));
        assert!(rendered.contains("client-abc"));
    }

    #[test]
    fn secret_is_high_entropy_and_verifier_is_one_way() {
        let a = generate_secret();
        let b = generate_secret();
        assert_ne!(a, b);
        assert!(a.len() >= 43);
        assert_ne!(verifier_of(&a).to_vec(), a.into_bytes());
    }

    // Roundtrip / revocation / oracle-uniformity are exercised against a real
    // temp database in `tests/agent_control_store.rs`.
}
