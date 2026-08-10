//! Demo generators for the wallet current snapshot and token holdings.
//!
//! Holdings mirror the open positions (those ARE the tokens the wallet holds), so
//! the wallet worth reconciles exactly with the positions/home dashboards.

use chrono::Utc;

use crate::webserver::routes::wallet::{
    TokenBalanceInfo, WalletCurrentResponse, WalletTokenHolding, WalletTokensResponse,
};

use super::data::*;

const DEMO_TOKEN_DECIMALS: u8 = 6;

pub fn get_demo_wallet_address() -> &'static str {
    DEMO_WALLET_ADDRESS
}

/// (mint, ui_amount, price_sol, value_sol) for each held token, derived from the
/// open positions so token count and worth match everywhere.
fn holdings() -> Vec<(&'static str, &'static str, &'static str, f64, f64, f64)> {
    DEMO_OPEN_TOKENS
        .iter()
        .map(|(symbol, name, mint, _logo, entry, current, size, _hold)| {
            let value_sol = size * current / entry; // current SOL value of the holding
            let ui_amount = value_sol / current; // tokens held = value / price
            (*symbol, *name, *mint, ui_amount, *current, value_sol)
        })
        .collect()
}

/// Generate demo wallet current response.
pub fn get_demo_wallet_current() -> WalletCurrentResponse {
    let now = Utc::now();

    let token_balances: Vec<TokenBalanceInfo> = holdings()
        .iter()
        .map(
            |(_symbol, _name, mint, ui_amount, _price, _value)| TokenBalanceInfo {
                mint: (*mint).to_owned(),
                balance: (ui_amount * 10f64.powi(DEMO_TOKEN_DECIMALS as i32)) as u64,
                balance_ui: *ui_amount,
                decimals: DEMO_TOKEN_DECIMALS,
                is_token_2022: false,
            },
        )
        .collect();

    WalletCurrentResponse {
        sol_balance: DEMO_SOL_BALANCE,
        sol_balance_lamports: DEMO_SOL_LAMPORTS,
        total_tokens_count: token_balances.len() as u32,
        token_balances,
        snapshot_time: now.to_rfc3339(),
    }
}

/// Generate demo wallet tokens response.
pub fn get_demo_wallet_tokens() -> WalletTokensResponse {
    let tokens: Vec<WalletTokenHolding> = holdings()
        .iter()
        .map(
            |(symbol, name, mint, ui_amount, price_sol, value_sol)| WalletTokenHolding {
                mint: (*mint).to_owned(),
                symbol: Some((*symbol).to_owned()),
                name: Some((*name).to_owned()),
                logo_url: None,
                balance: (ui_amount * 10f64.powi(DEMO_TOKEN_DECIMALS as i32)) as u64,
                ui_amount: *ui_amount,
                price_sol: Some(*price_sol),
                value_sol: Some(*value_sol),
                decimals: DEMO_TOKEN_DECIMALS,
                is_token_2022: false,
            },
        )
        .collect();

    WalletTokensResponse { tokens }
}
