//! Promo generators for the Assistant page's tabs other than Overview
//! (`ai.rs` owns `/api/ai/status`).
//!
//! Every one of these tabs reads a local database that a fresh install has never
//! written: Providers lists the operator's own configured keys, Instructions and
//! Automation are user-authored records, History and Chat are the operator's own
//! conversations. Captured live they either publish private working material or —
//! more often — photograph an empty state, which is what a screenshot must never
//! show of a feature that works.
//!
//! The tabs describe one coherent desk: the same five providers `ai.rs` reports as
//! configured, six instruction rules, three scheduled tasks whose run history and
//! aggregate statistics reconcile with each other, and a decision log whose newest
//! entries are the ones the Overview tab shows as recent decisions.

use chrono::{DateTime, Duration, Utc};

use crate::assistant::chat::database::{ChatMessage, ChatSession};
use crate::assistant::scheduled::types::{AutomationStats, ScheduledTask, TaskRun};
use crate::webserver::routes::ai::types::{
    AiStatsResponse, CacheStatsResponse, DecisionHistoryResponse, GetChatSessionResponse,
    HistoryListResponse, InstructionResponse, InstructionsListResponse, ProvidersListResponse,
};

use super::ai::promo_provider_statuses;

/// The one clock every fixture in this module is measured against, so a run's
/// timestamps stay ordered relative to each other.
fn ago(minutes: i64) -> DateTime<Utc> {
    Utc::now() - Duration::minutes(minutes)
}

fn stamp(minutes: i64) -> String {
    ago(minutes).to_rfc3339()
}

// =============================================================================
// PROVIDERS
// =============================================================================

/// Generate the Providers tab list.
///
/// Shares `ai.rs`'s provider table rather than repeating it: the Overview tab
/// counts "5 of 9 active" from the same rows, and two tables would drift.
pub fn get_promo_providers() -> ProvidersListResponse {
    ProvidersListResponse {
        providers: promo_provider_statuses(),
        default_provider: "anthropic".to_owned(),
    }
}

// =============================================================================
// OVERVIEW METRICS
// =============================================================================

/// Generate the request/latency counters.
///
/// The totals match `get_promo_ai_status`'s `total_evaluations`, and the failure
/// count is what makes the success rate on screen a real division rather than a
/// flat 100%.
pub fn get_promo_ai_stats() -> AiStatsResponse {
    AiStatsResponse {
        total_requests: 18_432,
        successful_requests: 18_301,
        failed_requests: 131,
        avg_latency_ms: 742.0,
        cache_hit_rate: 0.86,
    }
}

/// Generate the evaluation-cache counters, matching the Overview tab's own.
pub fn get_promo_cache_stats() -> CacheStatsResponse {
    CacheStatsResponse {
        total_entries: 1_284,
        fresh_entries: 947,
        ttl_seconds: 1_800,
    }
}

// =============================================================================
// INSTRUCTIONS
// =============================================================================

/// (id, name, category, priority, enabled, content)
const PROMO_INSTRUCTIONS: &[(i64, &str, &str, i32, bool, &str)] = &[
    (
        1,
        "Reject unlocked liquidity",
        "filtering",
        100,
        true,
        "Reject any token whose liquidity is not burned or locked for at least 30 days. Treat an unverifiable lock as unlocked. This rule outranks every momentum signal.",
    ),
    (
        2,
        "Creator concentration ceiling",
        "filtering",
        90,
        true,
        "Reject when the creator wallet, or any single non-pool holder, controls more than 8% of supply. Bundled launch wallets count together when they funded from the same source.",
    ),
    (
        3,
        "Require two independent price sources",
        "filtering",
        80,
        true,
        "Do not approve a token priced by only one venue. A single thin pool is not a market and its price cannot be trusted for entry sizing.",
    ),
    (
        4,
        "Size down into thin books",
        "entry",
        70,
        true,
        "When pool liquidity is below 40 SOL, halve the configured entry size. Slippage on the exit, not the entry, is what decides whether the trade was affordable.",
    ),
    (
        5,
        "Hold through the first retrace",
        "exit",
        60,
        true,
        "Do not recommend an exit on the first pullback inside the opening 15 minutes unless volume is falling with price. Early volatility is not a trend break.",
    ),
    (
        6,
        "Explain every rejection",
        "general",
        10,
        false,
        "Always name the specific rule that produced a rejection and the measured value that failed it. A verdict with no measurement behind it is not reviewable.",
    ),
];

