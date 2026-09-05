/**
 * Settings > Updates controller.
 *
 * Status, release notes, and preferences have separate views, while this owner
 * keeps one abortable updater session and one backend-derived phase story.
 */
import * as Utils from "../../core/utils.js";
import { Poller } from "../../core/poller.js";
import { DialogTabBar, renderDialogTabRow } from "../dialog_tab_bar.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";
import { createUpdatesView } from "./updates_view.js";

const BUSY_PHASES = new Set(["checking", "downloading", "verifying", "applying"]);
const UPDATE_TABS = [
  { id: "status", label: "Status" },
  { id: "release-notes", label: "Release Notes" },
  { id: "preferences", label: "Preferences" },
];

const view = createUpdatesView(Utils);
let poller = null;
let activeSession = null;
let activeUpdatesTab = "status";
let versionInfo = { version: "0.0.0", platform: "" };

export function buildUpdatesTab(info) {
  versionInfo = info || versionInfo;
  return `
    <div class="updates" id="updatesRoot">
      ${renderDialogTabRow({
        tabs: UPDATE_TABS,
        activeTab: activeUpdatesTab,
        idPrefix: "settings-updates",
        ariaLabel: "Update sections",
      })}
      <div class="updates-announcer sr-only" id="updatesAnnouncement" aria-live="polite"></div>
      <div class="updates-panels">
        <section class="updates-panel" data-tab-content="status">
          <div id="updatesStatus">
            <div class="updates-loading">Checking this installation...</div>
          </div>
        </section>
        <section class="updates-panel" data-tab-content="release-notes" hidden>
          <div id="updatesNotes"></div>
        </section>
        <section class="updates-panel" data-tab-content="preferences" hidden>
          <div id="updatesPreferences">
            <div class="updates-loading">Loading update preferences...</div>
          </div>
        </section>
      </div>
    </div>
  `;
}

