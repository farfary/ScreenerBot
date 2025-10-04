/// Wallet API routes
///
/// Provides wallet balance and monitoring endpoints

use axum::{ extract::State, response::Json, routing::get, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::webserver::state::AppState;
use crate::wallet::{ get_current_wallet_status, WalletSnapshot };

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletCurrentResponse {
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub total_tokens_count: u32,
    pub token_balances: Vec<TokenBalanceInfo>,
    pub snapshot_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalanceInfo {
    pub mint: String,
    pub balance: u64,
    pub balance_ui: f64,
    pub decimals: Option<u8>,
    pub is_token_2022: bool,
}

/// Create wallet routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/wallet/current", get(get_wallet_current))
}

/// Get current wallet balance
async fn get_wallet_current() -> Json<Option<WalletCurrentResponse>> {
    match get_current_wallet_status().await {
        Ok(Some(snapshot)) => {
            let token_balances = snapshot.token_balances
                .iter()
                .map(|tb| {
                    TokenBalanceInfo {
                        mint: tb.mint.clone(),
                        balance: tb.balance,
                        balance_ui: tb.balance_ui,
                        decimals: tb.decimals,
                        is_token_2022: tb.is_token_2022,
                    }
                })
                .collect();

            Json(
                Some(WalletCurrentResponse {
                    sol_balance: snapshot.sol_balance,
                    sol_balance_lamports: snapshot.sol_balance_lamports,
                    total_tokens_count: snapshot.total_tokens_count,
                    token_balances,
                    snapshot_time: snapshot.snapshot_time.to_rfc3339(),
                })
            )
        }
        _ => Json(None),
    }
}
