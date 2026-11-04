// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! A live sweep of the direct engine across a list of real pools.
//!
//! `direct_swaps_mainnet` proves each venue on a handful of chosen pools; this
//! sweep asks the same question of every pool in a list, so the shapes a chosen
//! pool never exercises (a Token-2022 extension, a newer account layout, a quiet
//! tick range) show up as a refused or failed simulation before a trade meets
//! them. Every pool is loaded, quoted and simulated at ZERO slippage in both
//! directions, so a quote that over-states its output by one raw unit fails;
//! a pool that fails at zero slippage is simulated again at 3% to separate an
//! over-stated quote from a transaction that cannot execute at all. Nothing is
//! signed or submitted.
//!
//! Input: `SB_DIRECT_SWEEP_POOLS`, a file with one pool address per line, such
//! as the SOL pools a real profile tracks in `tokens.db`. Output: one
//! tab-separated row per pool and leg on stdout and in `SB_DIRECT_SWEEP_REPORT`
//! when set. The RPC endpoints are the real profile's when one is present, the
//! default public endpoint otherwise. Self-skips when no pool list is given.

use crate::common;

use futures::StreamExt;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::spl_token_2022::extension::{
    BaseStateWithExtensions, StateWithExtensions,
};
use screenerbot::chains::solana::spl_token_2022::state::Mint;
use screenerbot::chains::solana::swaps::direct::{
    self, AccountReader, DirectSwapIntent, NodeAccounts, PoolMarket,
};
use std::str::FromStr;

const WSOL: &str = "So11111111111111111111111111111111111111112";

/// A 0.01 SOL buy: twice the smallest trade, still small enough for any pool
/// the bot would ever trade.
const BUY_LAMPORTS: u64 = 10_000_000;

/// Pools probed at once. Each probe makes a handful of reads and two or four
/// simulations; more than this trips the rate limit of a paid endpoint too.
const CONCURRENCY: usize = 4;

/// Attempts at zero slippage against fresh state, as in `direct_swaps_mainnet`:
/// a deep pool can trade between the load and the simulation.
const ZERO_SLIPPAGE_ATTEMPTS: usize = 3;

