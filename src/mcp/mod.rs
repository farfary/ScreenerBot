//! MCP protocol adapter (stdio transport only).
//!
//! This process is a thin bridge client. It contains NO trading/domain logic
//! and NO local tool registry or policy: it discovers the already-running
//! ScreenerBot from the token-free `agent-runtime.json`, then calls a narrowly
//! scoped internal HTTP bridge (`/api/agent-bridge/*`) on that live process.
//! The live app authenticates the pairing credential, resolves the stored
//! scope, applies `agent_control.enabled` / `required_scope` / the single
//! `agent_control::decide` gate, and executes the canonical registry tool where
//! services and databases exist.
//!
//! Security posture:
//! - Transport is stdio only. Streamable HTTP MCP is never mounted anywhere.
//! - No pairing credential, or no running app, means zero capabilities and no
//!   tool can run. There is no local execution fallback.
//! - Approval-gated tools are listed once an approval route exists, but still
//!   cannot run without an explicit in-app decision.
//! - The pairing secret is read from `SCREENERBOT_PAIRING_SECRET` (never a CLI
//!   argument) and never written to stdout or any diagnostic.
//! - stdout carries JSON-RPC framing exclusively; all diagnostics go to stderr.

use std::time::{Duration, Instant};

use rmcp::{
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
    },
    service::{RequestContext, RoleServer},
    ServerHandler, ServiceExt,
};
use std::{borrow::Cow, sync::Arc};

use crate::agent_control::{is_read_only, ToolDefinition};

const BRIDGE_LIST: &str = "/api/agent-bridge/list-tools";
const BRIDGE_CALL: &str = "/api/agent-bridge/call-tool";
const BRIDGE_STATUS: &str = "/api/agent-bridge/approval-status";
const BRIDGE_PING: &str = "/api/agent-bridge/ping";

const APPROVAL_POLL_INTERVAL: Duration = Duration::from_secs(2);
const APPROVAL_WAIT_LIMIT: Duration = Duration::from_secs(5 * 60);

/// Bounds on every bridge HTTP call so approval polling cannot hang past its
/// contract and a wedged socket cannot stall the adapter.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

const CLIENT_ID_ENV: &str = "SCREENERBOT_CLIENT_ID";
const SECRET_ENV: &str = "SCREENERBOT_PAIRING_SECRET";

/// The only hosts a runtime file may point the bridge at.
const LOOPBACK_HOSTS: [&str; 2] = ["127.0.0.1", "localhost"];

/// What `agent-runtime.json` tells us about the running app. Carries no secret.
#[derive(Clone, Debug)]
struct RuntimeInfo {
    url: String,
    pid: Option<u64>,
    version: Option<String>,
}

#[derive(Clone)]
pub struct McpServer {
    http: reqwest::Client,
    runtime: Option<RuntimeInfo>,
    client_id: Option<String>,
    secret: Option<String>,
}

impl McpServer {
    /// The base URL + credential, only when all three are present.
    fn ready(&self) -> Option<(&str, &str, &str)> {
        Some((
            self.runtime.as_ref()?.url.as_str(),
            self.client_id.as_deref()?,
            self.secret.as_deref()?,
        ))
    }

    /// POST a JSON body to a bridge path. Returns the parsed body on 2xx, or a
    /// short non-secret error string otherwise (unreachable app, rejected
    /// credential, disabled surface, …).
    async fn bridge_post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (url, client_id, secret) = self
            .ready()
            .ok_or_else(|| "ScreenerBot is not running or this client is not paired".to_owned())?;

