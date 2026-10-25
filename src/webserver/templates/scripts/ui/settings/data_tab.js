// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Data Tab Module - storage usage and limit, config management, data cleanup
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { setIconLabel } from "../../core/dom.js";
import { formatSizeAt } from "../../core/format.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";
import { renderStateView } from "../state_view.js";
import { apiErrorMessage } from "../../core/request_manager.js";
import { downloadJson } from "../../core/download.js";
import { createStorageView } from "./storage_view.js";

/** How often the report is read again while a cleanup pass runs. */
const CLEANUP_POLL_MS = 2000;

const storageView = createStorageView({
  escapeHtml: Utils.escapeHtml,
  formatBytes: Utils.formatBytes,
  formatTimeAgo: Utils.formatTimeAgo,
  formatSizeAt,
});

/** The open tab's requests and poll timer; null while the tab is closed. */
let storageSession = null;

/** Stop the open tab's requests and its cleanup poll. */
export function teardownDataTab() {
  if (!storageSession) return;
  storageSession.controller.abort();
  clearTimeout(storageSession.timer);
  storageSession = null;
}

/**
 * Build Data tab HTML
 */
export function buildDataTab() {
  return `
      <!-- Storage Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-hard-drive"></i>
          <span data-l10n-id="settings-data-usage-title"></span>
        </h3>
        <p class="settings-section-description" data-l10n-id="settings-data-usage-description"></p>

        <div class="settings-group">
          <div class="data-overview-card" id="dataOverviewCard">
            <div class="data-stats-loading"><i class="icon-loader"></i> <span data-l10n-id="settings-data-usage-loading"></span></div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label for="storageLimitSelect" data-l10n-id="settings-data-limit-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-data-limit-hint"></span>
            </div>
            <div class="settings-field-control">
              <select id="storageLimitSelect" class="settings-select" data-custom-select disabled></select>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-data-cleanup-now-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-data-cleanup-now-hint"></span>
            </div>
            <div class="settings-field-control">
              <button id="storageCleanupBtn" class="btn btn-secondary btn-sm">
                <i class="icon-brush-cleaning"></i>
                <span data-l10n-id="settings-data-cleanup-now-action"></span>
              </button>
            </div>
          </div>

          <div class="config-info-box">
            <div class="config-info-item">
              <span class="config-info-label" data-l10n-id="settings-data-directory-label"></span>
              <span class="config-info-value" id="dataPathDisplay" data-l10n-id="common-loading"></span>
            </div>
          </div>
        </div>
      </div>

      <!-- Configuration Backup Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-settings"></i>
          <span data-l10n-id="settings-data-config-title"></span>
        </h3>
        <p class="settings-section-description" data-l10n-id="settings-data-config-description"></p>

        <div class="settings-group">
          <div class="config-actions-row">
            <button id="exportConfigBtn" class="btn btn-primary">
              <i class="icon-download"></i>
              <span data-l10n-id="settings-data-config-export"></span>
            </button>
            <button id="importConfigBtn" class="btn btn-secondary">
              <i class="icon-upload"></i>
              <span data-l10n-id="settings-data-config-import"></span>
            </button>
            <button id="resetConfigBtn" class="btn btn-warning">
              <i class="icon-refresh-cw"></i>
              <span data-l10n-id="settings-data-config-reset"></span>
            </button>
          </div>
          <input type="file" id="configFileInput" accept=".json,.toml" style="display: none;" />

          <div class="config-info-box">
            <div class="config-info-item">
              <span class="config-info-label" data-l10n-id="settings-data-config-location-label"></span>
              <span class="config-info-value" id="configPathDisplay" data-l10n-id="common-loading"></span>
            </div>
          </div>
        </div>
      </div>

      <!-- Data Cleanup Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-trash-2"></i>
          <span data-l10n-id="settings-data-cleanup-title"></span>
        </h3>
        <p class="settings-section-description" data-l10n-id="settings-data-cleanup-description"></p>

        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-data-cache-clear-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-data-cache-clear-hint"></span>
            </div>
            <div class="settings-field-control">
              <button id="clearOhlcvCacheBtn" class="btn btn-warning btn-sm">
                <i class="icon-trash-2"></i>
                <span data-l10n-id="settings-data-cache-clear"></span>
              </button>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-data-ui-cache-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-data-ui-cache-hint"></span>
            </div>
            <div class="settings-field-control">
              <button id="clearUiStateBtn" class="btn btn-warning btn-sm">
                <i class="icon-refresh-cw"></i>
                <span data-l10n-id="settings-data-ui-cache-clear"></span>
              </button>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label data-l10n-id="settings-data-folder-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-data-folder-hint"></span>
            </div>
            <div class="settings-field-control">
              <button id="openDataFolderBtn" class="btn btn-secondary btn-sm">
                <i class="icon-folder"></i>
                <span data-l10n-id="settings-data-folder-open"></span>
              </button>
            </div>
          </div>
        </div>
      </div>
    `;
}

