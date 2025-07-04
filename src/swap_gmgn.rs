#![allow(warnings)]

use anyhow::{ Context, Result };
use base64::{ engine::general_purpose, Engine };
use reqwest::Client;
use serde_json::Value;
use solana_sdk::{ signature::{ Keypair, Signer }, transaction::VersionedTransaction };
use solana_client::rpc_client::RpcClient;
use std::time::Duration;
use std::str::FromStr; // ← add this one line

use crate::configs::CONFIGS;
use crate::helpers::get_biggest_token_amount;

use bs58;
use solana_sdk::{ pubkey::Pubkey };

use crate::utilitis::effective_swap_price;

/// Submit a swap to GMGN router and return the signature string.
/// Also prints the **effective on-chain price** paid for the swap.
pub async fn buy_gmgn(
    token_mint_address: &str,
    in_amount: u64 // lamports you want to swap
) -> Result<String> {
    // -------- 0. setup -----------------------------------------------------
    let wallet = {
        let bytes = bs58::decode(&CONFIGS.main_wallet_private).into_vec()?;
        Keypair::try_from(&bytes[..])?
    };
    let wallet_pk = wallet.pubkey();
    let owner = wallet_pk.to_string();
    let client = Client::new();
    let rpc_client = RpcClient::new(CONFIGS.rpc_url.clone());
    let token_mint_pk = Pubkey::from_str(token_mint_address).context("bad token mint pubkey")?;

    // -------- 1. get quote --------------------------------------------------
    let wrapped_sol = "So11111111111111111111111111111111111111112";
    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}",
        wrapped_sol,
        token_mint_address,
        in_amount,
        owner,
        0.5,
        "ExactIn",
        0.00002,
        false
    );
    println!("🔍 GET QUOTE URL:\n{url}");

    let body: Value = client
        .get(&url)
        .send().await?
        .error_for_status()?
        .json().await
        .context("decode quote JSON")?;
    println!("✅ QUOTE RESPONSE:\n{}", serde_json::to_string_pretty(&body)?);

    let raw_tx = body["data"]["raw_tx"]["swapTransaction"]
        .as_str()
        .context("missing swapTransaction")?;
    let last_blk = body["data"]["raw_tx"]["lastValidBlockHeight"]
        .as_u64()
        .context("missing lastValidBlockHeight")?;

    // -------- 2. sign -------------------------------------------------------
    let tx_bytes: Vec<u8> = general_purpose::STANDARD.decode(raw_tx)?;
    let mut vtx: VersionedTransaction = bincode::deserialize(&tx_bytes)?;
    let sig = wallet.sign_message(&vtx.message.serialize());
    vtx.signatures = vec![sig];
    let signed_tx_b64 = general_purpose::STANDARD.encode(bincode::serialize(&vtx)?);
    println!("✍️ Signed TX (base64 len {}):", signed_tx_b64.len());

    // -------- 3. submit -----------------------------------------------------
    match rpc_client.send_and_confirm_transaction(&vtx) {
        Ok(signature) => {
            println!("✅ submitted: {signature}");
            // poll until finalised (existing helper)
            let sig_str = poll_transaction_status(
                &rpc_client,
                &signature.to_string(),
                last_blk
            ).await?;

            // -------- 4. derive effective price -----------------------------
            match
                effective_swap_price(
                    &rpc_client,
                    &sig_str,
                    &wallet_pk,
                    &token_mint_pk,
                    in_amount // lamports we fed in
                )
            {
                Ok(price) => println!("📈 EFFECTIVE BUY PRICE: {:.9} SOL per token", price),
                Err(e) => eprintln!("⚠️  could not derive price: {e}"),
            }

            Ok(sig_str) // return only signature, as before
        }
        Err(e) => anyhow::bail!("❌ submit error: {e}"),
    }
}

async fn poll_transaction_status(
    _rpc_client: &RpcClient, // We don't actually need to use this here
    tx_signature: &str, // Use the string version of the signature
    last_valid: u64
) -> Result<String> {
    // Return only signature now
    let status_url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_transaction_status?hash={}&last_valid_height={}",
        tx_signature,
        last_valid
    );

    let client = Client::new();
    println!("🔄 Start polling status...");

    for i in 0..15 {
        let check = client.get(&status_url).send().await?;
        let status: Value = check.json().await?;
        println!("📡 POLL {} RESPONSE:\n{}", i + 1, serde_json::to_string_pretty(&status)?);

        let success = status["data"]["success"].as_bool().unwrap_or(false);
        let expired = status["data"]["expired"].as_bool().unwrap_or(false);

        if success {
            println!("🎉 Tx confirmed successfully!");
            return Ok(tx_signature.to_string()); // Return only the signature now
        }
        if expired {
            anyhow::bail!("⏰ Tx expired before confirmation");
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    anyhow::bail!("❌ Tx not confirmed in time")
}

// --------------------------------------------------
// SELL FUNCTION WITH MIN-OUT-AMOUNT CHECK
// --------------------------------------------------
pub async fn sell_all_gmgn(
    token_mint_address: &str,
    min_out_amount: f64 // require at least this SOL out
) -> anyhow::Result<String> {
    // load wallet
    let wallet = {
        let bytes = bs58::decode(&CONFIGS.main_wallet_private).into_vec()?;
        Keypair::try_from(&bytes[..])?
    };
    let owner = wallet.pubkey().to_string();
    let client = Client::new();
    let rpc_client = RpcClient::new(CONFIGS.rpc_url.clone());

    // get token balance (lamports)
    let in_amount = get_biggest_token_amount(token_mint_address);
    if in_amount == 0 {
        anyhow::bail!("❌ No spendable balance for {}", token_mint_address);
    }

    // build quote URL
    let wrapped_sol = "So11111111111111111111111111111111111111112";
    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route\
?token_in_address={}&token_out_address={}&in_amount={}\
&from_address={}&slippage={}&swap_mode=ExactIn&fee={}&is_anti_mev=false",
        token_mint_address,
        wrapped_sol,
        in_amount,
        owner,
        0.5,
        0.00002
    );

    // fetch quote
    let resp = client.get(&url).send().await?.error_for_status()?;
    let body: Value = resp.json().await.context("Failed to decode quote JSON")?;

    // parse out amount (SOL)
    let quote = &body["data"]["quote"];
    let out_amount_raw = quote["outAmount"].as_str().context("Missing outAmount")?.parse::<u64>()?;
    let out_decimals = quote["outDecimals"].as_u64().context("Missing outDecimals")? as i32;
    let out_amount_sol = (out_amount_raw as f64) / (10f64).powi(out_decimals);

    // check minimum
    if out_amount_sol < min_out_amount {
        anyhow::bail!(
            "❌ Quoted SOL out {:.9} is below required {:.9}, aborting",
            out_amount_sol,
            min_out_amount
        );
    }

    // prepare and sign tx
    let raw_tx_b64 = body["data"]["raw_tx"]["swapTransaction"]
        .as_str()
        .context("Missing swapTransaction")?;
    let last_valid = body["data"]["raw_tx"]["lastValidBlockHeight"]
        .as_u64()
        .context("Missing lastValidBlockHeight")?;
    let tx_bytes = general_purpose::STANDARD.decode(raw_tx_b64)?;
    let mut vtx: VersionedTransaction = bincode::deserialize(&tx_bytes)?;
    let sig = wallet.sign_message(&vtx.message.serialize());
    vtx.signatures = vec![sig];

    // send
    let signature = rpc_client.send_and_confirm_transaction(&vtx)?;
    poll_transaction_status(&rpc_client, &signature.to_string(), last_valid).await
}
