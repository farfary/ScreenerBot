//! Solana account primitives: keypair generation, parsing and signing.
//!
//! Owns every place that touches a raw `Keypair`/`Pubkey` outside of
//! transaction construction: generating one, parsing one from base58 or JSON
//! array formats, validating an address string, and deriving an address from
//! a keypair. Encryption of the resulting secret is chain-neutral
//! (`crate::secure_storage`) and stays outside this module.

pub mod balances;
pub mod keypair;
pub mod signing;

pub use balances::{fetch_wallet_sol_balance, fetch_wallet_token_balances};
pub use keypair::{
    address_from_encrypted_key, address_from_private_key, decrypt_to_keypair, export_private_key,
    generate_and_encrypt_keypair, generate_keypair, generate_keypair_strings,
    generate_wallet_material, import_and_encrypt, import_wallet_material, keypair_to_address,
    parse_private_key, parse_pubkey_safe, validate_address,
};
pub use signing::{
    cached_main_wallet_id, configured_address, configured_keypair, configured_pubkey,
    invalidate_main_wallet_cache, keypair_for_wallet, main_keypair,
    sign_message_for_active_wallets, sign_message_with_main_wallet,
};
