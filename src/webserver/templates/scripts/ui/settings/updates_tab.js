/**
 * Updates Tab — the state of this installation and what it is about to become.
 *
 * The tab owns its whole loop: it reads /api/updates/status, renders from that
 * one source, polls only while something is actually moving, and writes
 * preferences straight to /api/config/updates. Nothing about the update state is
 * mirrored anywhere else, so the panel can never disagree with the backend.
 *
 * Deliberately flat: rows of text and one accent, no cards, no tiles. The
 * information is a status and a short list of switches; framing each line in a
 * box would only make it harder to read down.
 */
import * as Utils from "../../core/utils.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";
import { Poller } from "../../core/poller.js";

/** Phases where something is in flight and the view has to keep up. */
const BUSY_PHASES = new Set(["checking", "downloading", "verifying", "applying"]);

let poller = null;
let versionInfo = { version: "0.0.0", platform: "", shell_revision: null, core_staged: false };

/**
 * Initial markup. Content is filled in by the first status read, so the tab
 * never renders a guess it then has to correct.
 */
export function buildUpdatesTab(dialog, info) {
  versionInfo = info || versionInfo;
  return `
    <div class="updates" id="updatesRoot">
      <div class="updates-status" id="updatesStatus">
        <div class="updates-headline">Checking this installation...</div>
      </div>
    </div>
  `;
}

/** Wire the tab up and start its loop. */
export function attachUpdatesHandlers(dialog, content, onBadgeChange) {
  const root = content.querySelector("#updatesRoot");
  if (!root) return;

  const refresh = async ({ recheck = false } = {}) => {
    if (recheck) {
      renderBusy(root, "Checking for updates...");
      await request("/api/updates/check");
    }
    const state = await readStatus();
    if (!state) return;
    render(root, state, refresh);
    if (onBadgeChange) onBadgeChange(Boolean(state.available_update));
    syncPoller(state, refresh);
  };

  refresh();
}

/** Stop polling when the dialog closes. */
export function teardownUpdatesTab() {
  if (poller) {
    poller.cleanup();
    poller = null;
  }
}

// ===========================================================================
// Data
// ===========================================================================

async function readStatus() {
  try {
    const response = await fetch("/api/updates/status");
    const body = await response.json();
    if (!response.ok || body.success === false) return null;
    const payload = body.data || body;
    const state = payload.state || payload;
    state.staged_core = payload.staged_core || null;
    state.blocked_reason = payload.blocked_reason || null;
    return state;
  } catch {
    return null;
  }
}

async function request(url, options) {
  try {
    const response = await fetch(url, options);
    const body = await response.json().catch(() => ({}));
    if (!response.ok || body.success === false) {
      throw new Error(body.error?.message || body.error || "Request failed");
    }
    return body;
  } catch (err) {
    Utils.showToast({ type: "error", title: "Update", message: err.message });
    return null;
  }
}

async function readPreferences() {
  try {
    const response = await fetch("/api/config/updates");
    if (!response.ok) return null;
    const body = await response.json();
    return body.data?.data || body.data || null;
  } catch {
    return null;
  }
}

function syncPoller(state, refresh) {
  const busy = BUSY_PHASES.has(state.phase);
  if (busy && !poller) {
    poller = new Poller(() => refresh(), {
      label: "UpdateStatus",
      intervalMs: 1000,
      pauseWhenHidden: false,
    });
    poller.start();
  } else if (!busy && poller) {
    poller.cleanup();
    poller = null;
  }
}

// ===========================================================================
// Rendering
// ===========================================================================

function renderBusy(root, message) {
  root.innerHTML = `
    <div class="updates-status">
      <div class="updates-headline">${Utils.escapeHtml(message)}</div>
    </div>
  `;
}

function render(root, state, refresh) {
  const update = state.available_update;
  root.innerHTML = `
    ${renderStatus(state, update)}
    ${update?.release_notes ? renderNotes(update.release_notes) : ""}
    <div class="updates-preferences" id="updatesPreferences"></div>
    ${renderDetails(state, update)}
  `;
  attachActions(root, state, refresh);
  renderPreferences(root.querySelector("#updatesPreferences"));
}

