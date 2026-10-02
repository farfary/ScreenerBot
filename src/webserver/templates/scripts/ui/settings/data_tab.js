// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Data Tab Module - Database stats, config management, data cleanup
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { setIconLabel } from "../../core/dom.js";
import { formatSizeAt } from "../../core/format.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";
import { apiErrorMessage } from "../../core/request_manager.js";

/** Database id -> label key, keyed by `DatabaseId` (webserver/routes/system/types.rs). */
const DATABASE_LABELS = Object.freeze({
  tokens: "settings-data-db-tokens",
  transactions: "settings-data-db-transactions",
  positions: "settings-data-db-positions",
  events: "settings-data-db-events",
  ohlcv: "settings-data-db-ohlcv",
  wallet: "settings-data-db-wallet",
  pools: "settings-data-db-pools",
  strategies: "settings-data-db-strategies",
  actions: "settings-data-db-actions",
});

/**
 * Build Data tab HTML
 */
export function buildDataTab() {
  return `
      <!-- Database Overview Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-database"></i>
          <span data-l10n-id="settings-data-storage-title"></span>
        </h3>
        <p class="settings-section-description" data-l10n-id="settings-data-storage-description"></p>

        <div class="data-overview-card" id="dataOverviewCard">
          <div class="data-stats-loading"><i class="icon-loader"></i> <span data-l10n-id="settings-data-stats-loading"></span></div>
        </div>

        <div class="config-info-box">
          <div class="config-info-item">
            <span class="config-info-label" data-l10n-id="settings-data-directory-label"></span>
            <span class="config-info-value" id="dataPathDisplay" data-l10n-id="common-loading"></span>
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
              <label data-l10n-id="settings-data-ohlcv-cleanup-label"></label>
              <span class="settings-field-hint" data-l10n-id="settings-data-ohlcv-cleanup-hint"></span>
            </div>
            <div class="settings-field-control data-action-group">
              <input type="number" id="cleanupHours" class="settings-input small" value="24" min="1" max="720" />
              <span class="input-unit" data-l10n-id="settings-data-cleanup-hours-unit"></span>
              <button id="cleanupOhlcvBtn" class="btn btn-warning btn-sm">
                <i class="icon-trash-2"></i>
                <span data-l10n-id="settings-data-cleanup-ohlcv"></span>
              </button>
            </div>
          </div>

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
              <button id="clearUiStateBtn" class="btn btn-secondary btn-sm">
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
  // Load data overview
  loadDataOverview(content);

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

  // OHLCV cleanup button
  const cleanupBtn = content.querySelector("#cleanupOhlcvBtn");
  const hoursInput = content.querySelector("#cleanupHours");

  if (cleanupBtn && hoursInput) {
    cleanupBtn.addEventListener("click", async () => {
      const hours = parseInt(hoursInput.value, 10);
      if (isNaN(hours) || hours < 1) {
        Utils.showToast(I18n.t("settings-data-cleanup-hours-invalid"), "error");
        return;
      }

      const confirmResult = await ConfirmationDialog.show({
        title: I18n.t("settings-data-cleanup-confirm-title"),
        message: I18n.t("settings-data-cleanup-confirm-message", { hours }),
        confirmLabel: I18n.t("common-action-delete"),
        cancelLabel: I18n.t("common-action-cancel"),
        variant: "danger",
      });
      if (!confirmResult.confirmed) return;

      cleanupBtn.disabled = true;
      setIconLabel(cleanupBtn, "icon-loader spin", I18n.t("settings-data-cleanup-running"));

      try {
        const response = await fetch("/api/ohlcv/cleanup", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ inactive_hours: hours }),
        });

        if (response.ok) {
          const data = await response.json();
          Utils.showToast(
            I18n.t("settings-data-cleanup-done", { count: data.deleted_count }),
            "success"
          );
          loadDataOverview(content);
        } else {
          Utils.showToast(I18n.t("settings-data-cleanup-failed"), "error");
        }
      } catch (err) {
        Utils.showToast(
          I18n.t("settings-data-cleanup-failed-detail", { message: err.message }),
          "error"
        );
      } finally {
        cleanupBtn.disabled = false;
        setIconLabel(cleanupBtn, "icon-trash-2", I18n.t("settings-data-cleanup-ohlcv"));
      }
    });
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
          loadDataOverview(content);
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
 * Load comprehensive data overview
 */
async function loadDataOverview(content) {
  const card = content.querySelector("#dataOverviewCard");
  if (!card) return;

  try {
    const response = await fetch("/api/system/data-stats");
    if (!response.ok) throw new Error(I18n.t("settings-data-stats-load-failed"));

    const data = await response.json();
    const maxSize = Math.max(...data.databases.map((db) => db.size_bytes), 1);

    const dbItemsHtml = data.databases
      .filter((db) => db.exists)
      .map((db) => {
        const percentage = (db.size_bytes / maxSize) * 100;
        const sizeDisplay =
          db.size_mb >= 1
            ? formatSizeAt(db.size_mb, { unit: "mb", decimals: 1 })
            : formatSizeAt(db.size_bytes / 1024, { unit: "kb", decimals: 0 });
        return `
            <div class="data-db-item">
              <span class="data-db-name">${Utils.escapeHtml(I18n.label(DATABASE_LABELS, db.id))}</span>
              <div class="data-db-bar-container">
                <div class="data-db-bar" style="width: ${percentage}%"></div>
              </div>
              <span class="data-db-size">${sizeDisplay}</span>
            </div>
          `;
      })
      .join("");

    card.innerHTML = `
        <div class="data-total-bar">
          <span class="data-total-label">${Utils.escapeHtml(I18n.t("settings-data-total-storage"))}</span>
          <span class="data-total-value">${formatSizeAt(data.total_size_mb, { unit: "mb", decimals: 1 })}</span>
        </div>
        <div class="data-db-list">
          ${dbItemsHtml}
        </div>
      `;

    // Update config path display
    const pathDisplay = content.querySelector("#configPathDisplay");
    if (pathDisplay && data.config_path) {
      pathDisplay.textContent = data.config_path;
      pathDisplay.dir = "ltr";
      pathDisplay.title = data.config_path;
    }
  } catch {
    card.innerHTML = `<div class="data-stats-loading">${Utils.escapeHtml(I18n.t("settings-data-stats-load-failed"))}</div>`;
  }
}

/**
 * Export configuration to JSON file
 */
async function exportConfig() {
  try {
    const response = await fetch("/api/config");
    if (!response.ok) throw new Error(I18n.t("settings-data-config-fetch-failed"));

    const config = await response.json();

    // Remove sensitive data
    const exportData = { ...config };
    delete exportData.wallet_encrypted;
    delete exportData.wallet_nonce;

    const dataStr = JSON.stringify(exportData, null, 2);
    const blob = new Blob([dataStr], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `screenerbot-config-${new Date().toISOString().split("T")[0]}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);

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

    // Import each section separately to preserve wallet
    const sections = [
      "trader",
      "positions",
      "filtering",
      "swaps",
      "tokens",
      "rpc",
      "sol_price",
      "events",
      "services",
      "monitoring",
      "ohlcv",
      "gui",
    ];

    for (const section of sections) {
      if (imported[section]) {
        const response = await fetch(`/api/config/${section}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(imported[section]),
        });

        if (!response.ok) {
          console.warn(`Failed to import ${section} section`);
        }
      }
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
