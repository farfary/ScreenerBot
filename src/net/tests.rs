// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Unit tests for the net module, verifying that TLS crypto provider
//! installation is idempotent and yields a usable rustls client config.

use super::*;

#[test]
fn install_tls_crypto_provider_is_idempotent_and_enables_client_config() {
    install_tls_crypto_provider();
    install_tls_crypto_provider();
    assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    let _config = rustls::ClientConfig::builder()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();
}
