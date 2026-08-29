/**
 * Agent Connections Tab — pair external MCP clients with the running app.
 *
 * The app itself is the authority and the executable: `screenerbot mcp serve` is
 * a thin stdio bridge into this process, gated by the pairing's scope and by the
 * global in-app approval prompt. This tab creates, lists and revokes those
 * pairings and hands the operator client-specific setup built from the new
 * pairing.
 *
 * There is no universal MCP-client configuration format, so each client gets its
 * own native artifact: a copyable `claude mcp add` / `codex mcp add` command,
 * Claude Desktop JSON, a Codex TOML fallback, an OpenClaw native command,
 * Hermes YAML, and a plain stdio JSON object for generic clients. Nothing here
 * installs, downloads, or launches anything.
 *
 * Uses the existing dashboard-authenticated API only:
 *   GET    /api/agent-control/pairings          — list (never the secret)
 *   POST   /api/agent-control/pairings          — create, returns the secret ONCE
 *   DELETE /api/agent-control/pairings/:client   — revoke (effective next call)
 *
 * The one-time secret lives only in a closure variable for the lifetime of the
 * open panel. It is rendered once into the visible setup text (that is the whole
 * point of the panel) but is never written to a DOM attribute, a URL, browser
 * storage, a log, or a later list response, and it cannot be shown again once
 * dismissed. ScreenerBot stores only a one-way SHA-256 verifier; the MCP client,
 * once configured, keeps the plaintext under its own config (e.g. `claude mcp
 * get` prints it back; Codex masks it).
 *
 * The DOM-coupled dependencies (`core/utils.js`, `ui/confirmation_dialog.js`) are
 * dynamically imported inside the browser-only loader, so the pure config
 * generators below import cleanly under node for unit testing.
 */

const LIST_URL = "/api/agent-control/pairings";
const pairingUrl = (clientId) => `${LIST_URL}/${encodeURIComponent(clientId)}`;

/** The native stdio command a paired client runs. */
export const SERVE_ARGS = ["mcp", "serve"];
export const CLIENT_ID_ENV = "SCREENERBOT_CLIENT_ID";
export const SECRET_ENV = "SCREENERBOT_PAIRING_SECRET";
export const DATA_DIR_ENV = "SCREENERBOT_DATA_DIR";

/** The registered MCP server name across every client (matches JSON/TOML keys). */
export const MCP_SERVER_NAME = "screenerbot";

/**
 * The pairing response normally reports the running app's absolute backend
 * path. This placeholder is only the fail-safe for an unusual platform path
 * that cannot be represented as UTF-8 JSON.
 */
export const EXE_PLACEHOLDER = "/absolute/path/to/screenerbot";

/** Label constraints mirror `agent_control::pairing` (1..=64, no control chars). */
export const MAX_LABEL = 64;

/**
 * Least-privilege first: `read` is preselected and every stronger scope only
 * widens what a person can later approve — nothing here runs unattended.
 */
export const SCOPE_OPTIONS = [
  {
    value: "read",
    label: "Read only",
    description: "Analysis and portfolio reads. No configuration changes and no trades.",
  },
  {
    value: "operate",
    label: "Operate",
    description: "Adds configuration and system actions. Each one still needs in-app approval.",
  },
  {
    value: "trade",
    label: "Trade",
    description: "Adds trade execution. Every trade always needs in-app approval.",
  },
];
export const DEFAULT_SCOPE = "read";

/**
 * Client kinds offered for setup guidance. The `id` doubles as the pairing's
 * `agent_kind` slug (all are valid `[a-z0-9_-]`).
 */
export const SETUP_CLIENTS = [
  { id: "claude", label: "Claude Code / Desktop" },
  { id: "codex", label: "Codex CLI" },
  { id: "openclaw", label: "OpenClaw" },
  { id: "hermes", label: "Hermes" },
  { id: "generic", label: "Generic stdio MCP" },
];
export const DEFAULT_CLIENT = "claude";

