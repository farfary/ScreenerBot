// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The live-app bridge: the transport-free logic that the stdio MCP adapter
//! reaches through a narrowly-scoped internal HTTP route.
//!
//! Every entry point authenticates the pairing credential, resolves that
//! connection's own stored policy *in the running process*, honours
//! `agent_control.enabled` and the single `agent_control::decide` gate, and then either runs the canonical
//! registry tool where services and databases exist, or parks the call on the
//! durable approval queue. There is no local execution fallback anywhere in the
//! MCP adapter — this module is the only place an agent tool runs.
//!
//! `get_trade_status` is the one tool the bridge answers itself: it reads a
//! trade the calling connection submitted, so it needs the caller's identity,
//! which registry tools do not carry. It is never parked on approval and never
//! gated by a category policy; it answers only for the caller's own trades.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::agent_control::approvals;
use crate::agent_control::audit::{self, AuditContext, AuditKind};
use crate::agent_control::error::{Error, Result};
use crate::agent_control::pairing::{self, AuthedClient};
use crate::agent_control::store::{OUTCOME_NOT_STORED, TASK_ENDED};
use crate::agent_control::submissions::{self, SubmissionState};
use crate::agent_control::{
    create_tool_registry, decide, Decision, InvocationSource, PermissionLevel, Tool, ToolCategory,
    ToolDefinition, ToolResult,
};
use crate::errors::{DatabaseError, ErrorClass};
use crate::global::ActiveToolGuard;
use crate::logger::{self, LogTag};

/// The bridge-answered read of a trade the calling connection submitted.
pub const TRADE_STATUS_TOOL: &str = "get_trade_status";

/// How many times a finished request's outcome write is tried against a busy
/// store before the request is recorded as interrupted instead.
const OUTCOME_WRITE_ATTEMPTS: u32 = 5;
/// The pause between two outcome write attempts.
const OUTCOME_WRITE_BACKOFF: Duration = Duration::from_millis(500);

/// The outcome of a `call_tool` bridge request, translated verbatim by the web
/// layer into the MCP adapter's response.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CallOutcome {
    /// The tool ran in the live process.
    Executed { result: ToolResult },
    /// A transaction-sending tool was submitted and runs on its own task, or an
    /// identical call already submitted it (`reused`). Its outcome is read with
    /// `get_trade_status`; `result` is present once it finished.
    Submitted {
        trade_id: String,
        state: SubmissionState,
        reused: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<serde_json::Value>,
    },
    /// Policy denies this tool for this client.
    Denied { reason: String },
    /// A human must approve the call inside ScreenerBot before it can run.
    ApprovalRequired {
        approval_id: String,
        expires_at: i64,
    },
    /// A human denied the call.
    ApprovalDenied,
    /// The request's approval window closed without a decision.
    ApprovalExpired,
    /// An approved request started and ended without an answer. It may have
    /// taken effect; `result` says what to read before repeating it.
    Interrupted {
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
    },
    /// The referenced tool is not in the registry.
    UnknownTool,
}

fn enabled() -> bool {
    crate::config::is_config_initialized()
        && crate::config::with_config(|cfg| cfg.agent_control.enabled)
}

fn authed_context(
    client: &AuthedClient,
    tool: Option<&str>,
    correlation_id: Option<&str>,
) -> AuditContext {
    AuditContext {
        client_id: Some(client.client_id.clone()),
        tool: tool.map(str::to_owned),
        correlation_id: correlation_id.map(str::to_owned),
    }
}

