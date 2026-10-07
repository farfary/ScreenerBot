// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The exit slippage ladder shared by full and partial exits: quote and sell at
//! each rung in turn, and send again only when the last attempt provably sold
//! nothing.

use std::future::Future;

use crate::logger::{self, LogTag};
use crate::swaps::{
    failed_swap, not_submitted_reason, program_error, FailedSwap, NotSubmittedReason, Quote,
    SwapResult,
};

/// The venue a graduated token's closed bonding curve is excluded as.
const CLOSED_CURVE_VENUE: &str = "Pump.fun Amm";

/// How an exit ladder ended.
#[derive(Debug)]
pub(super) enum ExitSwap {
    /// A rung's swap confirmed.
    Executed(SwapResult),
    /// A swap reached the chain or may still land: record and verify
    /// `signature`, and never sell again in its place.
    Submitted(String),
    /// Nothing was sold. `not_submitted` says why the last attempt stopped
    /// before it was sent, when it did.
    Failed {
        detail: String,
        not_submitted: Option<NotSubmittedReason>,
    },
}

/// The Pump.fun program's error for a trade against a bonding curve that has
/// completed and migrated.
const BONDING_CURVE_COMPLETE: u32 = 0x1787;

/// Whether a swap failed on a graduated token's closed bonding curve, which
/// the same trade routed around that venue can still fill.
fn closed_bonding_curve(error: &crate::Error) -> bool {
    program_error(error) == Some(BONDING_CURVE_COMPLETE)
}

