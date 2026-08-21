/**
 * Updates Tab Module - Version checking and auto-update UI
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";

/**
 * Build Updates tab content - Modern design with version history
 * @param {object} dialog - SettingsDialog instance
 * @param {object} versionInfo - Current version info
 * @param {object} updateState - Global update state object
 * @returns {string} HTML content for updates tab
 */
export function buildUpdatesTab(dialog, versionInfo, updateState) {
  const { version, platform } = versionInfo;
  const state = updateState;

  // Build status section based on current state
  let statusSection = "";

  if (state.checking) {
    statusSection = buildCheckingState();
  } else if (state.error) {
    statusSection = buildErrorState(state.error);
  } else if (state.available && state.info) {
    statusSection = buildUpdateAvailableState(state, version, versionInfo);
  } else {
    statusSection = buildUpToDateState(version);
  }

  return `
      <div class="updates-container">
        <!-- Main Content -->
        <div class="updates-main">
          <!-- Status Card -->
          ${statusSection}
        </div>
        
        <!-- Sidebar -->
        <div class="updates-sidebar">
          <!-- Current Version Card -->
          <div class="updates-version-card">
            <div class="version-card-header">
              <div class="version-icon">
                <i class="icon-box"></i>
              </div>
              <div class="version-info">
                <h4>Current Installation</h4>
                <span class="version-number">v${version}</span>
              </div>
            </div>
            <div class="version-details">
              <div class="detail-row">
                <span class="detail-label">Platform</span>
                <span class="detail-value">${platform || "Unknown"}</span>
              </div>
            </div>
          </div>

          <!-- System Info Section -->
          <div class="updates-system-section">
            <div class="system-header">
              <div class="system-title">
                <i class="icon-info"></i>
                <span>Installation Details</span>
              </div>
            </div>
            <div class="system-details">
              <div class="detail-row">
                <span class="detail-label">Version</span>
                <span class="detail-value">v${version}</span>
              </div>
              <div class="detail-row">
                <span class="detail-label">Platform</span>
                <span class="detail-value">${platform || "Unknown"}</span>
              </div>
              <div class="detail-row">
                <span class="detail-label">Update Checks</span>
                <span class="detail-value channel-badge">Enabled</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    `;
}

/**
 * Build checking for updates state
 */
function buildCheckingState() {
  return `
      <div class="updates-status-card checking">
        <div class="status-visual">
          <div class="pulse-ring"></div>
          <div class="status-icon-wrapper">
            <i class="icon-refresh-cw spinning"></i>
          </div>
        </div>
        <div class="status-content">
          <h3>Checking for Updates</h3>
          <p>Connecting to update server...</p>
        </div>
      </div>
    `;
}

/**
 * Build error state
 */
function buildErrorState(error) {
  return `
      <div class="updates-status-card error">
        <div class="status-visual">
          <div class="status-icon-wrapper error">
            <i class="icon-triangle-alert"></i>
          </div>
        </div>
        <div class="status-content">
          <h3>Update Check Failed</h3>
          <p class="error-message">${Utils.escapeHtml(error)}</p>
        </div>
        <div class="status-actions">
          <button class="updates-btn secondary" id="retryUpdateBtn">
            <i class="icon-refresh-cw"></i>
            <span>Try Again</span>
          </button>
        </div>
      </div>
    `;
}

/**
 * Build update available state
 */