/// Generate the Instructions tab list.
pub fn get_promo_instructions() -> InstructionsListResponse {
    let instructions: Vec<InstructionResponse> = PROMO_INSTRUCTIONS
        .iter()
        .map(
            |(id, name, category, priority, enabled, content)| InstructionResponse {
                id: *id,
                name: (*name).to_owned(),
                content: (*content).to_owned(),
                category: (*category).to_owned(),
                priority: *priority,
                enabled: *enabled,
                created_at: stamp(60 * 24 * (12 - id)),
                updated_at: stamp(60 * (30 - id * 3)),
            },
        )
        .collect();

    InstructionsListResponse {
        total: instructions.len(),
        instructions,
    }
}

// =============================================================================
// AUTOMATION
// =============================================================================

/// (id, name, schedule type, schedule value, permissions, enabled, run count,
/// error count, minutes since last run, minutes until next run, instruction)
///
/// `schedule_value` is stated in the exact form the Automation tab parses:
/// whole seconds for `interval`, `HH:MM` for `daily`, `day:HH:MM` for `weekly`.
/// A human-readable "6h" renders as "Every 6s".
type PromoAutomation = (
    i64,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    bool,
    i64,
    i64,
    i64,
    i64,
    &'static str,
);

const PROMO_AUTOMATIONS: &[PromoAutomation] = &[
    (
        1,
        "Morning portfolio review",
        "daily",
        "08:00",
        "read_only",
        true,
        41,
        0,
        212,
        1_228,
        "Summarise every open position: entry, current P&L, time held, and whether its exit rules still fit the current liquidity. Flag anything worth closing by hand.",
    ),
    (
        2,
        "Blacklist sweep",
        "interval",
        "21600",
        "full",
        true,
        168,
        2,
        94,
        266,
        "Re-check every token approved in the last 24 hours against current holder distribution and liquidity. Blacklist any that now fail the concentration ceiling.",
    ),
    (
        3,
        "Weekly strategy report",
        "weekly",
        "sun:18:00",
        "read_only",
        false,
        6,
        1,
        4_320,
        6_480,
        "Compare this week's closed trades against the configured exit rules and report which rule earned or cost the most, with the trades that prove it.",
    ),
];

/// Generate the Automation tab's task list.
pub fn get_promo_automation_tasks() -> Vec<ScheduledTask> {
    PROMO_AUTOMATIONS
        .iter()
        .map(|entry| {
            let (
                id,
                name,
                schedule_type,
                schedule_value,
                permissions,
                enabled,
                run_count,
                error_count,
                last_run_mins,
                next_run_mins,
                instruction,
            ) = *entry;

            ScheduledTask {
                id,
                name: name.to_owned(),
                instruction: instruction.to_owned(),
                instruction_ids: None,
                schedule_type: schedule_type.to_owned(),
                schedule_value: schedule_value.to_owned(),
                tool_permissions: permissions.to_owned(),
                priority: "normal".to_owned(),
                notify_telegram: true,
                notify_on_success: false,
                notify_on_failure: true,
                enabled,
                max_retries: 2,
                timeout_seconds: 300,
                last_run_at: Some(stamp(last_run_mins)),
                // A disabled task has no next run; showing one would claim work that
                // will never happen.
                next_run_at: if enabled {
                    Some((Utc::now() + Duration::minutes(next_run_mins)).to_rfc3339())
                } else {
                    None
                },
                run_count,
                error_count,
                created_at: stamp(60 * 24 * (30 + id * 9)),
                updated_at: stamp(last_run_mins),
            }
        })
        .collect()
}

/// (run id, task id, status, minutes ago, duration ms, tokens, response)
type PromoRun = (i64, i64, &'static str, i64, f64, i64, &'static str);

