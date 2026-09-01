//! Whether ScreenerBot data is available to this install, and what to say if not.
//!
//! ============================================================================
//! WHY THE SENTENCE LIVES IN RUST
//! ============================================================================
//! Three surfaces explain this to the same person: the first-run introduction,
//! the setup screen's account panel, and Settings. If each composed its own
//! wording they would drift, and the one that drifted would be the one the user
//! read first. So the state carries its own headline and detail, every surface
//! renders what it is given, and improving the sentence is a one-line change.
//!
//! ============================================================================
//! WHY THIS IS NEVER AN ERROR
//! ============================================================================
//! Signing in ADDS. Every consumer of ScreenerBot data keeps its direct-provider
//! fallback, so "unavailable" costs a shared cache and nothing else — no trade
//! is blocked, no chart is empty that would otherwise have filled. The UI says
//! what is missing and what to do about it; it does not raise an alarm.

use std::sync::LazyLock;

use arc_swap::ArcSwap;
use serde::Serialize;

/// Where this install stands with the ScreenerBot data service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataAccess {
    /// Working.
    Ready,
    /// The user turned the ScreenerBot source off in config.
    Disabled,
    /// No network. Says nothing about the account.
    Offline,
    /// No account on this machine, so there is nothing to authenticate with.
    SignedOut,
    /// Signed in, but this device's grant predates ScreenerBot data access.
    ReauthorizationRequired,
    /// This build is older than the service will answer.
    VersionUnsupported { minimum: String },
    /// Reachable in principle, not answering now.
    Unreachable,
    /// Not asked yet this run.
    Unknown,
}

impl DataAccess {
    pub fn key(&self) -> &'static str {
        match self {
            DataAccess::Ready => "ready",
            DataAccess::Disabled => "disabled",
            DataAccess::Offline => "offline",
            DataAccess::SignedOut => "signed_out",
            DataAccess::ReauthorizationRequired => "reauthorization_required",
            DataAccess::VersionUnsupported { .. } => "version_unsupported",
            DataAccess::Unreachable => "unreachable",
            DataAccess::Unknown => "unknown",
        }
    }

    pub fn available(&self) -> bool {
        matches!(self, DataAccess::Ready)
    }

    /// The short line, written as a statement of fact.
    pub fn headline(&self) -> &'static str {
        match self {
            DataAccess::Ready => "ScreenerBot data is active",
            DataAccess::Disabled => "ScreenerBot data is switched off",
            DataAccess::Offline => "ScreenerBot data is offline",
            DataAccess::SignedOut => "ScreenerBot data needs an account",
            DataAccess::ReauthorizationRequired => "ScreenerBot data needs you to sign in again",
            DataAccess::VersionUnsupported { .. } => "ScreenerBot data needs a newer version",
            DataAccess::Unreachable => "ScreenerBot data is not responding",
            DataAccess::Unknown => "ScreenerBot data has not been checked yet",
        }
    }

    /// The explanation, which must always answer "so what happens instead?".
    pub fn detail(&self) -> String {
        match self {
            DataAccess::Ready => "Shared candles, pool registry, security reports and token \
                                  identity are being served from screenerbot.io."
                .to_string(),
            DataAccess::Disabled => "The ScreenerBot source is turned off in your settings, so \
                                     data comes from the public providers only."
                .to_string(),
            DataAccess::Offline => "There is no network connection. Data will resume on its own \
                                    once the connection returns."
                .to_string(),
            DataAccess::SignedOut => "Charts, pools, security reports and token identity come \
                                      from the public providers instead. They are slower, rate \
                                      limited, and thinner on history. Signing in is free and \
                                      changes nothing else about how ScreenerBot runs."
                .to_string(),
            DataAccess::ReauthorizationRequired => {
                "This device was authorised before ScreenerBot data existed. Sign in again to \
                 restore it — the public providers are being used until then."
                    .to_string()
            }
            DataAccess::VersionUnsupported { minimum } => format!(
                "This version is no longer served. Update to {minimum} or newer to use \
                 ScreenerBot data again; the public providers are being used until then."
            ),
            DataAccess::Unreachable => "The service did not answer. The public providers are \
                                        being used, and ScreenerBot will keep retrying."
                .to_string(),
            DataAccess::Unknown => {
                "ScreenerBot has not yet needed shared data this session.".to_string()
            }
        }
    }

    /// True when the person can fix this themselves right now.
    pub fn actionable(&self) -> bool {
        matches!(
            self,
            DataAccess::SignedOut
                | DataAccess::ReauthorizationRequired
                | DataAccess::VersionUnsupported { .. }
                | DataAccess::Disabled
        )
    }
}

/// What every surface renders. Never a token, never an endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct DataAccessStatus {
    pub state: &'static str,
    pub available: bool,
    pub actionable: bool,
    pub headline: &'static str,
    pub detail: String,
    /// Present only for `version_unsupported`, so the UI can name the release.
    pub minimum_version: Option<String>,
    /// When the state was last established, or null if never.
    pub checked_at: Option<i64>,
}

#[derive(Debug, Clone)]
struct Snapshot {
    access: DataAccess,
    checked_at: Option<i64>,
}

static STATE: LazyLock<ArcSwap<Snapshot>> = LazyLock::new(|| {
    ArcSwap::from_pointee(Snapshot {
        access: DataAccess::Unknown,
        checked_at: None,
    })
});