let activeTabCleanup = null;
let loadGeneration = 0;

/**
 * Release listeners and one-time credentials whenever Settings leaves this tab.
 * The content node survives tab switches, so relying on innerHTML replacement
 * would retain delegated handlers and their secret-bearing closures.
 */
export function teardownAgentConnectionsTab() {
  loadGeneration += 1;
  activeTabCleanup?.();
  activeTabCleanup = null;
}

// ── Pure config generators (unit-tested under node) ──────────────────────────

/** Wrap a value in POSIX single quotes, escaping any embedded single quote. */
export function shQuote(value) {
  return `'${String(value).replace(/'/g, "'\\''")}'`;
}

/** A TOML basic string with the mandatory escapes. */
export function tomlString(value) {
  return `"${String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

/** A double-quoted YAML scalar with the mandatory escapes. */
export function yamlString(value) {
  return `"${String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

/** The binary path to emit: a real one if supplied, otherwise the placeholder. */
export function exePath(supplied) {
  const trimmed = String(supplied ?? "").trim();
  return trimmed || EXE_PLACEHOLDER;
}

/** The common stdio server object used by JSON-configured MCP clients. */
export function mcpServerEntry(exe, clientId, secret) {
  return {
    command: exePath(exe),
    args: [...SERVE_ARGS],
    env: { [CLIENT_ID_ENV]: clientId, [SECRET_ENV]: secret },
  };
}

/** `mcpServers.screenerbot` wrapper — Claude Desktop and generic stdio clients. */
export function genericStdioJson(exe, clientId, secret) {
  return JSON.stringify(
    { mcpServers: { [MCP_SERVER_NAME]: mcpServerEntry(exe, clientId, secret) } },
    null,
    2
  );
}

/** `[mcp_servers.screenerbot]` block for Codex CLI `~/.codex/config.toml`. */
export function codexToml(exe, clientId, secret) {
  return [
    `[mcp_servers.${MCP_SERVER_NAME}]`,
    `command = ${tomlString(exePath(exe))}`,
    `args = [${SERVE_ARGS.map(tomlString).join(", ")}]`,
    "",
    `[mcp_servers.${MCP_SERVER_NAME}.env]`,
    `${CLIENT_ID_ENV} = ${tomlString(clientId)}`,
    `${SECRET_ENV} = ${tomlString(secret)}`,
    "",
  ].join("\n");
}

/** `mcp_servers:` YAML block in Hermes' documented shape. */
export function hermesYaml(exe, clientId, secret) {
  return [
    "mcp_servers:",
    `  ${MCP_SERVER_NAME}:`,
    `    command: ${yamlString(exePath(exe))}`,
    `    args: [${SERVE_ARGS.map(yamlString).join(", ")}]`,
    "    env:",
    `      ${CLIENT_ID_ENV}: ${yamlString(clientId)}`,
    `      ${SECRET_ENV}: ${yamlString(secret)}`,
    "",
  ].join("\n");
}

/** Copyable `claude mcp add` command (Claude Code). Every dynamic token quoted. */
export function claudeCodeCommand(exe, clientId, secret) {
  return [
    "claude mcp add --scope user",
    MCP_SERVER_NAME,
    `-e ${shQuote(`${CLIENT_ID_ENV}=${clientId}`)}`,
    `-e ${shQuote(`${SECRET_ENV}=${secret}`)}`,
    `-- ${shQuote(exePath(exe))} ${SERVE_ARGS.join(" ")}`,
  ].join(" ");
}

/** Copyable `codex mcp add` command (Codex CLI). Every dynamic token quoted. */
export function codexCommand(exe, clientId, secret) {
  return [
    "codex mcp add",
    MCP_SERVER_NAME,
    `--env ${shQuote(`${CLIENT_ID_ENV}=${clientId}`)}`,
    `--env ${shQuote(`${SECRET_ENV}=${secret}`)}`,
    `-- ${shQuote(exePath(exe))} ${SERVE_ARGS.join(" ")}`,
  ].join(" ");
}

/** Copyable `openclaw mcp add` command using its saved stdio-server registry. */
export function openClawCommand(exe, clientId, secret) {
  return [
    "openclaw mcp add",
    MCP_SERVER_NAME,
    `--command ${shQuote(exePath(exe))}`,
    ...SERVE_ARGS.map((arg) => `--arg ${shQuote(arg)}`),
    `--env ${shQuote(`${CLIENT_ID_ENV}=${clientId}`)}`,
    `--env ${shQuote(`${SECRET_ENV}=${secret}`)}`,
  ].join(" ");
}

/**
 * Per-client setup: one or more separately labelled blocks plus honest notes.
 * Claude, Codex, and OpenClaw get their native `mcp add` commands. Hermes gets
 * its documented YAML shape. `exe` is the installed binary path when known,
 * else the placeholder.
 */
export function clientSetup(id, exe, clientId, secret) {
  const shared = [];
  if (exePath(exe) === EXE_PLACEHOLDER) {
    shared.push(
      `Replace ${EXE_PLACEHOLDER} with the absolute path to your ScreenerBot binary — ` +
        "the running app could not represent its executable path on this system."
    );
  }
  shared.push(
    `If you run ScreenerBot with a non-default data directory, also set ${DATA_DIR_ENV} ` +
      "on the client (another -e / --env flag, or an env entry) to the same path."
  );

  switch (id) {
    case "codex":
      return {
        notes: [
          "Run the command, or add the TOML block to ~/.codex/config.toml " +
            "($CODEX_HOME/config.toml). Restart Codex afterwards.",
          "`codex mcp get screenerbot` masks the secret in its output.",
          ...shared,
        ],
        blocks: [
          {
            label: "Codex CLI — terminal command",
            lang: "sh",
            body: codexCommand(exe, clientId, secret),
          },
          {
            label: "Codex CLI — ~/.codex/config.toml (fallback)",
            lang: "toml",
            body: codexToml(exe, clientId, secret),
          },
        ],
      };
    case "claude":
      return {
        notes: [
          "Claude Code: run the command, then restart Claude Code. `claude mcp get " +
            "screenerbot` will print the configured environment, including the secret.",
          "Claude Desktop: merge the JSON into claude_desktop_config.json under " +
            "`mcpServers` and restart the app.",
          ...shared,
        ],
        blocks: [
          {
            label: "Claude Code — terminal command",
            lang: "sh",
            body: claudeCodeCommand(exe, clientId, secret),
          },
          {
            label: "Claude Desktop — claude_desktop_config.json",
            lang: "json",
            body: genericStdioJson(exe, clientId, secret),
          },
        ],
      };
    case "openclaw":
      return {
        notes: [
          "Run the command, then use `openclaw mcp doctor screenerbot --probe` to verify " +
            "that the saved stdio server starts and exposes tools.",
          ...shared,
        ],
        blocks: [
          {
            label: "OpenClaw — terminal command",
            lang: "sh",
            body: openClawCommand(exe, clientId, secret),
          },
        ],
      };
    case "hermes":
      return {
        notes: [
          "Add this under `mcp_servers` in Hermes' configuration file, then restart Hermes.",
          ...shared,
        ],
        blocks: [
          {
            label: "Hermes — mcp_servers (YAML)",
            lang: "yaml",
            body: hermesYaml(exe, clientId, secret),
          },
        ],
      };
    case "generic":
    default:
      return {
        notes: [
          "Any MCP client that speaks stdio: run this command with these args and " +
            "environment, wherever the client keeps its server list.",
          ...shared,
        ],
        blocks: [
          {
            label: "Generic stdio MCP client",
            lang: "json",
            body: genericStdioJson(exe, clientId, secret),
          },
        ],
      };
  }
}

/** Validate a label the way the backend will, so the error shows before the POST. */
export function validateLabel(raw) {
  const trimmed = String(raw ?? "").trim();
  if (!trimmed) return { ok: false, error: "Enter a name for this connection." };
  if ([...trimmed].length > MAX_LABEL) {
    return { ok: false, error: `Name must be ${MAX_LABEL} characters or fewer.` };
  }
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f]/.test(trimmed)) {
    return { ok: false, error: "Name must not contain control characters." };
  }
  return { ok: true, value: trimmed };
}