        let response = self
            .http
            .post(format!("{url}{path}"))
            .header("x-screenerbot-client", client_id)
            .header("x-screenerbot-pairing-secret", secret)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("bridge unreachable: {e}"))?;

        let status = response.status();
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("unreadable bridge response: {e}"))?;

        if status.is_success() {
            return Ok(value);
        }
        let message = value
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("request rejected")
            .to_owned();
        Err(message)
    }

    async fn poll_approval(&self, approval_id: &str) -> CallToolResponse {
        let deadline = Instant::now() + APPROVAL_WAIT_LIMIT;
        loop {
            // Stop a full cycle before the deadline so a bounded-but-slow
            // request can never push the total wait past the 5-minute contract.
            if Instant::now() + APPROVAL_POLL_INTERVAL + REQUEST_TIMEOUT >= deadline {
                return error_result(&format!(
                    "This action requires approval inside ScreenerBot and is still pending \
                     (request {approval_id}). Approve it in ScreenerBot, then retry the same \
                     call — it will not run twice."
                ));
            }
            tokio::time::sleep(APPROVAL_POLL_INTERVAL).await;

            let value = match self
                .bridge_post(
                    BRIDGE_STATUS,
                    serde_json::json!({ "approval_id": approval_id }),
                )
                .await
            {
                Ok(v) => v,
                Err(e) => return error_result(&e),
            };
            match value.get("state").and_then(|s| s.as_str()).unwrap_or("") {
                "pending" | "claimed" | "executing" => continue,
                "done" => return result_to_call(value.get("result")),
                "failed" => {
                    return error_result_from(value.get("result"), "The approved request failed.")
                }
                "denied" => return error_result("A person denied this request in ScreenerBot."),
                "expired" => {
                    return error_result(
                        "The approval request expired in ScreenerBot without a decision.",
                    )
                }
                other => return error_result(&format!("Unexpected approval state: {other}")),
            }
        }
    }
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info =
            Implementation::new("screenerbot", env!("CARGO_PKG_VERSION")).with_title("ScreenerBot");
        info.instructions = Some(
            "Local ScreenerBot control surface. Capabilities come from the running ScreenerBot \
             process and are gated by the paired client's scope and by explicit in-app approval. \
             If ScreenerBot is not running or this client is not paired, no capabilities are \
             offered."
                .to_owned(),
        );
        info
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::model::ErrorData> {
        match self.bridge_post(BRIDGE_LIST, serde_json::json!({})).await {
            Ok(value) => {
                let defs: Vec<ToolDefinition> = value
                    .get("tools")
                    .cloned()
                    .and_then(|t| serde_json::from_value(t).ok())
                    .unwrap_or_default();
                Ok(ListToolsResult::with_all_items(
                    defs.into_iter().map(to_mcp_tool).collect(),
                ))
            }
            Err(reason) => {
                eprintln!("ScreenerBot MCP: no capabilities available ({reason}).");
                Ok(ListToolsResult::with_all_items(Vec::new()))
            }
        }
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, rmcp::model::ErrorData> {
        let arguments = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or_else(|| serde_json::json!({}));
        let correlation_id = uuid::Uuid::new_v4().to_string();

        let value = match self
            .bridge_post(
                BRIDGE_CALL,
                serde_json::json!({
                    "name": request.name,
                    "arguments": arguments,
                    "correlation_id": correlation_id,
                }),
            )
            .await
        {
            Ok(v) => v,
            Err(reason) => return Ok(error_result(&reason)),
        };

        match value.get("status").and_then(|s| s.as_str()).unwrap_or("") {
            "executed" => Ok(result_to_call(value.get("result"))),
            "denied" => Ok(error_result(
                value
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("This paired client is not authorized for this tool."),
            )),
            "unknown_tool" => Ok(error_result("Unknown ScreenerBot tool")),
            "approval_denied" => Ok(error_result("A person denied this request in ScreenerBot.")),
            "approval_expired" => Ok(error_result(
                "The approval request expired in ScreenerBot without a decision.",
            )),
            "approval_required" => {
                let approval_id = value
                    .get("approval_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned();
                Ok(self.poll_approval(&approval_id).await)
            }
            other => Ok(error_result(&format!(
                "Unexpected bridge response: {other}"
            ))),
        }
    }
}

fn to_mcp_tool(definition: ToolDefinition) -> Tool {
    let input_schema = definition
        .parameters
        .as_object()
        .cloned()
        .unwrap_or_default();
    let read_only = is_read_only(&definition);
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

/// Turn a `{success, data?, error?}` tool-result value into an MCP response.
fn result_to_call(result: Option<&serde_json::Value>) -> CallToolResponse {
    let Some(value) = result else {
        return error_result("The request completed but returned no result.");
    };
    let success = value
        .get("success")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);
    if success {
        CallToolResult::structured(value.clone()).into()
    } else {
        let text =
            serde_json::to_string(value).unwrap_or_else(|_| "{\"success\":false}".to_owned());
        CallToolResult::error(vec![ContentBlock::text(text)]).into()
    }
}

fn error_result(message: &str) -> CallToolResponse {
    CallToolResult::error(vec![ContentBlock::text(message)]).into()
}

fn error_result_from(result: Option<&serde_json::Value>, fallback: &str) -> CallToolResponse {
    match result {
        Some(value) => {
            let text = serde_json::to_string(value).unwrap_or_else(|_| fallback.to_owned());
            CallToolResult::error(vec![ContentBlock::text(text)]).into()
        }
        None => error_result(fallback),
    }
}

/// Accept a runtime `url` ONLY when it is an exact http loopback origin: `http`
/// scheme, `127.0.0.1`/`localhost` host, an explicit non-zero port, and no
/// userinfo, path, query or fragment. Returns the canonicalised
/// `http://host:port` (everything else discarded) or `None`.
fn validate_runtime_origin(raw: &str) -> Option<String> {
    let url = reqwest::Url::parse(raw).ok()?;
    if url.scheme() != "http" {
        return None;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    if url.path() != "/" && !url.path().is_empty() {
        return None;
    }
    if url.query().is_some() || url.fragment().is_some() {
        return None;
    }
    let host = url.host_str()?;
    if !LOOPBACK_HOSTS.contains(&host) {
        return None;
    }
    let port = url.port()?;
    if port == 0 {
        return None;
    }
    Some(format!("http://{host}:{port}"))
}

/// Read `agent-runtime.json`. Absence, or a `url` that is not an exact loopback
/// origin, means "not running / not usable".
fn read_runtime_info() -> Option<RuntimeInfo> {
    let path = crate::paths::get_data_directory().join("agent-runtime.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let url = match value.get("url").and_then(|u| u.as_str()) {
        Some(candidate) => match validate_runtime_origin(candidate) {
            Some(origin) => origin,
            None => {
                eprintln!(
                    "ScreenerBot MCP: agent-runtime.json url is not an accepted loopback origin; \
                     serving zero capabilities."
                );
                return None;
            }
        },
        None => return None,
    };
    Some(RuntimeInfo {
        url,
        pid: value.get("pid").and_then(|p| p.as_u64()),
        version: value
            .get("version")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
    })
}

/// A reqwest client bounded for bridge use: no redirects, bounded connect and
/// total request time. Construction is fallible; callers propagate the failure
/// and never fall back to an unbounded client that would silently lose the
/// redirect and timeout policy.
fn build_http_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build the bounded MCP HTTP client: {e}"))
}

fn credential_from_env(cli_client_id: Option<&str>) -> (Option<String>, Option<String>) {
    let client_id = std::env::var(CLIENT_ID_ENV)
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| cli_client_id.map(str::to_owned));
    let secret = std::env::var(SECRET_ENV).ok().filter(|s| !s.is_empty());
    (client_id, secret)
}