function renderStatus(state, update) {
  const current = versionInfo.version;
  const progress = state.download_progress || {};

  // headline / detail / actions per phase — one place, so the panel can only
  // ever describe a state the backend actually reported.
  let headline = "You are up to date";
  let detail = `ScreenerBot v${current} is the latest version.`;
  let actions = [button("updatesCheck", "Check again", "icon-refresh-cw", "ghost")];
  let bar = "";

  switch (state.phase) {
    case "checking":
      headline = "Checking for updates";
      detail = "Asking screenerbot.io for the latest release.";
      actions = [];
      break;

    case "available":
      headline = `Version ${update.version} is available`;
      detail = describeKind(update);
      actions = [button("updatesDownload", "Download now", "icon-arrow-down-to-line", "primary")];
      break;

    case "downloading":
    case "verifying":
      headline = `Downloading v${update?.version || ""}`;
      detail =
        state.phase === "verifying"
          ? "Verifying the download against its published checksum."
          : transferLine(progress);
      actions = [];
      bar = progressBar(progress.progress_percent || 0, state.phase === "verifying");
      break;

    case "ready_to_apply":
      headline = `Version ${update.version} is ready`;
      detail =
        state.blocked_reason || "Installing takes a few seconds and needs no further confirmation.";
      actions = [
        button("updatesApply", "Install and restart", "icon-refresh-cw", "primary"),
        button("updatesLater", "Install on next start", "", "ghost"),
      ];
      break;

    case "ready_to_install":
      headline = `Version ${update.version} is downloaded`;
      detail =
        state.blocked_reason ||
        "This release also replaces the desktop app, so its installer has to run once.";
      actions = [button("updatesInstall", "Open installer and quit", "icon-package", "primary")];
      break;

    case "applying":
      headline = "Installing";
      detail = "ScreenerBot is restarting onto the new version.";
      actions = [];
      bar = progressBar(100, true);
      break;

    case "applied":
      headline = `Updated to v${current}`;
      detail = "The update was installed silently. Nothing else is needed.";
      break;

    case "failed":
      headline = "The update did not finish";
      detail = progress.error || "Try again, or download the installer from screenerbot.io.";
      actions = [button("updatesRetry", "Try again", "icon-refresh-cw", "primary")];
      break;

    case "check_failed":
      headline = "Could not check for updates";
      detail = state.check_error || "screenerbot.io could not be reached.";
      actions = [button("updatesCheck", "Try again", "icon-refresh-cw", "primary")];
      break;
  }

  const transition =
    update && state.phase !== "applied"
      ? `<div class="updates-versions">
           <span class="updates-version-from">v${Utils.escapeHtml(current)}</span>
           <i class="icon-arrow-right"></i>
           <span class="updates-version-to">v${Utils.escapeHtml(update.version)}</span>
           ${update.kind === "core" ? '<span class="updates-tag">silent</span>' : ""}
         </div>`
      : `<div class="updates-versions"><span class="updates-version-from">v${Utils.escapeHtml(current)}</span></div>`;

  return `
    <div class="updates-status">
      <div class="updates-headline">${Utils.escapeHtml(headline)}</div>
      <div class="updates-detail">${Utils.escapeHtml(detail)}</div>
      ${transition}
      ${bar}
      ${actions.length ? `<div class="updates-actions">${actions.join("")}</div>` : ""}
    </div>
  `;
}

function describeKind(update) {
  const size = formatBytes(update.kind === "core" ? update.core?.size : update.file_size);
  return update.kind === "core"
    ? `Only the trading core changed, so this is a ${size} download that installs with a short restart.`
    : `This release also rebuilds the desktop app, so the full ${size} installer is needed.`;
}

function transferLine(progress) {
  const done = formatBytes(progress.bytes_downloaded || 0);
  const total = formatBytes(progress.total_bytes || 0);
  return `${done} of ${total}`;
}

function progressBar(percent, indeterminate) {
  // No inline width while indeterminate: the sweep animation owns the element,
  // and a stray width would fight the stylesheet for it.
  const width = Math.max(0, Math.min(100, Math.round(percent)));
  return `
    <div class="updates-progress${indeterminate ? " is-indeterminate" : ""}">
      <div class="updates-progress-fill"${indeterminate ? "" : ` style="width:${width}%"`}></div>
    </div>
  `;
}

function button(id, label, icon, variant) {
  return `
    <button class="updates-btn ${variant}" id="${id}">
      ${icon ? `<i class="${icon}"></i>` : ""}<span>${Utils.escapeHtml(label)}</span>
    </button>
  `;
}

function renderNotes(notes) {
  return `
    <section class="updates-notes">
      <h3 class="updates-subhead"><i class="icon-file-text"></i>What changed</h3>
      <div class="updates-notes-body">${Utils.escapeHtml(notes)}</div>
    </section>
  `;
}

function renderDetails(state, update) {
  const rows = [
    ["Platform", versionInfo.platform || "Unknown"],
    [
      "Core version",
      `v${versionInfo.version}${versionInfo.core_staged ? " (updated in place)" : ""}`,
    ],
    [
      "Desktop shell",
      versionInfo.shell_revision ? `revision ${versionInfo.shell_revision}` : "not reported",
    ],
    ["Last checked", formatTime(state.last_check || state.last_check_attempt)],
  ];
  if (update) {
    rows.push([
      "Update type",
      update.kind === "core" ? "Core only — no installer" : "Full — installer required",
    ]);
    rows.push([
      "Download size",
      formatBytes(update.kind === "core" ? update.core?.size : update.file_size),
    ]);
  }
  if (state.staged_core) {
    rows.push([
      "Staged",
      `v${state.staged_core.version}, verified ${formatTime(state.staged_core.staged_at)}`,
    ]);
  }

  return `
    <section class="updates-details">
      <h3 class="updates-subhead"><i class="icon-info"></i>Installation</h3>
      <dl class="updates-detail-list">
        ${rows
          .map(
            ([label, value]) => `
          <div class="updates-detail-row">
            <dt>${Utils.escapeHtml(label)}</dt>
            <dd>${Utils.escapeHtml(String(value))}</dd>
          </div>`
          )
          .join("")}
      </dl>
    </section>
  `;
}

