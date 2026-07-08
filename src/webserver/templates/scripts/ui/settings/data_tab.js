/**
 * Data Tab Module - Database stats, config management, data cleanup
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";

/**
 * Build Data tab HTML
 */
export function buildDataTab() {
  return `
      <!-- Database Overview Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-database"></i>
          Database Storage
        </h3>
        <p class="settings-section-description">
          Overview of all databases storing your trading data, positions, and historical information.
        </p>
        
        <div class="data-overview-card" id="dataOverviewCard">
          <div class="data-stats-loading"><i class="icon-loader"></i> Loading database statistics...</div>
        </div>
      </div>

      <!-- Configuration Backup Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-settings"></i>
          Configuration Management
        </h3>
        <p class="settings-section-description">
          Export, import, and manage your bot configuration. Keep backups before making major changes.
        </p>
        
        <div class="settings-group">
          <div class="config-actions-row">
            <button id="exportConfigBtn" class="btn btn-primary">
              <i class="icon-download"></i>
              Export Config
            </button>
            <button id="importConfigBtn" class="btn btn-secondary">
              <i class="icon-upload"></i>
              Import Config
            </button>
            <button id="resetConfigBtn" class="btn btn-warning">
              <i class="icon-refresh-cw"></i>
              Reset to Defaults
            </button>
          </div>
          <input type="file" id="configFileInput" accept=".json,.toml" style="display: none;" />
          
          <div class="config-info-box">
            <div class="config-info-item">
              <span class="config-info-label">Config Location</span>
              <span class="config-info-value" id="configPathDisplay">Loading...</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Data Cleanup Section -->
      <div class="settings-section">
        <h3 class="settings-section-title">
          <i class="icon-trash-2"></i>
          Data Cleanup
        </h3>
        <p class="settings-section-description">
          Free up disk space by removing old or unused data. These actions cannot be undone.
        </p>
        
        <div class="settings-group">
          <div class="settings-field">
            <div class="settings-field-info">
              <label>OHLCV Data Cleanup</label>
              <span class="settings-field-hint">
                Remove candlestick data for tokens that haven't been active for the specified time.
              </span>
            </div>
            <div class="settings-field-control data-action-group">
              <input type="number" id="cleanupHours" class="settings-input small" value="24" min="1" max="720" />
              <span class="input-suffix">hours</span>
              <button id="cleanupOhlcvBtn" class="btn btn-warning btn-sm">
                <i class="icon-trash-2"></i>
                Cleanup OHLCV
              </button>
            </div>
          </div>
          
          <div class="settings-field">
            <div class="settings-field-info">
              <label>Clear All OHLCV Cache</label>
              <span class="settings-field-hint">
                Wipe all cached candlestick data and re-fetch every monitored token from scratch. Use if charts look wrong or after a data logic update.
              </span>
            </div>
            <div class="settings-field-control">
              <button id="clearOhlcvCacheBtn" class="btn btn-warning btn-sm">
                <i class="icon-trash-2"></i>
                Clear OHLCV Cache
              </button>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>UI State Cache</label>
              <span class="settings-field-hint">
                Clear saved table preferences, filter states, and view settings.
              </span>
            </div>
            <div class="settings-field-control">
              <button id="clearUiStateBtn" class="btn btn-secondary btn-sm">
                <i class="icon-refresh-cw"></i>
                Clear UI Cache
              </button>
            </div>
          </div>

          <div class="settings-field">
            <div class="settings-field-info">
              <label>Open Data Folder</label>
              <span class="settings-field-hint">
                Open the folder containing all ScreenerBot data in your file manager.
              </span>
            </div>
            <div class="settings-field-control">
              <button id="openDataFolderBtn" class="btn btn-secondary btn-sm">
                <i class="icon-folder"></i>
                Open Folder
              </button>
            </div>
          </div>
        </div>
      </div>
    `;
}

/**
 * Attach handlers for Data tab
 */
