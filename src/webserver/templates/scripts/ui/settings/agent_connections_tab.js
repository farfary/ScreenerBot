/**
 * Agent Connections Tab — pair external MCP clients with the running app.
 *
 * The app itself is the authority and the executable: `screenerbot mcp serve` is
 * a thin stdio bridge into this process, gated by that connection's OWN
 * per-category permissions. This tab creates, lists, limits and revokes those
 * pairings and hands the operator client-specific setup built from the new
 * pairing.
 *
 * A new connection starts at full access — it can do anything the owner can,
 * except read or write wallet private-key material, which no agent surface can
 * reach at any level. Each category can then be set to Ask (the call parks until
 * a person decides in the app) or Off (refused), per connection, at any time.
 *
 * There is no universal MCP-client configuration format, so each client gets its
 * own native artifact: a copyable `claude mcp add` / `codex mcp add` command,
 * Claude Desktop JSON, a Codex TOML fallback, an OpenClaw native command,
 * Hermes YAML, and a plain stdio JSON object for generic clients. Nothing here
 * installs, downloads, or launches anything.
 *
 * Uses the existing dashboard-authenticated API only:
 *   GET    /api/agent-control/pairings                      — list (never the secret)
 *   POST   /api/agent-control/pairings                      — create, returns the secret ONCE
 *   PATCH  /api/agent-control/pairings/:client/permissions  — limit or widen one connection
 *   DELETE /api/agent-control/pairings/:client              — revoke (effective next call)
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
import { apiErrorMessage } from "../../core/request_manager.js";
import { formatList } from "../../core/format.js";

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

/** Tool categories, in the order the permission grid renders them. */
export const CATEGORIES = [
  { key: "analysis" },
  { key: "portfolio" },
  { key: "trading" },
  { key: "config" },
  { key: "system" },
];

// Ids are the `ToolCategory` values (src/agent_control/tools/mod.rs); the Rust
// test `category_labels_exist_in_the_catalog` pins the keys.
const CATEGORY_LABELS = Object.freeze({
  analysis: "settings-agent-category-analysis",
  portfolio: "settings-agent-category-portfolio",
  trading: "settings-agent-category-trading",
  config: "settings-agent-category-config",
  system: "settings-agent-category-system",
});

const CATEGORY_DESCRIPTION_LABELS = Object.freeze({
  analysis: "settings-agent-category-analysis-description",
  portfolio: "settings-agent-category-portfolio-description",
  trading: "settings-agent-category-trading-description",
  config: "settings-agent-category-config-description",
  system: "settings-agent-category-system-description",
});

// The category as it reads inside a sentence.
const CATEGORY_INLINE_LABELS = Object.freeze({
  analysis: "settings-agent-category-analysis-inline",
  portfolio: "settings-agent-category-portfolio-inline",
  trading: "settings-agent-category-trading-inline",
  config: "settings-agent-category-config-inline",
  system: "settings-agent-category-system-inline",
});

/** The three levels a category can be set to, weakest last. */
export const LEVELS = [{ value: "allow" }, { value: "ask_user" }, { value: "deny" }];

// Ids are the serialized `PermissionLevel` values (src/agent_control/permissions.rs).
const LEVEL_LABELS = Object.freeze({
  allow: "settings-agent-level-allow",
  ask_user: "settings-agent-level-ask-user",
  deny: "settings-agent-level-deny",
});

const LEVEL_HINT_LABELS = Object.freeze({
  allow: "settings-agent-level-allow-hint",
  ask_user: "settings-agent-level-ask-user-hint",
  deny: "settings-agent-level-deny-hint",
});

const PRESET_LABELS = Object.freeze({
  full: "settings-agent-preset-full",
  ask: "settings-agent-preset-ask",
  read: "settings-agent-preset-read",
});

const PRESET_DESCRIPTION_LABELS = Object.freeze({
  full: "settings-agent-preset-full-description",
  ask: "settings-agent-preset-ask-description",
  read: "settings-agent-preset-read-description",
});

const SETUP_CLIENT_LABELS = Object.freeze({
  claude: "settings-agent-client-claude",
  codex: "settings-agent-client-codex",
  openclaw: "settings-agent-client-openclaw",
  hermes: "settings-agent-client-hermes",
  generic: "settings-agent-client-generic",
});