const PROMO_RUNS: &[PromoRun] = &[
    (
        41,
        2,
        "success",
        94,
        8_420.0,
        3_180,
        "Re-checked 34 approvals. Two now exceed the 8% concentration ceiling (BONKAI 11.4%, SNAPZ 9.2%) and were blacklisted. The remaining 32 still pass.",
    ),
    (
        40,
        1,
        "success",
        212,
        12_970.0,
        5_640,
        "10 open positions, +0.417 SOL unrealised. TRUMP has run 16.4% past its trailing activation and is the only position whose exit rule is fully armed. Nothing needs a manual close.",
    ),
    (
        39,
        2,
        "success",
        454,
        7_980.0,
        2_940,
        "Re-checked 28 approvals. All still pass the concentration ceiling. Liquidity fell below 40 SOL on one (FWOG), which now sizes at half.",
    ),
    (
        38,
        2,
        "failed",
        814,
        30_000.0,
        0,
        "",
    ),
    (
        37,
        3,
        "success",
        4_320,
        21_450.0,
        9_120,
        "Trailing stop closed 6 of 9 winners and gave back 3.1% on average from peak. Stop loss saved 0.28 SOL across 5 trades. Take profit was the weakest rule this week: three of its exits kept running afterwards.",
    ),
    (
        36,
        1,
        "success",
        1_652,
        11_240.0,
        5_010,
        "9 open positions, +0.298 SOL unrealised. No position was outside its exit rules.",
    ),
];

/// Generate the Automation tab's run feed.
///
/// The one failed run carries an error and no response, because a run that failed
/// produced nothing to report — a failure row with a summary in it would be
/// describing work that did not finish.
pub fn get_promo_automation_runs() -> Vec<TaskRun> {
    PROMO_RUNS
        .iter()
        .map(
            |(id, task_id, status, mins_ago, duration_ms, tokens, response)| {
                let failed = *status != "success";
                TaskRun {
                    id: *id,
                    task_id: *task_id,
                    status: (*status).to_owned(),
                    started_at: stamp(*mins_ago),
                    completed_at: Some(stamp(mins_ago - (*duration_ms / 60_000.0).ceil() as i64)),
                    duration_ms: Some(*duration_ms),
                    ai_response: if failed {
                        None
                    } else {
                        Some((*response).to_owned())
                    },
                    tool_calls: None,
                    tokens_used: if failed { None } else { Some(*tokens) },
                    provider: Some("anthropic".to_owned()),
                    model: Some("claude-sonnet-5".to_owned()),
                    error_message: if failed {
                        Some("Provider request timed out after 300s".to_owned())
                    } else {
                        None
                    },
                    session_id: None,
                }
            },
        )
        .collect()
}

/// Generate the Automation tab's aggregate statistics.
///
/// Every field is counted from the task and run fixtures rather than stated, so the
/// summary cards cannot disagree with the table underneath them.
pub fn get_promo_automation_stats() -> AutomationStats {
    let tasks = get_promo_automation_tasks();
    let runs = get_promo_automation_runs();

    let total_runs: i64 = tasks.iter().map(|task| task.run_count).sum();
    let failed_runs: i64 = tasks.iter().map(|task| task.error_count).sum();
    let successful_runs = total_runs - failed_runs;

    let avg_duration_ms = if runs.is_empty() {
        0.0
    } else {
        runs.iter().filter_map(|run| run.duration_ms).sum::<f64>() / runs.len() as f64
    };

    AutomationStats {
        total_tasks: tasks.len() as i64,
        active_tasks: tasks.iter().filter(|task| task.enabled).count() as i64,
        total_runs,
        successful_runs,
        failed_runs,
        success_rate: if total_runs == 0 {
            0.0
        } else {
            successful_runs as f64 / total_runs as f64 * 100.0
        },
        avg_duration_ms,
        // Counted from the fixture's own age offsets, not by comparing the rendered
        // timestamp strings.
        runs_today: PROMO_RUNS
            .iter()
            .filter(|(_, _, _, mins_ago, ..)| *mins_ago < 60 * 24)
            .count() as i64,
    }
}

// =============================================================================
// DECISION HISTORY
// =============================================================================

