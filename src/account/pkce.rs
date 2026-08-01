//! PKCE (RFC 7636) and the loopback callback's one-time state.
//!
//! ============================================================================
//! WHY A PUBLIC REPOSITORY IS NOT A PROBLEM HERE
//! ============================================================================
//! This binary holds no client secret, because PKCE does not use one. The app
//! invents a random VERIFIER, sends only its SHA-256 hash to the server, and
//! reveals the verifier when it redeems the authorization code. Anyone who
//! intercepts the code — from a browser history, a shell log, another process
//! watching the loopback port — cannot redeem it without the verifier, which
//! never left this process.
//!
//! That is what lets a program whose source anyone can read authenticate
//! safely, and it is why the alternative (embedding a Google desktop client id
//! and secret in the release) is not on the table: a secret in a public binary
//! is not a secret, secret scanners flag it, and it offers no way to revoke a
//! single user.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// One in-flight browser authorization.
#[derive(Debug, Clone)]
pub struct PendingAuth {
    /// Kept here and revealed only at the token exchange.
    pub verifier: String,
    /// Sent to the server in the browser URL.
    pub challenge: String,
    /// Echoed back by the callback. Proves the redirect belongs to THIS attempt
    /// and not to a page that guessed the port.
    pub state: String,
    /// The exact loopback URL registered with the server. The token exchange
    /// sends it again and the server compares — a mismatch is refused.
    pub redirect_uri: String,
    pub created_at: std::time::Instant,
}

fn random_base64(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buffer);
    URL_SAFE_NO_PAD.encode(buffer)
}

impl PendingAuth {
    /// 32 random bytes encode to 43 base64url characters — the minimum RFC 7636
    /// permits, and 256 bits of entropy.
    pub fn new(redirect_uri: String) -> Self {
        let verifier = random_base64(32);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

        Self {
            verifier,
            challenge,
            state: random_base64(32),
            redirect_uri,
            created_at: std::time::Instant::now(),
        }
    }

    /// Ten minutes, matching the server's own window for a consent screen.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > std::time::Duration::from_secs(10 * 60)
    }

    /// Constant-time comparison of the state a browser handed back.
    ///
    /// A plain `==` on a secret leaks its prefix through timing. The cost of
    /// doing it properly is four lines.
    pub fn state_matches(&self, supplied: &str) -> bool {
        let expected = self.state.as_bytes();
        let actual = supplied.as_bytes();
        if expected.len() != actual.len() {
            return false;
        }

        let mut difference = 0u8;
        for (a, b) in expected.iter().zip(actual.iter()) {
            difference |= a ^ b;
        }
        difference == 0
    }
}