export function attachUpdatesHandlers(content, onAttentionChange) {
  teardownUpdatesTab();

  const root = content.querySelector("#updatesRoot");
  if (!root) return;

  const session = {
    controller: new AbortController(),
    root,
    tabBar: null,
    statusHtml: null,
    notesHtml: null,
    lastPhase: null,
    refreshing: false,
  };
  activeSession = session;

  session.tabBar = new DialogTabBar({
    root,
    tabs: UPDATE_TABS,
    activeTab: activeUpdatesTab,
    panelSelector: ".updates-panels > [data-tab-content]",
    onChange: (tabId) => {
      activeUpdatesTab = tabId;
    },
  });

  const refresh = async ({ recheck = false } = {}) => {
    if (session.refreshing) return;
    session.refreshing = true;
    try {
      if (recheck) {
        setButtonBusy(root, "updatesCheck", "Checking...");
        await request(
          "/api/updates/check",
          { signal: session.controller.signal },
          "Could not check for updates",
          "updates:check"
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
      onAttentionChange?.(Boolean(state.requires_user_action));
      syncPoller(state, refresh, session);
    } finally {
      session.refreshing = false;
    }
  };

  void renderPreferences(root.querySelector("#updatesPreferences"), session);
  void refresh();
}

export function teardownUpdatesTab() {
  if (activeSession) {
    activeSession.controller.abort();
    activeSession.tabBar?.destroy();
    activeSession = null;
  }
  if (poller) {
    poller.cleanup();
    poller = null;
  }
}

async function readStatus(signal) {
  try {
    const response = await fetch("/api/updates/status", { signal });
    const body = await response.json();
    if (!response.ok || body.success === false) return null;
    const payload = body.data || body;
    const state = payload.state || payload;
    state.blocked_reason = payload.blocked_reason || null;
    state.requires_user_action = Boolean(payload.requires_user_action);
    return state;
  } catch {
    return null;
  }
}

async function request(url, options = {}, errorTitle, key) {
  try {
    const response = await fetch(url, options);
    const body = await response.json().catch(() => ({}));
    if (!response.ok || body.success === false) {
      throw new Error(body.error?.message || body.error || "Request failed");
    }
    return body;
  } catch (err) {
    if (err.name === "AbortError") return null;
    Utils.showToast({ key, type: "error", title: errorTitle, message: err.message });
    return null;
  }
}

async function readPreferenceModel(signal) {
  try {
    const [preferencesResponse, metadataResponse] = await Promise.all([
      fetch("/api/config/updates", { signal }),
      fetch("/api/config/metadata", { signal }),
    ]);
    if (!preferencesResponse.ok || !metadataResponse.ok) return null;

    const preferencesBody = await preferencesResponse.json();
    const metadataBody = await metadataResponse.json();
    const values = preferencesBody.data?.data || preferencesBody.data || null;
    const metadata = metadataBody.data?.updates || metadataBody.data?.data?.updates || null;
    return values && metadata ? { values, metadata } : null;
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
      { label: "UpdateStatus", intervalMs: 1000, pauseWhenHidden: false }
    );
    poller.start();
  } else if (!busy && poller) {
    poller.cleanup();
    poller = null;
  }
}

function releaseForState(state) {
  if (state.available_update) {
    return {
      version: state.available_update.version,
      release_date: state.available_update.release_date,
      release_notes: state.available_update.release_notes,
    };
  }
  return state.last_release || null;
}

function render(session, state, refresh) {
  const displayState = {
    ...state,
    currentVersion: versionInfo.version,
    platform: versionInfo.platform,
  };
  const status = view.renderStatus(displayState);
  const notesHtml = view.renderReleaseNotes(releaseForState(state));

  if (session.statusHtml !== status.html) {
    session.root.querySelector("#updatesStatus").innerHTML = status.html;
    session.statusHtml = status.html;
    attachActions(session.root, state, refresh, session);
  }
  if (session.notesHtml !== notesHtml) {
    session.root.querySelector("#updatesNotes").innerHTML = notesHtml;
    session.notesHtml = notesHtml;
  }
  if (session.lastPhase !== state.phase) {
    session.root.querySelector("#updatesAnnouncement").textContent = status.announcement;
    session.lastPhase = state.phase;
  }
}

function renderLoadError(root, refresh) {
  const host = root.querySelector("#updatesStatus");
  if (!host) return;
  activeSession.statusHtml = null;
  host.innerHTML = `
    <section class="updates-status" data-tone="error">
      <div class="updates-status-copy">
        <i class="updates-status-icon icon-circle-alert" aria-hidden="true"></i>
        <div>
          <h3 class="updates-headline">Update status is unavailable</h3>
          <p class="updates-detail">The installation status could not be loaded.</p>
        </div>
      </div>
      <div class="updates-actions">
        <button class="btn btn-primary" id="updatesStatusRetry" type="button">
          <i class="icon-refresh-cw" aria-hidden="true"></i><span>Try again</span>
        </button>
      </div>
    </section>
  `;
  host.querySelector("#updatesStatusRetry")?.addEventListener("click", () => refresh());
}

function setButtonBusy(root, id, label) {
  const button = root.querySelector(`#${id}`);
  if (!button) return;
  button.disabled = true;
  const icon = button.querySelector("i");
  const text = button.querySelector("span");
  if (icon) icon.className = "icon-loader";
  if (text) text.textContent = label;
}

async function renderPreferences(host, session) {
  if (!host) return;
  const model = await readPreferenceModel(session.controller.signal);
  if (activeSession !== session) return;

  host.innerHTML = view.renderPreferences(model?.values || {}, model?.metadata || {});
  host.querySelector("#updatesPrefsRetry")?.addEventListener("click", () => {
    host.innerHTML = '<div class="updates-loading">Loading update preferences...</div>';
    void renderPreferences(host, session);
  });
  if (!model) return;

  const syncDependencies = () => {
    const automatic = host.querySelector('[data-pref="auto_check"]')?.checked !== false;
    const interval = host.querySelector('[data-pref="check_interval_hours"]');
    if (interval) interval.disabled = !automatic;
    host
      .querySelector('[data-update-field="check_interval_hours"]')
      ?.classList.toggle("updates-field--inactive", !automatic);
  };

  host.querySelectorAll("input[data-pref]").forEach((input) => {
    input.addEventListener("change", async () => {
      const key = input.dataset.pref;
      const savedValue = input.dataset.savedValue;
      let value = input.type === "checkbox" ? input.checked : Number(input.value);

      if (input.type === "number") {
        if (!Number.isFinite(value)) {
          input.value = savedValue;
          return;
        }
        value = Math.round(Math.min(Number(input.max), Math.max(Number(input.min), value)));
        input.value = String(value);
      }

      input.disabled = true;
      const label = input
        .closest(".settings-field")
        ?.querySelector(".settings-field-info label")
        ?.textContent?.trim();
      const result = await request(
        "/api/config/updates",
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ [key]: value }),
          signal: session.controller.signal,
        },
        `Could not save ${(label || "update preference").toLowerCase()}`,
        `updates:preference:${key}`
      );

      if (activeSession !== session) return;
      if (result) {
        input.dataset.savedValue = String(value);
      } else if (input.type === "checkbox") {
        input.checked = savedValue === "true";
      } else {
        input.value = savedValue;
      }
      input.disabled = false;
      syncDependencies();
    });
  });

  syncDependencies();
}

function attachActions(root, state, refresh, session) {
  const on = (id, handler) => root.querySelector(`#${id}`)?.addEventListener("click", handler);

  on("updatesCheck", () => refresh({ recheck: true }));
  on("updatesRetry", () => refresh({ recheck: true }));

  on("updatesDownload", async () => {
    const update = state.available_update;
    if (!update) return;
    setButtonBusy(root, "updatesDownload", "Starting download...");
    await request(
      "/api/updates/download",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ version: update.version }),
        signal: session.controller.signal,
      },
      "Could not start update download",
      "updates:download"
    );
    if (activeSession === session) void refresh();
  });

  on("updatesApply", async () => {
    const confirmation = await ConfirmationDialog.show({
      title: `Install v${state.available_update?.version}`,
      message:
        "ScreenerBot restarts onto the new version. Trading stops for a few seconds and resumes automatically; open positions are untouched.",
      confirmLabel: "Restart to update",
      cancelLabel: "Cancel",
      variant: "warning",
    });
    if (!confirmation.confirmed || activeSession !== session) return;

    setButtonBusy(root, "updatesApply", "Restarting...");
    await request(
      "/api/updates/apply",
      { method: "POST", signal: session.controller.signal },
      "Could not install update",
      "updates:apply"
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

    setButtonBusy(root, "updatesInstall", "Opening installer...");
    const result = await request(
      "/api/updates/install",
      { method: "POST", signal: session.controller.signal },
      "Could not open update installer",
      "updates:install"
    );
    if (!result) {
      if (activeSession === session) void refresh();
      return;
    }
    Utils.showToast({
      key: "updates:installer-opened",
      type: "success",
      title: "Installer opened",
      message: "ScreenerBot will quit cleanly now.",
    });
    setTimeout(() => window.electronAPI?.quitForUpdate?.(), 1000);
  });
}