// ── DOM (browser only) ──────────────────────────────────────────────────────

/** Set by the loader before any builder runs; keeps the pure helpers node-safe. */
let Utils = null;

function scopeRow(option) {
  const checked = option.value === DEFAULT_SCOPE ? " checked" : "";
  return `
    <label class="checkbox-label agent-scope-option">
      <input type="radio" name="agentPairScope" value="${option.value}"${checked}>
      <span>
        <strong>${Utils.escapeHtml(option.label)}</strong>
        <span class="agent-scope-desc">${Utils.escapeHtml(option.description)}</span>
      </span>
    </label>`;
}

function clientOptions(selected) {
  return SETUP_CLIENTS.map(
    (c) =>
      `<option value="${c.id}"${c.id === selected ? " selected" : ""}>${Utils.escapeHtml(
        c.label
      )}</option>`
  ).join("");
}

function clientLabel(kind) {
  return SETUP_CLIENTS.find((client) => client.id === kind)?.label || kind;
}

function buildShell() {
  return `
    <div class="settings-section agent-connections">
      <h3 class="settings-section-title">
        <i class="icon-plug"></i>
        Agent Connections
      </h3>
      <p class="settings-section-description">
        Connect Claude, Codex, Hermes, OpenClaw, or any stdio MCP client; ScreenerBot must remain
        running, and configuration changes or trades still require approval in the dashboard.
      </p>

      <div class="settings-group agent-pair-create">
        <div class="settings-field">
          <div class="settings-field-info">
            <label for="agentPairLabel">Connection name</label>
            <span class="settings-field-hint">Shown in the list below so you can tell connections apart.</span>
          </div>
          <div class="settings-field-control">
            <input type="text" id="agentPairLabel" class="settings-input" maxlength="${MAX_LABEL}"
                   placeholder="Laptop coding agent" autocomplete="off" spellcheck="false">
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label for="agentPairClient">Client</label>
            <span class="settings-field-hint">Picks the setup shown after the connection is created.</span>
          </div>
          <div class="settings-field-control">
            <select id="agentPairClient" class="settings-select" data-custom-select>
              ${clientOptions(DEFAULT_CLIENT)}
            </select>
          </div>
        </div>

        <div class="settings-field agent-scope-field" role="radiogroup"
             aria-labelledby="agentPairScopeLabel" aria-describedby="agentPairScopeHint">
          <div class="settings-field-info">
            <span class="settings-field-label" id="agentPairScopeLabel">Scope</span>
            <span class="settings-field-hint" id="agentPairScopeHint">Grant the least a connection needs. Stronger scopes only widen what you can approve.</span>
          </div>
          <div class="settings-field-control agent-scope-list">
            ${SCOPE_OPTIONS.map(scopeRow).join("")}
          </div>
        </div>

        <div class="form-group agent-pair-actions">
          <p class="form-error" id="agentPairError" hidden></p>
          <button type="button" class="btn btn-primary btn-sm" id="agentPairCreate">
            <i class="icon-plus"></i>
            Create connection
          </button>
        </div>
      </div>

      <div class="agent-issued" id="agentIssued" role="group"
           aria-label="New connection credential" hidden>
        <div class="agent-issued-warn">
          <i class="icon-triangle-alert"></i>
          <span>Copy the secret now. It is shown once and cannot be retrieved again — revoke and
          recreate the connection if you lose it. ScreenerBot keeps only a one-way verifier; your
          MCP client stores the plaintext under its own configuration.</span>
        </div>
        <div class="agent-issued-fields">
          <div class="agent-issued-row">
            <span class="agent-issued-key">Client ID</span>
            <code class="agent-issued-value" id="agentIssuedClientId"></code>
            <button type="button" class="btn btn-secondary btn-sm" data-copy-issued="client-id">Copy</button>
          </div>
          <div class="agent-issued-row">
            <span class="agent-issued-key">One-time secret</span>
            <code class="agent-issued-value agent-issued-secret" id="agentIssuedSecret"></code>
            <button type="button" class="btn btn-secondary btn-sm" data-copy-issued="secret">Copy</button>
          </div>
        </div>

        <div class="agent-setup">
          <div class="agent-setup-head">
            <label for="agentSetupClient">Setup for</label>
            <select id="agentSetupClient" class="settings-select" data-custom-select>
              ${clientOptions(DEFAULT_CLIENT)}
            </select>
          </div>
          <ul class="agent-setup-notes" id="agentSetupNotes"></ul>
          <div class="agent-setup-blocks" id="agentSetupBlocks"></div>
        </div>

        <button type="button" class="btn btn-secondary btn-sm" id="agentIssuedDone">Done</button>
      </div>

      <div class="agent-pair-section">
        <div class="agent-pair-list-head">
          <h4>Connections</h4>
          <span id="agentPairCount"></span>
        </div>
        <div class="agent-pair-list" id="agentPairList">
          <div class="settings-loading"><i class="icon-loader spin"></i> Loading connections...</div>
        </div>
      </div>
    </div>
  `;
}