/** Every category at one level. */
export function uniformPermissions(level) {
  return Object.fromEntries(CATEGORIES.map((category) => [category.key, level]));
}

/**
 * What a new connection gets: full access. The owner limits it from this tab
 * afterwards, per connection, without recreating it.
 */
export function defaultPermissions() {
  return uniformPermissions("allow");
}

/**
 * One-click shapes offered above the grid. `custom` is not offered as a button;
 * it is what the grid falls into once a category is set individually.
 */
export const PRESETS = [
  {
    id: "full",
    permissions: () => defaultPermissions(),
  },
  {
    id: "ask",
    permissions: () => uniformPermissions("ask_user"),
  },
  {
    id: "read",
    permissions: () => ({
      ...uniformPermissions("deny"),
      analysis: "allow",
      portfolio: "allow",
    }),
  },
];

/** Normalize an arbitrary API/response value into a complete permission map. */
export function normalizePermissions(raw) {
  const known = new Set(LEVELS.map((level) => level.value));
  const source = raw && typeof raw === "object" ? raw : {};
  return Object.fromEntries(
    CATEGORIES.map(({ key }) => [key, known.has(source[key]) ? source[key] : "deny"])
  );
}

/** The preset id a permission map corresponds to, or "custom". */
export function presetFor(permissions) {
  const normalized = normalizePermissions(permissions);
  const match = PRESETS.find((preset) =>
    CATEGORIES.every(({ key }) => preset.permissions()[key] === normalized[key])
  );
  return match ? match.id : "custom";
}

/**
 * The one-line summary shown on a connection row: the preset name when it is
 * one, otherwise what is actually restricted — never a bare "custom".
 */
export function summarizePermissions(permissions) {
  const normalized = normalizePermissions(permissions);
  const preset = presetFor(normalized);
  if (preset !== "custom") {
    return { tone: preset, text: I18n.label(PRESET_LABELS, preset) };
  }
  const names = (level, type) =>
    formatList(
      CATEGORIES.filter(({ key }) => normalized[key] === level).map(({ key }) =>
        I18n.label(CATEGORY_INLINE_LABELS, key)
      ),
      { type }
    );
  // "asks for A and B; no C or D".
  const asking = names("ask_user", "conjunction");
  const off = names("deny", "disjunction");
  let text;
  if (asking && off) text = I18n.t("settings-agent-summary-asks-and-off", { asking, off });
  else if (asking) text = I18n.t("settings-agent-summary-asks-only", { asking });
  else text = I18n.t("settings-agent-summary-off-only", { off });
  return { tone: "custom", text };
}

/**
 * Client kinds offered for setup guidance. The `id` doubles as the pairing's
 * `agent_kind` slug (all are valid `[a-z0-9_-]`).
 */
