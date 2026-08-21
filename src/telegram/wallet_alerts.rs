//! Telegram consumer for alert-only wallet-watch targets.

use std::sync::Arc;

use tokio::sync::{broadcast::error::RecvError, Notify};

use crate::logger::{self, LogTag};
use crate::telegram::{queue_notification, Notification};
use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

/// Convert one observed wallet activity into the existing tracked-wallet trade
/// notification. Non-alert sources and non-swap activity are deliberately ignored:
/// `NotificationType::TradeAlert` is governed by the user's trade-alert preference
/// and minimum-SOL threshold, while transfers have no matching notification policy.
pub(super) fn notification_for_activity(activity: &WalletActivity) -> Option<Notification> {
    if !activity
        .sources
        .iter()
        .any(|source| matches!(source, WatchSource::Alert { .. }))
    {
        return None;
    }

    let ActivityKind::Swap {
        mint,
        side,
        sol_amount,
        ..
    } = &activity.kind
    else {
        return None;
    };

    let symbol = crate::tokens::get_cached_token(crate::chains::ChainId::Solana, mint)
        .map(|token| token.symbol)
        .filter(|symbol| !symbol.trim().is_empty())
        .unwrap_or_else(|| "Unknown".to_owned());
    let trade_type = match side {
        SwapSide::Buy => "buy",
        SwapSide::Sell => "sell",
    };

    Some(Notification::trade_alert(
        symbol,
        mint.clone(),
        trade_type,
        *sol_amount,
        activity.subject.clone(),
    ))
}

/// Consume the shared wallet-watch broadcast without ever blocking its producer.
/// The broadcast is bounded, so a slow Telegram connection drops alerts for this
/// consumer rather than delaying copy decisions or own-wallet processing.
pub(crate) async fn run(shutdown: Arc<Notify>) {
    let mut activity_rx = crate::wallets::watch::subscribe_activity();

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            activity = activity_rx.recv() => {
                match activity {
                    Ok(activity) => {
                        if let Some(notification) = notification_for_activity(&activity) {
                            queue_notification(notification);
                        }
                    }
                    Err(RecvError::Lagged(skipped)) => logger::warning(
                        LogTag::Telegram,
                        &format!("Wallet alert consumer lagged by {skipped} activities"),
                    ),
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::telegram::NotificationType;

    fn activity(kind: ActivityKind, sources: Vec<WatchSource>) -> WalletActivity {
        WalletActivity {
            subject: "11111111111111111111111111111111".to_owned(),
            signature: "signature".to_owned(),
            slot: 1,
            block_time: Some(1),
            detected_at: Utc::now(),
            decoded_at: Utc::now(),
            success: true,
            kind,
            sources,
        }
    }

    #[test]
    fn alert_swap_becomes_a_trade_notification() {
        let notification = notification_for_activity(&activity(
            ActivityKind::Swap {
                mint: "Mint111111111111111111111111111111111111111".to_owned(),
                side: SwapSide::Buy,
                sol_amount: 0.25,
                token_amount: 10.0,
                venue: None,
                price_sol: Some(0.025),
            },
            vec![WatchSource::Alert { rule_id: 7 }],
        ))
        .expect("alert notification");

        match notification.notification_type {
            NotificationType::TradeAlert {
                token_mint,
                trade_type,
                amount_sol,
                wallet,
                ..
            } => {
                assert_eq!(token_mint, "Mint111111111111111111111111111111111111111");
                assert_eq!(trade_type, "buy");
                assert_eq!(amount_sol, 0.25);
                assert_eq!(wallet, "11111111111111111111111111111111");
            }
            other => panic!("unexpected notification: {other:?}"),
        }
    }

    #[test]
    fn own_wallet_and_transfer_activity_do_not_send_trade_alerts() {
        let swap = ActivityKind::Swap {
            mint: "Mint222222222222222222222222222222222222222".to_owned(),
            side: SwapSide::Sell,
            sol_amount: 1.0,
            token_amount: 5.0,
            venue: None,
            price_sol: Some(0.2),
        };
        assert!(notification_for_activity(&activity(swap, vec![WatchSource::OwnWallet])).is_none());

        let transfer = ActivityKind::Transfer {
            mint: "Mint333333333333333333333333333333333333333".to_owned(),
            amount: 2.0,
            direction: crate::wallets::watch::TransferDirection::In,
        };
        assert!(notification_for_activity(&activity(
            transfer,
            vec![WatchSource::Alert { rule_id: 9 }]
        ))
        .is_none());
    }
}
