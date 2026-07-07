//! Derived demo aggregates — every demo total (P&L, win rate, invested, trade
//! counts, wallet worth, period buckets) is computed here from the two token
//! arrays in `data.rs`, so the home / overview / positions / trader / header /
//! calendar endpoints all agree. Never hand-tune a total elsewhere; add it here.

use chrono::{DateTime, Duration, Utc};

use super::data::{DEMO_CLOSED_TOKENS, DEMO_OPEN_TOKENS};

// =============================================================================
// OPEN POSITIONS
// =============================================================================

#[derive(Clone, Copy)]
pub(super) struct Performer {
    pub symbol: &'static str,
    pub pnl_percent: f64,
}

pub(super) struct OpenAgg {
    pub count: usize,
    pub invested_sol: f64,
    /// Current market value of all open holdings (invested + unrealized).
    pub current_value_sol: f64,
    pub unrealized_pnl_sol: f64,
    pub unrealized_pnl_percent: f64,
    pub best: Performer,
    pub worst: Performer,
    pub avg_hold_minutes: i64,
}

pub(super) fn open_agg() -> OpenAgg {
    let mut invested = 0.0f64;
    let mut pnl = 0.0f64;
    let mut hold_sum = 0i64;
    let mut best = Performer {
        symbol: "",
        pnl_percent: f64::NEG_INFINITY,
    };
    let mut worst = Performer {
        symbol: "",
        pnl_percent: f64::INFINITY,
    };

    for (symbol, _name, _mint, _logo, entry, current, size, hold) in DEMO_OPEN_TOKENS {
        let pct = (current - entry) / entry * 100.0;
        invested += size;
        pnl += (current - entry) / entry * size;
        hold_sum += hold;
        if pct > best.pnl_percent {
            best = Performer {
                symbol,
                pnl_percent: pct,
            };
        }
        if pct < worst.pnl_percent {
            worst = Performer {
                symbol,
                pnl_percent: pct,
            };
        }
    }

    let count = DEMO_OPEN_TOKENS.len();
    OpenAgg {
        count,
        invested_sol: invested,
        current_value_sol: invested + pnl,
        unrealized_pnl_sol: pnl,
        unrealized_pnl_percent: if invested > 0.0 {
            pnl / invested * 100.0
        } else {
            0.0
        },
        best,
        worst,
        avg_hold_minutes: if count > 0 {
            hold_sum / count as i64
        } else {
            0
        },
    }
}

// =============================================================================
// CLOSED POSITIONS (realized) — time-bucketed
// =============================================================================

/// A synthesized closed trade with a deterministic exit timestamp. The same
/// schedule feeds the positions list, the period stats, and the calendar so
/// timestamps line up across every demo surface.
#[derive(Clone)]
pub(super) struct ClosedTrade {
    pub symbol: &'static str,
    pub reason: &'static str,
    pub pnl_sol: f64,
    pub pnl_percent: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub hold_minutes: i64,
}

/// Hours-ago the i-th closed trade exited (newest first): a realistic burst of
/// recent activity fanning out to fill the trailing month.
pub(super) fn closed_exit_offset_hours(i: usize) -> i64 {
    let i = i as i64;
    if i < 4 {
        (i + 1) * 6 // 6h..24h  → "today"
    } else if i < 8 {
        24 + (i - 3) * 6 // 30h..48h → "yesterday"
    } else {
        48 + (i - 7) * 36 // 84h..~23d → rest of the month
    }
}

pub(super) fn closed_trades(now: DateTime<Utc>) -> Vec<ClosedTrade> {
    DEMO_CLOSED_TOKENS
        .iter()
        .enumerate()
        .map(
            |(i, (symbol, _name, _mint, _logo, entry, exit, size, reason))| {
                let exit_time = now - Duration::hours(closed_exit_offset_hours(i));
                let hold = 90 + (i as i64 % 6) * 45; // 90m..315m, varied
                let entry_time = exit_time - Duration::minutes(hold);
                ClosedTrade {
                    symbol,
                    reason,
                    pnl_sol: (exit - entry) / entry * size,
                    pnl_percent: (exit - entry) / entry * 100.0,
                    entry_time,
                    exit_time,
                    hold_minutes: hold,
                }
            },
        )
        .collect()
}