/// (id, symbol, mint, decision, confidence, risk, minutes ago, latency, cached,
/// reasoning)
type PromoDecision = (
    i64,
    &'static str,
    &'static str,
    &'static str,
    u8,
    &'static str,
    i64,
    f64,
    bool,
    &'static str,
);

const PROMO_DECISIONS: &[PromoDecision] = &[
    (
        912,
        "MOODENG",
        "ED5nyyWEzpPPiWimP8vYm7sD7TD3LAt3Q3gRTWHzPJBY",
        "allow",
        92,
        "low",
        1,
        684.0,
        false,
        "Liquidity burned, top non-pool holder at 3.1%, priced by two venues within 0.4% of each other. Nothing in the holder graph suggests a bundled launch.",
    ),
    (
        911,
        "SAFEMOON2",
        "8DkRZ4TfPmYqLpVnW2cXbGh7sJkNmQrTyUiOpAsDfGh3",
        "reject",
        88,
        "high",
        3,
        731.0,
        false,
        "Mint authority is still live and the deployer holds 22% across four wallets funded by the same source. Liquidity lock could not be verified.",
    ),
    (
        910,
        "GIGA",
        "63LfDmNb3MQ8mw9MtZ2To9bEA2M71kZUUGq5tiJxcqj9",
        "allow",
        81,
        "medium",
        6,
        902.0,
        false,
        "Entry approved at half size: pool liquidity is 34 SOL, under the 40 SOL threshold, so the thin-book rule applies. Distribution and lock are both clean.",
    ),
    (
        909,
        "FWOG",
        "A8C3xuqscfmyLrte3VmTqrAq8kgMASius9AFNANwpump",
        "allow",
        90,
        "low",
        11,
        655.0,
        true,
        "Unchanged from the cached evaluation 22 minutes ago: liquidity burned, holder spread wide, two independent price sources.",
    ),
    (
        908,
        "RUGME",
        "5PqXwErTyUiOpAsDfGhJkLzXcVbNm2QwErTyUiOpAsDf",
        "reject",
        95,
        "critical",
        17,
        812.0,
        false,
        "Freeze authority retained and a Token-2022 transfer fee of 8% set to an authority the deployer still controls. The fee can be raised after entry.",
    ),
    (
        907,
        "TRUMP",
        "6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN",
        "allow",
        86,
        "low",
        24,
        588.0,
        true,
        "Exit review: the position is 16.4% past trailing activation with volume still rising into the move. No reason to close early.",
    ),
    (
        906,
        "PONZI",
        "3ZxCvBnMqWeRtYuIoPaSdFgHjKlZxCvBnMqWeRtYuIoP",
        "reject",
        93,
        "critical",
        33,
        767.0,
        false,
        "Single pool, 6 SOL deep, and 71% of supply in one wallet that has never sent to another address. There is no market here to enter.",
    ),
    (
        905,
        "GOAT",
        "CzLSujWBLFsSjncfkh59rUFqvafWcY5tzedWJSuypump",
        "allow",
        89,
        "low",
        41,
        623.0,
        false,
        "Liquidity locked 180 days, top ten holders at 18.6% combined, no mint or freeze authority. Clears every filtering rule.",
    ),
];

/// Generate the History tab's decision log.
///
/// The newest eight entries are the same decisions the Overview tab lists as recent,
/// because both tabs read one decision log — a History page that disagreed with the
/// Overview feed would be describing a different bot.
pub fn get_promo_decision_history(page: usize, per_page: usize) -> HistoryListResponse {
    let offset = page.saturating_sub(1).saturating_mul(per_page);

    let decisions: Vec<DecisionHistoryResponse> = PROMO_DECISIONS
        .iter()
        .skip(offset)
        .take(per_page)
        .map(|entry| {
            let (id, symbol, mint, decision, confidence, risk, mins_ago, latency, cached, why) =
                *entry;
            DecisionHistoryResponse {
                id,
                mint: mint.to_owned(),
                symbol: Some(symbol.to_owned()),
                decision: decision.to_owned(),
                confidence,
                reasoning: Some(why.to_owned()),
                risk_level: Some(risk.to_owned()),
                provider: "anthropic".to_owned(),
                model: Some("claude-sonnet-5".to_owned()),
                // A cached verdict spent no tokens — that is what the cache hit rate
                // on the Overview tab is measuring.
                tokens_used: if cached { 0 } else { 1_180 },
                latency_ms: latency,
                cached,
                created_at: stamp(mins_ago),
            }
        })
        .collect();

    HistoryListResponse {
        decisions,
        total: PROMO_DECISIONS.len(),
        page,
        per_page,
    }
}

