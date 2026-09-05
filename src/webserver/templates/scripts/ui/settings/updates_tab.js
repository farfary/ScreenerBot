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
let activeSession = null;
let versionInfo = { version: "0.0.0", platform: "" };

/**
 * Initial markup. Content is filled in by the first status read, so the tab
 * never renders a guess it then has to correct.
 */
export function buildUpdatesTab(info) {
  versionInfo = info || versionInfo;
  return `
    <div class="updates" id="updatesRoot">
      <div class="updates-status" id="updatesStatus" role="status" aria-live="polite" aria-atomic="true">
        <div class="updates-headline">Checking this installation...</div>
      </div>
      <div class="updates-section-host" id="updatesNotes"></div>
      <div class="updates-preferences updates-section-host" id="updatesPreferences"></div>
      <div class="updates-section-host" id="updatesDetails"></div>
    </div>
  `;
}

/** Wire the tab up and start its loop. */
export function attachUpdatesHandlers(content, onBadgeChange) {
  teardownUpdatesTab();

  const root = content.querySelector("#updatesRoot");
  if (!root) return;

  const session = {
    controller: new AbortController(),
    root,
    statusHtml: null,
    notesHtml: null,
    detailsHtml: null,
    refreshing: false,
  };
  activeSession = session;

  const refresh = async ({ recheck = false } = {}) => {
    if (session.refreshing) return;
    session.refreshing = true;
    try {
      if (recheck) {
        renderBusy(root, "Checking for updates...");
        await request(
          "/api/updates/check",
          { signal: session.controller.signal },
          "Could not check for updates"
        );
        if (activeSession !== session) return;
      }
      const state = await readStatus(session.controller.signal);
      if (activeSession !== session) return;
      if (!state) {
        renderLoadError(root, refresh);
        return;
      }
      render(session, state, refresh);
      if (onBadgeChange) onBadgeChange(Boolean(state.available_update));
      syncPoller(state, refresh, session);
    } finally {
      session.refreshing = false;
    }
  };

  void renderPreferences(root.querySelector("#updatesPreferences"), session);
  void refresh();
}

/** Stop polling and invalidate requests when the tab deactivates. */
export function teardownUpdatesTab() {
  if (activeSession) {
    activeSession.controller.abort();
    activeSession = null;
  }
  if (poller) {
    poller.cleanup();
    poller = null;
  }
}

// ===========================================================================
// Data
// ===========================================================================

async function readStatus(signal) {
  try {
    const response = await fetch("/api/updates/status", { signal });
    const body = await response.json();
    if (!response.ok || body.success === false) return null;
    const payload = body.data || body;
    const state = payload.state || payload;
    state.blocked_reason = payload.blocked_reason || null;
    return state;
  } catch {
    return null;
  }
}

async function request(url, options = {}, errorTitle = "Update action failed") {
  try {
    const response = await fetch(url, options);
    const body = await response.json().catch(() => ({}));
    if (!response.ok || body.success === false) {
      throw new Error(body.error?.message || body.error || "Request failed");
    }
    return body;
  } catch (err) {
    if (err.name === "AbortError") return null;
    Utils.showToast({ type: "error", title: errorTitle, message: err.message });
    return null;
  }
}

async function readPreferences(signal) {
  try {
    const response = await fetch("/api/config/updates", { signal });
    if (!response.ok) return null;
    const body = await response.json();
    return body.data?.data || body.data || null;
  } catch {
    return null;
  }
}

