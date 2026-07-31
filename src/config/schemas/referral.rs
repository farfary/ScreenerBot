//! Referral attribution — an OPT-IN way to credit whoever introduced you.
//!
//! ============================================================================
//! THIS IS THE ONLY PART OF SCREENERBOT THAT REPORTS ANYTHING ABOUT YOU, AND IT
//! IS OFF UNTIL YOU TYPE A CODE.
//! ============================================================================
//!
//! With `code` empty — the default — nothing is ever sent anywhere. No ping, no
//! heartbeat, no wallet address, no telemetry of any kind. The activation
//! request is not made at all.
//!
//! With a code set, ONE request goes to screenerbot.io carrying exactly:
//!
//!     { referral_code, wallet_pubkeys[], platform, version }
//!
//! `wallet_pubkeys` are PUBLIC addresses — the same strings any block explorer
//! shows for your trades. A private key never leaves this machine and is never
//! read by this module. The addresses are needed because that is the only way
//! the website can match a swap fee it observed on chain to the person who
//! referred you: the fee transaction names your wallet, and nothing else about
//! it says where you came from.
//!
//! This repository is public. That claim is checkable by reading
//! `src/services/implementations/referral_service.rs`, which is the only code
//! that sends anything, and it is why the guarantee is worth stating.

use crate::config_struct;
use crate::field_metadata;

config_struct! {
    /// Referral attribution settings.
    pub struct ReferralConfig {
        /// The referral code of whoever introduced you. EMPTY = the feature is
        /// entirely inert and nothing is sent.
        ///
        /// Stored uppercase; the server normalizes the same way, so case never
        /// matters to a user typing it off a video.
        #[metadata(field_metadata! {
            label: "Referral code",
            hint: "Optional. If someone introduced you to ScreenerBot, their code credits them with a share of the fees you pay us — at no extra cost to you. Leave empty and nothing is ever sent from this machine.",
            placeholder: "e.g. FARHAD",
            category: "General"
        })]
        code: String = String::new(),

        /// Where the activation is sent. Configurable so the devnet/staging
        /// harness can point elsewhere; never a value a user needs to change.
        #[metadata(field_metadata! {
            label: "Activation endpoint",
            hint: "Where the referral code is registered. Only change this if you are testing against a staging server.",
            category: "Debug"
        })]
        endpoint: String = "https://screenerbot.io/api/referral/activate".to_string(),

        /// Re-announce this often (hours) so a wallet added after the first
        /// activation still gets attributed. 0 disables re-announcing.
        ///
        /// Daily rather than per-launch: the server upserts by wallet, so a
        /// faster cadence would add load and change nothing.
        #[metadata(field_metadata! {
            label: "Re-announce every (hours)",
            hint: "How often to re-send, so a wallet you add later is still attributed. 0 sends only once per launch.",
            category: "Debug"
        })]
        reannounce_hours: u64 = 24,
    }
}

impl ReferralConfig {
    /// Is the feature switched on at all? Everything the service does is gated
    /// on this returning true.
    pub fn is_enabled(&self) -> bool {
        !self.code.trim().is_empty()
    }

    /// The code as the server expects it: uppercase, trimmed, and stripped of
    /// anything outside the code alphabet, so a pasted "  farhad!" still works.
    pub fn normalized_code(&self) -> String {
        self.code
            .trim()
            .to_uppercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect()
    }
}