async function renderPreferences(host) {
  if (!host) return;
  const prefs = await readPreferences();
  if (!prefs) {
    host.innerHTML = "";
    return;
  }

  const fields = [
    ["auto_check", "Check for updates", "Ask screenerbot.io for a newer release in the background"],
    [
      "auto_download",
      "Download automatically",
      "Fetch and verify a new release as soon as it is published",
    ],
    [
      "auto_install",
      "Install automatically",
      "Apply a core update on its own with a short restart. Off means ScreenerBot waits for you.",
    ],
    [
      "defer_while_trading",
      "Wait while positions are open",
      "Postpone the restart until nothing is at risk; the update still applies on the next start",
    ],
    [
      "notify_telegram",
      "Announce on Telegram",
      "Message the configured chat when an update is found or applied",
    ],
  ];

  host.innerHTML = `
    <section class="updates-prefs">
      <h3 class="updates-subhead"><i class="icon-sliders-horizontal"></i>Automatic updates</h3>
      <div class="settings-group">
        ${fields
          .map(
            ([key, label, hint]) => `
          <div class="settings-field">
            <div class="settings-field-info">
              <label for="updatePref_${key}">${label}</label>
              <span class="settings-field-hint">${hint}</span>
            </div>
            <div class="settings-field-control">
              <label class="toggle">
                <input type="checkbox" id="updatePref_${key}" data-pref="${key}" ${prefs[key] ? "checked" : ""}>
                <span class="toggle-track"></span>
              </label>
            </div>
          </div>`
          )
          .join("")}
      </div>
    </section>
  `;

  host.querySelectorAll("input[data-pref]").forEach((input) => {
    input.addEventListener("change", async () => {
      const key = input.dataset.pref;
      const body = { [key]: input.checked };
      const result = await request("/api/config/updates", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (!result) input.checked = !input.checked;
    });
  });
}

// ===========================================================================
// Actions
// ===========================================================================

function attachActions(root, state, refresh) {
  const on = (id, handler) => {
    const element = root.querySelector(`#${id}`);
    if (element) element.addEventListener("click", handler);
  };

  on("updatesCheck", () => refresh({ recheck: true }));
  on("updatesRetry", () => refresh({ recheck: true }));

  const startDownload = async () => {
    const update = state.available_update;
    if (!update) return;
    renderBusy(root, `Downloading v${update.version}...`);
    await request("/api/updates/download", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ version: update.version }),
    });
    refresh();
  };
  on("updatesDownload", startDownload);

  on("updatesApply", async () => {
    const confirmation = await ConfirmationDialog.show({
      title: `Install v${state.available_update?.version}`,
      message:
        "ScreenerBot restarts onto the new version. Trading stops for a few seconds and resumes automatically; open positions are untouched.",
      confirmLabel: "Install and restart",
      cancelLabel: "Cancel",
      variant: "warning",
    });
    if (!confirmation.confirmed) return;
    renderBusy(root, "Installing...");
    await request("/api/updates/apply", { method: "POST" });
    refresh();
  });

  on("updatesLater", () => {
    Utils.showToast({
      type: "info",
      title: "Update scheduled",
      message: "The update installs the next time ScreenerBot starts.",
    });
  });

  on("updatesInstall", async () => {
    const confirmation = await ConfirmationDialog.show({
      title: "Run the installer",
      message:
        "The verified installer opens and ScreenerBot quits cleanly. Complete the installer, then reopen ScreenerBot.",
      confirmLabel: "Open installer",
      cancelLabel: "Cancel",
      variant: "warning",
    });
    if (!confirmation.confirmed) return;

    const result = await request("/api/updates/install", { method: "POST" });
    if (!result) return;
    Utils.showToast({
      type: "success",
      title: "Installer opened",
      message: "ScreenerBot will quit cleanly now.",
    });
    // Electron owns the backend child and must coordinate the quit; a raw
    // backend exit reads as a crash to the shell.
    setTimeout(() => window.electronAPI?.quitForUpdate?.(), 1000);
  });
}

// ===========================================================================
// Formatting
// ===========================================================================

function formatBytes(bytes) {
  if (!bytes) return "unknown size";
  const units = ["B", "KB", "MB", "GB"];
  const index = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
  return `${parseFloat((bytes / Math.pow(1024, index)).toFixed(1))} ${units[index]}`;
}

function formatTime(value) {
  if (!value) return "never";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "never";
  return date.toLocaleString();
}
