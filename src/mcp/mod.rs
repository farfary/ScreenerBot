//! MCP protocol adapter. It deliberately contains no trading/domain logic:
//! every authorization decision is delegated to `crate::agent_control`, and the
//! product capabilities themselves live behind the tool registry.
//!
//! Security posture for this checkpoint:
//! - The transport is stdio only. Streamable HTTP is NOT mounted, because no
//!   authenticated pairing mechanism exists yet.
//! - A connection with no resolved paired-client scope receives NO capabilities
//!   and cannot execute any tool (fail closed).
//! - Mutating capabilities are not even listed, because there is no in-app
//!   approval record/route to gate them yet.

use std::{borrow::Cow, sync::Arc};

use rmcp::{
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
    },
    service::{RequestContext, RoleServer},
    ServerHandler, ServiceExt,
};

use crate::agent_control::{
    self, create_tool_registry, ClientScope, Decision, InvocationSource, ToolDefinition,
};

#[derive(Clone)]
pub struct McpServer {
    /// The capability scope resolved for the connected paired client. `None`
    /// means unknown / unpaired / missing — the client gets nothing.
    scope: Option<ClientScope>,
}

impl McpServer {
    pub fn new(scope: Option<ClientScope>) -> Self {
        Self { scope }
    }

    /// The tools this connection may both see and execute. An unpaired client
    /// (`scope == None`) sees nothing. A paired client sees only tools whose
    /// required scope it holds AND that resolve to `Decision::Execute` right
    /// now — approval-gated mutations are withheld entirely because no approval
    /// route exists in this checkpoint.
    fn definitions(&self) -> Vec<ToolDefinition> {
        let Some(scope) = self.scope else {
            return Vec::new();
        };
        create_tool_registry()
            .list_definitions()
            .into_iter()
            .filter(|definition| {
                agent_control::required_scope(definition) <= scope
                    && matches!(
                        agent_control::decide(definition, InvocationSource::Mcp { scope }),
                        Decision::Execute
                    )
            })
            .collect()
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info =
            Implementation::new("screenerbot", env!("CARGO_PKG_VERSION")).with_title("ScreenerBot");
        info.instructions = Some(
            "Local ScreenerBot control surface. Every tool is gated by the paired client's scope \
             and by in-app approval; unpaired clients receive no capabilities."
                .to_owned(),
        );
        info
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::model::ErrorData> {
        Ok(ListToolsResult::with_all_items(
            self.definitions().into_iter().map(to_mcp_tool).collect(),
        ))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, rmcp::model::ErrorData> {
        let Some(scope) = self.scope else {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "This client is not paired with ScreenerBot and has no capabilities.",
            )])
            .into());
        };

        let Some(tool) = create_tool_registry().get(request.name.as_ref()) else {
            return Ok(
                CallToolResult::error(vec![ContentBlock::text("Unknown ScreenerBot tool")]).into(),
            );
        };
        let definition = tool.definition();

        match agent_control::decide(&definition, InvocationSource::Mcp { scope }) {
            Decision::Deny => Ok(CallToolResult::error(vec![ContentBlock::text(
                "This paired client is not authorized for this tool.",
            )])
            .into()),
            Decision::RequireApproval => Ok(CallToolResult::error(vec![ContentBlock::text(
                "This action requires explicit approval inside ScreenerBot. External agents \
                 cannot approve their own actions, and no approval route exists in this \
                 checkpoint, so the action is refused.",
            )])
            .into()),
            Decision::Execute => {
                let args = request
                    .arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or_else(|| serde_json::json!({}));
                let result = tool.execute(args).await;
                if result.success {
                    Ok(CallToolResult::structured(
                        serde_json::to_value(&result).unwrap_or_default(),
                    )
                    .into())
                } else {
                    let text = serde_json::to_string(&result)
                        .unwrap_or_else(|_| "{\"success\":false}".to_owned());
                    Ok(CallToolResult::error(vec![ContentBlock::text(text)]).into())
                }
            }
        }
    }
}

fn to_mcp_tool(definition: ToolDefinition) -> Tool {
    let input_schema = definition
        .parameters
        .as_object()
        .cloned()
        .unwrap_or_default();
    let read_only = agent_control::is_read_only(&definition);
    Tool::new_with_raw(
        Cow::Owned(definition.name),
        Some(Cow::Owned(definition.description)),
        Arc::new(input_schema),
    )
    .annotate(ToolAnnotations::from_raw(
        None,
        Some(read_only),
        Some(!read_only),
        Some(read_only),
        Some(false),
    ))
}

