#![allow(warnings)]

use std::{ fs, str::FromStr };
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::TokenAccountsFilter;
use solana_sdk::{ pubkey::Pubkey, signature::{ Keypair, Signer } };
use solana_account_decoder::UiAccountData;
use once_cell::sync::Lazy;
use bs58;

// ────────────── CONFIG & RPC ──────────────
#[derive(Debug, Deserialize)]
pub struct Configs {
    pub main_wallet_private: String,
    pub rpc_url: String,
}

pub static CONFIGS: Lazy<Configs> = Lazy::new(|| {
    let raw = fs::read_to_string("configs.json").expect("❌ Failed to read configs.json");
    serde_json::from_str(&raw).expect("❌ Failed to parse configs.json")
});

pub static RPC: Lazy<RpcClient> = Lazy::new(|| { RpcClient::new(CONFIGS.rpc_url.clone()) });

// ────────────── SCAN HELPER ──────────────
/// Scans all token accounts under `owner` for a given SPL program,
/// returning `(mint_address, raw_amount_u64)` for each ATA.
fn scan_program_tokens(owner: &Pubkey, program_id: &Pubkey) -> Vec<(String, u64)> {
    let filter = TokenAccountsFilter::ProgramId(*program_id);
    let accounts = RPC.get_token_accounts_by_owner(owner, filter).expect("❌ RPC error");

    let mut result = Vec::new();
    for keyed in accounts {
        let acc = keyed.account;
        if let UiAccountData::Json(parsed) = acc.data {
            let info = &parsed.parsed["info"];
            let mint = info["mint"].as_str().expect("Missing mint").to_string();
            let raw_amount_str = info["tokenAmount"]["amount"]
                .as_str()
                .expect("Missing raw amount");
            let raw_amount = raw_amount_str.parse::<u64>().expect("Invalid raw amount");
            result.push((mint, raw_amount));
        }
    }
    result
}

// ────────────── PUBLIC API ──────────────
/// Returns _all_ SPL (and Token-2022) balances for the main wallet,
/// as a list of `(mint_address, raw_amount)`.
pub fn get_all_tokens() -> Vec<(String, u64)> {
    // decode keypair
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();

    let mut tokens = Vec::new();

    // standard SPL
    tokens.extend(scan_program_tokens(&owner, &spl_token::id()));

    // Token-2022
    let token2022 = Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap();
    tokens.extend(scan_program_tokens(&owner, &token2022));

    tokens
}

/// Returns the total raw token amount for `token_mint`,
/// summing across all ATAs (SPL + Token-2022).
pub fn get_token_amount(token_mint: &str) -> u64 {
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();
    let mint = Pubkey::from_str(token_mint).expect("Invalid mint");

    let programs = vec![
        spl_token::id(),
        Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap()
    ];

    let mut total = 0;
    for program_id in programs {
        let filter = TokenAccountsFilter::Mint(mint);
        let accounts = RPC.get_token_accounts_by_owner(&owner, filter).expect("❌ RPC error");
        for keyed in accounts {
            if let UiAccountData::Json(parsed) = keyed.account.data {
                let amt_str = &parsed.parsed["info"]["tokenAmount"]["amount"];
                if let Some(s) = amt_str.as_str() {
                    total += s.parse::<u64>().unwrap_or(0);
                }
            }
        }
    }

    println!("🔢 On-chain balance for {}: {}", token_mint, total);
    total
}

/// Returns the largest single-ATA raw amount for `token_mint`.
pub fn get_biggest_token_amount(token_mint: &str) -> u64 {
    let secret_bytes = bs58
        ::decode(&CONFIGS.main_wallet_private)
        .into_vec()
        .expect("❌ Invalid base58 key");
    let keypair = Keypair::try_from(&secret_bytes[..]).expect("❌ Invalid keypair bytes");
    let owner = keypair.pubkey();
    let mint = Pubkey::from_str(token_mint).expect("Invalid mint");

    let programs = vec![
        spl_token::id(),
        Pubkey::from_str("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").unwrap()
    ];

    let mut biggest = 0;
    for program_id in programs {
        let filter = TokenAccountsFilter::Mint(mint);
        let accounts = RPC.get_token_accounts_by_owner(&owner, filter).expect("❌ RPC error");
        for keyed in accounts {
            if let UiAccountData::Json(parsed) = keyed.account.data {
                let amt_str = &parsed.parsed["info"]["tokenAmount"]["amount"];
                if let Some(s) = amt_str.as_str() {
                    let v = s.parse::<u64>().unwrap_or(0);
                    if v > biggest {
                        biggest = v;
                    }
                }
            }
        }
    }

    println!("🔢 Biggest single ATA for {}: {}", token_mint, biggest);
    biggest
}
