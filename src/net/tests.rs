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