function syncPoller(state, refresh, session) {
  const busy = BUSY_PHASES.has(state.phase);
  if (busy && !poller) {
    poller = new Poller(
      () => {
        if (activeSession !== session) return;
        return refresh();
      },
      {
        label: "UpdateStatus",
        intervalMs: 1000,
        pauseWhenHidden: false,
      }
    );
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
  const host = root.querySelector("#updatesStatus");
  if (!host) return;
  if (activeSession?.root === root) activeSession.statusHtml = null;
  host.innerHTML = `<div class="updates-headline">${Utils.escapeHtml(message)}</div>`;
}

function renderLoadError(root, refresh) {
  const host = root.querySelector("#updatesStatus");
  if (!host) return;
  if (activeSession?.root === root) activeSession.statusHtml = null;
  host.innerHTML = `
    <div class="updates-headline">Update status is unavailable</div>
    <div class="updates-detail">The installation status could not be loaded.</div>
    <div class="updates-actions">
      ${button("updatesStatusRetry", "Try again", "icon-refresh-cw", "primary")}
    </div>
  `;
  host.querySelector("#updatesStatusRetry")?.addEventListener("click", () => refresh());
}

function render(session, state, refresh) {
  const { root } = session;
  const update = state.available_update;
  const statusHtml = renderStatus(state, update);
  const notesHtml = update?.release_notes ? renderNotes(update.release_notes) : "";
  const detailsHtml = renderDetails(state, update);

  if (session.statusHtml !== statusHtml) {
    root.querySelector("#updatesStatus").innerHTML = statusHtml;
    session.statusHtml = statusHtml;
    attachActions(root, state, refresh, session);
  }
  if (session.notesHtml !== notesHtml) {
    root.querySelector("#updatesNotes").innerHTML = notesHtml;
    session.notesHtml = notesHtml;
  }
  if (session.detailsHtml !== detailsHtml) {
    root.querySelector("#updatesDetails").innerHTML = detailsHtml;
    session.detailsHtml = detailsHtml;
  }
}

function renderStatus(state, update) {
  const current = versionInfo.version;
  const progress = state.download_progress || {};

  // headline / detail / actions per phase — one place, so the panel can only
  // ever describe a state the backend actually reported.
  let headline = "Update status is unavailable";
  let detail = "The reported update state is not recognized.";
  let actions = [button("updatesCheck", "Check again", "icon-refresh-cw", "ghost")];
  let bar = "";

  switch (state.phase) {
    case "idle":
      headline = "Ready to check for updates";
      detail = `ScreenerBot v${current} is installed.`;
      actions = [button("updatesCheck", "Check now", "icon-refresh-cw", "primary")];
      break;

    case "up_to_date":
      headline = "You are up to date";
      detail = `ScreenerBot v${current} is the latest version.`;
      break;

    case "checking":
      headline = "Checking for updates";
      detail = "Looking for the latest published release.";
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
      bar = progressBar(
        progress.progress_percent || 0,
        state.phase === "verifying",
        state.phase === "verifying" ? "Verifying update" : "Downloading update"
      );
      break;

    case "ready_to_apply":
      headline = `Version ${update.version} is ready`;
      detail =
        state.blocked_reason ||
        "Installing takes a few seconds. Otherwise, it installs automatically on the next start.";
      actions = [button("updatesApply", "Install and restart", "icon-refresh-cw", "primary")];
      break;

    case "ready_to_install":
      headline = `Version ${update.version} is downloaded`;
      detail = state.blocked_reason || "This update requires the desktop installer to finish.";
      actions = [button("updatesInstall", "Open installer and quit", "icon-package", "primary")];
      break;

    case "applying":
      headline = "Installing";
      detail = "ScreenerBot is restarting onto the new version.";
      actions = [];
      bar = progressBar(100, true, "Installing update");
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

  return `
    <div class="updates-headline">${Utils.escapeHtml(headline)}</div>
    <div class="updates-detail">${Utils.escapeHtml(detail)}</div>
    ${
      update && state.phase !== "applied"
        ? `<div class="updates-versions">
           <span class="updates-version-from">v${Utils.escapeHtml(current)}</span>
           <i class="icon-arrow-right"></i>
           <span class="updates-version-to">v${Utils.escapeHtml(update.version)}</span>
         </div>`
        : `<div class="updates-versions"><span class="updates-version-from">v${Utils.escapeHtml(current)}</span></div>`
    }
    ${bar}
    ${actions.length ? `<div class="updates-actions">${actions.join("")}</div>` : ""}
  `;
}

function describeKind(update) {
  const size = Utils.formatBytes(
    update.kind === "core" ? update.core?.size : update.file_size,
    "unknown size"
  );
  return update.kind === "core"
    ? `This ${size} update installs with a short restart.`
    : `This update requires the ${size} desktop installer.`;
}

function transferLine(progress) {
  const done = Utils.formatBytes(progress.bytes_downloaded || 0);
  const total = Utils.formatBytes(progress.total_bytes || 0);
  return `${done} of ${total}`;
}

function progressBar(percent, indeterminate, label) {
  // No inline width while indeterminate: the sweep animation owns the element,
  // and a stray width would fight the stylesheet for it.
  const width = Math.max(0, Math.min(100, Math.round(percent)));
  return `
    <div class="updates-progress${indeterminate ? " is-indeterminate" : ""}" role="progressbar" aria-label="${label}" aria-valuemin="0" aria-valuemax="100"${
      indeterminate ? "" : ` aria-valuenow="${width}"`
    }>
      <div class="updates-progress-fill"${indeterminate ? "" : ` style="width:${width}%"`}></div>
    </div>
  `;
}

function button(id, label, icon, variant) {
  return `
    <button class="btn btn-${variant}" id="${id}" type="button">
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
    ["Current version", `v${versionInfo.version}`],
    ["System", versionInfo.platform || "Unknown"],
    [
      "Last checked",
      Utils.formatTimestamp(state.last_check || state.last_check_attempt, {
        fallback: "never",
        includeSeconds: false,
      }),
    ],
  ];
  if (update) {
    rows.push(["Update version", `v${update.version}`]);
    rows.push([
      "Download size",
      Utils.formatBytes(
        update.kind === "core" ? update.core?.size : update.file_size,
        "unknown size"
      ),
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

async function renderPreferences(host, session) {
  if (!host) return;
  const prefs = await readPreferences(session.controller.signal);
  if (activeSession !== session) return;
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
      "Install compatible updates with a short restart. Off means ScreenerBot waits for you.",
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
      const fieldLabel = input
        .closest(".settings-field")
        ?.querySelector(".settings-field-info label")
        ?.textContent?.trim();
      const body = { [key]: input.checked };
      const result = await request(
        "/api/config/updates",
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
          signal: session.controller.signal,
        },
        `Could not save ${(fieldLabel || "update preference").toLowerCase()}`
      );
      if (!result) input.checked = !input.checked;
    });
  });
}

// ===========================================================================
// Actions
// ===========================================================================

function attachActions(root, state, refresh, session) {
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
    await request(
      "/api/updates/download",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ version: update.version }),
        signal: session.controller.signal,
      },
      "Could not start update download"
    );
    if (activeSession === session) void refresh();
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
    if (!confirmation.confirmed || activeSession !== session) return;
    renderBusy(root, "Installing...");
    await request(
      "/api/updates/apply",
      { method: "POST", signal: session.controller.signal },
      "Could not install update"
    );
    if (activeSession === session) void refresh();
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
    if (!confirmation.confirmed || activeSession !== session) return;

    const result = await request(
      "/api/updates/install",
      { method: "POST", signal: session.controller.signal },
      "Could not open update installer"
    );
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
