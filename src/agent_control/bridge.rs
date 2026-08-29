//! The live-app bridge: the transport-free logic that the stdio MCP adapter
//! reaches through a narrowly-scoped internal HTTP route.
//!
//! Every entry point authenticates the pairing credential, resolves the stored
//! scope *in the running process*, honours `agent_control.enabled` and the
//! single `agent_control::decide` gate, and then either runs the canonical
//! registry tool where services and databases exist, or parks the call on the
//! durable approval queue. There is no local execution fallback anywhere in the
//! MCP adapter — this module is the only place an agent tool runs.

use serde::Serialize;
use serde_json::Value;

use crate::agent_control::approvals;
use crate::agent_control::audit::{self, AuditContext, AuditKind};
use crate::agent_control::error::{Error, Result};
use crate::agent_control::pairing::{self, AuthedClient};
use crate::agent_control::{
    create_tool_registry, decide, required_scope, Decision, InvocationSource, ToolDefinition,
    ToolResult,
};

/// The outcome of a `call_tool` bridge request, translated verbatim by the web
/// layer into the MCP adapter's response.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CallOutcome {
    /// The tool ran in the live process.
    Executed { result: ToolResult },
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
/// reports the running app version and the client's scope. Never returns a
/// secret.
#[derive(Debug, Clone, Serialize)]
pub struct PingInfo {
    pub ok: bool,
    pub version: &'static str,
    pub scope: &'static str,
    pub client_label: String,
}

pub fn ping(client_id: &str, secret: &str) -> Result<PingInfo> {
    let client = authenticate(client_id, secret)?;
    Ok(PingInfo {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        scope: client.scope.as_str(),
        client_label: client.label,
    })
}

/// The tools this paired client may see. Includes approval-gated tools (they
/// are listed but cannot run without an in-app decision); excludes anything the
/// client's scope does not cover or that policy denies outright.
pub fn list_tools(client_id: &str, secret: &str) -> Result<Vec<ToolDefinition>> {
    let client = authenticate(client_id, secret)?;
    let scope = client.scope;

    let mut defs: Vec<ToolDefinition> = create_tool_registry()
        .list_definitions()
        .into_iter()
        .filter(|def| {
            required_scope(def) <= scope
                && !matches!(decide(def, InvocationSource::Mcp { scope }), Decision::Deny)
        })
        .collect();
    defs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(defs)
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
    let scope = client.scope;
    let ctx = authed_context(&client, Some(name), Some(correlation_id));

    let Some(tool) = create_tool_registry().get(name) else {
        audit::record(AuditKind::ToolRequest, &ctx, "unknown_tool", None);
        return Ok(CallOutcome::UnknownTool);
    };
    let definition = tool.definition();
    audit::record(AuditKind::ToolRequest, &ctx, "received", None);

    match decide(&definition, InvocationSource::Mcp { scope }) {
        Decision::Deny => {
            audit::record(AuditKind::AuthzDecision, &ctx, "deny", None);
            Ok(CallOutcome::Denied {
                reason: "This paired client is not authorized for this tool.".to_owned(),
            })
        }
        Decision::Execute => {
            audit::record(AuditKind::AuthzDecision, &ctx, "execute", None);
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

/// Poll one approval. Scoped to the owning client.
pub fn approval_status(
    client_id: &str,
    secret: &str,
    approval_id: &str,
) -> Result<approvals::ApprovalHandle> {
    let client = authenticate(client_id, secret)?;
    approvals::view_for_client(approval_id, &client.client_id)
}

/// Run a human-approved request exactly once, in the live process. Invoked only
/// by the dashboard `decide` route — never reachable from the bridge. Claims
/// the row (exactly-once), re-checks policy against the pairing's *current*
/// scope, executes the stored canonical arguments, and records the sanitized
/// result for the MCP client to poll.
pub async fn execute_approved(approval_id: &str) -> Result<()> {
    let claimed = approvals::claim(approval_id)?;
    let ctx = AuditContext {
        client_id: Some(claimed.client_id.clone()),
        tool: Some(claimed.tool.clone()),
        correlation_id: Some(claimed.correlation_id.clone()),
    };
    audit::record(AuditKind::ApprovalDecided, &ctx, "approved", None);

    let fail = |detail: &str| -> Result<()> {
        let _ = approvals::mark_executing(approval_id);
        let _ = approvals::finish(
            approval_id,
            false,
            &serde_json::json!({ "success": false, "error": detail }),
        );
        audit::record(AuditKind::Execution, &ctx, "failed", Some(detail));
        Ok(())
    };

    if !enabled() {
        return fail("agent control was disabled before this request could run");
    }
    let Some(scope) = pairing::active_scope(&claimed.client_id)? else {
        return fail("the paired client was revoked before this request could run");
    };
    let Some(tool) = create_tool_registry().get(&claimed.tool) else {
        return fail("the requested tool is no longer available");
    };
    if matches!(
        decide(&tool.definition(), InvocationSource::Mcp { scope }),
        Decision::Deny
    ) {
        return fail("policy denies this tool for the paired client");
    }

    approvals::mark_executing(approval_id)?;
    let result = tool.execute(claimed.canonical_args.clone()).await;
    let value = serde_json::to_value(&result).unwrap_or(Value::Null);
    approvals::finish(approval_id, result.success, &value)?;
    audit::record(
        AuditKind::Execution,
        &ctx,
        if result.success { "done" } else { "failed" },
        None,
    );
    Ok(())
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