function buildUpdateAvailableState(state, currentVersion, versionInfo) {
  const info = state.info;
  const isDownloading = state.downloading;
  const isDownloaded = state.downloaded;
  const isHeadless = !window.__SCREENERBOT_GUI_MODE;
  const fileSize = info.file_size ? formatBytes(info.file_size) : null;

  let actionContent = "";

  if (isHeadless) {
    actionContent = `
        <div class="download-success">
          <div class="success-badge">
            <i class="icon-terminal"></i>
            <span>Headless Update Available</span>
          </div>
          <p class="install-hint">Run <code>sudo screenerbot-manager update</code> on the server. The manager verifies, applies, health-checks, and rolls back automatically on failure.</p>
        </div>
      `;
  } else if (isDownloaded) {
    actionContent = `
        <div class="download-success">
          <div class="success-badge">
            <i class="icon-circle-check"></i>
            <span>Ready to Install</span>
          </div>
          <p class="install-hint">${getInstallHint(versionInfo)}</p>
        </div>
        <div class="status-actions">
          <button class="updates-btn success" id="installUpdateBtn">
            <i class="icon-download"></i>
            <span>Open Installer & Quit</span>
          </button>
        </div>
      `;
  } else if (isDownloading) {
    actionContent = `
        <div class="download-progress">
          <div class="progress-header">
            <span class="progress-status" id="downloadStatusText">Downloading update...</span>
            <span class="progress-stats">
              <span id="download-speed-text"></span>
              <span id="download-percent-text">${Math.round(state.progress)}%</span>
            </span>
          </div>
          <div class="progress-track">
            <div class="progress-fill" id="downloadProgressBar" style="width: ${state.progress}%">
              <div class="progress-glow"></div>
            </div>
          </div>
          <div class="progress-footer">
            <span id="downloadSizeText">${fileSize ? `0 / ${fileSize}` : ""}</span>
            <span id="downloadEtaText"></span>
          </div>
        </div>
      `;
  } else {
    actionContent = `
        <div class="status-actions">
          <button class="updates-btn primary" id="downloadUpdateBtn">
            <i class="icon-download"></i>
            <span>Download Update</span>
            ${fileSize ? `<span class="btn-meta">(${fileSize})</span>` : ""}
          </button>
        </div>
      `;
  }

  return `
      <div class="updates-status-card available">
        <div class="update-badge">New Version Available</div>
        <div class="status-visual">
          <div class="version-transition">
            <span class="old-version">v${currentVersion}</span>
            <i class="icon-arrow-right"></i>
            <span class="new-version">v${info.version}</span>
          </div>
        </div>
        <div class="status-content">
          ${
            info.release_notes
              ? `
            <div class="release-notes-preview">
              <h4>What's New</h4>
              <div class="notes-text">${Utils.escapeHtml(info.release_notes)}</div>
            </div>
          `
              : ""
          }
        </div>
        ${actionContent}
      </div>
    `;
}

/**
 * Build up to date state
 */
function buildUpToDateState(version) {
  return `
      <div class="updates-status-card success">
        <div class="status-visual">
          <div class="status-icon-wrapper success">
            <i class="icon-circle-check"></i>
          </div>
        </div>
        <div class="status-content">
          <h3>You're Up to Date</h3>
          <p>ScreenerBot v${version} is the latest version.</p>
        </div>
        <div class="status-actions">
          <button class="updates-btn secondary" id="checkUpdatesBtn">
            <i class="icon-refresh-cw"></i>
            <span>Check Again</span>
          </button>
        </div>
      </div>
    `;
}

/**
 * Get platform-specific install hint
 */
function getInstallHint(versionInfo) {
  const platform = versionInfo.platform || "";
  if (platform.toLowerCase().includes("macos") || platform.toLowerCase().includes("darwin")) {
    return "The installer will open. Drag ScreenerBot to your Applications folder.";
  } else if (platform.toLowerCase().includes("windows")) {
    return "The installer will guide you through the update process.";
  } else if (platform.toLowerCase().includes("linux")) {
    return "The verified .deb installer will open in your system package manager.";
  }
  return "Follow the installer instructions to complete the update.";
}

/**
 * Format bytes to human readable size
 */