/**
 * Fill one path row and let a click copy it. A path we do not have yet says so
 * rather than leaving the placeholder claiming it is still loading.
 */
function bindPathRow(content, selector, path, label) {
  const row = content.querySelector(selector);
  if (!row) return;

  if (!path) {
    row.textContent = I18n.t("settings-data-path-unavailable");
    return;
  }

  row.textContent = path;
  row.dir = "ltr";
  row.title = I18n.t("settings-data-path-copy-title");
  row.addEventListener("click", async () => {
    try {
      await Utils.copyToClipboard(path);
      Utils.notifyCopied(label);
    } catch {
      Utils.showToast(I18n.t("settings-data-path-copy-failed"), "error");
    }
  });
}

/**
 * Attach handlers for Data tab
 */
export function attachDataHandlers(dialog, content, pathsInfo) {
  teardownDataTab();
  const session = { controller: new AbortController(), timer: null, content };
  storageSession = session;
  void loadStorage(session);
  attachStorageHandlers(session);

  /* Both locations are click-to-copy rows of the same component. This tab is
     their only home: About used to print the data directory a second time,
     with a second #openDataFolderBtn of its own. */
  bindPathRow(
    content,
    "#dataPathDisplay",
    pathsInfo?.data_directory,
    I18n.t("settings-data-directory-copied")
  );
  bindPathRow(
    content,
    "#configPathDisplay",
    pathsInfo?.config_path,
    I18n.t("settings-data-config-path-copied")
  );

  // Export config button
  const exportBtn = content.querySelector("#exportConfigBtn");
  if (exportBtn) {
    exportBtn.addEventListener("click", () => exportConfig());
  }

  // Import config button
  const importBtn = content.querySelector("#importConfigBtn");
  const fileInput = content.querySelector("#configFileInput");
  if (importBtn && fileInput) {
    importBtn.addEventListener("click", () => fileInput.click());
    fileInput.addEventListener("change", (e) => importConfig(e));
  }

  // Reset config button
  const resetBtn = content.querySelector("#resetConfigBtn");
  if (resetBtn) {
    resetBtn.addEventListener("click", () => resetConfig());
  }

  // Clear all OHLCV cache button
  const clearOhlcvBtn = content.querySelector("#clearOhlcvCacheBtn");
  if (clearOhlcvBtn) {
    clearOhlcvBtn.addEventListener("click", async () => {
      const confirmResult = await ConfirmationDialog.show({
        title: I18n.t("settings-data-cache-confirm-title"),
        message: I18n.t("settings-data-cache-confirm-message"),
        confirmLabel: I18n.t("common-action-clear"),
        cancelLabel: I18n.t("common-action-cancel"),
        variant: "danger",
      });
      if (!confirmResult.confirmed) return;

      clearOhlcvBtn.disabled = true;
      setIconLabel(clearOhlcvBtn, "icon-loader spin", I18n.t("settings-data-cache-clearing"));

      try {
        const response = await fetch("/api/ohlcv/cache/clear", { method: "POST" });
        if (response.ok) {
          const data = await response.json();
          Utils.showToast(
            I18n.t("settings-data-cache-cleared", {
              candles: I18n.t("settings-data-candles-count", { count: data.candles_deleted }),
              tokens: I18n.t("settings-data-tokens-count", { count: data.tokens_reset }),
            }),
            "success"
          );
          void loadStorage(storageSession);
        } else {
          Utils.showToast(I18n.t("settings-data-cache-clear-failed"), "error");
        }
      } catch (err) {
        Utils.showToast(
          I18n.t("settings-data-cache-clear-failed-detail", { message: err.message }),
          "error"
        );
      } finally {
        clearOhlcvBtn.disabled = false;
        setIconLabel(clearOhlcvBtn, "icon-trash-2", I18n.t("settings-data-cache-clear"));
      }
    });
  }

  // Clear UI state button
  const clearUiBtn = content.querySelector("#clearUiStateBtn");
  if (clearUiBtn) {
    clearUiBtn.addEventListener("click", async () => {
      const confirmResult = await ConfirmationDialog.show({
        title: I18n.t("settings-data-ui-cache-confirm-title"),
        message: I18n.t("settings-data-ui-cache-confirm-message"),
        confirmLabel: I18n.t("common-action-clear"),
        cancelLabel: I18n.t("common-action-cancel"),
        variant: "danger",
      });
      if (!confirmResult.confirmed) return;

      const keysToRemove = [];
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (
          key &&
          (key.startsWith("table.") ||
            key.startsWith("tokens-table") ||
            key.startsWith("positions-table") ||
            key.includes(".state"))
        ) {
          keysToRemove.push(key);
        }
      }

      keysToRemove.forEach((key) => localStorage.removeItem(key));
      Utils.showToast(
        I18n.t("settings-data-ui-cache-cleared", { count: keysToRemove.length }),
        "success"
      );
    });
  }

  // Open data folder button
  const openFolderBtn = content.querySelector("#openDataFolderBtn");
  if (openFolderBtn) {
    openFolderBtn.addEventListener("click", async () => {
      try {
        // Success is the folder appearing in front of the user; only a
        // failure needs saying.
        const response = await fetch("/api/system/paths/open-data", { method: "POST" });
        if (!response.ok) {
          Utils.showToast({ type: "error", title: I18n.t("settings-data-folder-open-failed") });
        }
      } catch (err) {
        Utils.showToast({
          type: "error",
          title: I18n.t("settings-data-folder-open-failed"),
          message: err.message,
        });
      }
    });
  }
}