function scopeBadge(scope) {
  const known = SCOPE_OPTIONS.find((s) => s.value === scope);
  return `<span class="agent-scope-badge agent-scope-badge--${Utils.escapeHtml(
    scope
  )}">${Utils.escapeHtml(known ? known.label : scope)}</span>`;
}

function renderList(container, rows) {
  const countEl = container.closest(".agent-pair-section")?.querySelector("#agentPairCount");
  if (!Array.isArray(rows) || rows.length === 0) {
    if (countEl) countEl.textContent = "0 active";
    container.innerHTML =
      '<div class="settings-empty">No connections yet. Create one above to pair a client.</div>';
    return;
  }
  const active = rows.filter((r) => !r.revoked);
  const revoked = rows.filter((r) => r.revoked);
  if (countEl) countEl.textContent = `${active.length} active`;
  const rowHtml = (r) => `
    <div class="agent-pair-row${r.revoked ? " agent-pair-row--revoked" : ""}" role="listitem">
      <div class="agent-pair-main">
        <span class="agent-pair-label">${Utils.escapeHtml(r.label)}</span>
        <span class="agent-pair-meta">
          ${Utils.escapeHtml(clientLabel(r.agent_kind))} · ${scopeBadge(r.scope)}
        </span>
      </div>
      <div class="agent-pair-times">
        <span>Created ${Utils.escapeHtml(Utils.formatTimeAgo(r.created_at))}</span>
        <span>${
          r.last_used_at
            ? "Last used " + Utils.escapeHtml(Utils.formatTimeAgo(r.last_used_at))
            : "Never used"
        }</span>
      </div>
      ${
        r.revoked
          ? ""
          : `<div class="agent-pair-action"><button type="button" class="btn btn-danger btn-sm" data-revoke="${Utils.escapeHtml(
              r.client_id
            )}" data-label="${Utils.escapeHtml(r.label)}">Revoke</button></div>`
      }
    </div>`;
  container.innerHTML =
    (active.length
      ? `<div class="agent-pair-active" role="list">${active.map(rowHtml).join("")}</div>`
      : '<div class="settings-empty">No active connections.</div>') +
    (revoked.length
      ? `<details class="agent-pair-revoked-group">
          <summary><i class="icon-chevron-right"></i> Revoked connections <span>${revoked.length}</span></summary>
          <div role="list">${revoked.map(rowHtml).join("")}</div>
        </details>`
      : "");
}

/**
 * Load and wire the Agent Connections tab. Follows the security/telegram loader
 * pattern: this owns `content.innerHTML`.
 */
export async function loadAgentConnectionsTab(_dialog, content) {
  teardownAgentConnectionsTab();
  const generation = loadGeneration;
  content.innerHTML =
    '<div class="settings-loading"><i class="icon-loader spin"></i> Loading connections...</div>';

  let ConfirmationDialog;
  try {
    [Utils, { ConfirmationDialog }] = await Promise.all([
      import("../../core/utils.js"),
      import("../confirmation_dialog.js"),
    ]);
  } catch {
    if (generation !== loadGeneration) return;
    content.innerHTML = '<div class="settings-error">Failed to load Agent Connections</div>';
    return;
  }

  if (generation !== loadGeneration) return;

  // One-time credential, held only while the issued panel is on screen.
  let issued = null; // { clientId, secret, exe }
  // The blocks currently rendered, so `data-copy-block` can resolve by index.
  let currentBlocks = [];

  content.innerHTML = buildShell();

  const listEl = content.querySelector("#agentPairList");
  const errorEl = content.querySelector("#agentPairError");
  const labelEl = content.querySelector("#agentPairLabel");
  const clientEl = content.querySelector("#agentPairClient");
  const createBtn = content.querySelector("#agentPairCreate");
  const issuedEl = content.querySelector("#agentIssued");
  const issuedClientIdEl = content.querySelector("#agentIssuedClientId");
  const issuedSecretEl = content.querySelector("#agentIssuedSecret");
  const setupClientEl = content.querySelector("#agentSetupClient");
  const setupNotesEl = content.querySelector("#agentSetupNotes");
  const setupBlocksEl = content.querySelector("#agentSetupBlocks");

  function showError(message) {
    errorEl.textContent = message;
    errorEl.hidden = !message;
  }

  function clearIssued() {
    issued = null;
    currentBlocks = [];
    issuedEl.hidden = true;
    issuedClientIdEl.textContent = "";
    issuedSecretEl.textContent = "";
    setupNotesEl.innerHTML = "";
    setupBlocksEl.innerHTML = "";
  }

  function renderSetup() {
    if (!issued) return;
    const setup = clientSetup(setupClientEl.value, issued.exe, issued.clientId, issued.secret);
    currentBlocks = setup.blocks;
    setupNotesEl.innerHTML = setup.notes.map((n) => `<li>${Utils.escapeHtml(n)}</li>`).join("");
    setupBlocksEl.innerHTML = setup.blocks
      .map(
        (b, i) => `
        <div class="agent-setup-block">
          <div class="agent-setup-block-head">
            <span class="agent-setup-block-label">${Utils.escapeHtml(b.label)}</span>
            <button type="button" class="btn btn-secondary btn-sm" data-copy-block="${i}">Copy</button>
          </div>
          <pre class="agent-setup-body"><code>${Utils.escapeHtml(b.body)}</code></pre>
        </div>`
      )
      .join("");
  }

  async function refreshList() {
    try {
      const res = await fetch(LIST_URL, {
        headers: { Accept: "application/json" },
        signal: controller.signal,
      });
      if (!res.ok) {
        listEl.innerHTML = '<div class="settings-error">Could not load connections</div>';
        return;
      }
      renderList(listEl, await res.json());
    } catch {
      if (controller.signal.aborted) return;
      listEl.innerHTML = '<div class="settings-error">Could not load connections</div>';
    }
  }

  async function createPairing() {
    showError("");
    const label = validateLabel(labelEl.value);
    if (!label.ok) {
      showError(label.error);
      return;
    }
    const scopeEl = content.querySelector('input[name="agentPairScope"]:checked');
    const scope = scopeEl ? scopeEl.value : DEFAULT_SCOPE;
    const agentKind = clientEl.value;

    createBtn.disabled = true;
    try {
      const res = await fetch(LIST_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ label: label.value, agent_kind: agentKind, scope }),
        signal: controller.signal,
      });
      const body = await res.json().catch(() => null);
      if (controller.signal.aborted) return;
      if (!res.ok) {
        showError((body && body.error && body.error.message) || "Could not create the connection.");
        return;
      }
      // Success: hold the secret in memory only, render the one-time panel.
      issued = {
        clientId: body.client_id,
        secret: body.pairing_secret,
        exe: body.binary_path || null,
      };
      issuedClientIdEl.textContent = issued.clientId;
      issuedSecretEl.textContent = issued.secret;
      setupClientEl.value = SETUP_CLIENTS.some((c) => c.id === agentKind) ? agentKind : "generic";
      renderSetup();
      issuedEl.hidden = false;
      labelEl.value = "";
      await refreshList();
    } catch {
      if (controller.signal.aborted) return;
      showError("Could not reach ScreenerBot to create the connection.");
    } finally {
      createBtn.disabled = false;
    }
  }

  async function revokePairing(clientId, label) {
    const { confirmed } = await ConfirmationDialog.show({
      title: "Revoke connection",
      message: `Revoke "${label}"? The client stops working on its next request and cannot be restored.`,
      confirmLabel: "Revoke",
      cancelLabel: "Cancel",
      variant: "danger",
    });
    if (!confirmed) return;
    if (controller.signal.aborted) return;
    try {
      const res = await fetch(pairingUrl(clientId), {
        method: "DELETE",
        signal: controller.signal,
      });
      if (!res.ok && res.status !== 404) {
        Utils.showToast({ type: "error", title: "Could not revoke the connection" });
        return;
      }
      await refreshList();
    } catch {
      if (controller.signal.aborted) return;
      Utils.showToast({ type: "error", title: "Could not reach ScreenerBot to revoke" });
    }
  }

  function copyIssuedValue(what) {
    if (!issued) return;
    const value = what === "secret" ? issued.secret : issued.clientId;
    const labelText = what === "secret" ? "One-time secret" : "Client ID";
    Utils.copyToClipboard(value)
      .then(() => Utils.notifyCopied(labelText))
      .catch((err) => Utils.notifyCopyFailed(err));
  }

  function copyBlock(index) {
    const block = currentBlocks[Number(index)];
    if (!block) return;
    Utils.copyToClipboard(block.body)
      .then(() => Utils.notifyCopied(block.label))
      .catch((err) => Utils.notifyCopyFailed(err));
  }

  const controller = new AbortController();
  const listenerOptions = { signal: controller.signal };
  const issuedDoneBtn = content.querySelector("#agentIssuedDone");

  createBtn.addEventListener("click", createPairing, listenerOptions);
  issuedDoneBtn.addEventListener("click", clearIssued, listenerOptions);
  setupClientEl.addEventListener("change", renderSetup, listenerOptions);
  content.addEventListener(
    "click",
    (e) => {
      const copyIssuedBtn = e.target.closest("[data-copy-issued]");
      if (copyIssuedBtn) {
        copyIssuedValue(copyIssuedBtn.dataset.copyIssued);
        return;
      }
      const copyBlockBtn = e.target.closest("[data-copy-block]");
      if (copyBlockBtn) {
        copyBlock(copyBlockBtn.dataset.copyBlock);
        return;
      }
      const revokeBtn = e.target.closest("[data-revoke]");
      if (revokeBtn) {
        revokePairing(revokeBtn.dataset.revoke, revokeBtn.dataset.label || "this connection");
      }
    },
    listenerOptions
  );

  activeTabCleanup = () => {
    controller.abort();
    clearIssued();
  };

  await refreshList();
}