/// Record the outcome of a call. Cheap enough to run on every request.
pub fn record(access: DataAccess) {
    let previous = STATE.load();
    if previous.access == access {
        // Same answer as last time: refresh the timestamp only. Logging every
        // repeat would fill the log with "still signed out" while a bot polls.
        STATE.store(std::sync::Arc::new(Snapshot {
            access,
            checked_at: Some(chrono::Utc::now().timestamp()),
        }));
        return;
    }

    // A transition is worth one line, at a level that matches what it means: a
    // missing account is a choice the user made, not a fault.
    let message = format!("Data Server: {}", access.headline());
    match access {
        DataAccess::Ready => crate::logger::info(crate::logger::LogTag::System, &message),
        DataAccess::Unreachable => crate::logger::warning(crate::logger::LogTag::System, &message),
        _ => crate::logger::info(crate::logger::LogTag::System, &message),
    }

    STATE.store(std::sync::Arc::new(Snapshot {
        access,
        checked_at: Some(chrono::Utc::now().timestamp()),
    }));
}

pub fn current() -> DataAccess {
    STATE.load().access.clone()
}

/// Forget a refusal that a session change may have answered.
///
/// `ReauthorizationRequired` and `VersionUnsupported` are the two states
/// `is_usable` treats as sticky, because neither can change on its own. Signing
/// in again is exactly what answers the first, so a stale refusal must not
/// survive it — otherwise the user does what the app asked and the app carries
/// on declining to try.
///
/// Deliberately NOT a full reset. This runs on every token rotation as well as
/// on a real sign-in, and wiping a healthy `Ready` every fifteen minutes would
/// make the Settings panel say "not checked yet" to somebody whose data is
/// working perfectly.
pub fn forget_refusals() {
    let current = STATE.load();
    if !matches!(
        current.access,
        DataAccess::ReauthorizationRequired | DataAccess::VersionUnsupported { .. }
    ) {
        return;
    }
    STATE.store(std::sync::Arc::new(Snapshot {
        access: DataAccess::Unknown,
        checked_at: current.checked_at,
    }));
}

/// The recorded state, as the UI renders it.
///
/// Callers that know more than the record does — `account::status` knows whether
/// a session exists at all — correct it with [`describe`] rather than by
/// composing a second set of sentences.
pub fn status() -> DataAccessStatus {
    let snapshot = STATE.load();
    describe(snapshot.access.clone(), snapshot.checked_at)
}

/// Render any state into what the UI shows.
pub fn describe(access: DataAccess, checked_at: Option<i64>) -> DataAccessStatus {
    DataAccessStatus {
        state: access.key(),
        available: access.available(),
        actionable: access.actionable(),
        headline: access.headline(),
        detail: access.detail(),
        minimum_version: match &access {
            DataAccess::VersionUnsupported { minimum } => Some(minimum.clone()),
            _ => None,
        },
        checked_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The state under test is one process-wide `ArcSwap`, so the cases that
    /// write to it must not run beside each other. Without this they interleave
    /// and fail on whichever value the other test had just stored.
    static SERIALIZE: Mutex<()> = Mutex::new(());

    #[test]
    fn every_state_explains_what_happens_instead() {
        let states = [
            DataAccess::Ready,
            DataAccess::Disabled,
            DataAccess::Offline,
            DataAccess::SignedOut,
            DataAccess::ReauthorizationRequired,
            DataAccess::VersionUnsupported {
                minimum: "0.2.0".to_string(),
            },
            DataAccess::Unreachable,
            DataAccess::Unknown,
        ];

        for state in states {
            assert!(!state.headline().is_empty(), "{:?}", state.key());
            assert!(state.detail().len() > 40, "{:?}", state.key());
            // Only the working state may claim to be working.
            assert_eq!(state.available(), state == DataAccess::Ready);
        }
    }

    #[test]
    fn the_version_refusal_names_the_release_to_update_to() {
        let state = DataAccess::VersionUnsupported {
            minimum: "1.4.2".to_string(),
        };
        assert!(state.detail().contains("1.4.2"));
    }

    #[test]
    fn forgetting_clears_only_a_refusal_a_new_session_could_answer() {
        let _guard = SERIALIZE.lock().unwrap_or_else(|e| e.into_inner());

        // A working state must survive a token rotation: `forget_refusals` runs
        // on every refresh, not only on a real sign-in.
        record(DataAccess::Ready);
        forget_refusals();
        assert_eq!(current(), DataAccess::Ready);

        record(DataAccess::Unreachable);
        forget_refusals();
        assert_eq!(current(), DataAccess::Unreachable);

        // The two states `is_usable` treats as sticky are exactly the two that a
        // sign-in or an upgrade can answer, so both must clear.
        record(DataAccess::ReauthorizationRequired);
        forget_refusals();
        assert_eq!(current(), DataAccess::Unknown);

        record(DataAccess::VersionUnsupported {
            minimum: "1.0.0".to_string(),
        });
        forget_refusals();
        assert_eq!(current(), DataAccess::Unknown);
    }

    #[test]
    fn recording_publishes_the_state_and_a_timestamp() {
        let _guard = SERIALIZE.lock().unwrap_or_else(|e| e.into_inner());

        record(DataAccess::SignedOut);
        let signed_out = status();
        assert_eq!(signed_out.state, "signed_out");
        assert!(!signed_out.available);
        assert!(signed_out.actionable);
        assert!(signed_out.checked_at.is_some());
        assert!(signed_out.minimum_version.is_none());

        record(DataAccess::VersionUnsupported {
            minimum: "9.9.9".to_string(),
        });
        assert_eq!(status().minimum_version.as_deref(), Some("9.9.9"));
    }
}