/// Run the exit ladder over `steps` (slippage percentages).
///
/// `quote` prices the exit at a slippage with optional excluded venues, and
/// `execute_quote` sends a quote. After a failed send the ladder reads
/// [`failed_swap`]: a signature that may still land ends it as
/// [`ExitSwap::Submitted`], a failure that cannot prove it sold nothing ends it
/// as [`ExitSwap::Failed`], and only a failure that provably sold nothing
/// moves on to another send. A partial exit re-sent after an unproven failure
/// would sell the same share twice while the position books one.
pub(super) async fn run_exit_ladder<Q, QF, X, XF>(
    exit: &str,
    symbol: &str,
    steps: &[f64],
    mut quote: Q,
    mut execute_quote: X,
) -> ExitSwap
where
    Q: FnMut(f64, Option<Vec<String>>) -> QF,
    QF: Future<Output = crate::Result<Quote>>,
    X: FnMut(Quote) -> XF,
    XF: Future<Output = crate::Result<SwapResult>>,
{
    let mut detail = format!("{exit} swap failed");
    let mut not_submitted = None;

    for (index, slippage) in steps.iter().copied().enumerate() {
        let rung = format!("step {} ({slippage}%)", index + 1);
        let rung_quote = match quote(slippage, None).await {
            Ok(rung_quote) => rung_quote,
            Err(e) => {
                detail = format!("Quote failed at {rung}: {e}");
                not_submitted = None;
                super::backoff_after(&e).await;
                continue;
            }
        };

        let error = match execute_quote(rung_quote).await {
            Ok(result) => return ExitSwap::Executed(result),
            Err(error) => error,
        };
        match failed_swap(&error) {
            FailedSwap::Reconcile { signature } => {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "{exit} swap {signature} for {symbol} may still land - not selling again; verification will settle it"
                    ),
                );
                return ExitSwap::Submitted(signature);
            }
            FailedSwap::Unresolved => {
                logger::error(
                    LogTag::Positions,
                    &format!(
                        "{exit} swap for {symbol} failed at {rung}, and nothing proves it sold nothing - not selling again: {error}"
                    ),
                );
                return ExitSwap::Failed {
                    detail: format!("{exit} swap failed at {rung}: {error}"),
                    not_submitted: None,
                };
            }
            FailedSwap::Resendable => {}
        }

        if closed_bonding_curve(&error) {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Pump.fun bonding curve error for {symbol}, retrying the {exit} without {CLOSED_CURVE_VENUE}"
                ),
            );
            let routed_around =
                match quote(slippage, Some(vec![CLOSED_CURVE_VENUE.to_owned()])).await {
                    Ok(routed_around) => routed_around,
                    Err(e) => {
                        detail = format!(
                            "Retry without {CLOSED_CURVE_VENUE} also failed (quote) at {rung}: {e}"
                        );
                        not_submitted = None;
                        continue;
                    }
                };
            let retry_error = match execute_quote(routed_around).await {
                Ok(result) => return ExitSwap::Executed(result),
                Err(retry_error) => retry_error,
            };
            match failed_swap(&retry_error) {
                FailedSwap::Reconcile { signature } => return ExitSwap::Submitted(signature),
                FailedSwap::Unresolved => {
                    return ExitSwap::Failed {
                        detail: format!(
                            "Retry without {CLOSED_CURVE_VENUE} failed at {rung}: {retry_error}"
                        ),
                        not_submitted: None,
                    }
                }
                FailedSwap::Resendable => {
                    detail = format!(
                        "Retry without {CLOSED_CURVE_VENUE} failed at {rung}: {retry_error}"
                    );
                    not_submitted = not_submitted_reason(&retry_error);
                    continue;
                }
            }
        }

        detail = format!("{exit} swap failed at {rung}: {error}");
        not_submitted = not_submitted_reason(&error);
        super::backoff_after(&error).await;
    }

    ExitSwap::Failed {
        detail,
        not_submitted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ExecutionFailure;
    use crate::swaps::{SwapExecutionError, SwapMode};
    use std::sync::Mutex;

    const STEPS: [f64; 3] = [3.0, 10.0, 25.0];

    fn quote_at(slippage: f64) -> Quote {
        Quote {
            chain: crate::chains::ChainId::Solana,
            router_id: "jupiter".to_owned(),
            router_name: "Jupiter".to_owned(),
            input_mint: "TokenMint1111111111111111111111111111111111".to_owned(),
            output_mint: "So11111111111111111111111111111111111111112".to_owned(),
            input_amount: 1_000u64.into(),
            output_amount: 900u64.into(),
            minimum_output_amount: 850u64.into(),
            price_impact_pct: 0.1,
            platform_fee_lamports: None,
            estimated_network_fee_lamports: None,
            slippage_bps: (slippage * 100.0) as u16,
            route_plan: "test".to_owned(),
            swap_mode: SwapMode::ExactIn,
            wallet_address: "Wallet111111111111111111111111111111111111".to_owned(),
            exclude_dexes: None,
            execution_data: Vec::new(),
        }
    }

    /// Runs the ladder with every send failing as `fail` and returns the
    /// outcome with the quotes that were sent, as (slippage bps, exclusions).
    async fn ladder(
        fail: impl Fn() -> crate::Error,
    ) -> (ExitSwap, Vec<(u16, Option<Vec<String>>)>) {
        let sent = Mutex::new(Vec::new());
        let outcome = run_exit_ladder(
            "Partial exit",
            "TEST",
            &STEPS,
            |slippage, exclude_dexes| {
                let mut quote = quote_at(slippage);
                quote.exclude_dexes = exclude_dexes;
                async move { Ok(quote) }
            },
            |quote: Quote| {
                sent.lock()
                    .unwrap()
                    .push((quote.slippage_bps, quote.exclude_dexes));
                let error = fail();
                async move { Err(error) }
            },
        )
        .await;
        (outcome, sent.into_inner().unwrap())
    }

    /// A send that failed without proving it sold nothing is the last send:
    /// another rung would sell the same share a second time.
    #[tokio::test(start_paused = true)]
    async fn an_unproven_failure_stops_the_ladder_after_one_send() {
        let (outcome, sent) = ladder(|| {
            crate::Error::Rpc(crate::rpc::RpcError::Network {
                message: "connection reset during sendTransaction".to_owned(),
                is_timeout: true,
            })
        })
        .await;
        assert_eq!(sent.len(), 1);
        assert!(matches!(
            outcome,
            ExitSwap::Failed {
                not_submitted: None,
                ..
            }
        ));
    }

    /// A signature that may still land is handed back to be verified, never
    /// sold again.
    #[tokio::test(start_paused = true)]
    async fn a_swap_that_may_still_land_is_submitted_not_resold() {
        let (outcome, sent) = ladder(|| {
            crate::Error::Solana(crate::chains::solana::Error::Execution(
                ExecutionFailure::ConfirmationTimeout {
                    reference: "sig".to_owned(),
                    waited_ms: 60_000,
                },
            ))
        })
        .await;
        assert_eq!(sent.len(), 1);
        assert!(matches!(outcome, ExitSwap::Submitted(signature) if signature == "sig"));
    }

    /// Only a failure that provably sold nothing climbs the ladder: a refusal
    /// before the send, and a sell the chain reverted at `Confirmed` (the usual
    /// slippage miss the ladder exists for).
    #[tokio::test(start_paused = true)]
    async fn a_provably_unsold_failure_climbs_every_rung() {
        let (outcome, sent) = ladder(|| {
            crate::Error::Swaps(SwapExecutionError::NotSubmitted {
                router: "Jupiter".to_owned(),
                reason: NotSubmittedReason::SimulationFailed {
                    detail: "slippage".to_owned(),
                },
            })
        })
        .await;
        assert_eq!(
            sent.iter().map(|(bps, _)| *bps).collect::<Vec<_>>(),
            vec![300, 1000, 2500]
        );
        assert!(matches!(
            outcome,
            ExitSwap::Failed {
                not_submitted: Some(NotSubmittedReason::SimulationFailed { .. }),
                ..
            }
        ));

        let (_, sent) = ladder(|| {
            crate::Error::Solana(crate::chains::solana::Error::Execution(
                ExecutionFailure::Reverted {
                    reference: "sig".to_owned(),
                    detail: "custom program error: 0x1771".to_owned(),
                },
            ))
        })
        .await;
        assert_eq!(sent.len(), 3);
    }

    /// A closed bonding curve is routed around once per rung, and only after
    /// the curve's own failure proved nothing was sold.
    #[tokio::test(start_paused = true)]
    async fn a_closed_curve_is_routed_around_only_after_a_provable_refusal() {
        let (_, sent) = ladder(|| {
            crate::Error::Swaps(SwapExecutionError::NotSubmitted {
                router: "Jupiter".to_owned(),
                reason: NotSubmittedReason::SimulationFailed {
                    detail: "custom program error: 0x1787".to_owned(),
                },
            })
        })
        .await;
        assert_eq!(sent.len(), 6);
        assert_eq!(sent[1].1, Some(vec![CLOSED_CURVE_VENUE.to_owned()]));

        let (_, sent) = ladder(|| {
            crate::Error::Rpc(crate::rpc::RpcError::Other(
                "custom program error: 0x1787".to_owned(),
            ))
        })
        .await;
        assert_eq!(
            sent.len(),
            1,
            "an unproven curve failure is not routed around"
        );
    }

    /// The closed curve is read from the program error a provable failure
    /// carries, in either form a node reports it, and never from other numbers
    /// in the failure's text.
    #[tokio::test(start_paused = true)]
    async fn a_closed_curve_is_read_from_the_program_error_alone() {
        let closed: [fn() -> crate::Error; 3] = [
            || {
                crate::Error::Swaps(SwapExecutionError::NotSubmitted {
                    router: "Jupiter".to_owned(),
                    reason: NotSubmittedReason::SimulationFailed {
                        detail: r#"{"InstructionError":[2,{"Custom":6023}]}"#.to_owned(),
                    },
                })
            },
            || {
                crate::Error::Solana(crate::chains::solana::Error::DirectSwap(
                    crate::chains::solana::swaps::direct::DirectSwapError::SimulationRejected {
                        detail: "Error processing Instruction 2: custom program error: 0x1787"
                            .to_owned(),
                        logs: Vec::new(),
                    },
                ))
            },
            || {
                crate::Error::Solana(crate::chains::solana::Error::Execution(
                    ExecutionFailure::Reverted {
                        reference: "sig".to_owned(),
                        detail: "Error processing Instruction 2: custom program error: 0x1787"
                            .to_owned(),
                    },
                ))
            },
        ];
        for fail in closed {
            let (_, sent) = ladder(fail).await;
            assert_eq!(sent[1].1, Some(vec![CLOSED_CURVE_VENUE.to_owned()]));
        }

        let other: [fn() -> crate::Error; 2] = [
            || {
                crate::Error::Swaps(SwapExecutionError::NotSubmitted {
                    router: "Jupiter".to_owned(),
                    reason: NotSubmittedReason::SimulationFailed {
                        detail: "insufficient lamports 60230, need 6023".to_owned(),
                    },
                })
            },
            || {
                crate::Error::Swaps(SwapExecutionError::NotSubmitted {
                    router: "Jupiter".to_owned(),
                    reason: NotSubmittedReason::SimulationFailed {
                        detail: "custom program error: 0x1771 at slot 6023".to_owned(),
                    },
                })
            },
        ];
        for fail in other {
            let (_, sent) = ladder(fail).await;
            assert!(
                sent.iter().all(|(_, excluded)| excluded.is_none()),
                "another failure is taken for a closed curve"
            );
        }
    }
}
