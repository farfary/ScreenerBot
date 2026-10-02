// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The Home update notice.
 *
 * One slim row above the portfolio overview that appears only while a release
 * is in play: available, downloading, verifying, waiting to install, installing
 * or failed, and after the app has updated itself until that is acknowledged. It
 * reads the same updater state as Settings > Updates and links into it; every
 * decision (download, restart, open the installer) stays in that tab.
 */
import {
  readUpdateStatus,
  UPDATE_PHASE_DETAIL_LABELS,
  UPDATE_PHASE_HEADLINE_LABELS,
} from "../../ui/settings/update_status.js";

// Phase -> icon and tone. Phases absent here (idle, checking, up_to_date,
// check_failed, applied) leave the notice hidden; Settings owns them.
const PHASE_PRESENTATION = Object.freeze({
  available: { icon: "icon-arrow-down-to-line", tone: "primary" },
  downloading: { icon: "icon-arrow-down-to-line", tone: "primary" },
  verifying: { icon: "icon-shield-check", tone: "primary" },
  ready_to_apply: { icon: "icon-circle-check", tone: "success" },
  ready_to_install: { icon: "icon-package", tone: "primary" },
  applying: { icon: "icon-loader", tone: "primary" },
  failed: { icon: "icon-circle-x", tone: "error" },
});

function progressSummary(progress, Utils) {
  const percent = Utils.formatPercentValue(
    Math.max(0, Math.min(100, Math.round(progress.progress_percent || 0))),
    { decimals: 0, includeSign: false }
  );
  const transferred = I18n.t("updates-progress-transferred", {
    done: Utils.formatBytes(progress.bytes_downloaded || 0),
    total: Utils.formatBytes(progress.total_bytes || 0),
  });
  return { percent, text: I18n.t("updates-progress-summary", { transferred, percent }) };
}

/**
 * What the notice says for an updater state, or null when it stays hidden.
 * Pure: `Utils` supplies the formatters, so it runs outside a browser.
 */
export function describeUpdateNotice(state, Utils) {
  const update = state.available_update;
  const presentation = PHASE_PRESENTATION[state.phase];
  if (update && presentation) {
    const progress = state.download_progress || {};
    let detail = Object.hasOwn(UPDATE_PHASE_DETAIL_LABELS, state.phase)
      ? I18n.label(UPDATE_PHASE_DETAIL_LABELS, state.phase, { version: state.current_version })
      : "";
    let percent = null;
    if (state.phase === "available") {
      detail = state.self_install
        ? I18n.t("updates-notice-available-detail")
        : I18n.t("updates-headless-install-detail");
    } else if (state.phase === "downloading") {
      const summary = progressSummary(progress, Utils);
      detail = summary.text;
      percent = Math.max(0, Math.min(100, progress.progress_percent || 0));
    } else if (state.phase === "ready_to_apply" || state.phase === "ready_to_install") {
      detail = I18n.text(state.blocked_reason) || detail;
    }
    return {
      key: `${state.phase}:${update.version}`,
      ...presentation,
      headline: I18n.label(UPDATE_PHASE_HEADLINE_LABELS, state.phase, { version: update.version }),
      detail,
      percent,
      action: { label: I18n.t("updates-notice-view"), view: "status" },
      dismissible: false,
    };
  }

  // The release this process is running landed through the updater and has not
  // been acknowledged. The flag lives in the backend: the desktop shell serves
  // the dashboard from a new origin (port) on every launch, so browser storage
  // would forget it.
  const current = state.current_version;
  if (!current || state.applied_version !== current || state.applied_acknowledged) return null;
  return {
    key: `updated:${current}`,
    icon: "icon-circle-check",
    tone: "success",
    headline: I18n.label(UPDATE_PHASE_HEADLINE_LABELS, "applied", { version: current }),
    detail: I18n.t("updates-notice-updated-detail"),
    percent: null,
    action: { label: I18n.t("updates-notice-whats-new"), view: "release-notes" },
    dismissible: true,
  };
}

export function createUpdateNotice(Utils) {
  let inFlight = false;
  let controller = null;
  let renderedKey = null;
  let current = null;
  // The notice acknowledged in this session: a status read already in flight when
  // it was dismissed still reports it unacknowledged and must not bring it back.
  let acknowledgedKey = null;

  const root = () => document.getElementById("homeUpdateNotice");

  function hide() {
    const el = root();
    if (el) el.hidden = true;
    renderedKey = null;
    current = null;
  }

  function render(notice) {
    const el = root();
    if (!el) return;
    current = notice;
    el.hidden = false;
    el.dataset.tone = notice.tone;

    // Rebuild only when the phase or version changes; progress updates in place.
    if (renderedKey !== notice.key) {
      renderedKey = notice.key;
      el.innerHTML = `
        <i class="home-update-icon ${notice.icon}" aria-hidden="true"></i>
        <div class="home-update-copy">
          <strong class="home-update-headline"></strong>
          <span class="home-update-detail"></span>
        </div>
        <div class="home-update-progress" role="progressbar" aria-valuemin="0" aria-valuemax="100" hidden>
          <div class="home-update-progress-fill"></div>
        </div>
        <div class="home-update-actions">
          <button type="button" class="btn btn-sm btn-secondary home-update-open"></button>
          <button type="button" class="home-update-dismiss" hidden>
            <i class="icon-x" aria-hidden="true"></i>
          </button>
        </div>
      `;
      const dismiss = el.querySelector(".home-update-dismiss");
      dismiss.setAttribute("aria-label", I18n.t("common-action-dismiss"));
      dismiss.title = I18n.t("common-action-dismiss");
      el.querySelector(".home-update-open").addEventListener("click", () => {
        const view = current?.action.view || "status";
        if (current?.dismissible) acknowledge();
        void import("../../ui/settings_dialog.js").then(({ showSettingsDialog }) =>
          showSettingsDialog({ tab: "updates", updatesView: view })
        );
      });
      dismiss.addEventListener("click", acknowledge);
    }

    el.querySelector(".home-update-headline").textContent = notice.headline;
    el.querySelector(".home-update-detail").textContent = notice.detail;
    el.querySelector(".home-update-open").textContent = notice.action.label;
    el.querySelector(".home-update-dismiss").hidden = !notice.dismissible;

    const bar = el.querySelector(".home-update-progress");
    bar.hidden = notice.percent === null;
    if (notice.percent !== null) {
      bar.setAttribute("aria-valuenow", String(Math.round(notice.percent)));
      bar.setAttribute("aria-valuetext", notice.detail);
      bar.setAttribute("aria-label", notice.headline);
      bar.firstElementChild.style.width = `${notice.percent}%`;
    }
  }

  function acknowledge() {
    acknowledgedKey = current?.key || null;
    hide();
    fetch("/api/updates/acknowledge", { method: "POST" })
      .then((response) => {
        // Not recorded: let the next poll show it again rather than lose it silently.
        if (!response.ok) acknowledgedKey = null;
      })
      .catch(() => {
        acknowledgedKey = null;
      });
  }

  async function refresh() {
    if (inFlight) return;
    inFlight = true;
    controller = new AbortController();
    try {
      const state = await readUpdateStatus(controller.signal);
      if (!state) return;
      const notice = describeUpdateNotice(state, Utils);
      if (notice && notice.key !== acknowledgedKey) render(notice);
      else hide();
    } finally {
      inFlight = false;
      controller = null;
    }
  }

  function dispose() {
    controller?.abort();
    renderedKey = null;
    current = null;
  }

  return { refresh, dispose };
}
