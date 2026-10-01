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
import { readUpdateStatus } from "./update_status.js";
import { apiErrorMessage } from "../../core/request_manager.js";

const BUSY_PHASES = new Set(["checking", "downloading", "verifying", "applying"]);
const UPDATE_TAB_LABELS = Object.freeze({
  status: "updates-tab-status",
  "release-notes": "updates-tab-release-notes",
  preferences: "updates-tab-preferences",
});
const UPDATE_TABS = Object.keys(UPDATE_TAB_LABELS).map((id) => ({
  id,
  label: I18n.label(UPDATE_TAB_LABELS, id),
}));

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
        ariaLabel: I18n.t("updates-tab-sections"),
      })}
      <div class="updates-announcer sr-only" id="updatesAnnouncement" aria-live="polite"></div>
      <div class="updates-panels">
        <section class="updates-panel" data-tab-content="status">
          <div id="updatesStatus">
            <div class="updates-loading" data-l10n-id="updates-checking-installation"></div>
          </div>
        </section>
        <section class="updates-panel" data-tab-content="release-notes" hidden>
          <div id="updatesNotes"></div>
        </section>
        <section class="updates-panel" data-tab-content="preferences" hidden>
          <div id="updatesPreferences">
            <div class="updates-loading" data-l10n-id="common-loading"></div>
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
    pendingRecheck: false,
    lastState: null,
    history: { status: "idle", releases: [] },
  };
  activeSession = session;

  session.tabBar = new DialogTabBar({
    root,
    tabs: UPDATE_TABS,
    activeTab: activeUpdatesTab,
    panelSelector: ".updates-panels > [data-tab-content]",
    onChange: (tabId) => {
      activeUpdatesTab = tabId;
      if (tabId === "release-notes") void loadHistory(session);
    },
  });

  const refresh = async ({ recheck = false } = {}) => {
    if (session.refreshing) {
      // Opening the tab starts a status read immediately. A native/manual check
      // arriving during that read must run next, not disappear as a no-op.
      if (recheck) session.pendingRecheck = true;
      return;
    }
    session.refreshing = true;
    try {
      if (recheck) {
        setButtonBusy(root, "updatesCheck", I18n.t("updates-busy-checking"));
        await request(
          "/api/updates/check",
          { signal: session.controller.signal },
          I18n.t("updates-check-request-failed"),
          "updates:check"
        );
        if (activeSession !== session) return;
        // A check can be the moment a new release exists at all, so the notes
        // the panel is holding are the ones that just went stale.
        if (session.history.status === "ready") session.history.status = "idle";
        if (activeUpdatesTab === "release-notes") void loadHistory(session);
      }

      const state = await readUpdateStatus(session.controller.signal);
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
      if (session.pendingRecheck && activeSession === session) {
        session.pendingRecheck = false;
        void refresh({ recheck: true });
      }
    }
  };
  session.refresh = refresh;

  void renderPreferences(root.querySelector("#updatesPreferences"), session);
  if (activeUpdatesTab === "release-notes") void loadHistory(session);
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

/** Show one Updates sub-tab now if the tab is open, and the next time it opens. */
export function selectUpdatesView(tabId) {
  if (!Object.hasOwn(UPDATE_TAB_LABELS, tabId)) return;
  activeUpdatesTab = tabId;
  activeSession?.tabBar?.setActive(tabId);
}

export function requestUpdateCheck() {
  return activeSession?.refresh?.({ recheck: true });
}