/// Aggregate stats over an arbitrary set of closed trades.
pub(super) struct PeriodAgg {
    pub sells: i64,
    pub profit_sol: f64,
    pub loss_sol: f64,
    pub net_pnl_sol: f64,
    pub wins: i64,
    pub win_rate: f64,
    /// Largest single-trade loss magnitude in %, as a plausible drawdown proxy.
    pub drawdown_percent: f64,
}

pub(super) fn period_over<'a>(trades: impl Iterator<Item = &'a ClosedTrade>) -> PeriodAgg {
    let mut sells = 0i64;
    let mut profit = 0.0f64;
    let mut loss = 0.0f64;
    let mut wins = 0i64;
    let mut max_loss_pct = 0.0f64;

    for t in trades {
        sells += 1;
        if t.pnl_sol >= 0.0 {
            profit += t.pnl_sol;
            wins += 1;
        } else {
            loss += -t.pnl_sol;
            max_loss_pct = max_loss_pct.max(-t.pnl_percent);
        }
    }

    PeriodAgg {
        sells,
        profit_sol: profit,
        loss_sol: loss,
        net_pnl_sol: profit - loss,
        wins,
        win_rate: if sells > 0 {
            wins as f64 / sells as f64 * 100.0
        } else {
            0.0
        },
        drawdown_percent: max_loss_pct,
    }
}

/// Closed trades whose exit falls within the last `hours`.
pub(super) fn within_hours(trades: &[ClosedTrade], now: DateTime<Utc>, hours: i64) -> PeriodAgg {
    let cutoff = now - Duration::hours(hours);
    period_over(trades.iter().filter(|t| t.exit_time >= cutoff))
}

/// Closed trades whose exit falls within [from, to) hours ago.
pub(super) fn between_hours(
    trades: &[ClosedTrade],
    now: DateTime<Utc>,
    from_hours: i64,
    to_hours: i64,
) -> PeriodAgg {
    let newer = now - Duration::hours(from_hours);
    let older = now - Duration::hours(to_hours);
    period_over(
        trades
            .iter()
            .filter(|t| t.exit_time < newer && t.exit_time >= older),
    )
}

/// Per-exit-reason breakdown (count + average P&L %), for the trader stats tab.
pub(super) struct ReasonAgg {
    pub reason: String,
    pub count: usize,
    pub avg_profit_pct: f64,
}

pub(super) fn reason_breakdown(trades: &[ClosedTrade]) -> Vec<ReasonAgg> {
    use std::collections::BTreeMap;
    let mut map: BTreeMap<&str, (usize, f64)> = BTreeMap::new();
    for t in trades {
        let e = map.entry(t.reason).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += t.pnl_percent;
    }
    let mut out: Vec<ReasonAgg> = map
        .into_iter()
        .map(|(reason, (count, sum_pct))| ReasonAgg {
            reason: reason.to_owned(),
            count,
            avg_profit_pct: if count > 0 {
                sum_pct / count as f64
            } else {
                0.0
            },
        })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count));
    out
}

/// Best / worst realized trade by P&L %.
pub(super) fn best_worst(trades: &[ClosedTrade]) -> (Performer, Performer) {
    let mut best = Performer {
        symbol: "",
        pnl_percent: f64::NEG_INFINITY,
    };
    let mut worst = Performer {
        symbol: "",
        pnl_percent: f64::INFINITY,
    };
    for t in trades {
        if t.pnl_percent > best.pnl_percent {
            best = Performer {
                symbol: t.symbol,
                pnl_percent: t.pnl_percent,
            };
        }
        if t.pnl_percent < worst.pnl_percent {
            worst = Performer {
                symbol: t.symbol,
                pnl_percent: t.pnl_percent,
            };
        }
    }
    (best, worst)
}

/// Average hold time across closed trades, in hours.
pub(super) fn avg_hold_hours(trades: &[ClosedTrade]) -> f64 {
    if trades.is_empty() {
        return 0.0;
    }
    let mins: i64 = trades.iter().map(|t| t.hold_minutes).sum();
    (mins as f64 / trades.len() as f64) / 60.0
}
