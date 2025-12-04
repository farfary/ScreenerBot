/**
 * Updates Page Module
 * Handles update checking, downloading, and installation
 */

import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $, $$ } from "../core/dom.js";
import * as Utils from "../core/utils.js";

// =============================================================================
// State
// =============================================================================

let updateState = null;
let statusPoller = null;

// =============================================================================
// DOM Elements
// =============================================================================

const getElements = () => ({
  // Version info
  currentVersion: $("#currentVersion"),
  platform: $("#platform"),
  lastCheck: $("#lastCheck"),

  // Status sections
  noUpdateSection: $("#noUpdateSection"),
  updateAvailableSection: $("#updateAvailableSection"),
  downloadingSection: $("#downloadingSection"),
  downloadCompleteSection: $("#downloadCompleteSection"),
  errorSection: $("#errorSection"),
  releaseNotesSection: $("#releaseNotesSection"),

  // Dynamic content
  newVersion: $("#newVersion"),
  downloadStatus: $("#downloadStatus"),
  downloadProgress: $("#downloadProgress"),
  downloadPercent: $("#downloadPercent"),
  downloadedVersion: $("#downloadedVersion"),
  errorMessage: $("#errorMessage"),
  releaseNotes: $("#releaseNotes"),

  // Buttons
  checkUpdatesBtn: $("#checkUpdatesBtn"),
  downloadBtn: $("#downloadBtn"),
  installBtn: $("#installBtn"),
  retryBtn: $("#retryBtn"),
});

// =============================================================================
// API Calls
// =============================================================================

async function fetchVersion() {
  const response = await fetch("/api/version");
  const data = await response.json();
  return data; // Returns {version, build_number, platform} directly
}

async function checkForUpdates() {
  const response = await fetch("/api/updates/check");
  const data = await response.json();
  return data; // Returns {update_available, current_version, update?, last_check?} directly
}

async function startDownload() {
  const response = await fetch("/api/updates/download", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({}),
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error?.message || "Download request failed");
  }
  return await response.json();
}

async function fetchStatus() {
  const response = await fetch("/api/updates/status");
  const data = await response.json();
  return data?.state; // Returns {state: {...}} so extract state
}

async function installUpdate() {
  const response = await fetch("/api/updates/install", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
  });
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error?.message || "Install request failed");
  }
  return await response.json();
}

// =============================================================================
// UI Updates
// =============================================================================

function hideAllSections(els) {
  els.noUpdateSection.style.display = "none";
  els.updateAvailableSection.style.display = "none";
  els.downloadingSection.style.display = "none";
  els.downloadCompleteSection.style.display = "none";
  els.errorSection.style.display = "none";
  els.releaseNotesSection.style.display = "none";
}

function showSection(els, sectionName) {
  hideAllSections(els);
  switch (sectionName) {
    case "noUpdate":
      els.noUpdateSection.style.display = "flex";
      break;
    case "updateAvailable":
      els.updateAvailableSection.style.display = "flex";
      break;
    case "downloading":
      els.downloadingSection.style.display = "flex";
      break;
    case "downloadComplete":
      els.downloadCompleteSection.style.display = "flex";
      break;
    case "error":
      els.errorSection.style.display = "flex";
      break;
  }
}

function updateVersionInfo(els, versionData) {
  if (!versionData) return;
  const version = versionData.version || "--";
  const buildNumber = versionData.build_number;
  if (buildNumber && buildNumber !== "0") {
    els.currentVersion.textContent = `${version} (build ${buildNumber})`;
  } else {
    els.currentVersion.textContent = version;
  }
  els.platform.textContent = versionData.platform || "--";
}

function updateFromState(els, state) {
  if (!state) return;

  // Update last check time
  if (state.last_check) {
    const date = new Date(state.last_check);
    els.lastCheck.textContent = date.toLocaleString();
  }

  // Handle download progress
  const progress = state.download_progress;
  if (progress) {
    if (progress.error) {
      showSection(els, "error");
      els.errorMessage.textContent = progress.error;
      return;
    }

    if (progress.completed && progress.downloaded_path) {
      showSection(els, "downloadComplete");
      if (state.available_update) {
        els.downloadedVersion.textContent = state.available_update.version;
      }
      return;
    }

    if (progress.downloading) {
      showSection(els, "downloading");
      els.downloadProgress.style.width = `${progress.progress_percent}%`;
      els.downloadPercent.textContent = Math.round(progress.progress_percent);

      const downloaded = Utils.formatBytes(progress.bytes_downloaded);
      const total = Utils.formatBytes(progress.total_bytes);
      els.downloadStatus.textContent = `${downloaded} / ${total}`;
      return;
    }
  }

  // Handle update available
  if (state.available_update) {
    showSection(els, "updateAvailable");
    els.newVersion.textContent = state.available_update.version;

    if (state.available_update.release_notes) {
      els.releaseNotesSection.style.display = "block";
      els.releaseNotes.textContent = state.available_update.release_notes;
    }
    return;
  }

  // No update available
  showSection(els, "noUpdate");
}