async function request(url, options = {}, errorTitle, key) {
  try {
    const response = await fetch(url, options);
    const body = await response.json().catch(() => ({}));
    if (!response.ok || body.success === false) {
      throw new Error(apiErrorMessage(body, I18n.t("updates-request-failed")));
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
      // l10n-ignore: poller diagnostic label
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

/**
 * Read the published release history from screenerbot.io, through the backend.
 *
 * Loaded the first time the panel is opened rather than with the dialog: a
 * reader who never opens Release Notes never pays for the request. The backend
 * caches the answer, so a re-open is free.
 */
async function loadHistory(session, { force = false } = {}) {
  if (!force && session.history.status !== "idle") return;
  // A reload keeps what is already on screen: replacing a read list with a
  // spinner would make a manual retry look like the panel lost its content.
  session.history = { status: "loading", releases: session.history.releases };
  renderNotes(session);

  let history = null;
  try {
    const response = await fetch("/api/updates/history", { signal: session.controller.signal });
    const body = await response.json();
    if (response.ok && body.success !== false) history = body.data || body;
  } catch (err) {
    if (err.name === "AbortError") return;
  }
  if (activeSession !== session) return;

  session.history = history?.releases?.length
    ? { status: "ready", releases: history.releases }
    : { status: "error", releases: [] };
  renderNotes(session);
}

/**
 * The panel shows the website's full history. When that cannot be read it falls
 * back to the one release the updater itself knows about, which is all an
 * offline installation has.
 */
function renderNotes(session) {
  const host = session.root.querySelector("#updatesNotes");
  if (!host) return;

  const state = session.lastState;
  const fallback = state ? releaseForState(state) : null;
  const failed = session.history.status === "error";
  const releases = session.history.releases.length
    ? session.history.releases
    : fallback
      ? [fallback]
      : [];

  const html = view.renderReleaseNotes({
    releases,
    currentVersion: versionInfo.version,
    availableVersion: state?.available_update?.version || null,
    loading: session.history.status === "loading" && !releases.length,
    error: failed,
  });
  if (session.notesHtml === html) return;

  host.innerHTML = html;
  session.notesHtml = html;
  host
    .querySelector("#updatesNotesRetry")
    ?.addEventListener("click", () => void loadHistory(session, { force: true }));
}

function render(session, state, refresh) {
  const displayState = {
    ...state,
    currentVersion: versionInfo.version,
    platform: versionInfo.platform,
  };
  const status = view.renderStatus(displayState);
  session.lastState = state;

  if (session.statusHtml !== status.html) {
    session.root.querySelector("#updatesStatus").innerHTML = status.html;
    session.statusHtml = status.html;
    attachActions(session.root, state, refresh, session);
  }
  renderNotes(session);
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
          <h3 class="updates-headline" data-l10n-id="updates-status-unavailable-headline"></h3>
          <p class="updates-detail" data-l10n-id="updates-status-load-failed-detail"></p>
        </div>
      </div>
      <div class="updates-actions">
        <button class="btn btn-primary" id="updatesStatusRetry" type="button">
          <i class="icon-refresh-cw" aria-hidden="true"></i><span data-l10n-id="updates-action-try-again"></span>
        </button>
      </div>
    </section>
  `;
  I18n.localizeTree(host);
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
    host.innerHTML = `<div class="updates-loading">${Utils.escapeHtml(I18n.t("common-loading"))}</div>`;
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
        I18n.t("updates-preference-save-failed", {
          preference: label || I18n.t("updates-preference-fallback-name"),
        }),
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
  on("updatesRetry", async () => {
    const update = state.available_update;
    if (!update) return refresh({ recheck: true });
    setButtonBusy(root, "updatesRetry", I18n.t("updates-busy-resuming"));
    await request(
      "/api/updates/download",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ version: update.version }),
        signal: session.controller.signal,
      },
      I18n.t("updates-resume-failed"),
      "updates:retry"
    );
    if (activeSession === session) void refresh();
  });

  on("updatesDownload", async () => {
    const update = state.available_update;
    if (!update) return;
    setButtonBusy(root, "updatesDownload", I18n.t("updates-busy-starting-download"));
    await request(
      "/api/updates/download",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ version: update.version }),
        signal: session.controller.signal,
      },
      I18n.t("updates-download-failed"),
      "updates:download"
    );
    if (activeSession === session) void refresh();
  });

  on("updatesApply", async () => {
    const confirmation = await ConfirmationDialog.show({
      title: I18n.t("updates-apply-confirm-title", {
        version: state.available_update?.version ?? "",
      }),
      message: I18n.t("updates-apply-confirm-message"),
      confirmLabel: I18n.t("updates-action-restart"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "warning",
    });
    if (!confirmation.confirmed || activeSession !== session) return;

    setButtonBusy(root, "updatesApply", I18n.t("updates-busy-restarting"));
    await request(
      "/api/updates/apply",
      { method: "POST", signal: session.controller.signal },
      I18n.t("updates-apply-failed"),
      "updates:apply"
    );
    if (activeSession === session) void refresh();
  });

  on("updatesInstall", async () => {
    const confirmation = await ConfirmationDialog.show({
      title: I18n.t("updates-install-confirm-title"),
      message: I18n.t("updates-install-confirm-message"),
      confirmLabel: I18n.t("updates-action-open-installer"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "warning",
    });
    if (!confirmation.confirmed || activeSession !== session) return;

    setButtonBusy(root, "updatesInstall", I18n.t("updates-busy-opening-installer"));
    const result = await request(
      "/api/updates/install",
      { method: "POST", signal: session.controller.signal },
      I18n.t("updates-install-failed"),
      "updates:install"
    );
    if (!result) {
      if (activeSession === session) void refresh();
      return;
    }
    Utils.showToast({
      key: "updates:installer-opened",
      type: "success",
      title: I18n.t("updates-installer-toast-title"),
      message: I18n.t("updates-installer-toast-message"),
    });
    setTimeout(() => window.electronAPI?.quitForUpdate?.(), 1000);
  });
}
