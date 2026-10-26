// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana asset mechanics: SPL/Token-2022 mint reads and the ATA lifecycle.
//!
//! Owns everything that reads a token account or mint on Solana: mint account
//! decoding (decimals, token-program detection, mint/freeze authority),
//! Associated Token Account balance reads and close instructions. Built on
//! `crate::chains::solana::rpc`; shared cache/persistence policy for these
//! reads (memory cache, DB, server fallback) stays in `crate::tokens`, which
//! calls into `mint` as its on-chain adapter.

pub mod ata;
pub mod metaplex;
pub mod mint;

pub use ata::{
    cleanup_all_empty_atas, close_all_empty_atas, close_single_ata, close_token_account,
    close_token_account_with_context, get_all_token_accounts, get_sol_balance, get_token_balance,
    get_total_token_balance,
};
pub use metaplex::{fetch_nft_metadata, fetch_nft_metadata_batch, NftMetadata, NftMetadataError};
pub use mint::{fetch_mint_account, MintAccountData};
