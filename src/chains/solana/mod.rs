//! Solana chain adapter: vendor dependency boundary plus Solana RPC.
//!
//! Application code imports Solana SDK crates through this façade so the
//! chain-specific dependency surface has one enforceable owner.

pub use solana_account_decoder;
pub use solana_client;
pub use solana_program;
pub use solana_sdk;
pub use solana_transaction_status;
pub use spl_associated_token_account;
pub use spl_token;
pub use spl_token_2022;

pub mod accounts;
pub mod assets;
pub mod constants;
pub mod pools;
pub mod rpc;
pub mod swaps;
pub mod transactions;
pub mod wallets;