/**
 * Read the storage report and render it, then keep reading it while a cleanup
 * pass runs. A closed tab, or a tab opened again since, stops the chain.
 */
async function loadStorage(session) {
  if (!session || storageSession !== session) return;
  const card = session.content.querySelector("#dataOverviewCard");
  if (!card) return;

  let report;
  try {
    const response = await fetch("/api/system/storage", { signal: session.controller.signal });
    if (!response.ok) throw new Error(I18n.t("settings-data-usage-load-failed"));
    report = await response.json();
  } catch (err) {
    if (err.name === "AbortError" || storageSession !== session) return;
    card.innerHTML = renderStateView({
      kind: "error",
      icon: "icon-triangle-alert",
      title: I18n.t("settings-data-usage-load-failed"),
    });
    return;
  }
  if (storageSession !== session) return;

  card.innerHTML = storageView.overviewHtml(report);
  renderLimit(session, report);
  const cleanupBtn = session.content.querySelector("#storageCleanupBtn");
  if (cleanupBtn) {
    cleanupBtn.disabled = report.running;
    setIconLabel(
      cleanupBtn,
      report.running ? "icon-loader spin" : "icon-brush-cleaning",
      report.running
        ? I18n.t("settings-data-cleanup-now-running")
        : I18n.t("settings-data-cleanup-now-action")
    );
  }

  clearTimeout(session.timer);
  if (report.running) {
    session.timer = setTimeout(() => void loadStorage(session), CLEANUP_POLL_MS);
  }
}

/** Fill the limit select from the report, unless a save is in flight. */
function renderLimit(session, report) {
  const select = session.content.querySelector("#storageLimitSelect");
  if (!select || select.dataset.saving === "true") return;
  const options = storageView.limitOptions(report);
  select.replaceChildren(
    ...options.map((option) => {
      const element = document.createElement("option");
      element.value = option.value;
      element.textContent = option.label;
      element.selected = option.selected;
      return element;
    })
  );
  select.dataset.savedValue = String(report.limit_megabytes);
  select.disabled = false;
}

function attachStorageHandlers(session) {
  const select = session.content.querySelector("#storageLimitSelect");
  if (select) {
    select.addEventListener("change", async () => {
      const megabytes = Number(select.value);
      if (!Number.isInteger(megabytes) || megabytes < 0) {
        select.value = select.dataset.savedValue;
        return;
      }
      select.dataset.saving = "true";
      select.disabled = true;
      let saved = false;
      try {
        const response = await fetch("/api/config/maintenance", {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ storage_limit_mb: megabytes }),
          signal: session.controller.signal,
        });
        const body = await response.json().catch(() => ({}));
        if (!response.ok || body.success === false) {
          throw new Error(apiErrorMessage(body, I18n.t("settings-data-unknown-error")));
        }
        saved = true;
      } catch (err) {
        if (err.name === "AbortError") return;
        Utils.showToast({
          key: "settings:storage-limit",
          type: "error",
          title: I18n.t("settings-data-limit-save-failed"),
          message: err.message,
        });
      }
      if (storageSession !== session) return;
      if (!saved) select.value = select.dataset.savedValue;
      delete select.dataset.saving;
      select.disabled = false;
      void loadStorage(session);
    });
  }

  const cleanupBtn = session.content.querySelector("#storageCleanupBtn");
  if (cleanupBtn) {
    cleanupBtn.addEventListener("click", async () => {
      cleanupBtn.disabled = true;
      setIconLabel(cleanupBtn, "icon-loader spin", I18n.t("settings-data-cleanup-now-running"));
      try {
        const response = await fetch("/api/system/storage/cleanup", {
          method: "POST",
          signal: session.controller.signal,
        });
        if (!response.ok) {
          const body = await response.json().catch(() => ({}));
          Utils.showToast({
            key: "settings:storage-cleanup",
            type: response.status === 409 ? "info" : "error",
            title: apiErrorMessage(body, I18n.t("settings-data-cleanup-now-failed")),
          });
        }
      } catch (err) {
        if (err.name === "AbortError") return;
        Utils.showToast({
          key: "settings:storage-cleanup",
          type: "error",
          title: I18n.t("settings-data-cleanup-now-failed"),
          message: err.message,
        });
      }
      void loadStorage(session);
    });
  }
}