// =============================================================================
// CHAT
// =============================================================================

/// (session id, title, minutes ago, summary)
const PROMO_SESSIONS: &[(i64, &str, i64, Option<&str>)] = &[
    (
        3,
        "Why was BONKAI rejected?",
        7,
        None,
    ),
    (
        2,
        "Trailing stop vs take profit this week",
        96,
        Some("Compared the two exit rules across 14 closed trades; trailing stop kept 3.1% more of peak on average."),
    ),
    (
        1,
        "Tighten the filtering rules for thin pools",
        1_490,
        Some("Raised the minimum liquidity to 40 SOL and added the half-size rule below it."),
    ),
];

/// The messages of the newest session — the one the chat widget selects on load.
///
/// (role, minutes ago, content)
const PROMO_MESSAGES: &[(&str, i64, &str)] = &[
    (
        "user",
        7,
        "Why was BONKAI rejected? It looked fine on the chart.",
    ),
    (
        "assistant",
        7,
        "It failed one rule, and not the chart one.\n\nBONKAI's deployer holds 11.4% of supply spread over four wallets, all funded within the same minute from a single source. Your \"Creator concentration ceiling\" rule counts wallets funded from a common source together, so that reads as 11.4% in one hand against your 8% limit.\n\nEverything else passed: liquidity is burned, the mint and freeze authorities are revoked, and two venues price it within 0.6% of each other. The chart is real — the distribution behind it is the problem, and it is the half that decides whether you can sell.",
    ),
    (
        "user",
        6,
        "Would it have passed at a 12% ceiling?",
    ),
    (
        "assistant",
        6,
        "Yes, at 12% it would have been approved — 11.4% is under it and nothing else failed.\n\nWorth knowing what that trades away: of the last 30 tokens this rule rejected, 19 were rejected on concentration alone, and 11 of those 19 lost more than 40% within six hours of the rejection. Raising the ceiling to 12% would have admitted 7 of the 19, 4 of which are in that losing group.\n\nI can show the seven with their outcomes if you want to decide on the evidence rather than the threshold.",
    ),
];

/// Generate the chat session list, newest first.
pub fn get_promo_chat_sessions() -> Vec<ChatSession> {
    PROMO_SESSIONS
        .iter()
        .map(|(id, title, mins_ago, summary)| ChatSession {
            id: *id,
            title: (*title).to_owned(),
            summary: summary.map(str::to_owned),
            // The open session's count is its real message count; the older two are
            // longer conversations that were summarised and closed.
            message_count: if *id == 3 {
                PROMO_MESSAGES.len() as i64
            } else {
                12 - id
            },
            created_at: stamp(mins_ago + 30),
            updated_at: stamp(*mins_ago),
        })
        .collect()
}

/// Generate one chat session with its transcript.
///
/// Only the newest session carries messages: it is the one the widget opens on
/// load, and the older two are summarised records whose transcripts the fixture has
/// no reason to invent.
pub fn get_promo_chat_session(id: i64) -> Option<GetChatSessionResponse> {
    let session = get_promo_chat_sessions()
        .into_iter()
        .find(|session| session.id == id)?;

    let messages = if id == 3 {
        PROMO_MESSAGES
            .iter()
            .enumerate()
            .map(|(index, (role, mins_ago, content))| ChatMessage {
                id: index as i64 + 1,
                session_id: id,
                role: (*role).to_owned(),
                content: (*content).to_owned(),
                tool_calls: None,
                created_at: stamp(*mins_ago),
            })
            .collect()
    } else {
        Vec::new()
    };

    Some(GetChatSessionResponse { session, messages })
}