export function attachDataHandlers(dialog, content, pathsInfo) {
  // Load data overview
  loadDataOverview(content);

  // Display config path with copy on click
  if (pathsInfo?.config_path) {
    const pathDisplay = content.querySelector("#configPathDisplay");
    if (pathDisplay) {
      pathDisplay.textContent = pathsInfo.config_path;
      pathDisplay.title = "Click to copy path";
      pathDisplay.addEventListener("click", async () => {
        try {
          await Utils.copyToClipboard(pathsInfo.config_path);
          Utils.showToast("Config path copied to clipboard", "success");
        } catch {
          Utils.showToast("Failed to copy path", "error");
        }
      });
    }
  }

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

  // Trading preset buttons - handle both card click and button click
  content.querySelectorAll(".preset-card").forEach((card) => {
    card.addEventListener("click", (e) => {
      // Don't trigger if clicking the button (let button handler work)
      if (e.target.closest(".preset-apply-btn")) return;
      const preset = card.dataset.preset;
      applyPreset(preset);
    });
  });

  content.querySelectorAll(".preset-apply-btn").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      const preset = btn.dataset.preset;
      applyPreset(preset);
    });
  });

  // OHLCV cleanup button
  const cleanupBtn = content.querySelector("#cleanupOhlcvBtn");
  const hoursInput = content.querySelector("#cleanupHours");

  if (cleanupBtn && hoursInput) {
    cleanupBtn.addEventListener("click", async () => {
      const hours = parseInt(hoursInput.value, 10);
      if (isNaN(hours) || hours < 1) {
        Utils.showToast("Invalid hours value", "error");
        return;
      }

      const confirmResult = await ConfirmationDialog.show({
        title: "Delete OHLCV Data",
        message: `Delete OHLCV data for tokens inactive for more than ${hours} hours?`,
        confirmLabel: "Delete",
        cancelLabel: "Cancel",
        variant: "danger",
      });
      if (!confirmResult.confirmed) return;

      cleanupBtn.disabled = true;
      cleanupBtn.innerHTML = '<i class="icon-loader spin"></i> Cleaning...';

      try {
        const response = await fetch("/api/ohlcv/cleanup", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ inactive_hours: hours }),
        });

        if (response.ok) {
          const data = await response.json();
          Utils.showToast(`Cleaned up ${data.deleted_count} inactive tokens`, "success");
          loadDataOverview(content);
        } else {
          Utils.showToast("Cleanup failed", "error");
        }
      } catch (err) {
        Utils.showToast("Cleanup failed: " + err.message, "error");
      } finally {
        cleanupBtn.disabled = false;
        cleanupBtn.innerHTML = '<i class="icon-trash-2"></i> Cleanup OHLCV';
      }
    });
  }

  // Clear all OHLCV cache button
  const clearOhlcvBtn = content.querySelector("#clearOhlcvCacheBtn");
  if (clearOhlcvBtn) {
    clearOhlcvBtn.addEventListener("click", async () => {
      const confirmResult = await ConfirmationDialog.show({
        title: "Clear All OHLCV Cache",
        message:
          "Wipe all cached candlestick data for every token? Monitored tokens will re-fetch their history from scratch. This cannot be undone.",
        confirmLabel: "Clear",
        cancelLabel: "Cancel",
        variant: "danger",
      });
      if (!confirmResult.confirmed) return;

      clearOhlcvBtn.disabled = true;
      clearOhlcvBtn.innerHTML = '<i class="icon-loader spin"></i> Clearing...';

      try {
        const response = await fetch("/api/ohlcv/cache/clear", { method: "POST" });
        if (response.ok) {
          const data = await response.json();
          Utils.showToast(
            `Cleared ${data.candles_deleted} candles across ${data.tokens_reset} tokens; re-fetching`,
            "success",
          );
          loadDataOverview(content);
        } else {
          Utils.showToast("Failed to clear OHLCV cache", "error");
        }
      } catch (err) {
        Utils.showToast("Failed to clear OHLCV cache: " + err.message, "error");
      } finally {
        clearOhlcvBtn.disabled = false;
        clearOhlcvBtn.innerHTML = '<i class="icon-trash-2"></i> Clear OHLCV Cache';
      }
    });
  }

  // Clear UI state button
  const clearUiBtn = content.querySelector("#clearUiStateBtn");
  if (clearUiBtn) {
    clearUiBtn.addEventListener("click", async () => {
      const confirmResult = await ConfirmationDialog.show({
        title: "Clear UI State",
        message:
          "Clear all saved UI preferences? This will reset table columns, filters, and view settings.",
        confirmLabel: "Clear",
        cancelLabel: "Cancel",
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
      Utils.showToast(`Cleared ${keysToRemove.length} cached UI settings`, "success");
    });
  }

  // Open data folder button
  const openFolderBtn = content.querySelector("#openDataFolderBtn");
  if (openFolderBtn) {
    openFolderBtn.addEventListener("click", async () => {
      try {
        const response = await fetch("/api/system/paths/open-data", { method: "POST" });
        if (response.ok) {
          Utils.showToast("Data folder opened", "success");
        } else {
          Utils.showToast("Failed to open folder", "error");
        }
      } catch (err) {
        Utils.showToast("Failed to open folder: " + err.message, "error");
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
    if (!response.ok) throw new Error("Failed to load stats");

    const data = await response.json();
    const maxSize = Math.max(...data.databases.map((db) => db.size_bytes), 1);

    const dbItemsHtml = data.databases
      .filter((db) => db.exists)
      .map((db) => {
        const percentage = (db.size_bytes / maxSize) * 100;
        const sizeDisplay =
          db.size_mb >= 1
            ? `${db.size_mb.toFixed(1)} MB`
            : `${(db.size_bytes / 1024).toFixed(0)} KB`;
        return `
            <div class="data-db-item">
              <span class="data-db-name">${db.name}</span>
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
          <span class="data-total-label">Total Database Storage</span>
          <span class="data-total-value">${data.total_size_mb.toFixed(1)} MB</span>
        </div>
        <div class="data-db-list">
          ${dbItemsHtml}
        </div>
      `;

    // Update config path display
    const pathDisplay = content.querySelector("#configPathDisplay");
    if (pathDisplay && data.config_path) {
      pathDisplay.textContent = data.config_path;
      pathDisplay.title = data.config_path;
    }
  } catch {
    card.innerHTML = '<div class="data-stats-loading">Failed to load database statistics</div>';
  }
}

/**
 * Export configuration to JSON file
 */
async function exportConfig() {
  try {
    const response = await fetch("/api/config");
    if (!response.ok) throw new Error("Failed to fetch config");

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

    Utils.showToast("Configuration exported successfully", "success");
  } catch (err) {
    Utils.showToast("Failed to export config: " + err.message, "error");
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
      title: "Import Configuration",
      message:
        "Import this configuration? Current settings will be overwritten. Wallet credentials will be preserved.",
      confirmLabel: "Import",
      cancelLabel: "Cancel",
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

    Utils.showToast(
      "Configuration imported successfully. Some changes may require restart.",
      "success",
    );
  } catch (err) {
    Utils.showToast("Failed to import config: " + err.message, "error");
  }
}

/**
 * Reset configuration to defaults
 */
async function resetConfig() {
  const confirmResult = await ConfirmationDialog.show({
    title: "Reset Configuration",
    message:
      "Reset all settings to defaults? Your wallet credentials will be preserved, but all other settings will be reset.",
    confirmLabel: "Reset",
    cancelLabel: "Cancel",
    variant: "danger",
  });
  if (!confirmResult.confirmed) return;

  try {
    const response = await fetch("/api/config/reset", { method: "POST" });

    if (response.ok) {
      Utils.showToast("Configuration reset to defaults", "success");
    } else {
      const data = await response.json();
      Utils.showToast("Failed to reset config: " + (data.error || "Unknown error"), "error");
    }
  } catch (err) {
    Utils.showToast("Failed to reset config: " + err.message, "error");
  }
}

/**
 * Apply trading preset
 */
async function applyPreset(presetName) {
  const presets = {
    conservative: {
      trader: {
        max_open_positions: 2,
        trade_size_sol: 0.005,
        roi_target_percent: 15,
      },
      filtering: {
        min_liquidity_usd: 10000,
      },
      positions: {
        stop_loss_percent: 25,
      },
    },
    moderate: {
      trader: {
        max_open_positions: 5,
        trade_size_sol: 0.01,
        roi_target_percent: 20,
      },
      filtering: {
        min_liquidity_usd: 5000,
      },
      positions: {
        stop_loss_percent: 20,
      },
    },
    aggressive: {
      trader: {
        max_open_positions: 10,
        trade_size_sol: 0.02,
        roi_target_percent: 30,
      },
      filtering: {
        min_liquidity_usd: 1000,
      },
      positions: {
        stop_loss_percent: 15,
      },
    },
  };

  const preset = presets[presetName];
  if (!preset) {
    Utils.showToast("Unknown preset", "error");
    return;
  }

  const presetDisplayName = presetName.charAt(0).toUpperCase() + presetName.slice(1);
  const confirmResult = await ConfirmationDialog.show({
    title: "Apply Trading Preset",
    message: `Apply ${presetDisplayName} trading preset? This will update your trader, filtering, and position settings.`,
    confirmLabel: "Apply",
    cancelLabel: "Cancel",
    variant: "warning",
  });
  if (!confirmResult.confirmed) return;

  try {
    // Apply each section
    for (const [section, values] of Object.entries(preset)) {
      const response = await fetch(`/api/config/${section}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(values),
      });

      if (!response.ok) {
        console.warn(`Failed to apply ${section} preset`);
      }
    }

    Utils.showToast(`${presetDisplayName} preset applied successfully`, "success");
  } catch (err) {
    Utils.showToast("Failed to apply preset: " + err.message, "error");
  }
}
