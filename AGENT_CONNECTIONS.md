# Agent Connections (MCP)

Agent Connections let an external [Model Context Protocol](https://modelcontextprotocol.io)
client — typically an AI coding agent — drive ScreenerBot's own tools: portfolio and
analysis reads, configuration, and (with approval) trades.

The connection is **native**. `screenerbot mcp serve` is a thin stdio bridge built into
the ScreenerBot binary; it holds no trading logic. It discovers the running app from
`agent-runtime.json`, then calls an internal loopback bridge that
resolves the pairing's scope, applies the permission policy, and runs each tool inside the
live process. There is no separate package, extension, sidecar, hosted endpoint, or
installer script, and no second implementation of any tool.

## Prerequisites

- ScreenerBot installed and **running**. A paired client has zero capabilities while the
  app process is closed; the dashboard window may be closed as long as the process stays
  up, but a person must open it to approve any money- or config-changing action.
- The absolute path to the `screenerbot` binary (or the binary on your `PATH`).
- An MCP client that speaks **stdio** (Claude Code / Claude Desktop, Codex CLI, Hermes,
  OpenClaw, or any standards-based stdio MCP client).

## Pair a client (in-app)

1. Open **Settings → Agent Connections**.
2. Enter a **name** (how the connection shows up in the list), pick the **client**, and
   choose the least-privilege **scope** you need (see below).
3. Click **Create connection**. ScreenerBot shows a **client id** and a **one-time
   secret**, plus setup text tailored to the client you picked.
4. Copy the secret now. It is shown **once** and cannot be retrieved again — ScreenerBot
   keeps only a SHA-256 verifier. If you lose it, revoke the connection and create a new
   one.
5. Apply the generated setup for your client (below), then restart the client.

### Scopes

| Scope | Grants | Notes |
|-------|--------|-------|
| `read` | Analysis and portfolio reads | No config changes, no trades. Start here. |
| `operate` | Adds configuration and system actions | Each still needs in-app approval. |
| `trade` | Adds trade execution | Every trade always needs in-app approval. |

A stronger scope only widens what a person can approve. Nothing runs unattended.

## Configure a client

Every stdio MCP client runs the same command with the same two environment variables:

```
command:  <absolute path to screenerbot>
args:     ["mcp", "serve"]
env:      SCREENERBOT_CLIENT_ID       = <client id>
          SCREENERBOT_PAIRING_SECRET  = <one-time secret>
```

The secret is read **only** from the environment, never from a command-line argument. If
you run ScreenerBot with a non-default data directory, also pass
`SCREENERBOT_DATA_DIR=<same path>` to the client so the bridge finds the right
`agent-runtime.json`.

The in-app panel fills the client id, secret and (where known) the binary path into the
snippets below. Until the app reports its own install path, the snippets use the marked
placeholder `/absolute/path/to/screenerbot` — replace it with the real path.

### Claude Code

```sh
claude mcp add --scope user screenerbot \
  -e 'SCREENERBOT_CLIENT_ID=<client id>' \
  -e 'SCREENERBOT_PAIRING_SECRET=<one-time secret>' \
  -- '/absolute/path/to/screenerbot' mcp serve
```

Restart Claude Code afterwards. `claude mcp get screenerbot` prints the configured
environment, **including the secret** — that is the client's own storage, not ScreenerBot.

### Claude Desktop

Merge into `claude_desktop_config.json` (macOS:
`~/Library/Application Support/Claude/`, Windows: `%APPDATA%\Claude\`) and restart:

```json
{
  "mcpServers": {
    "screenerbot": {
      "command": "/absolute/path/to/screenerbot",
      "args": ["mcp", "serve"],
      "env": {
        "SCREENERBOT_CLIENT_ID": "<client id>",
        "SCREENERBOT_PAIRING_SECRET": "<one-time secret>"
      }
    }
  }
}
```

### Codex CLI

```sh
codex mcp add screenerbot \
  --env 'SCREENERBOT_CLIENT_ID=<client id>' \
  --env 'SCREENERBOT_PAIRING_SECRET=<one-time secret>' \
  -- '/absolute/path/to/screenerbot' mcp serve
```

Or add the block to `~/.codex/config.toml` (`$CODEX_HOME/config.toml`) and restart Codex:

```toml
[mcp_servers.screenerbot]
command = "/absolute/path/to/screenerbot"
args = ["mcp", "serve"]

[mcp_servers.screenerbot.env]
SCREENERBOT_CLIENT_ID = "<client id>"
SCREENERBOT_PAIRING_SECRET = "<one-time secret>"
```

`codex mcp get screenerbot` masks the secret in its output.

### Hermes

Add under `mcp_servers` in the Hermes configuration file and restart:

```yaml
mcp_servers:
  screenerbot:
    command: "/absolute/path/to/screenerbot"
    args: ["mcp", "serve"]
    env:
      SCREENERBOT_CLIENT_ID: "<client id>"
      SCREENERBOT_PAIRING_SECRET: "<one-time secret>"
```

### OpenClaw

```sh
openclaw mcp add screenerbot \
  --command '/absolute/path/to/screenerbot' \
  --arg 'mcp' --arg 'serve' \
  --env 'SCREENERBOT_CLIENT_ID=<client id>' \
  --env 'SCREENERBOT_PAIRING_SECRET=<one-time secret>'
```

Then run `openclaw mcp doctor screenerbot --probe` to verify that the saved server starts
and exposes tools.

### Other generic stdio clients

Use the same JSON object as Claude Desktop (`mcpServers.screenerbot`) wherever the client
keeps its stdio server list.

### Windows

Use the JSON/TOML/YAML shapes above with the full path to `screenerbot.exe`:

- Claude Desktop: `%APPDATA%\Claude\claude_desktop_config.json`
- Codex: `%USERPROFILE%\.codex\config.toml`

## Approval behavior

- Read-only tools within scope run immediately.
- Any trade, and anything the permission policy marks *ask*, is **never** run by the agent
  directly. It creates a request that a person approves or denies in ScreenerBot; the
  global approval prompt is visible from every dashboard page.
- An approved request runs **at most once**. A denied or expired request does not run.
- The client call waits up to five minutes for a decision, then returns a "still pending"
  message; approving later and retrying the same call does not double-execute it.

## Offline and restart behavior

If ScreenerBot is not running when the client starts, each later bridge request checks for
`agent-runtime.json`, so the client can pick up capabilities once the app is up without a
restart. If the app restarts on a different port, a transport failure triggers **one**
rediscovery against the new origin (read-only probes only; a state-changing call is never
replayed). The bridge only accepts a `127.0.0.1` / `localhost` origin and never starts the
app itself.

## Doctor

```sh
SCREENERBOT_CLIENT_ID=<id> SCREENERBOT_PAIRING_SECRET=<secret> screenerbot mcp doctor
```

`doctor` prints app reachability and pairing status to stderr and never prints the secret.
Its **exit code** is the machine-readable contract:

| Code | Meaning |
|------|---------|
| `0` | Live app reached and the pairing probe succeeded |
| `3` | No runtime — ScreenerBot is not running (`agent-runtime.json` absent or not a loopback origin) |
| `4` | `SCREENERBOT_CLIENT_ID` / `SCREENERBOT_PAIRING_SECRET` not both set |
| `5` | Runtime found but the bridge could not be reached |
| `6` | Bridge reached but the pairing was rejected — revoked/unknown/malformed, or agent control disabled |

## Troubleshooting

| Symptom | Cause / fix |
|---------|-------------|
| Client lists no tools | ScreenerBot is not running, or the pairing is missing/revoked. Start the app; re-check the client id and secret. `mcp doctor` gives the exact reason and exit code. |
| `doctor` exits `3` | The app is not running, or its `agent-runtime.json` url is not an accepted loopback origin. |
| `doctor` exits `6` | The pairing was revoked or is invalid, or **Settings → agent control** is disabled. Recreate the pairing / re-enable control. |
| Every trade call "requires approval" | Expected. Approve it in ScreenerBot, then retry the same call. |
| Config value rejected as out of scope | The pairing's scope is too low. Revoke and recreate with the scope you need. |
| Secret lost | It cannot be recovered. Revoke the connection and create a new one. |

## Revocation

Revoke from **Settings → Agent Connections → Revoke**. It takes effect on the client's
**next** request — no restart of ScreenerBot needed — and cannot be undone. Then remove
the server from the client:

```sh
claude mcp remove --scope user screenerbot
codex mcp remove screenerbot
```

For other clients, delete the `screenerbot` entry from their MCP configuration.

## Security model

- **The app is the authority and the executable.** The bridge subprocess carries no tool
  registry and no policy; it cannot act on its own.
- **stdio only.** No Streamable HTTP MCP endpoint is mounted anywhere, and the bridge only
  ever dials a `127.0.0.1` / `localhost` origin.
- **The pairing credential is the only key.** The bridge path is exempt from the dashboard
  token/cookie but authenticates the pairing on every request, in every mode. Unknown,
  malformed, and revoked credentials all fail the same way.
- **ScreenerBot never persists the plaintext secret.** It stores only a SHA-256 verifier
  and compares it in constant time. Your MCP client, once configured, keeps the plaintext
  in its own local configuration under its own security model (`claude mcp get` will
  display it; Codex masks it) — that copy is what launches the bridge. ScreenerBot never
  writes the secret to logs, URLs, DOM attributes, or browser storage.
- **Scope is resolved live** from the pairing store on every call, so revocation is
  effective immediately. To change scope, revoke the pairing and create another.
- **Money and config moves are approval-gated**, verified against on-chain reality by the
  trading engine, and never trusted from the client.