// =============================================================================
// Event Handlers
// =============================================================================

async function handleCheckUpdates(els) {
  els.checkUpdatesBtn.disabled = true;
  els.checkUpdatesBtn.innerHTML = '<i class="icon-loader spinning"></i> Checking...';

  try {
    const result = await checkForUpdates();
    // Bot API returns data directly, not wrapped in {success, data}
    if (result && result.update_available) {
      updateState = {
        available_update: result.update,
        last_check: result.last_check,
        download_progress: {},
      };
      updateFromState(els, updateState);
      els.lastCheck.textContent = "Just now";
    } else if (result && !result.update_available) {
      // No update available
      showSection(els, "noUpdate");
      if (result.last_check) {
        const date = new Date(result.last_check);
        els.lastCheck.textContent = date.toLocaleString();
      } else {
        els.lastCheck.textContent = "Just now";
      }
    } else {
      // Unexpected response
      showSection(els, "error");
      els.errorMessage.textContent = "Unexpected response from server";
    }
  } catch (err) {
    showSection(els, "error");
    els.errorMessage.textContent = err.message;
  } finally {
    els.checkUpdatesBtn.disabled = false;
    els.checkUpdatesBtn.innerHTML = '<i class="icon-refresh-cw"></i> Check for Updates';
  }
}

async function handleDownload(els) {
  els.downloadBtn.disabled = true;

  try {
    await startDownload();
    showSection(els, "downloading");
    // Poller is already managed by lifecycle, will be running
  } catch (err) {
    showSection(els, "error");
    els.errorMessage.textContent = err.message;
  } finally {
    els.downloadBtn.disabled = false;
  }
}

async function handleInstall(els) {
  els.installBtn.disabled = true;
  els.installBtn.innerHTML = '<i class="icon-loader spinning"></i> Opening...';

  try {
    await installUpdate();
    els.installBtn.innerHTML = '<i class="icon-check"></i> Opened';
  } catch (err) {
    showSection(els, "error");
    els.errorMessage.textContent = err.message;
    els.installBtn.disabled = false;
    els.installBtn.innerHTML = '<i class="icon-package"></i> Install Update';
  }
}

// =============================================================================
// Lifecycle
// =============================================================================

function createLifecycle() {
  return {
    async init(ctx) {
      const els = getElements();

      // Create status poller
      statusPoller = new Poller(
        async () => {
          const state = await fetchStatus();
          if (state) {
            updateState = state;
            updateFromState(els, state);

            // Stop polling if download complete or error
            if (state.download_progress?.completed || state.download_progress?.error) {
              statusPoller.stop();
            }
          }
        },
        {
          label: "UpdatesStatus",
          getInterval: () => 1000, // Poll every second during download
        }
      );

      ctx.managePoller(statusPoller);

      // Button handlers
      els.checkUpdatesBtn?.addEventListener("click", () => handleCheckUpdates(els));
      els.downloadBtn?.addEventListener("click", () => handleDownload(els));
      els.installBtn?.addEventListener("click", () => handleInstall(els));
      els.retryBtn?.addEventListener("click", () => handleCheckUpdates(els));
    },

    async activate(ctx) {
      const els = getElements();

      // Fetch initial version info
      try {
        const versionData = await fetchVersion();
        updateVersionInfo(els, versionData);
      } catch (err) {
        console.error("Failed to fetch version:", err);
        showSection(els, "error");
        els.errorMessage.textContent = `Failed to load version info: ${err.message}`;
      }

      // Fetch initial status
      try {
        const state = await fetchStatus();
        if (state) {
          updateState = state;
          updateFromState(els, state);
        } else {
          // No error but no state - show no update section
          showSection(els, "noUpdate");
        }
      } catch (err) {
        console.error("Failed to fetch status:", err);
        // Don't overwrite existing error from version fetch
        if (els.errorSection.style.display !== "flex") {
          showSection(els, "error");
          els.errorMessage.textContent = `Failed to check update status: ${err.message}`;
        }
      }
    },

    deactivate() {
      statusPoller?.stop();
    },

    dispose() {
      statusPoller = null;
      updateState = null;
    },
  };
}

// Register page
registerPage("updates", createLifecycle());

export { createLifecycle };