/// Resolve the capability scope for a paired-client id.
///
/// There is no pairing store yet, so this always returns `None`: no client is
/// known, therefore no client is trusted. When the Agent Connections UI lands it
/// will look `client_id` up in the paired-client registry and return the stored
/// scope only for an authenticated, non-revoked pairing.
fn resolve_client_scope(client_id: Option<&str>) -> Option<ClientScope> {
    // Honor the master availability switch. When config is loaded and
    // `agent_control.enabled` is false, the surface is off regardless of
    // pairing — no client is trusted. (With no pairing store this is already
    // the effective state; the check makes the flag authoritative.)
    if crate::config::is_config_initialized()
        && !crate::config::with_config(|cfg| cfg.agent_control.enabled)
    {
        eprintln!("ScreenerBot MCP: agent_control.enabled is false; serving zero capabilities.");
        return None;
    }

    match client_id {
        Some(id) => {
            eprintln!(
                "ScreenerBot MCP: client id {id:?} is not paired (no pairing store in this \
                 build); serving zero capabilities."
            );
            None
        }
        None => {
            eprintln!(
                "ScreenerBot MCP: no --client-id supplied; serving zero capabilities until \
                 pairing exists."
            );
            None
        }
    }
}

/// Run an MCP server over stdio. Never initializes logging to stdout or prints
/// to stdout — stdout is exclusively JSON-RPC framing.
pub async fn serve_stdio(client_id: Option<&str>) -> anyhow::Result<()> {
    let scope = resolve_client_scope(client_id);
    let (stdin, stdout) = rmcp::transport::stdio();
    McpServer::new(scope)
        .serve((stdin, stdout))
        .await?
        .waiting()
        .await?;
    Ok(())
}

pub fn is_mcp_command(args: &[String]) -> bool {
    args.get(1).is_some_and(|arg| arg == "mcp")
}

/// Dispatch a `screenerbot mcp <...>` invocation. Runs before the normal boot
/// path. Loads the minimum configuration state quietly (all diagnostics on
/// stderr) so saved tool policy is honored, then hands off to the transport.
pub async fn dispatch(args: &[String]) -> anyhow::Result<bool> {
    if !is_mcp_command(args) {
        return Ok(false);
    }

    // Keep protocol stdout clean: every log line from here on goes to stderr.
    crate::logger::route_console_to_stderr();

    // Load saved configuration so tool permission policy is real, not defaults.
    // A failure here is non-fatal: the policy layer falls back to safe defaults
    // and the connection still fails closed without a paired client.
    if let Err(error) = crate::config::load_config() {
        eprintln!("ScreenerBot MCP: continuing with default policy ({error})");
    }

    match args.get(2).map(String::as_str) {
        Some("serve") => {
            serve_stdio(
                args.windows(2)
                    .find(|pair| pair[0] == "--client-id")
                    .map(|pair| pair[1].as_str()),
            )
            .await?;
            Ok(true)
        }
        Some("doctor") => {
            eprintln!(
                "ScreenerBot MCP: stdio transport available. No pairing store exists in this \
                 build, so every connection is served zero capabilities and no tool can execute. \
                 Streamable HTTP is intentionally not mounted."
            );
            Ok(true)
        }
        _ => {
            eprintln!("Usage: screenerbot mcp <serve [--client-id UUID] | doctor>");
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpaired_client_gets_no_tools() {
        let tools = McpServer::new(None).definitions();
        assert!(tools.is_empty());
    }

    #[test]
    fn resolve_scope_is_fail_closed_without_pairing_store() {
        assert_eq!(resolve_client_scope(None), None);
        assert_eq!(
            resolve_client_scope(Some("00000000-0000-0000-0000-000000000000")),
            None
        );
    }

    #[test]
    fn read_scope_lists_are_deterministic_and_exclude_mutations() {
        let tools = McpServer::new(Some(ClientScope::Read)).definitions();
        let names: Vec<_> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert!(names.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(!names.contains(&"buy_token"));
        assert!(!names.contains(&"sell_token"));
        assert!(!names.contains(&"close_position"));
    }

    #[test]
    fn trade_scope_still_withholds_unapproved_mutations() {
        // A Trade-scoped client may reach trade tools by scope, but with no
        // approval route they resolve to RequireApproval and are not listed.
        let tools = McpServer::new(Some(ClientScope::Trade)).definitions();
        let names: Vec<_> = tools.iter().map(|tool| tool.name.as_str()).collect();
        assert!(!names.contains(&"buy_token"));
    }
}
