use anyhow::{ Context, Result };
use base64::{ engine::general_purpose, Engine };
use reqwest::Client;
use serde_json::Value;
use solana_sdk::{ signature::{ Keypair, Signer }, transaction::VersionedTransaction };
use std::time::Duration;

use crate::configs::CONFIGS;


pub async fn buy_gmgn(
    token_mint_address: &str,
    in_amount: u64,
) -> Result<String> {
    let wallet = {
        let bytes = bs58::decode(&CONFIGS.main_wallet_private).into_vec()?;
        Keypair::try_from(&bytes[..])?
    };
    let owner = wallet.pubkey().to_string();
    let client = Client::new();

    // ──────────────── 1) GET QUOTE ────────────────
    let wrapped_sol = "So11111111111111111111111111111111111111112";
    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}",
        wrapped_sol,
        token_mint_address,
        in_amount,
        owner,
        0.5,
        "ExactIn",
        0.006,
        true
    );
    println!("🔍 GET QUOTE URL:\n{}", url);

    let resp = client.get(&url).send().await?.error_for_status()?;
    let body: Value = resp.json().await.context("Failed to decode quote JSON")?;
    println!("✅ QUOTE RESPONSE:\n{}", serde_json::to_string_pretty(&body)?);

    // ── Parse correct path
    let raw_tx = body["data"]["raw_tx"]["swapTransaction"]
        .as_str()
        .context("Missing swapTransaction")?;

    let last_valid = body["data"]["raw_tx"]["lastValidBlockHeight"]
        .as_u64()
        .context("Missing lastValidBlockHeight")?;

    println!("🔑 Raw TX (base64 length): {}", raw_tx.len());
    println!("⏰ Last valid block height: {}", last_valid);

    // ──────────────── 2) SIGN TRANSACTION ────────────────
    let tx_bytes = general_purpose::STANDARD.decode(raw_tx)?;
    let mut vtx: VersionedTransaction = bincode::deserialize(&tx_bytes)?;

    let sig = wallet.sign_message(&vtx.message.serialize());
    vtx.signatures = vec![sig];

    let signed_tx = general_purpose::STANDARD.encode(bincode::serialize(&vtx)?);
    println!("✍️ Signed TX (base64 length): {}", signed_tx.len());

    // ──────────────── 3) SUBMIT ────────────────
    let submit_url = "https://gmgn.ai/txproxy/v1/send_transaction";
    let payload = serde_json::json!({
        "chain": "sol",
        "signedTx": signed_tx,
        "isAntiMev": true
    });
    println!("🚀 SUBMIT PAYLOAD:\n{}", serde_json::to_string_pretty(&payload)?);

    let submit_resp = client.post(submit_url).json(&payload).send().await?;
    let submit_json: Value = submit_resp.json().await?;
    println!("✅ SUBMIT RESPONSE:\n{}", serde_json::to_string_pretty(&submit_json)?);

    let hash = submit_json["data"]["hash"]
        .as_str()
        .context("Missing tx hash")?;

    println!("✅ Submitted Tx Hash: {hash}");

    // ──────────────── 4) POLL STATUS ────────────────
    let status_url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_transaction_status?hash={}&last_valid_height={}",
        hash, last_valid
    );

    println!("🔄 Start polling status...");

    for i in 0..15 {
        let check = client.get(&status_url).send().await?;
        let status: Value = check.json().await?;
        println!("📡 POLL {} RESPONSE:\n{}", i + 1, serde_json::to_string_pretty(&status)?);

        let success = status["data"]["success"].as_bool().unwrap_or(false);
        let expired = status["data"]["expired"].as_bool().unwrap_or(false);

        if success {
            println!("🎉 Tx confirmed successfully!");
            return Ok(hash.to_string());
        }
        if expired {
            anyhow::bail!("⏰ Tx expired before confirmation");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    anyhow::bail!("❌ Tx not confirmed in time")
}


use crate::helpers::get_biggest_token_amount;

pub async fn sell_all_gmgn(token_mint_address: &str) -> Result<String> {
    let wallet = {
        let bytes = bs58::decode(&CONFIGS.main_wallet_private).into_vec()?;
        Keypair::try_from(&bytes[..])?
    };
    let owner = wallet.pubkey().to_string();
    let client = Client::new();

    // ✅ Get biggest valid ATA amount
    let token_amount = get_biggest_token_amount(token_mint_address);
    if token_amount == 0 {
        anyhow::bail!("❌ No spendable balance found for token {}", token_mint_address);
    }
    println!("🔢 Using ATA amount: {}", token_amount);

    let wrapped_sol = "So11111111111111111111111111111111111111112";

    // ──────────────── 1) GET QUOTE ────────────────
    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}",
        token_mint_address,
        wrapped_sol,
        token_amount,
        owner,
        0.5,
        "ExactIn",
        0.006,
        true
    );
    println!("🔍 GET QUOTE URL:\n{}", url);

    let resp = client.get(&url).send().await?.error_for_status()?;
    let body: Value = resp.json().await.context("Failed to decode quote JSON")?;
    println!("✅ QUOTE RESPONSE:\n{}", serde_json::to_string_pretty(&body)?);

    let raw_tx = body["data"]["raw_tx"]["swapTransaction"]
        .as_str()
        .context("Missing swapTransaction")?;

    let last_valid = body["data"]["raw_tx"]["lastValidBlockHeight"]
        .as_u64()
        .context("Missing lastValidBlockHeight")?;

    println!("🔑 Raw TX (base64 length): {}", raw_tx.len());
    println!("⏰ Last valid block height: {}", last_valid);

    // ──────────────── 2) SIGN TRANSACTION ────────────────
    let tx_bytes = general_purpose::STANDARD.decode(raw_tx)?;
    let mut vtx: VersionedTransaction = bincode::deserialize(&tx_bytes)?;

    let sig = wallet.sign_message(&vtx.message.serialize());
    vtx.signatures = vec![sig];

    let signed_tx = general_purpose::STANDARD.encode(bincode::serialize(&vtx)?);
    println!("✍️ Signed TX (base64 length): {}", signed_tx.len());

    // ──────────────── 3) SUBMIT ────────────────
    let submit_url = "https://gmgn.ai/txproxy/v1/send_transaction";
    let payload = serde_json::json!({
        "chain": "sol",
        "signedTx": signed_tx,
        "isAntiMev": true
    });
    println!("🚀 SUBMIT PAYLOAD:\n{}", serde_json::to_string_pretty(&payload)?);

    let submit_resp = client.post(submit_url).json(&payload).send().await?;
    let submit_json: Value = submit_resp.json().await?;
    println!("✅ SUBMIT RESPONSE:\n{}", serde_json::to_string_pretty(&submit_json)?);

    let hash = submit_json["data"]["hash"]
        .as_str()
        .context("Missing tx hash")?;

    println!("✅ Submitted Tx Hash: {hash}");

    // ──────────────── 4) POLL STATUS ────────────────
    let status_url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_transaction_status?hash={}&last_valid_height={}",
        hash, last_valid
    );

    println!("🔄 Start polling status...");

    for i in 0..15 {
        let check = client.get(&status_url).send().await?;
        let status: Value = check.json().await?;
        println!("📡 POLL {} RESPONSE:\n{}", i + 1, serde_json::to_string_pretty(&status)?);

        let success = status["data"]["success"].as_bool().unwrap_or(false);
        let expired = status["data"]["expired"].as_bool().unwrap_or(false);

        if success {
            println!("🎉 Tx confirmed successfully!");
            return Ok(hash.to_string());
        }
        if expired {
            anyhow::bail!("⏰ Tx expired before confirmation");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    anyhow::bail!("❌ Tx not confirmed in time")
}