export const SETUP_CLIENTS = [
  { id: "claude" },
  { id: "codex" },
  { id: "openclaw" },
  { id: "hermes" },
  { id: "generic" },
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
    shared.push(I18n.t("settings-agent-note-placeholder"));
  }
  shared.push(I18n.t("settings-agent-note-data-dir"));

  switch (id) {
    case "codex":
      return {
        notes: [
          I18n.t("settings-agent-note-codex-run"),
          I18n.t("settings-agent-note-codex-get"),
          ...shared,
        ],
        blocks: [
          {
            label: I18n.t("settings-agent-block-codex-command"),
            lang: "sh",
            body: codexCommand(exe, clientId, secret),
          },
          {
            label: I18n.t("settings-agent-block-codex-toml"),
            lang: "toml",
            body: codexToml(exe, clientId, secret),
          },
        ],
      };
    case "claude":
      return {
        notes: [
          I18n.t("settings-agent-note-claude-code"),
          I18n.t("settings-agent-note-claude-desktop"),
          ...shared,
        ],
        blocks: [
          {
            label: I18n.t("settings-agent-block-claude-command"),
            lang: "sh",
            body: claudeCodeCommand(exe, clientId, secret),
          },
          {
            label: I18n.t("settings-agent-block-claude-desktop"),
            lang: "json",
            body: genericStdioJson(exe, clientId, secret),
          },
        ],
      };
    case "openclaw":
      return {
        notes: [I18n.t("settings-agent-note-openclaw"), ...shared],
        blocks: [
          {
            label: I18n.t("settings-agent-block-openclaw"),
            lang: "sh",
            body: openClawCommand(exe, clientId, secret),
          },
        ],
      };
    case "hermes":
      return {
        notes: [I18n.t("settings-agent-note-hermes"), ...shared],
        blocks: [
          {
            label: I18n.t("settings-agent-block-hermes"),
            lang: "yaml",
            body: hermesYaml(exe, clientId, secret),
          },
        ],
      };
    case "generic":
    default:
      return {
        notes: [I18n.t("settings-agent-note-generic"), ...shared],
        blocks: [
          {
            label: I18n.t("settings-agent-block-generic"),
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
  if (!trimmed) return { ok: false, error: I18n.t("settings-agent-name-required") };
  if ([...trimmed].length > MAX_LABEL) {
    return { ok: false, error: I18n.t("settings-agent-name-too-long", { max: MAX_LABEL }) };
  }
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f]/.test(trimmed)) {
    return { ok: false, error: I18n.t("settings-agent-name-control-characters") };
  }
  return { ok: true, value: trimmed };
}

// ── DOM (browser only) ──────────────────────────────────────────────────────

/** Set by the loader before any builder runs; keeps the pure helpers node-safe. */
let Utils = null;

/**
 * One category's three-way choice, as a segmented track. A fixed choice set of
 * three gets a segmented control, not three separate buttons or a dropdown.
 * `name` scopes the radios so the create form and each row editor stay
 * independent when several are on screen.
 */
function permissionRow(category, level, name) {
  const options = LEVELS.map(
    (option) => `
      <label class="agent-perm-choice" title="${Utils.escapeHtml(
        I18n.label(LEVEL_HINT_LABELS, option.value)
      )}">
        <input type="radio" name="${Utils.escapeHtml(name)}-${category.key}"
               value="${option.value}" data-perm-key="${category.key}"${
                 option.value === level ? " checked" : ""
               }>
        <span>${Utils.escapeHtml(I18n.label(LEVEL_LABELS, option.value))}</span>
      </label>`
  ).join("");
  return `
    <div class="agent-perm-row">
      <div class="agent-perm-info">
        <span class="agent-perm-label">${Utils.escapeHtml(
          I18n.label(CATEGORY_LABELS, category.key)
        )}</span>
        <span class="agent-perm-desc">${Utils.escapeHtml(
          I18n.label(CATEGORY_DESCRIPTION_LABELS, category.key)
        )}</span>
      </div>
      <div class="agent-perm-choices" role="radiogroup"
           aria-label="${Utils.escapeHtml(
             I18n.t("settings-agent-permission-group", {
               category: I18n.label(CATEGORY_LABELS, category.key),
             })
           )}">${options}</div>
    </div>`;
}

/** The preset track plus the five category rows, for one `name` namespace. */
function permissionGrid(permissions, name) {
  const normalized = normalizePermissions(permissions);
  const active = presetFor(normalized);
  const presets = PRESETS.map(
    (preset) => `
      <button type="button" class="agent-perm-preset${
        preset.id === active ? " active" : ""
      }" data-preset="${preset.id}" title="${Utils.escapeHtml(
        I18n.label(PRESET_DESCRIPTION_LABELS, preset.id)
      )}">${Utils.escapeHtml(I18n.label(PRESET_LABELS, preset.id))}</button>`
  ).join("");
  return `
    <div class="agent-perm-grid" data-perm-grid="${Utils.escapeHtml(name)}">
      <div class="agent-perm-presets" role="group" data-l10n-id="settings-agent-preset-group">
        ${presets}
        <span class="agent-perm-custom-note"${
          active === "custom" ? "" : " hidden"
        } data-l10n-id="settings-agent-preset-custom"></span>
      </div>
      ${CATEGORIES.map((category) => permissionRow(category, normalized[category.key], name)).join(
        ""
      )}
    </div>`;
}

/** Read a grid's current selection back out of the DOM. */
function readPermissionGrid(grid) {
  const permissions = {};
  for (const { key } of CATEGORIES) {
    const checked = grid.querySelector(`input[data-perm-key="${key}"]:checked`);
    if (checked) permissions[key] = checked.value;
  }
  return normalizePermissions(permissions);
}

/** Write a permission map into a grid and re-mark the matching preset. */
function writePermissionGrid(grid, permissions) {
  const normalized = normalizePermissions(permissions);
  for (const { key } of CATEGORIES) {
    const input = grid.querySelector(`input[data-perm-key="${key}"][value="${normalized[key]}"]`);
    if (input) input.checked = true;
  }
  syncPresetState(grid);
}

/** Keep the preset track in step with whatever the category rows now say. */
function syncPresetState(grid) {
  const active = presetFor(readPermissionGrid(grid));
  grid.querySelectorAll("[data-preset]").forEach((button) => {
    button.classList.toggle("active", button.dataset.preset === active);
  });
  const note = grid.querySelector(".agent-perm-custom-note");
  if (note) note.hidden = active !== "custom";
}

function clientOptions(selected) {
  return SETUP_CLIENTS.map(
    (c) =>
      `<option value="${c.id}"${c.id === selected ? " selected" : ""}>${Utils.escapeHtml(
        I18n.label(SETUP_CLIENT_LABELS, c.id)
      )}</option>`
  ).join("");
}

function clientLabel(kind) {
  return Object.hasOwn(SETUP_CLIENT_LABELS, kind) ? I18n.label(SETUP_CLIENT_LABELS, kind) : kind;
}

function buildShell() {
  return `
    <div class="settings-section agent-connections">
      <h3 class="settings-section-title">
        <i class="icon-plug"></i>
        <span data-l10n-id="settings-agent-title"></span>
      </h3>
      <p class="settings-section-description" data-l10n-id="settings-agent-description"></p>

      <div class="settings-group agent-pair-create">
        <div class="settings-field">
          <div class="settings-field-info">
            <label for="agentPairLabel" data-l10n-id="settings-agent-name-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-agent-name-hint"></span>
          </div>
          <div class="settings-field-control">
            <input type="text" id="agentPairLabel" class="settings-input" maxlength="${MAX_LABEL}"
                   data-l10n-id="settings-agent-name-input" autocomplete="off" spellcheck="false">
          </div>
        </div>

        <div class="settings-field">
          <div class="settings-field-info">
            <label for="agentPairClient" data-l10n-id="settings-agent-client-label"></label>
            <span class="settings-field-hint" data-l10n-id="settings-agent-client-hint"></span>
          </div>
          <div class="settings-field-control">
            <select id="agentPairClient" class="settings-select" data-custom-select>
              ${clientOptions(DEFAULT_CLIENT)}
            </select>
          </div>
        </div>

        <div class="settings-field agent-perm-field">
          <div class="settings-field-info">
            <span class="settings-field-label" data-l10n-id="settings-agent-permissions-label"></span>
            <span class="settings-field-hint" data-l10n-id="settings-agent-permissions-hint"></span>
          </div>
          <div class="settings-field-control">
            ${permissionGrid(defaultPermissions(), "create")}
          </div>
        </div>

        <div class="form-group agent-pair-actions">
          <p class="form-error" id="agentPairError" hidden></p>
          <button type="button" class="btn btn-primary btn-sm" id="agentPairCreate">
            <i class="icon-plus"></i>
            <span data-l10n-id="settings-agent-create"></span>
          </button>
        </div>
      </div>

      <div class="agent-issued" id="agentIssued" role="group"
           data-l10n-id="settings-agent-issued-group" hidden>
        <div class="agent-issued-warn">
          <i class="icon-triangle-alert"></i>
          <span data-l10n-id="settings-agent-issued-warning"></span>
        </div>
        <div class="agent-issued-fields">
          <div class="agent-issued-row">
            <span class="agent-issued-key" data-l10n-id="settings-agent-issued-client-id"></span>
            <code class="agent-issued-value" id="agentIssuedClientId" dir="ltr"></code>
            <button type="button" class="btn btn-secondary btn-sm" data-copy-issued="client-id" data-l10n-id="common-action-copy"></button>
          </div>
          <div class="agent-issued-row">
            <span class="agent-issued-key" data-l10n-id="settings-agent-issued-secret"></span>
            <code class="agent-issued-value agent-issued-secret" id="agentIssuedSecret" dir="ltr"></code>
            <button type="button" class="btn btn-secondary btn-sm" data-copy-issued="secret" data-l10n-id="common-action-copy"></button>
          </div>
        </div>

        <div class="agent-setup">
          <div class="agent-setup-head">
            <label for="agentSetupClient" data-l10n-id="settings-agent-setup-for"></label>
            <select id="agentSetupClient" class="settings-select" data-custom-select>
              ${clientOptions(DEFAULT_CLIENT)}
            </select>
          </div>
          <ul class="agent-setup-notes" id="agentSetupNotes"></ul>
          <div class="agent-setup-blocks" id="agentSetupBlocks"></div>
        </div>

        <button type="button" class="btn btn-secondary btn-sm" id="agentIssuedDone" data-l10n-id="settings-agent-done"></button>
      </div>

      <div class="agent-pair-section">
        <div class="agent-pair-list-head">
          <h4 data-l10n-id="settings-agent-list-title"></h4>
          <span id="agentPairCount"></span>
        </div>
        <div class="agent-pair-list" id="agentPairList">
          <div class="settings-loading"><i class="icon-loader spin"></i> <span data-l10n-id="settings-agent-loading"></span></div>
        </div>
      </div>
    </div>
  `;
}

function permissionBadge(permissions) {
  const summary = summarizePermissions(permissions);
  return `<span class="agent-perm-badge agent-perm-badge--${Utils.escapeHtml(
    summary.tone
  )}">${Utils.escapeHtml(summary.text)}</span>`;
}

function renderList(container, rows) {
  const countEl = container.closest(".agent-pair-section")?.querySelector("#agentPairCount");
  if (!Array.isArray(rows) || rows.length === 0) {
    if (countEl) countEl.textContent = I18n.t("settings-agent-active-count", { count: 0 });
    container.innerHTML = '<div class="settings-empty" data-l10n-id="settings-agent-empty"></div>';
    I18n.localizeTree(container);
    return;
  }
  const active = rows.filter((r) => !r.revoked);
  const revoked = rows.filter((r) => r.revoked);
  if (countEl)
    countEl.textContent = I18n.t("settings-agent-active-count", { count: active.length });
  const rowHtml = (r) => `
    <div class="agent-pair-row${r.revoked ? " agent-pair-row--revoked" : ""}" role="listitem">
      <div class="agent-pair-main">
        <span class="agent-pair-label">${Utils.escapeHtml(r.label)}</span>
        <span class="agent-pair-meta">
          ${Utils.escapeHtml(clientLabel(r.agent_kind))} · ${permissionBadge(r.permissions)}
        </span>
      </div>
      <div class="agent-pair-times">
        <span>${Utils.escapeHtml(
          I18n.t("settings-agent-created", { time: Utils.formatTimeAgo(r.created_at) })
        )}</span>
        <span>${Utils.escapeHtml(
          r.last_used_at
            ? I18n.t("settings-agent-last-used", { time: Utils.formatTimeAgo(r.last_used_at) })
            : I18n.t("settings-agent-never-used")
        )}</span>
      </div>
      ${
        r.revoked
          ? ""
          : `<div class="agent-pair-action">
              <button type="button" class="btn btn-secondary btn-sm" data-edit-perms="${Utils.escapeHtml(
                r.client_id
              )}" aria-expanded="false" data-l10n-id="settings-agent-permissions-edit"></button>
              <button type="button" class="btn btn-danger btn-sm" data-revoke="${Utils.escapeHtml(
                r.client_id
              )}" data-label="${Utils.escapeHtml(r.label)}" data-l10n-id="settings-agent-revoke"></button>
            </div>`
      }
      ${
        r.revoked
          ? ""
          : `<div class="agent-perm-editor" data-perm-editor="${Utils.escapeHtml(
              r.client_id
            )}" hidden>
              ${permissionGrid(r.permissions, `row-${r.client_id}`)}
              <div class="agent-perm-editor-actions">
                <button type="button" class="btn btn-secondary btn-sm" data-perm-cancel="${Utils.escapeHtml(
                  r.client_id
                )}" data-l10n-id="common-action-cancel"></button>
                <button type="button" class="btn btn-primary btn-sm" data-perm-save="${Utils.escapeHtml(
                  r.client_id
                )}" data-l10n-id="settings-agent-permissions-save"></button>
              </div>
            </div>`
      }
    </div>`;
  container.innerHTML =
    (active.length
      ? `<div class="agent-pair-active" role="list">${active.map(rowHtml).join("")}</div>`
      : '<div class="settings-empty" data-l10n-id="settings-agent-empty-active"></div>') +
    (revoked.length
      ? `<details class="agent-pair-revoked-group">
          <summary><i class="icon-chevron-right"></i> <span data-l10n-id="settings-agent-revoked-title"></span> <span>${revoked.length}</span></summary>
          <div role="list">${revoked.map(rowHtml).join("")}</div>
        </details>`
      : "");
  I18n.localizeTree(container);
}

