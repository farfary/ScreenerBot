//! The transaction as the subject wallet experienced it.
//!
//! Every other analyzer in this module is account-agnostic: it builds a graph over
//! ALL accounts a transaction touched. That is the right shape for detecting a swap,
//! and the wrong shape for answering "what did this do to MY balance" — summing SOL
//! changes across every account of a Solana transaction always yields exactly
//! `-fee`, because lamports are conserved. That summation is what made the
//! dashboard's `Δ SOL` column read `-0.000005` on 627 of 629 recorded transactions.
//!
//! `WalletView` is the single place the subject's own deltas are derived, straight
//! from `pre/postBalances` at the wallet's account index and from the token balance
//! entries the RPC already tags with `owner`.

use std::collections::HashMap;

use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::rpc::TransactionDetails;

/// One mint's movement across every account the subject owns.
#[derive(Debug, Clone)]
pub struct WalletTokenDelta {
    pub mint: String,
    pub decimals: u8,
    /// Signed change in raw base units.
    pub raw: i128,
    /// Signed change in UI units.
    pub ui: f64,
}

/// The subject wallet's own view of one transaction.
#[derive(Debug, Clone)]
pub struct WalletView {
    pub wallet: String,
    /// True when the wallet is the fee payer (account index 0). An inbound credit on
    /// a transaction we did not sign is unsolicited by definition.
    pub signed: bool,
    /// True when the wallet appears in the account list at all.
    pub present: bool,
    /// Signed native-SOL change of the wallet account, fee included.
    pub lamport_delta: i64,
    /// Fee this transaction charged, and `0` when someone else paid it.
    pub fee_lamports: u64,
    /// Token movements for accounts this wallet owns, excluding wrapped SOL.
    pub token_deltas: Vec<WalletTokenDelta>,
    /// Wrapped-SOL movement across the wallet's WSOL accounts, in UI SOL.
    pub wsol_ui: f64,
    /// Rent the wallet paid opening accounts in this transaction.
    pub rent_paid_lamports: u64,
    /// Rent the wallet recovered closing accounts in this transaction.
    pub rent_recovered_lamports: u64,
    /// Mints whose account the wallet closed here, in close order.
    pub closed_mints: Vec<String>,
    /// Mints whose account the wallet opened here, in open order.
    pub created_mints: Vec<String>,
    /// Counterparties that credited the wallet native SOL, largest credit first.
    pub sol_credit_sources: Vec<(String, u64)>,
    /// Number of top-level system transfers in the transaction, whoever they pay.
    pub system_transfer_count: usize,
}

impl WalletView {
    /// The wallet's SOL movement with its own fee removed — the economic delta.
    pub fn lamport_delta_excluding_fee(&self) -> i64 {
        self.lamport_delta + self.fee_lamports as i64
    }

    /// Signed SOL change of the wallet account, fee included, in SOL.
    pub fn sol_delta(&self) -> f64 {
        self.lamport_delta as f64 / 1_000_000_000.0
    }

    /// Net raw token movement across every non-WSOL mint, used only for direction.
    pub fn token_delta_raw(&self) -> i128 {
        self.token_deltas.iter().map(|d| d.raw).sum()
    }

    /// The single mint that dominates this transaction, by absolute movement.
    pub fn dominant_token(&self) -> Option<&WalletTokenDelta> {
        self.token_deltas
            .iter()
            .max_by(|a, b| a.raw.abs().cmp(&b.raw.abs()))
    }

    /// Builds the view. Returns a `present: false` view when the wallet is not in the
    /// account list at all, which happens for a transaction reached through a
    /// signature index rather than through our own history.
    pub fn build(tx_data: &TransactionDetails, wallet: &str) -> Self {
        let account_keys = account_keys(&tx_data.transaction.message);
        let index = account_keys.iter().position(|key| key == wallet);

        let mut view = Self {
            wallet: wallet.to_owned(),
            signed: account_keys.first().map(String::as_str) == Some(wallet),
            present: index.is_some(),
            lamport_delta: 0,
            fee_lamports: 0,
            token_deltas: Vec::new(),
            wsol_ui: 0.0,
            rent_paid_lamports: 0,
            rent_recovered_lamports: 0,
            closed_mints: Vec::new(),
            created_mints: Vec::new(),
            sol_credit_sources: Vec::new(),
            system_transfer_count: 0,
        };

        let Some(meta) = tx_data.meta.as_ref() else {
            return view;
        };

        if view.signed {
            view.fee_lamports = meta.fee;
        }

        if let Some(i) = index {
            if i < meta.pre_balances.len() && i < meta.post_balances.len() {
                view.lamport_delta = meta.post_balances[i] as i64 - meta.pre_balances[i] as i64;
            }
        }

        view.collect_token_deltas(meta, &account_keys);
        view.collect_rent(meta, &account_keys);
        view.collect_system_transfers(&tx_data.transaction.message);

        view
    }

