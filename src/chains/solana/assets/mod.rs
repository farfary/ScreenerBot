//! Solana asset mechanics: SPL/Token-2022 mint reads, ATA lifecycle, transfers.
//!
//! Owns everything that reads or moves a token on Solana: mint account
//! decoding (decimals, token-program detection, mint/freeze authority),
//! Associated Token Account balance reads and close instructions, and native
//! SOL / SPL transfer construction and submission. Built on
//! `crate::chains::solana::rpc`; shared cache/persistence policy for these
//! reads (memory cache, DB, server fallback) stays in `crate::tokens`, which
//! calls into `mint` as its on-chain adapter.

pub mod ata;
pub mod burn;
pub mod metaplex;
pub mod mint;
pub mod transfer;

pub use ata::{
    cleanup_all_empty_atas, close_all_empty_atas, close_single_ata, close_token_account,
    close_token_account_with_context, get_all_token_accounts, get_sol_balance, get_token_balance,
    get_total_token_balance,
};
pub use burn::burn_configured_wallet_token;
pub use metaplex::{fetch_nft_metadata, fetch_nft_metadata_batch, NftMetadata, NftMetadataError};
pub use mint::{fetch_mint_account, is_token_2022_mint, MintAccountData};
pub use transfer::{close_ata as transfer_close_ata, transfer_sol, transfer_token};