/// Run an MCP server over stdio. Never prints to stdout.
pub async fn serve_stdio(cli_client_id: Option<&str>) -> anyhow::Result<()> {
    let runtime = read_runtime_info();
    let (client_id, secret) = credential_from_env(cli_client_id);

    if runtime.is_none() {
        eprintln!(
            "ScreenerBot MCP: ScreenerBot does not appear to be running (no agent-runtime.json); \
             serving zero capabilities."
        );
    }
    if client_id.is_none() || secret.is_none() {
        eprintln!(
            "ScreenerBot MCP: set {CLIENT_ID_ENV} and {SECRET_ENV} to pair with a running \
             ScreenerBot; serving zero capabilities until then."
        );
    }

    let server = McpServer {
        http: build_http_client()?,
        runtime,
        client_id,
        secret,
    };
    let (stdin, stdout) = rmcp::transport::stdio();
    server.serve((stdin, stdout)).await?.waiting().await?;
    Ok(())
}

pub fn is_mcp_command(args: &[String]) -> bool {
    args.get(1).is_some_and(|arg| arg == "mcp")
}

/// Dispatch a `screenerbot mcp <...>` invocation. Runs before the normal boot
/// path so protocol stdout stays clean.
pub async fn dispatch(args: &[String]) -> anyhow::Result<bool> {
    if !is_mcp_command(args) {
        return Ok(false);
    }

    // Every log line from here on goes to stderr.
    crate::logger::route_console_to_stderr();

    let cli_client_id = args
        .windows(2)
        .find(|pair| pair[0] == "--client-id")
        .map(|pair| pair[1].as_str());

    match args.get(2).map(String::as_str) {
        Some("serve") => {
            serve_stdio(cli_client_id).await?;
            Ok(true)
        }
        Some("doctor") => {
            run_doctor(cli_client_id).await;
            Ok(true)
        }
        _ => {
            eprintln!("Usage: screenerbot mcp <serve | doctor>");
            eprintln!("Pairing: set {CLIENT_ID_ENV} and {SECRET_ENV} in the environment.");
            Ok(true)
        }
    }
}