/**
 * Load and wire the Agent Connections tab. Follows the security/telegram loader
 * pattern: this owns `content.innerHTML`.
 */
export async function loadAgentConnectionsTab(_dialog, content) {
  teardownAgentConnectionsTab();
  const generation = loadGeneration;
  content.innerHTML =
    '<div class="settings-loading"><i class="icon-loader spin"></i> <span data-l10n-id="settings-agent-loading"></span></div>';
  I18n.localizeTree(content);

  let ConfirmationDialog;
  try {
    [Utils, { ConfirmationDialog }] = await Promise.all([
      import("../../core/utils.js"),
      import("../confirmation_dialog.js"),
    ]);
  } catch {
    if (generation !== loadGeneration) return;
    content.innerHTML =
      '<div class="settings-error" data-l10n-id="settings-agent-load-failed"></div>';
    I18n.localizeTree(content);
    return;
  }

  if (generation !== loadGeneration) return;

  // One-time credential, held only while the issued panel is on screen.
  let issued = null; // { clientId, secret, exe }
  // The blocks currently rendered, so `data-copy-block` can resolve by index.
  let currentBlocks = [];

  content.innerHTML = buildShell();
  I18n.localizeTree(content);

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
            <button type="button" class="btn btn-secondary btn-sm" data-copy-block="${i}" data-l10n-id="common-action-copy"></button>
          </div>
          <pre class="agent-setup-body" dir="ltr"><code>${Utils.escapeHtml(b.body)}</code></pre>
        </div>`
      )
      .join("");
    I18n.localizeTree(setupBlocksEl);
  }

  async function refreshList() {
    try {
      const res = await fetch(LIST_URL, {
        headers: { Accept: "application/json" },
        signal: controller.signal,
      });
      if (!res.ok) {
        listEl.innerHTML =
          '<div class="settings-error" data-l10n-id="settings-agent-list-failed"></div>';
        I18n.localizeTree(listEl);
        return;
      }
      renderList(listEl, await res.json());
    } catch {
      if (controller.signal.aborted) return;
      listEl.innerHTML =
        '<div class="settings-error" data-l10n-id="settings-agent-list-failed"></div>';
      I18n.localizeTree(listEl);
    }
  }

  async function createPairing() {
    showError("");
    const label = validateLabel(labelEl.value);
    if (!label.ok) {
      showError(label.error);
      return;
    }
    const createGrid = content.querySelector('[data-perm-grid="create"]');
    const permissions = createGrid ? readPermissionGrid(createGrid) : defaultPermissions();
    const agentKind = clientEl.value;

    createBtn.disabled = true;
    try {
      const res = await fetch(LIST_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ label: label.value, agent_kind: agentKind, permissions }),
        signal: controller.signal,
      });
      const body = await res.json().catch(() => null);
      if (controller.signal.aborted) return;
      if (!res.ok) {
        showError(apiErrorMessage(body, I18n.t("settings-agent-create-failed")));
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
      if (createGrid) writePermissionGrid(createGrid, defaultPermissions());
      await refreshList();
    } catch {
      if (controller.signal.aborted) return;
      showError(I18n.t("settings-agent-unreachable-create"));
    } finally {
      createBtn.disabled = false;
    }
  }

  /** Show or hide one connection's permission editor. */
  function togglePermissionEditor(clientId, open) {
    const editor = listEl.querySelector(`[data-perm-editor="${CSS.escape(clientId)}"]`);
    const button = listEl.querySelector(`[data-edit-perms="${CSS.escape(clientId)}"]`);
    if (!editor) return;
    const next = open ?? editor.hidden;
    editor.hidden = !next;
    button?.setAttribute("aria-expanded", String(next));
  }

  async function savePermissions(clientId) {
    const editor = listEl.querySelector(`[data-perm-editor="${CSS.escape(clientId)}"]`);
    const grid = editor?.querySelector("[data-perm-grid]");
    if (!grid) return;
    const permissions = readPermissionGrid(grid);
    const saveBtn = editor.querySelector("[data-perm-save]");
    if (saveBtn) saveBtn.disabled = true;
    try {
      const res = await fetch(`${pairingUrl(clientId)}/permissions`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ permissions }),
        signal: controller.signal,
      });
      if (controller.signal.aborted) return;
      if (!res.ok) {
        Utils.showToast({
          type: "error",
          title: I18n.t("settings-agent-permissions-update-failed"),
        });
        return;
      }
      Utils.showToast({
        type: "success",
        title: I18n.t("settings-agent-permissions-updated"),
        message: I18n.t("settings-agent-permissions-updated-detail"),
      });
      await refreshList();
    } catch {
      if (controller.signal.aborted) return;
      Utils.showToast({ type: "error", title: I18n.t("settings-agent-unreachable-save") });
    } finally {
      if (saveBtn) saveBtn.disabled = false;
    }
  }

  async function revokePairing(clientId, label) {
    const { confirmed } = await ConfirmationDialog.show({
      title: I18n.t("settings-agent-revoke-title"),
      message: I18n.t("settings-agent-revoke-message", { label }),
      confirmLabel: I18n.t("settings-agent-revoke"),
      cancelLabel: I18n.t("common-action-cancel"),
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
        Utils.showToast({ type: "error", title: I18n.t("settings-agent-revoke-failed") });
        return;
      }
      await refreshList();
    } catch {
      if (controller.signal.aborted) return;
      Utils.showToast({ type: "error", title: I18n.t("settings-agent-unreachable-revoke") });
    }
  }

  function copyIssuedValue(what) {
    if (!issued) return;
    const value = what === "secret" ? issued.secret : issued.clientId;
    const labelText =
      what === "secret"
        ? I18n.t("settings-agent-issued-secret")
        : I18n.t("settings-agent-issued-client-id");
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
      const presetBtn = e.target.closest("[data-preset]");
      if (presetBtn) {
        const grid = presetBtn.closest("[data-perm-grid]");
        const preset = PRESETS.find((p) => p.id === presetBtn.dataset.preset);
        if (grid && preset) writePermissionGrid(grid, preset.permissions());
        return;
      }
      const editBtn = e.target.closest("[data-edit-perms]");
      if (editBtn) {
        togglePermissionEditor(editBtn.dataset.editPerms);
        return;
      }
      const cancelBtn = e.target.closest("[data-perm-cancel]");
      if (cancelBtn) {
        togglePermissionEditor(cancelBtn.dataset.permCancel, false);
        return;
      }
      const saveBtn = e.target.closest("[data-perm-save]");
      if (saveBtn) {
        savePermissions(saveBtn.dataset.permSave);
        return;
      }
      const revokeBtn = e.target.closest("[data-revoke]");
      if (revokeBtn) {
        revokePairing(
          revokeBtn.dataset.revoke,
          revokeBtn.dataset.label || I18n.t("settings-agent-revoke-fallback-name")
        );
      }
    },
    listenerOptions
  );

  // Setting one category by hand is what moves a grid to "Custom".
  content.addEventListener(
    "change",
    (e) => {
      const input = e.target.closest("input[data-perm-key]");
      if (!input) return;
      const grid = input.closest("[data-perm-grid]");
      if (grid) syncPresetState(grid);
    },
    listenerOptions
  );

  activeTabCleanup = () => {
    controller.abort();
    clearIssued();
  };

  await refreshList();
}
