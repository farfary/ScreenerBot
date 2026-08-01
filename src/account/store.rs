//! Where the refresh token lives on disk.
//!
//! ============================================================================
//! THE THREAT MODEL, STATED PLAINLY
//! ============================================================================
//! Tokens are encrypted with `secure_storage` — AES-256-GCM under a key derived
//! from the machine id. That protects against a stolen file (a synced folder, a
//! backup, a support upload); it does NOT protect against code running as this
//! user on this machine, which can derive the same key.
//!
//! That is an honest limit rather than a gap, because the same directory holds
//! the encrypted TRADING KEY under the same scheme. An attacker who can decrypt
//! one can decrypt the other, and of the two the trading key is worth far more.
//! A refresh token adds no new class of exposure — and unlike the wallet it can
//! be revoked from the dashboard the moment it is suspected.
//!
//! What does NOT go here: the access token (15 minutes, held in memory only)
//! and passwords (never stored at all, at any point, in any form).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths::get_data_directory;
use crate::secure_storage::{decrypt_private_key, encrypt_private_key, EncryptedData};

/// Deliberately not inside `config.toml`: that file is plain text and gets
/// shared. This one is opaque and gets ignored.
const STORE_FILE: &str = "account.dat";

/// What survives a restart. The access token is absent on purpose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub refresh_token: String,
    pub device_id: String,
    pub scopes: Vec<String>,
    /// Cached for display only, so the settings dialog can show who is signed
    /// in before the first refresh round trip completes.
    pub account_label: Option<String>,
    pub account_email: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct EncryptedEnvelope {
    ciphertext: String,
    nonce: String,
}

fn store_path() -> PathBuf {
    get_data_directory().join(STORE_FILE)
}

/// Read the stored session, or None when there is none.
///
/// Every failure — missing file, corrupt file, a machine id that has changed
/// because the user restored to new hardware — is "not signed in". A user whose
/// token cannot be decrypted needs a sign-in button, not an error dialog about
/// cryptography.
pub fn load() -> Option<StoredSession> {
    let path = store_path();
    let raw = std::fs::read_to_string(&path).ok()?;

    let envelope: EncryptedEnvelope = serde_json::from_str(&raw).ok()?;
    let plaintext = decrypt_private_key(&EncryptedData {
        ciphertext: envelope.ciphertext,
        nonce: envelope.nonce,
    })
    .ok()?;

    serde_json::from_str(&plaintext).ok()
}

pub fn save(session: &StoredSession) -> Result<(), String> {
    let plaintext =
        serde_json::to_string(session).map_err(|e| format!("could not serialize session: {e}"))?;

    let encrypted = encrypt_private_key(&plaintext)?;

    let envelope = EncryptedEnvelope {
        ciphertext: encrypted.ciphertext,
        nonce: encrypted.nonce,
    };

    let body = serde_json::to_string(&envelope)
        .map_err(|e| format!("could not serialize session envelope: {e}"))?;

    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("could not create data dir: {e}"))?;
    }

    std::fs::write(&path, body).map_err(|e| format!("could not write session: {e}"))?;

    // Owner-only. The file is encrypted, but a token readable by every account
    // on a shared machine is one unnecessary step closer to being used.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

/// Remove the stored session. Signing out must leave nothing behind, so a
/// missing file is success rather than an error.
pub fn clear() -> Result<(), String> {
    match std::fs::remove_file(store_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not remove session: {error}")),
    }
}
