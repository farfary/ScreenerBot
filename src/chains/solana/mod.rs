//! Solana chain adapter: vendor dependency boundary plus Solana RPC.
//!
//! Application code imports Solana SDK crates through this façade so the
//! chain-specific dependency surface has one enforceable owner. Native-asset
//! metadata for the registry is constructed here from Solana constants.

pub use solana_account_decoder;
pub use solana_client;
pub use solana_program;
pub use solana_sdk;
pub use solana_transaction_status;
pub use spl_associated_token_account;
pub use spl_token;
pub use spl_token_2022;

pub mod accounts;
pub mod adapter;
pub mod assets;
pub mod constants;
mod error;
pub mod pools;
pub mod rpc;
pub mod swaps;
pub mod transactions;
pub mod wallets;

pub use error::{Error, Result};

use self::constants::{SOL_DECIMALS, SOL_MINT};
use super::{ChainId, ChainMetadata, NativeAsset};

/// Native-asset descriptor for Solana.
pub const NATIVE_ASSET: NativeAsset = NativeAsset {
    symbol: "SOL",
    decimals: SOL_DECIMALS,
    address: SOL_MINT,
};

/// Read-only metadata for the Solana chain.
pub const fn chain_metadata() -> ChainMetadata {
    ChainMetadata {
        id: ChainId::Solana,
        native_asset: NATIVE_ASSET,
    }
}
