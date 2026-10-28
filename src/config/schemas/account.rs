// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! ScreenerBot account settings.
//!
//! ============================================================================
//! WHAT IS DELIBERATELY *NOT* IN THIS FILE
//! ============================================================================
//! **The server address.** It is a compile-time constant in
//! `src/account/client.rs` and can never be a config field. `config.toml` is
//! plain text that people paste into bug reports and screenshots, and a
//! config-editable auth endpoint is a credential-phishing primitive: change one
//! line and the next sign-in posts an email and password to somebody else's
//! server. `referral.endpoint` IS editable because it carries nothing but
//! public wallet addresses; nothing here may follow that pattern.
//!
//! **Tokens.** Access and refresh tokens live in the encrypted store
//! (`src/account/store.rs`), never in the config file.
//!
//! An account is entirely OPTIONAL. Every switch here defaults to the
//! conservative answer, and with no account signed in the bot behaves exactly
//! as it always has.

use crate::config::metadata::ConfigCategory;
use crate::field_metadata;

config_struct! {
    /// ScreenerBot account settings.
    pub struct AccountConfig {
        /// Offer to sign in automatically when the main wallet already belongs
        /// to a screenerbot.io account.
        ///
        /// OFF by default, and deliberately so. Signing a message with the
        /// trading key without being asked is exactly what a malicious fork of
        /// this open-source bot would add; doing it silently by default would
        /// make that behaviour look normal. With this off, the app still
        /// DETECTS that the wallet has an account and offers a one-click
        /// sign-in — it just never signs on its own.
        #[metadata(field_metadata! {
            category: ConfigCategory::General
        })]
        auto_wallet_signin: bool = false,

        /// Route swap SUBMISSION through screenerbot.io's RPC when signed in.
        ///
        /// Submission only — the transaction is built and signed on this
        /// machine and the server merely broadcasts it. It cannot serve pool
        /// polling and is never asked to (see `src/rpc/provider`), so your own
        /// RPC is still required for the bot to work at all.
        ///
        /// OFF by default: relaying hands every signed transaction to a server
        /// other than the user's own RPC, so it is switched on by choice, never
        /// silently. A saved value is kept.
        #[metadata(field_metadata! {
            category: ConfigCategory::General
        })]
        use_gateway_rpc: bool = false,

        /// How long before expiry to refresh the access token, in seconds.
        ///
        /// A margin rather than a deadline: refreshing at the moment of expiry
        /// loses every request already in flight.
        #[metadata(field_metadata! {
            category: ConfigCategory::Debug
        })]
        refresh_margin_secs: u64 = 120,
    }
}

impl AccountConfig {
    /// Clamped so a hand-edited config cannot produce a margin longer than the
    /// token's own lifetime, which would refresh in a tight loop.
    pub fn refresh_margin(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.refresh_margin_secs.clamp(15, 600))
    }
}

#[cfg(test)]
mod tests {
    use super::AccountConfig;

    #[test]
    fn gateway_submission_is_off_by_default_and_a_saved_value_is_kept() {
        assert!(!AccountConfig::default().use_gateway_rpc);

        let unsaved: AccountConfig = toml::from_str("").expect("empty section parses");
        assert!(!unsaved.use_gateway_rpc);

        let saved: AccountConfig =
            toml::from_str("use_gateway_rpc = true").expect("saved section parses");
        assert!(saved.use_gateway_rpc);
    }
}