function formatBytes(bytes) {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

/**
 * Attach event handlers for Updates tab
 * @param {object} dialog - SettingsDialog instance with access to methods
 * @param {object} updateState - Global update state object
 * @param {Function} updateUICallback - Function to update the updates tab UI
 * @param {Function} startDownloadPoller - Function to start download polling
 * @param {Function} updateBadgeCallback - Function to update the updates badge
 */
export function attachUpdatesHandlers(
  dialog,
  updateState,
  updateUICallback,
  startDownloadPoller,
  updateBadgeCallback
) {
  if (!dialog.dialogEl) return;

  const checkBtn = dialog.dialogEl.querySelector("#checkUpdatesBtn");
  const retryBtn = dialog.dialogEl.querySelector("#retryUpdateBtn");
  const downloadBtn = dialog.dialogEl.querySelector("#downloadUpdateBtn");
  const installBtn = dialog.dialogEl.querySelector("#installUpdateBtn");

  // Check / Retry Handler
  const handleCheck = async () => {
    updateState.checking = true;
    updateState.error = null;
    updateUICallback();

    try {
      // Call the check API
      const response = await fetch("/api/updates/check");
      const data = await response.json();
      if (!response.ok || data.success === false) {
        throw new Error(data.error?.message || data.error || "Failed to check for updates");
      }

      updateState.checking = false;

      if (data.update_available) {
        updateState.available = true;
        updateState.info = data.update; // API returns 'update' not 'update_info'
      } else {
        updateState.available = false;
        updateState.info = null;
        updateState.downloaded = false;
        updateState.downloading = false;
      }
    } catch (err) {
      console.error("Update check failed:", err);
      updateState.checking = false;
      updateState.error = err.message || "Failed to check for updates";
    }

    updateBadgeCallback();
    updateUICallback();
  };

  if (checkBtn) checkBtn.addEventListener("click", handleCheck);
  if (retryBtn) retryBtn.addEventListener("click", handleCheck);

  // Download Handler
  if (downloadBtn) {
    downloadBtn.addEventListener("click", async () => {
      if (!updateState.info) return;

      updateState.downloading = true;
      updateState.progress = 0;
      updateState.downloadStartTime = Date.now();
      updateState.downloadedBytes = 0;
      updateUICallback();

      try {
        // Start download
        const response = await fetch("/api/updates/download", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ version: updateState.info.version }),
        });

        if (!response.ok) throw new Error("Failed to start download");

        // Start polling for progress
        startDownloadPoller();
      } catch (err) {
        console.error("Download start failed:", err);
        updateState.downloading = false;
        updateState.error = err.message;
        updateUICallback();
      }
    });
  }

  // Install Handler
  if (installBtn) {
    installBtn.addEventListener("click", async () => {
      const confirmResult = await ConfirmationDialog.show({
        title: "Install Update",
        message:
          "The verified operating-system installer will open, then ScreenerBot will quit cleanly. Complete the installer and reopen ScreenerBot. Continue?",
        confirmLabel: "Open Installer",
        cancelLabel: "Cancel",
        variant: "warning",
      });
      if (!confirmResult.confirmed) return;

      installBtn.disabled = true;
      const originalText = installBtn.innerHTML;
      installBtn.innerHTML = '<i class="icon-loader spinning"></i><span>Installing...</span>';

      try {
        const response = await fetch("/api/updates/install", {
          method: "POST",
        });
        const data = await response.json();

        if (!response.ok || !data.success) {
          throw new Error(data.error?.message || data.error || "Failed to open installer");
        }

        // Show success message
        Utils.showToast({
          type: "success",
          title: "Update Ready",
          message: "Installer opened. ScreenerBot will quit cleanly now.",
        });

        // Electron owns the Rust child process and must coordinate a graceful
        // quit. A raw backend exit is interpreted as a crash by the shell.
        setTimeout(() => {
          if (window.electronAPI?.quitForUpdate) {
            window.electronAPI.quitForUpdate();
          } else {
            dialog.close();
          }
        }, 1000);
      } catch (err) {
        console.error("Install failed:", err);
        installBtn.disabled = false;
        installBtn.innerHTML = originalText;

        Utils.showToast({
          type: "error",
          title: "Failed to Open Installer",
          message: err.message || "Please try downloading again.",
        });
      }
    });
  }
}
