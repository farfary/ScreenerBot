// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The single-account token balance must be the balance of the associated token account.
//!
//! Swap builders debit the wallet's associated token account for the mint, and the full
//! sell caps its amount to `get_token_balance`. When the wallet also holds another token
//! account of the same mint, the balance read must still describe the associated one,
//! whichever account the RPC lists first.
//!
//! A loopback JSON-RPC server stands in for the provider, so nothing leaves the machine.

use crate::common;

use screenerbot::chains::solana::assets::ata::get_token_balance;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::spl_associated_token_account::get_associated_token_address_with_program_id;
use screenerbot::chains::solana::spl_token;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn token_account(pubkey: &Pubkey, mint: &Pubkey, amount: u64) -> Value {
    json!({
        "pubkey": pubkey.to_string(),
        "account": { "data": { "parsed": { "info": {
            "mint": mint.to_string(),
            "state": "initialized",
            "tokenAmount": { "amount": amount.to_string(), "decimals": 6 }
        } } } }
    })
}

/// Answer one JSON-RPC request: the fixture for `getTokenAccountsByOwner`, an inert
/// string for every other method (health probes and the like).
fn answer(request: &Value, accounts: &[Value]) -> Value {
    let result = if request["method"] == "getTokenAccountsByOwner" {
        json!({ "context": { "slot": 1 }, "value": accounts })
    } else {
        json!("ok")
    };
    json!({ "jsonrpc": "2.0", "id": request["id"], "result": result })
}

async fn serve_connection(mut socket: TcpStream, accounts: Vec<Value>) {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let header_end = loop {
            if let Some(at) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                break at + 4;
            }
            match socket.read(&mut chunk).await {
                Ok(0) | Err(_) => return,
                Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            }
        };
        let head = String::from_utf8_lossy(&buffer[..header_end]).to_ascii_lowercase();
        let length = head
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        while buffer.len() < header_end + length {
            match socket.read(&mut chunk).await {
                Ok(0) | Err(_) => return,
                Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            }
        }
        let request: Value =
            serde_json::from_slice(&buffer[header_end..header_end + length]).unwrap_or(Value::Null);
        buffer.drain(..header_end + length);

        let body = answer(&request, &accounts).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        if socket.write_all(response.as_bytes()).await.is_err() {
            return;
        }
    }
}

async fn spawn_rpc(accounts: Vec<Value>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(serve_connection(socket, accounts.clone()));
        }
    });
    format!("http://{address}")
}

#[test]
fn the_token_balance_is_the_associated_account_even_when_another_account_is_listed_first() {
    common::run_isolated(
        module_path!(),
        "the_token_balance_is_the_associated_account_even_when_another_account_is_listed_first",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");

            let wallet = Pubkey::new_unique();
            let mint = Pubkey::new_unique();
            let associated =
                get_associated_token_address_with_program_id(&wallet, &mint, &spl_token::id());
            let stray = Pubkey::new_unique();

            let url = spawn_rpc(vec![
                token_account(&stray, &mint, 40),
                token_account(&associated, &mint, 100),
            ])
            .await;
            common::set_config(|cfg| cfg.chains.solana.rpc.urls = vec![url]);

            let balance = get_token_balance(&wallet.to_string(), &mint.to_string())
                .await
                .expect("read the token balance");
            assert_eq!(
                balance, 100,
                "the swap debits the associated account holding 100, not the first listed account holding 40"
            );
        },
    );
}