/// Report runtime reachability and pairing status. Never prints the secret.
async fn run_doctor(cli_client_id: Option<&str>) {
    let runtime = read_runtime_info();
    match &runtime {
        None => {
            eprintln!("ScreenerBot MCP: ScreenerBot is not running (no agent-runtime.json).");
        }
        Some(info) => {
            let pid = info
                .pid
                .map(|p| p.to_string())
                .unwrap_or_else(|| "?".to_owned());
            let version = info.version.clone().unwrap_or_else(|| "?".to_owned());
            eprintln!(
                "ScreenerBot MCP: runtime found — url {}, pid {pid}, version {version}.",
                info.url
            );
        }
    }

    let (client_id, secret) = credential_from_env(cli_client_id);
    if runtime.is_none() {
        return;
    }
    let (Some(client_id), Some(secret)) = (client_id, secret) else {
        eprintln!(
            "ScreenerBot MCP: {CLIENT_ID_ENV} / {SECRET_ENV} not both set; cannot check pairing."
        );
        return;
    };

    let http = match build_http_client() {
        Ok(client) => client,
        Err(_) => {
            eprintln!(
                "ScreenerBot MCP: could not construct the bounded HTTP client; cannot check pairing."
            );
            return;
        }
    };
    let server = McpServer {
        http,
        runtime,
        client_id: Some(client_id),
        secret: Some(secret),
    };
    match server.bridge_post(BRIDGE_PING, serde_json::json!({})).await {
        Ok(value) => {
            let scope = value
                .get("scope")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");
            let label = value
                .get("client_label")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            eprintln!(
                "ScreenerBot MCP: bridge reachable; pairing OK (scope {scope}, label {label:?})."
            );
        }
        Err(reason) => {
            eprintln!("ScreenerBot MCP: bridge check failed ({reason}).");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(runtime: bool, creds: bool) -> McpServer {
        McpServer {
            http: reqwest::Client::new(),
            runtime: runtime.then(|| RuntimeInfo {
                url: "http://127.0.0.1:65535".to_owned(),
                pid: Some(1),
                version: Some("test".to_owned()),
            }),
            client_id: creds.then(|| "cid".to_owned()),
            secret: creds.then(|| "sec".to_owned()),
        }
    }

    #[test]
    fn not_ready_without_runtime_or_credentials() {
        assert!(server(false, false).ready().is_none());
        assert!(server(true, false).ready().is_none());
        assert!(server(false, true).ready().is_none());
        assert!(server(true, true).ready().is_some());
    }

    #[test]
    fn result_mapping_follows_success_flag() {
        let ok = result_to_call(Some(
            &serde_json::json!({ "success": true, "data": { "x": 1 } }),
        ));
        let is_err = matches!(ok, CallToolResponse::Complete(r) if r.is_error == Some(true));
        assert!(!is_err);

        let bad = result_to_call(Some(
            &serde_json::json!({ "success": false, "error": "no" }),
        ));
        let is_err = matches!(bad, CallToolResponse::Complete(r) if r.is_error == Some(true));
        assert!(is_err);

        let none = result_to_call(None);
        let is_err = matches!(none, CallToolResponse::Complete(r) if r.is_error == Some(true));
        assert!(is_err);
    }

    #[test]
    fn runtime_origin_accepts_only_exact_http_loopback() {
        assert_eq!(
            validate_runtime_origin("http://127.0.0.1:8080"),
            Some("http://127.0.0.1:8080".to_owned())
        );
        assert_eq!(
            validate_runtime_origin("http://localhost:49999/"),
            Some("http://localhost:49999".to_owned())
        );

        for bad in [
            "https://127.0.0.1:8080",        // wrong scheme
            "http://127.0.0.1",              // no port
            "http://127.0.0.1:0",            // zero port
            "http://user:pw@127.0.0.1:8080", // userinfo
            "http://127.0.0.1:8080/api",     // path
            "http://127.0.0.1:8080/?x=1",    // query
            "http://127.0.0.1:8080/#frag",   // fragment
            "http://evil.example.com:8080",  // non-loopback host
            "http://[::1]:8080",             // ipv6 loopback not allowed
            "http://169.254.169.254:80",     // link-local
            "ftp://127.0.0.1:8080",          // wrong scheme
            "127.0.0.1:8080",                // not a URL
        ] {
            assert!(validate_runtime_origin(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn http_client_builds_with_bounded_policy() {
        // Redirects disabled + bounded timeouts. Construction is fallible and
        // the caller propagates a failure rather than falling back to an
        // unbounded client.
        build_http_client().expect("bounded MCP HTTP client builds");
    }

    #[test]
    fn secret_is_only_read_from_env_never_cli() {
        // `--client-id` supplies only the id; there is no CLI path for a secret.
        let args: Vec<String> = vec![
            "screenerbot".into(),
            "mcp".into(),
            "serve".into(),
            "--client-id".into(),
            "abc".into(),
        ];
        let cli = args
            .windows(2)
            .find(|p| p[0] == "--client-id")
            .map(|p| p[1].as_str());
        std::env::remove_var(SECRET_ENV);
        let (_id, secret) = credential_from_env(cli);
        assert!(secret.is_none());
    }
}
