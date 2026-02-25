//! Associated Token Account (ATA) operations
//!
//! Balance queries and token account management including ATA closing.
//! Note: Automatic ATA cleanup is handled by the background service (see ata_cleanup.rs).

mod balance;
mod close;
mod helpers;

pub use balance::{
    cleanup_all_empty_atas, get_all_token_accounts, get_sol_balance, get_token_balance,
    get_total_token_balance,
};
pub use close::{
    close_all_empty_atas, close_single_ata, close_token_account, close_token_account_with_context,
};