/// Authenticate, honouring the master switch. A disabled surface and a bad
/// credential are separate errors, but both deny access.
fn authenticate(client_id: &str, secret: &str) -> Result<AuthedClient> {
    if !enabled() {
        return Err(Error::Disabled);
    }
    match pairing::authenticate(client_id, secret) {
        Ok(client) => {
            audit::record(
                AuditKind::BridgeAuth,
                &authed_context(&client, None, None),
                "ok",
                None,
            );
            Ok(client)
        }
        Err(e) => {
            // Never persist the caller-supplied client id on a rejected auth: a
            // client can put a pairing secret in the wrong header, so the value
            // is potentially secret-bearing. Record the rejection with a fixed,
            // identity-free context; the error returned to the caller is
            // unchanged and uniform for every rejection cause.
            audit::record(
                AuditKind::BridgeAuth,
                &AuditContext::default(),
                "rejected",
                None,
            );
            Err(e)
        }
    }
}

/// Liveness + pairing probe for `mcp doctor`. Authenticates the credential and
/// reports the running app version and this connection's own policy. Never
/// returns a secret.
#[derive(Debug, Clone, Serialize)]
pub struct PingInfo {
    pub ok: bool,
    pub version: &'static str,
    pub permissions: crate::agent_control::ToolPermissions,
    pub client_label: String,
}

pub fn ping(client_id: &str, secret: &str) -> Result<PingInfo> {
    let client = authenticate(client_id, secret)?;
    Ok(PingInfo {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        permissions: client.permissions,
        client_label: client.label,
    })
}

/// The tools this paired client may see. Includes approval-gated tools (they
/// are listed but cannot run without an in-app decision); excludes any category
/// this connection's policy denies outright. `get_trade_status` is listed
/// wherever the connection may trade; it answers for the connection's own
/// trades whatever its policy.
pub fn list_tools(client_id: &str, secret: &str) -> Result<Vec<ToolDefinition>> {
    let client = authenticate(client_id, secret)?;
    let permissions = client.permissions;

    let mut defs: Vec<ToolDefinition> = create_tool_registry()
        .list_definitions()
        .into_iter()
        .filter(|def| {
            !matches!(
                decide(def, InvocationSource::Mcp { permissions }),
                Decision::Deny
            )
        })
        .collect();
    if permissions.get_permission(&ToolCategory::Trading) != PermissionLevel::Deny {
        defs.push(trade_status_definition());
    }
    defs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(defs)
}