/// What one leg of one pool did.
enum Leg {
    /// Accepted at zero slippage: the quote is exact.
    Exact { units: Option<u64> },
    /// Refused at zero slippage on every attempt, accepted at 3%: the quote
    /// over-states the output.
    OverStated { detail: String },
    /// Refused at 3% too: the transaction cannot execute.
    Failed { detail: String },
    /// The engine refused before a transaction existed.
    Refused { stage: &'static str, detail: String },
    /// The leg could not be probed (no holder to borrow for a sell).
    Skipped { detail: String },
}

impl Leg {
    fn row(&self) -> (&'static str, String) {
        match self {
            Leg::Exact { units } => (
                "exact",
                units.map(|units| format!("{units} CU")).unwrap_or_default(),
            ),
            Leg::OverStated { detail } => ("over-stated", detail.clone()),
            Leg::Failed { detail } => ("failed", detail.clone()),
            Leg::Refused { stage, detail } => (stage, detail.clone()),
            Leg::Skipped { detail } => ("skipped", detail.clone()),
        }
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(400)
        .collect()
}

/// The program error lines of a refused simulation, which name the failing
/// instruction far better than the summary does.
fn simulation_detail(
    outcome: &screenerbot::chains::solana::rpc::types::SimulationOutcome,
) -> String {
    let errors: Vec<&str> = outcome
        .logs
        .iter()
        .map(String::as_str)
        .filter(|line| line.contains("Error") || line.contains("error") || line.contains("failed"))
        .take(3)
        .collect();
    one_line(&format!(
        "{} | {}",
        outcome.failure_detail(),
        errors.join(" | ")
    ))
}

/// The Token-2022 extensions of `mint`, by name, or `spl` for a legacy mint.
async fn mint_shape(mint: &Pubkey) -> String {
    let account = match NodeAccounts.read_accounts(&[*mint]).await {
        Ok(mut accounts) => accounts.pop().flatten(),
        Err(error) => return format!("unreadable: {error}"),
    };
    let Some(account) = account else {
        return "missing".to_owned();
    };
    if account.owner != screenerbot::chains::solana::spl_token_2022::id() {
        return "spl".to_owned();
    }
    match StateWithExtensions::<Mint>::unpack(&account.data) {
        Ok(state) => {
            let names: Vec<String> = state
                .get_extension_types()
                .unwrap_or_default()
                .iter()
                .map(|extension| format!("{extension:?}"))
                .collect();
            format!("t22[{}]", names.join(","))
        }
        Err(error) => format!("t22-undecodable: {error}"),
    }
}

/// Simulate `intent` at zero slippage, then at 3% when every zero-slippage
/// attempt is refused.
async fn probe(intent: DirectSwapIntent, owner: Pubkey) -> Leg {
    let mut exact = intent.clone();
    exact.slippage_bps = 0;
    let mut last_refusal = String::new();
    for _ in 0..ZERO_SLIPPAGE_ATTEMPTS {
        let (quote, market) = match direct::quote(&exact).await {
            Ok(pair) => pair,
            Err(error) => {
                return Leg::Refused {
                    stage: "quote-refused",
                    detail: one_line(&format!("{error:?}")),
                }
            }
        };
        let plan = match direct::build_plan(&exact, market.as_ref(), &quote) {
            Ok(plan) => plan,
            Err(error) => {
                return Leg::Refused {
                    stage: "build-refused",
                    detail: one_line(&format!("{error:?}")),
                }
            }
        };
        match direct::simulate_plan(&plan, &owner).await {
            Ok(outcome) if outcome.succeeded() => {
                return Leg::Exact {
                    units: outcome.units_consumed,
                }
            }
            Ok(outcome) => last_refusal = simulation_detail(&outcome),
            Err(error) => {
                return Leg::Refused {
                    stage: "simulate-refused",
                    detail: one_line(&format!("{error:?}")),
                }
            }
        }
    }

    let mut loose = intent;
    loose.slippage_bps = 300;
    let outcome = match direct::quote(&loose).await {
        Ok((quote, market)) => match direct::build_plan(&loose, market.as_ref(), &quote) {
            Ok(plan) => direct::simulate_plan(&plan, &owner).await,
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    match outcome {
        Ok(outcome) if outcome.succeeded() => Leg::OverStated {
            detail: last_refusal,
        },
        Ok(outcome) => Leg::Failed {
            detail: simulation_detail(&outcome),
        },
        Err(error) => Leg::Failed {
            detail: one_line(&format!("{error:?}")),
        },
    }
}

/// Probe both legs of one pool and return its report rows.
async fn sweep_pool(pool: Pubkey) -> Vec<String> {
    let market: Box<dyn PoolMarket> = match direct::load_market(&pool, &NodeAccounts).await {
        Ok(market) => market,
        Err(error) => {
            return vec![format!(
                "{pool}\t-\t-\t-\tload\tload-refused\t{}",
                one_line(&format!("{error:?}"))
            )]
        }
    };
    let venue = format!("{:?}", market.program());
    let wsol = Pubkey::from_str(WSOL).expect("WSOL constant");
    let (mint_a, mint_b) = market.mints();
    let token = if mint_a == wsol {
        mint_b
    } else if mint_b == wsol {
        mint_a
    } else {
        return vec![format!(
            "{pool}\t{venue}\t-\t-\tload\tskipped\tno SOL side ({mint_a} / {mint_b})"
        )];
    };
    let shape = mint_shape(&token).await;
    let row = |leg: &str, outcome: &Leg| {
        let (kind, detail) = outcome.row();
        format!("{pool}\t{venue}\t{token}\t{shape}\t{leg}\t{kind}\t{detail}")
    };

    let buy_owner = common::deep_pockets_owner();
    let buy_intent = DirectSwapIntent {
        pool,
        owner: buy_owner,
        input_mint: wsol,
        output_mint: token,
        amount_in: BUY_LAMPORTS,
        slippage_bps: 300,
    };
    let buy = probe(buy_intent.clone(), buy_owner).await;

    let sell = match direct::quote_with_market(&buy_intent, market.as_ref()) {
        Err(error) => Leg::Skipped {
            detail: one_line(&format!("no buy to size the sell from: {error:?}")),
        },
        Ok(sized) => {
            let program = market
                .token_program(&token)
                .unwrap_or_else(|| screenerbot::chains::solana::spl_token_2022::id());
            let holders =
                common::sell_holders(&pool, &token, &program, sized.expected_net_out).await;
            match holders
                .iter()
                .copied()
                .find(|(_, held)| *held >= sized.expected_net_out)
                .or_else(|| holders.iter().copied().max_by_key(|(_, held)| *held))
                .filter(|(_, held)| *held > 0)
            {
                None => Leg::Skipped {
                    detail: "no holder with a balance".to_owned(),
                },
                Some((owner, held)) => {
                    probe(
                        DirectSwapIntent {
                            pool,
                            owner,
                            input_mint: token,
                            output_mint: wsol,
                            amount_in: sized.expected_net_out.min(held),
                            slippage_bps: 300,
                        },
                        owner,
                    )
                    .await
                }
            }
        }
    };
    vec![row("buy", &buy), row("sell", &sell)]
}

/// Point the RPC pool at the real profile's endpoints, when one is present.
fn use_real_endpoints() {
    let Some(text) = common::real_config_toml() else {
        return;
    };
    let urls: Option<Vec<String>> = toml::from_str::<toml::Value>(&text).ok().and_then(|value| {
        value
            .get("chains")?
            .get("solana")?
            .get("rpc")?
            .get("urls")?
            .as_array()
            .map(|urls| {
                urls.iter()
                    .filter_map(|url| url.as_str().map(str::to_owned))
                    .collect()
            })
    });
    if let Some(urls) = urls.filter(|urls| !urls.is_empty()) {
        common::set_config(|config| config.chains.solana.rpc.urls = urls);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live network"]
async fn every_listed_pool_quotes_exactly_in_both_directions() {
    let Ok(list) = std::env::var("SB_DIRECT_SWEEP_POOLS") else {
        eprintln!("SKIP: SB_DIRECT_SWEEP_POOLS names no pool list");
        return;
    };
    let _guard = common::isolated_env();
    use_real_endpoints();

    let text = std::fs::read_to_string(&list).expect("the pool list must be readable");
    let pools: Vec<Pubkey> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| Pubkey::from_str(line).expect("each line must be a pool address"))
        .collect();

    let mut rows = vec!["pool\tvenue\ttoken\tmint\tleg\toutcome\tdetail".to_owned()];
    let mut results =
        futures::stream::iter(pools.into_iter().map(sweep_pool)).buffer_unordered(CONCURRENCY);
    while let Some(pool_rows) = results.next().await {
        for row in pool_rows {
            println!("{row}");
            rows.push(row);
        }
    }
    if let Ok(report) = std::env::var("SB_DIRECT_SWEEP_REPORT") {
        std::fs::write(&report, rows.join("\n") + "\n").expect("the report must be writable");
    }
}