    fn collect_token_deltas(
        &mut self,
        meta: &crate::chains::solana::rpc::TransactionMeta,
        account_keys: &[String],
    ) {
        // Keyed by mint so several accounts of the same mint (a token ATA plus a
        // temporary one a router opened for us) collapse into one movement.
        let mut totals: HashMap<String, (i128, u8)> = HashMap::new();
        let mut wsol_raw: i128 = 0;

        let wallet = self.wallet.clone();
        let apply = |balances: Option<&Vec<crate::chains::solana::rpc::TokenBalance>>,
                     sign: i128,
                     totals: &mut HashMap<String, (i128, u8)>,
                     wsol_raw: &mut i128| {
            let Some(balances) = balances else { return };
            for balance in balances {
                let owned = balance.owner.as_deref() == Some(wallet.as_str())
                    || account_keys
                        .get(balance.account_index as usize)
                        .is_some_and(|key| key == &wallet);
                if !owned {
                    continue;
                }
                let raw: i128 = balance.ui_token_amount.amount.parse().unwrap_or_default();
                if balance.mint == SOL_MINT {
                    *wsol_raw += sign * raw;
                } else {
                    let entry = totals
                        .entry(balance.mint.clone())
                        .or_insert((0, balance.ui_token_amount.decimals));
                    entry.0 += sign * raw;
                    entry.1 = balance.ui_token_amount.decimals;
                }
            }
        };

        apply(
            meta.post_token_balances.as_ref(),
            1,
            &mut totals,
            &mut wsol_raw,
        );
        apply(
            meta.pre_token_balances.as_ref(),
            -1,
            &mut totals,
            &mut wsol_raw,
        );

        self.wsol_ui = wsol_raw as f64 / 1_000_000_000.0;
        self.token_deltas = totals
            .into_iter()
            .filter(|(_, (raw, _))| *raw != 0)
            .map(|(mint, (raw, decimals))| WalletTokenDelta {
                mint,
                decimals,
                raw,
                ui: raw as f64 / 10f64.powi(decimals as i32),
            })
            .collect();
        self.token_deltas
            .sort_by(|a, b| b.raw.abs().cmp(&a.raw.abs()));
    }

    fn collect_rent(
        &mut self,
        meta: &crate::chains::solana::rpc::TransactionMeta,
        account_keys: &[String],
    ) {
        // An account the wallet owns that went from zero to funded was opened here;
        // one that went to zero was closed and its rent came back to us. The mint is
        // read from whichever side of the token-balance snapshot still carries it.
        let wallet = self.wallet.clone();
        let owned_index = |index: usize,
                           balances: Option<&Vec<crate::chains::solana::rpc::TokenBalance>>|
         -> Option<String> {
            balances?.iter().find_map(|balance| {
                (balance.account_index as usize == index
                    && balance.owner.as_deref() == Some(wallet.as_str()))
                .then(|| balance.mint.clone())
            })
        };

        for index in 0..account_keys.len() {
            if index >= meta.pre_balances.len() || index >= meta.post_balances.len() {
                break;
            }
            let pre = meta.pre_balances[index];
            let post = meta.post_balances[index];
            if pre == post {
                continue;
            }
            if pre == 0 {
                if let Some(mint) = owned_index(index, meta.post_token_balances.as_ref()) {
                    self.rent_paid_lamports += post;
                    self.created_mints.push(mint);
                }
            } else if post == 0 {
                if let Some(mint) = owned_index(index, meta.pre_token_balances.as_ref()) {
                    self.rent_recovered_lamports += pre;
                    self.closed_mints.push(mint);
                }
            }
        }
    }

    fn collect_system_transfers(&mut self, message: &serde_json::Value) {
        let wallet = self.wallet.clone();
        let Some(instructions) = message.get("instructions").and_then(|v| v.as_array()) else {
            return;
        };
        let mut credits: Vec<(String, u64)> = Vec::new();
        for instruction in instructions {
            let Some(parsed) = instruction.get("parsed") else {
                continue;
            };
            let is_transfer = parsed.get("type").and_then(|v| v.as_str()) == Some("transfer")
                && instruction.get("program").and_then(|v| v.as_str()) == Some("system");
            if !is_transfer {
                continue;
            }
            self.system_transfer_count += 1;
            let Some(info) = parsed.get("info") else {
                continue;
            };
            if info.get("destination").and_then(|v| v.as_str()) != Some(wallet.as_str()) {
                continue;
            }
            let lamports = info
                .get("lamports")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default();
            let source = info
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned();
            credits.push((source, lamports));
        }
        credits.sort_by(|a, b| b.1.cmp(&a.1));
        self.sol_credit_sources = credits;
    }
}

/// Account keys in index order, across every encoding the RPC uses.
pub fn account_keys(message: &serde_json::Value) -> Vec<String> {
    if let Some(array) = message.get("accountKeys").and_then(|v| v.as_array()) {
        let strings: Vec<String> = array
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
        if !strings.is_empty() {
            return strings;
        }
        return array
            .iter()
            .filter_map(|v| v.get("pubkey").and_then(|p| p.as_str()).map(str::to_owned))
            .collect();
    }
    Vec::new()
}