/// The definition the bridge lists for `get_trade_status`.
pub fn trade_status_definition() -> ToolDefinition {
    ToolDefinition {
        name: TRADE_STATUS_TOOL.to_owned(),
        description: format!(
            "Read the outcome of a trade this connection submitted. buy_token, \
             add_to_position, sell_token and close_position answer at once with a trade_id \
             and run in ScreenerBot; poll this tool with that trade_id until the state is no \
             longer submitted. States: submitted (still running), done (the result holds the \
             signature and position), failed (the result holds the error), interrupted (the \
             trade ended without an answer, for example the app stopped while it ran; its \
             swap may have been sent, so read get_positions before trading again). \
             Repeating the same trade call within {} minutes returns the same trade_id \
             instead of trading again. Each call reads the live state.",
            submissions::REUSE_WINDOW.as_secs() / 60
        ),
        category: ToolCategory::Trading,
        parameters: json!({
            "type": "object",
            "properties": {
                "trade_id": { "type": "string", "description": "The trade_id a trade call returned" }
            },
            "required": ["trade_id"]
        }),
        mutating: false,
        requires_confirmation: false,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TradeStatusParams {
    trade_id: String,
}

/// The live state of a trade `client_id` submitted. A trade id this connection
/// did not submit answers exactly like one that does not exist.
async fn trade_status(client_id: &str, arguments: Value) -> ToolResult {
    let params: TradeStatusParams = match serde_json::from_value(arguments) {
        Ok(params) => params,
        Err(error) => return ToolResult::error(format!("Invalid parameters: {error}")),
    };
    let trade_id = params.trade_id;
    let read = tokio::task::spawn_blocking({
        let trade_id = trade_id.clone();
        let client_id = client_id.to_owned();
        move || submissions::view_for_client(&trade_id, &client_id)
    })
    .await;
    match read {
        Ok(Ok(Some(handle))) => ToolResult::success(json!(handle)),
        Ok(Ok(None)) => ToolResult::error(format!(
            "No trade {trade_id} of this connection is retained. Read get_positions for its \
             outcome."
        )),
        Ok(Err(error)) => ToolResult::error(error.to_string()),
        Err(_) => ToolResult::error("The trade status read did not complete."),
    }
}

/// Execute a tool, deny it, or park it on the approval queue.
pub async fn call_tool(
    client_id: &str,
    secret: &str,
    name: &str,
    arguments: Value,
    correlation_id: &str,
) -> Result<CallOutcome> {
    let client = authenticate(client_id, secret)?;
    let permissions = client.permissions;
    let ctx = authed_context(&client, Some(name), Some(correlation_id));

    if name == TRADE_STATUS_TOOL {
        audit::record(AuditKind::ToolRequest, &ctx, "received", None);
        let result = trade_status(&client.client_id, arguments).await;
        audit::record(
            AuditKind::Execution,
            &ctx,
            if result.success { "done" } else { "failed" },
            None,
        );
        return Ok(CallOutcome::Executed { result });
    }

    let Some(tool) = create_tool_registry().get(name) else {
        audit::record(AuditKind::ToolRequest, &ctx, "unknown_tool", None);
        return Ok(CallOutcome::UnknownTool);
    };
    let definition = tool.definition();
    audit::record(AuditKind::ToolRequest, &ctx, "received", None);

    match decide(&definition, InvocationSource::Mcp { permissions }) {
        Decision::Deny => {
            audit::record(AuditKind::AuthzDecision, &ctx, "deny", None);
            Ok(CallOutcome::Denied {
                reason: "This paired client is not authorized for this tool.".to_owned(),
            })
        }
        Decision::Execute => {
            audit::record(AuditKind::AuthzDecision, &ctx, "execute", None);
            let Some(active_tool) = crate::global::begin_tool() else {
                let reason = "An application update is restarting the tool runtime.";
                audit::record(AuditKind::Execution, &ctx, "failed", Some(reason));
                return Ok(CallOutcome::Executed {
                    result: ToolResult::error(reason),
                });
            };
            if tool.sends_transaction() {
                return submit(
                    &client.client_id,
                    tool,
                    arguments,
                    correlation_id,
                    ctx,
                    active_tool,
                )
                .await;
            }
            let result = tool.execute(arguments).await;
            audit::record(
                AuditKind::Execution,
                &ctx,
                if result.success { "done" } else { "failed" },
                None,
            );
            Ok(CallOutcome::Executed { result })
        }
        Decision::RequireApproval => {
            audit::record(AuditKind::AuthzDecision, &ctx, "require_approval", None);
            let handle =
                approvals::create_or_reuse(&client.client_id, name, &arguments, correlation_id)?;
            if handle.created {
                audit::record(AuditKind::ApprovalCreated, &ctx, "pending", None);
            }
            match handle.state.as_str() {
                "done" | "failed" => Ok(CallOutcome::Executed {
                    result: handle
                        .result
                        .and_then(|v| serde_json::from_value(v).ok())
                        .unwrap_or_else(|| {
                            ToolResult::error("approved request completed; result unavailable")
                        }),
                }),
                "interrupted" => Ok(CallOutcome::Interrupted {
                    result: handle.result,
                }),
                "denied" => Ok(CallOutcome::ApprovalDenied),
                "expired" => Ok(CallOutcome::ApprovalExpired),
                // pending / claimed / executing
                _ => Ok(CallOutcome::ApprovalRequired {
                    approval_id: handle.id,
                    expires_at: handle.expires_at,
                }),
            }
        }
    }
}

/// Submit a transaction-sending tool and answer at once with its trade id.
///
/// A new submission runs on its own task, holding the active-tool guard, and
/// stores its result for `get_trade_status`; nothing awaits it, so a dropped or
/// timed-out agent request can neither cancel nor repeat it. An identical call
/// inside the reuse window gets the existing trade id and starts nothing.
async fn submit(
    client_id: &str,
    tool: Arc<dyn Tool>,
    arguments: Value,
    correlation_id: &str,
    ctx: AuditContext,
    active_tool: ActiveToolGuard,
) -> Result<CallOutcome> {
    let name = tool.definition().name;
    let handle = tokio::task::spawn_blocking({
        let (client_id, arguments, correlation_id) = (
            client_id.to_owned(),
            arguments.clone(),
            correlation_id.to_owned(),
        );
        move || submissions::submit_or_reuse(&client_id, &name, &arguments, &correlation_id)
    })
    .await
    .map_err(|error| {
        Error::Database(DatabaseError::Query {
            operation: "submit_trade".to_owned(),
            message: error.to_string(),
        })
    })??;
    if !handle.created {
        audit::record(AuditKind::Execution, &ctx, "reused", Some(&handle.trade_id));
        return Ok(CallOutcome::Submitted {
            trade_id: handle.trade_id,
            state: handle.state,
            reused: true,
            result: handle.result,
        });
    }

    audit::record(
        AuditKind::Execution,
        &ctx,
        "submitted",
        Some(&handle.trade_id),
    );
    let trade_id = handle.trade_id.clone();
    tokio::spawn(async move {
        let _active_tool = active_tool;
        run_submitted(trade_id, tool, arguments, ctx).await;
    });

    Ok(CallOutcome::Submitted {
        trade_id: handle.trade_id,
        state: handle.state,
        reused: false,
        result: None,
    })
}

/// Run a submitted trade and store its outcome. The tool runs on a task of its
/// own, so a panic in it ends that task, not this one: the submission is then
/// recorded `interrupted` instead of staying `submitted` for the life of the
/// process. A result that cannot be stored is recorded the same way.
pub(crate) async fn run_submitted(
    trade_id: String,
    tool: Arc<dyn Tool>,
    arguments: Value,
    ctx: AuditContext,
) {
    let outcome = match tokio::spawn(async move { tool.execute(arguments).await }).await {
        Ok(result) => {
            let ok = result.success;
            let value = serde_json::to_value(&result).unwrap_or(Value::Null);
            let id = trade_id.clone();
            match write_outcome(move || submissions::finish(&id, ok, &value)).await {
                Ok(true) => {}
                Ok(false) => logger::warning(
                    LogTag::Security,
                    &format!(
                        "agent-control: trade {trade_id} finished after it was marked interrupted"
                    ),
                ),
                Err(error) => {
                    logger::error(
                        LogTag::Security,
                        &format!(
                            "agent-control: trade {trade_id} finished but its outcome was not stored: {error}"
                        ),
                    );
                    interrupt_submission(&trade_id, OUTCOME_NOT_STORED).await;
                }
            }
            if ok {
                "done"
            } else {
                "failed"
            }
        }
        Err(error) => {
            logger::error(
                LogTag::Security,
                &format!("agent-control: trade {trade_id} task ended without an answer: {error}"),
            );
            interrupt_submission(&trade_id, TASK_ENDED).await;
            "interrupted"
        }
    };
    audit::record(AuditKind::Execution, &ctx, outcome, Some(&trade_id));
}

/// Record a submission that ended without a stored answer as interrupted.
async fn interrupt_submission(trade_id: &str, cause: &'static str) {
    let id = trade_id.to_owned();
    if let Err(error) = write_outcome(move || submissions::interrupt(&id, cause)).await {
        logger::error(
            LogTag::Security,
            &format!(
                "agent-control: trade {trade_id} could not be marked interrupted and reads \
                 submitted until the next start: {error}"
            ),
        );
    }
}

/// Run a synchronous outcome write off the async runtime, retrying a busy store
/// up to `OUTCOME_WRITE_ATTEMPTS` times.
async fn write_outcome<T: Send + 'static>(
    write: impl Fn() -> Result<T> + Clone + Send + 'static,
) -> Result<T> {
    let mut attempt = 1;
    loop {
        let result = tokio::task::spawn_blocking(write.clone())
            .await
            .unwrap_or_else(|error| {
                Err(Error::Database(DatabaseError::Query {
                    operation: "store_outcome".to_owned(),
                    message: error.to_string(),
                }))
            });
        match result {
            Err(error) if error.is_retryable() && attempt < OUTCOME_WRITE_ATTEMPTS => {
                attempt += 1;
                tokio::time::sleep(OUTCOME_WRITE_BACKOFF).await;
            }
            other => return other,
        }
    }
}

/// Poll one approval. Scoped to the owning client.
pub fn approval_status(
    client_id: &str,
    secret: &str,
    approval_id: &str,
) -> Result<approvals::ApprovalHandle> {
    let client = authenticate(client_id, secret)?;
    approvals::view_for_client(approval_id, &client.client_id)
}

/// A claimed approval that passed its re-checks and is `executing`, ready to
/// run. Built by `start_approved`; consumed by `run`.
pub struct ApprovedRun {
    approval_id: String,
    tool: Arc<dyn Tool>,
    arguments: Value,
    ctx: AuditContext,
    _active_tool: ActiveToolGuard,
}

/// Claim a human-approved request exactly once and prepare it to run. Invoked
/// only by the dashboard `decide` route — never reachable from the bridge.
/// Claims the row (exactly-once) and re-checks policy against the pairing's
/// *current* permissions. Returns `None` when a re-check failed the request
/// closed; the stored result says why.
pub fn start_approved(approval_id: &str) -> Result<Option<ApprovedRun>> {
    let claimed = approvals::claim(approval_id)?;
    let ctx = AuditContext {
        client_id: Some(claimed.client_id.clone()),
        tool: Some(claimed.tool.clone()),
        correlation_id: Some(claimed.correlation_id.clone()),
    };
    audit::record(AuditKind::ApprovalDecided, &ctx, "approved", None);

    let fail = |detail: &str| -> Result<Option<ApprovedRun>> {
        let _ = approvals::mark_executing(approval_id);
        let _ = approvals::finish(
            approval_id,
            false,
            &serde_json::json!({ "success": false, "error": detail }),
        );
        audit::record(AuditKind::Execution, &ctx, "failed", Some(detail));
        Ok(None)
    };

    if !enabled() {
        return fail("agent control was disabled before this request could run");
    }
    let Some(permissions) = pairing::active_permissions(&claimed.client_id)? else {
        return fail("the paired client was revoked before this request could run");
    };
    let Some(tool) = create_tool_registry().get(&claimed.tool) else {
        return fail("the requested tool is no longer available");
    };
    if matches!(
        decide(&tool.definition(), InvocationSource::Mcp { permissions }),
        Decision::Deny
    ) {
        return fail("policy denies this tool for the paired client");
    }

    approvals::mark_executing(approval_id)?;
    let Some(active_tool) = crate::global::begin_tool() else {
        return fail("an application update is restarting the tool runtime");
    };
    Ok(Some(ApprovedRun {
        approval_id: approval_id.to_owned(),
        tool,
        arguments: claimed.canonical_args,
        ctx,
        _active_tool: active_tool,
    }))
}

impl ApprovedRun {
    /// Execute the stored canonical arguments and record the sanitized result
    /// for the MCP client to poll through `approval_status`. The tool runs on a
    /// task of its own: when it ends without an answer, or its result cannot be
    /// stored, the request is recorded `interrupted` rather than left
    /// `executing`.
    pub async fn run(self) -> Result<()> {
        let ApprovedRun {
            approval_id,
            tool,
            arguments,
            ctx,
            _active_tool,
        } = self;
        let sends_transaction = tool.sends_transaction();
        match tokio::spawn(async move { tool.execute(arguments).await }).await {
            Ok(result) => {
                let ok = result.success;
                let value = serde_json::to_value(&result).unwrap_or(Value::Null);
                let id = approval_id.clone();
                let stored = write_outcome(move || approvals::finish(&id, ok, &value)).await;
                audit::record(
                    AuditKind::Execution,
                    &ctx,
                    if ok { "done" } else { "failed" },
                    None,
                );
                if let Err(error) = stored {
                    if !matches!(error, Error::ApprovalNotPending) {
                        interrupt_approval(&approval_id, OUTCOME_NOT_STORED, sends_transaction)
                            .await?;
                    }
                    return Err(error);
                }
                Ok(())
            }
            Err(error) => {
                logger::error(
                    LogTag::Security,
                    &format!(
                        "agent-control: approved request {approval_id} ended without an answer: {error}"
                    ),
                );
                audit::record(AuditKind::Execution, &ctx, "interrupted", None);
                interrupt_approval(&approval_id, TASK_ENDED, sends_transaction).await
            }
        }
    }
}

/// Record an executing approval that ended without a stored answer as
/// interrupted.
async fn interrupt_approval(
    approval_id: &str,
    cause: &'static str,
    sends_transaction: bool,
) -> Result<()> {
    let id = approval_id.to_owned();
    write_outcome(move || approvals::interrupt(&id, cause, sends_transaction)).await
}

/// Deny a pending approval (dashboard `decide` route, `approve = false`).
pub fn deny_approval(approval_id: &str) -> Result<()> {
    let context = approvals::deny(approval_id)?;
    audit::record(
        AuditKind::ApprovalDecided,
        &AuditContext {
            client_id: Some(context.client_id),
            tool: Some(context.tool),
            correlation_id: Some(context.correlation_id),
        },
        "denied",
        None,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::json;

    use super::*;
    use crate::agent_control::store::{interrupted_result, test_support::setup};

    struct PanickingTrade;

    #[async_trait]
    impl Tool for PanickingTrade {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "buy_token".to_owned(),
                description: String::new(),
                category: ToolCategory::Trading,
                parameters: json!({}),
                mutating: true,
                requires_confirmation: true,
            }
        }

        async fn execute(&self, _params: Value) -> ToolResult {
            panic!("trade task failure");
        }

        fn sends_transaction(&self) -> bool {
            true
        }
    }

    /// A trade whose task panics after it may have sent its swap ends
    /// `interrupted` with the shared answer, never `submitted`, on both the
    /// submission path and the approval path.
    #[tokio::test]
    async fn a_trade_task_that_panics_is_recorded_interrupted() {
        let _guard = setup();
        let client = "panicking-trade-client";
        let args = json!({ "mint_address": "P" });
        let expected = interrupted_result(TASK_ENDED, true);

        let submission = submissions::submit_or_reuse(client, "buy_token", &args, "c1").unwrap();
        run_submitted(
            submission.trade_id.clone(),
            Arc::new(PanickingTrade),
            args.clone(),
            AuditContext::default(),
        )
        .await;
        let viewed = submissions::view_for_client(&submission.trade_id, client)
            .unwrap()
            .expect("retained");
        assert_eq!(viewed.state, SubmissionState::Interrupted);
        assert_eq!(viewed.result, Some(expected.clone()));

        let approval = approvals::create_or_reuse(client, "buy_token", &args, "c1").unwrap();
        approvals::claim(&approval.id).unwrap();
        approvals::mark_executing(&approval.id).unwrap();
        ApprovedRun {
            approval_id: approval.id.clone(),
            tool: Arc::new(PanickingTrade),
            arguments: args,
            ctx: AuditContext::default(),
            _active_tool: crate::global::begin_tool().expect("tool runtime open"),
        }
        .run()
        .await
        .expect("the interruption is stored");
        let viewed = approvals::view_for_client(&approval.id, client).unwrap();
        assert_eq!(viewed.state, "interrupted");
        assert_eq!(viewed.result, Some(expected));
    }
}
