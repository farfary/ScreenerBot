use anyhow::{ anyhow, Result };
use num_format::{ Locale, ToFormattedString };
use solana_client::rpc_client::RpcClient;
use solana_sdk::{ pubkey::Pubkey, account::Account };

pub fn decode_meteora_dlmm(
    rpc: &RpcClient,
    _pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    if acct.data.len() < 128 {
        println!("⚠️  DLMM account too small");
        return Ok((0, 0, Pubkey::default(), Pubkey::default()));
    }

    // Extract mint addresses from the beginning of the pool account
    let mint_x = Pubkey::new_from_array(acct.data[8..40].try_into().unwrap());
    let mint_y = Pubkey::new_from_array(acct.data[40..72].try_into().unwrap());

    let mut vaults: Vec<Pubkey> = Vec::new();

    // Slide 32-byte window in 4-byte steps (fast enough: account size = 1.8-2 kB)
    for i in (0..=acct.data.len() - 32).step_by(4) {
        let slice: &[u8] = &acct.data[i..i + 32];
        if slice.iter().all(|&b| b == 0) {
            continue; // skip null pubkeys
        }
        let pk = Pubkey::new_from_array(slice.try_into().unwrap());

        // de-dup
        if vaults.contains(&pk) {
            continue;
        }

        // quick check: does RPC think it is a token account?
        if rpc.get_token_account_balance(&pk).is_ok() {
            vaults.push(pk);
            if vaults.len() == 2 {
                break;
            }
        }
    }

    if vaults.len() != 2 {
        println!("⚠️  Couldn't locate both vaults inside DLMM account");
        return Ok((0, 0, Pubkey::default(), Pubkey::default()));
    }

    let bal_a = rpc
        .get_token_account_balance(&vaults[0])
        .map(|b| b.amount.parse::<u64>().unwrap_or(0))
        .unwrap_or(0);
    let bal_b = rpc
        .get_token_account_balance(&vaults[1])
        .map(|b| b.amount.parse::<u64>().unwrap_or(0))
        .unwrap_or(0);

    println!(
        "✅ Meteora DLMM  → vault-A: {} | vault-B: {}",
        bal_a.to_formatted_string(&Locale::en),
        bal_b.to_formatted_string(&Locale::en)
    );
    Ok((bal_a, bal_b, mint_x, mint_y))
}

/// Batch-friendly version: decode from already-fetched account
pub fn decode_meteora_dlmm_from_account(
    rpc: &RpcClient,
    pool_pk: &Pubkey,
    acct: &Account
) -> Result<(u64, u64, Pubkey, Pubkey)> {
    // Same logic as decode_meteora_dlmm, but account is already provided
    decode_meteora_dlmm(rpc, pool_pk, acct)
}