/**
 * Export configuration to JSON file
 */
async function exportConfig() {
  try {
    // The server export removes every secret listed in its sensitive-field set
    // (API keys, bot token, password hashes, RPC URLs), and never includes the wallet.
    const response = await fetch("/api/config/export", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ include_gui: true, include_metadata: true, sanitize_secrets: true }),
    });
    if (!response.ok) throw new Error(I18n.t("settings-data-config-fetch-failed"));

    const exported = await response.json();
    if (!exported || !exported.config) {
      throw new Error(I18n.t("settings-data-config-fetch-failed"));
    }

    downloadJson(
      `screenerbot-config-${new Date().toISOString().split("T")[0]}.json`,
      exported.config
    );

    Utils.showToast(I18n.t("settings-data-config-exported"), "success");
  } catch (err) {
    Utils.showToast(
      I18n.t("settings-data-config-export-failed", { message: err.message }),
      "error"
    );
  }
}

/**
 * Import configuration from JSON file
 */
async function importConfig(event) {
  const file = event.target.files?.[0];
  if (!file) return;

  // Reset file input for future imports
  event.target.value = "";

  try {
    const text = await file.text();
    const imported = JSON.parse(text);

    // Confirm import
    const confirmResult = await ConfirmationDialog.show({
      title: I18n.t("settings-data-config-import-title"),
      message: I18n.t("settings-data-config-import-message"),
      confirmLabel: I18n.t("common-action-import"),
      cancelLabel: I18n.t("common-action-cancel"),
      variant: "warning",
    });
    if (!confirmResult.confirmed) return;

    // One import call: the server relocates sections from older layouts and
    // validates them before applying. The wallet section is never imported.
    const response = await fetch("/api/config/import", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        config: imported,
        sections: [
          "trader",
          "positions",
          "filtering",
          "tokens",
          "chains",
          "events",
          "services",
          "monitoring",
          "ohlcv",
          "gui",
        ],
        merge: true,
        save_to_disk: true,
      }),
    });

    if (!response.ok) {
      const data = await response.json();
      Utils.showToast(
        I18n.t("settings-data-config-import-failed", {
          message: apiErrorMessage(data, I18n.t("settings-data-unknown-error")),
        }),
        "error"
      );
      return;
    }

    Utils.showToast(I18n.t("settings-data-config-imported"), "success");
  } catch (err) {
    Utils.showToast(
      I18n.t("settings-data-config-import-failed", { message: err.message }),
      "error"
    );
  }
}

/**
 * Reset configuration to defaults
 */
async function resetConfig() {
  const confirmResult = await ConfirmationDialog.show({
    title: I18n.t("settings-data-config-reset-title"),
    message: I18n.t("settings-data-config-reset-message"),
    confirmLabel: I18n.t("common-action-reset"),
    cancelLabel: I18n.t("common-action-cancel"),
    variant: "danger",
  });
  if (!confirmResult.confirmed) return;

  try {
    const response = await fetch("/api/config/reset", { method: "POST" });

    if (response.ok) {
      Utils.showToast(I18n.t("settings-data-config-reset-done"), "success");
    } else {
      const data = await response.json();
      Utils.showToast(
        I18n.t("settings-data-config-reset-failed", {
          message: apiErrorMessage(data, I18n.t("settings-data-unknown-error")),
        }),
        "error"
      );
    }
  } catch (err) {
    Utils.showToast(
      I18n.t("settings-data-config-reset-failed", { message: err.message }),
      "error"
    );
  }
}
