/**
 * Config Import/Export Dialog Component
 *
 * Advanced dialogs for importing and exporting bot configuration with:
 * - Section selection (choose which config sections to import/export)
 * - Preview with field-level changes
 * - Validation with error display
 * - Merge vs replace modes
 * - Partial config support
 */

import { $, on, off, create, show, hide } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { playPanelOpen, playPanelClose, playSuccess, playError } from "../core/sounds.js";
import { requestManager } from "../core/request_manager.js";

// ============================================================================
// SECTION METADATA
// ============================================================================

const SECTION_META = {
  rpc: { label: "RPC", icon: "icon-satellite", hint: "RPC endpoints and connection settings" },
  trader: { label: "Auto Trader", icon: "icon-briefcase", hint: "Trading rules and automation" },
  positions: {
    label: "Positions",
    icon: "icon-chart-candlestick",
    hint: "Position management settings",
  },
  filtering: {
    label: "Filtering",
    icon: "icon-target",
    hint: "Token filtering rules and thresholds",
  },
  swaps: { label: "Swaps", icon: "icon-repeat", hint: "Swap execution settings" },
  tokens: { label: "Tokens", icon: "icon-coins", hint: "Token discovery and data sources" },
  sol_price: { label: "SOL Price", icon: "icon-sun", hint: "SOL price service configuration" },
  events: { label: "Events", icon: "icon-radio", hint: "Event recording settings" },
  services: { label: "Services", icon: "icon-wrench", hint: "Background service settings" },
  monitoring: {
    label: "Monitoring",
    icon: "icon-trending-up",
    hint: "System monitoring configuration",
  },
  ohlcv: { label: "OHLCV", icon: "icon-clock", hint: "Candlestick data settings" },
  gui: { label: "GUI", icon: "icon-layout-dashboard", hint: "Dashboard and UI settings" },
  telegram: { label: "Telegram", icon: "icon-send", hint: "Telegram bot configuration" },
};

const SECTION_ORDER = [
  "rpc",
  "trader",
  "positions",
  "filtering",
  "swaps",
  "tokens",
  "sol_price",
  "events",
  "services",
  "monitoring",
  "ohlcv",
  "gui",
  "telegram",
];

// ============================================================================
// EXPORT DIALOG
// ============================================================================

export class ConfigExportDialog {
  static activeDialog = null;

  static async show() {
    if (ConfigExportDialog.activeDialog) {
      ConfigExportDialog.activeDialog.destroy();
    }

    return new Promise((resolve) => {
      const dialog = new ConfigExportDialog(resolve);
      ConfigExportDialog.activeDialog = dialog;
      dialog.render();
    });
  }

  constructor(resolver) {
    this.resolver = resolver;
    this.element = null;
    this.backdrop = null;
    this.selectedSections = new Set(SECTION_ORDER); // All selected by default
    this.includeGui = true;
    this.includeMetadata = true;
    this.isExporting = false;
  }

  render() {
    // Create backdrop
    this.backdrop = create("div", { className: "config-dialog-backdrop" });

    // Create dialog
    this.element = create("div", {
      className: "config-dialog config-export-dialog",
      role: "dialog",
      ariaModal: "true",
    });

    this.element.innerHTML = `
      <div class="config-dialog-header">
        <div class="config-dialog-title">
          <i class="icon-download"></i>
          <span>Export Configuration</span>
        </div>
        <button type="button" class="config-dialog-close" aria-label="Close">
          <i class="icon-x"></i>
        </button>
      </div>
      <div class="config-dialog-body">
        <div class="config-dialog-intro">
          <p>Select which configuration sections to export. The exported file can be imported later to restore or share settings.</p>
        </div>
        <div class="config-dialog-sections">
          <div class="config-dialog-sections-header">
            <span class="config-dialog-sections-title">Sections</span>
            <div class="config-dialog-sections-actions">
              <button type="button" class="config-dialog-link-btn" data-action="select-all">Select All</button>
              <span class="config-dialog-sep">•</span>
              <button type="button" class="config-dialog-link-btn" data-action="select-none">Select None</button>
            </div>
          </div>
          <div class="config-dialog-sections-grid" id="exportSectionsGrid"></div>
        </div>
        <div class="config-dialog-options">
          <label class="config-dialog-option">
            <input type="checkbox" id="exportIncludeMetadata" checked />
            <span>Include export timestamp</span>
          </label>
        </div>
      </div>
      <div class="config-dialog-footer">
        <div class="config-dialog-footer-info">
          <span id="exportSelectionCount">${this.selectedSections.size} sections selected</span>
        </div>
        <div class="config-dialog-footer-actions">
          <button type="button" class="config-dialog-btn secondary" data-action="cancel">Cancel</button>
          <button type="button" class="config-dialog-btn primary" data-action="export" id="exportBtn">
            <i class="icon-download"></i> Export
          </button>
        </div>
      </div>
    `;

    this._renderSections();
    this._attachEventListeners();

    document.body.appendChild(this.backdrop);
    document.body.appendChild(this.element);

    requestAnimationFrame(() => {
      this.backdrop.classList.add("visible");
      this.element.classList.add("visible");
      playPanelOpen();
    });
  }

  _renderSections() {
    const grid = this.element.querySelector("#exportSectionsGrid");
    grid.innerHTML = "";

    for (const sectionId of SECTION_ORDER) {
      const meta = SECTION_META[sectionId] || { label: sectionId, icon: "icon-settings", hint: "" };
      const isSelected = this.selectedSections.has(sectionId);

      const item = create("label", { className: "config-section-item" + (isSelected ? " selected" : "") });
      item.innerHTML = `
        <input type="checkbox" value="${sectionId}" ${isSelected ? "checked" : ""} />
        <div class="config-section-item-content">
          <div class="config-section-item-icon"><i class="${meta.icon}"></i></div>
          <div class="config-section-item-info">
            <span class="config-section-item-label">${Utils.escapeHtml(meta.label)}</span>
            <span class="config-section-item-hint">${Utils.escapeHtml(meta.hint)}</span>
          </div>
        </div>
      `;

      const checkbox = item.querySelector("input");
      on(checkbox, "change", () => {
        if (checkbox.checked) {
          this.selectedSections.add(sectionId);
          item.classList.add("selected");
        } else {
          this.selectedSections.delete(sectionId);
          item.classList.remove("selected");
        }
        this._updateSelectionCount();
      });

      grid.appendChild(item);
    }
  }

  _updateSelectionCount() {
    const countEl = this.element.querySelector("#exportSelectionCount");
    const exportBtn = this.element.querySelector("#exportBtn");
    const count = this.selectedSections.size;

    countEl.textContent = `${count} section${count === 1 ? "" : "s"} selected`;
    exportBtn.disabled = count === 0 || this.isExporting;
  }

  _attachEventListeners() {
    // Close button
    const closeBtn = this.element.querySelector(".config-dialog-close");
    on(closeBtn, "click", () => this._handleCancel());

    // Backdrop click
    on(this.backdrop, "click", () => this._handleCancel());

    // Cancel button
    const cancelBtn = this.element.querySelector('[data-action="cancel"]');
    on(cancelBtn, "click", () => this._handleCancel());

    // Export button
    const exportBtn = this.element.querySelector('[data-action="export"]');
    on(exportBtn, "click", () => this._handleExport());

    // Select all/none
    const selectAllBtn = this.element.querySelector('[data-action="select-all"]');
    on(selectAllBtn, "click", () => {
      this.selectedSections = new Set(SECTION_ORDER);
      this._renderSections();
      this._updateSelectionCount();
    });

    const selectNoneBtn = this.element.querySelector('[data-action="select-none"]');
    on(selectNoneBtn, "click", () => {
      this.selectedSections.clear();
      this._renderSections();
      this._updateSelectionCount();
    });

    // Metadata checkbox
    const metadataCheckbox = this.element.querySelector("#exportIncludeMetadata");
    on(metadataCheckbox, "change", () => {
      this.includeMetadata = metadataCheckbox.checked;
    });

    // Keyboard
    this._keydownHandler = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this._handleCancel();
      } else if (e.key === "Enter" && !this.isExporting && this.selectedSections.size > 0) {
        e.preventDefault();
        this._handleExport();
      }
    };
    document.addEventListener("keydown", this._keydownHandler);
  }

  async _handleExport() {
    if (this.isExporting || this.selectedSections.size === 0) return;

    this.isExporting = true;
    const exportBtn = this.element.querySelector("#exportBtn");
    exportBtn.disabled = true;
    exportBtn.innerHTML = '<i class="icon-loader spin"></i> Exporting...';

    try {
      const response = await requestManager.fetch("/api/config/export", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          sections: Array.from(this.selectedSections),
          include_gui: this.selectedSections.has("gui"),
          include_metadata: this.includeMetadata,
        }),
        priority: "high",
      });

      if (!response || !response.config) {
        throw new Error("Invalid response from server");
      }

      // Download the config
      const json = JSON.stringify(response.config, null, 2);
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      const date = new Date().toISOString().split("T")[0];
      a.download = `screenerbot-config-${date}.json`;
      a.click();
      URL.revokeObjectURL(url);

      playSuccess();
      Utils.showToast({
        type: "success",
        title: "Configuration Exported",
        message: `Exported ${response.sections.length} section(s)`,
      });

      this.destroy();
      this.resolver({ exported: true, sections: response.sections });
    } catch (error) {
      playError();
      Utils.showToast({
        type: "error",
        title: "Export Failed",
        message: error.message || "Failed to export configuration",
      });
      this.isExporting = false;
      exportBtn.disabled = false;
      exportBtn.innerHTML = '<i class="icon-download"></i> Export';
    }
  }

  _handleCancel() {
    playPanelClose();
    this.destroy();
    this.resolver({ exported: false });
  }

  destroy() {
    if (this._keydownHandler) {
      document.removeEventListener("keydown", this._keydownHandler);
    }

    if (this.element) {
      this.element.classList.remove("visible");
      this.backdrop.classList.remove("visible");

      setTimeout(() => {
        this.element?.remove();
        this.backdrop?.remove();
      }, 200);
    }

    if (ConfigExportDialog.activeDialog === this) {
      ConfigExportDialog.activeDialog = null;
    }
  }
}

// ============================================================================
// IMPORT DIALOG
// ============================================================================

export class ConfigImportDialog {
  static activeDialog = null;

  static async show() {
    if (ConfigImportDialog.activeDialog) {
      ConfigImportDialog.activeDialog.destroy();
    }

    return new Promise((resolve) => {
      const dialog = new ConfigImportDialog(resolve);
      ConfigImportDialog.activeDialog = dialog;
      dialog.render();
    });
  }

  constructor(resolver) {
    this.resolver = resolver;
    this.element = null;
    this.backdrop = null;
    this.configData = null;
    this.previewData = null;
    this.selectedSections = new Set();
    this.mergeMode = true; // true = merge, false = replace
    this.saveToDisk = true;
    this.isLoading = false;
    this.isImporting = false;
    this.step = "upload"; // 'upload' | 'preview'
  }

  render() {
    this.backdrop = create("div", { className: "config-dialog-backdrop" });

    this.element = create("div", {
      className: "config-dialog config-import-dialog",
      role: "dialog",
      ariaModal: "true",
    });

    this._renderUploadStep();
    this._attachEventListeners();

    document.body.appendChild(this.backdrop);
    document.body.appendChild(this.element);

    requestAnimationFrame(() => {
      this.backdrop.classList.add("visible");
      this.element.classList.add("visible");
      playPanelOpen();
    });
  }

  _renderUploadStep() {
    this.step = "upload";
    this.element.innerHTML = `
      <div class="config-dialog-header">
        <div class="config-dialog-title">
          <i class="icon-upload"></i>
          <span>Import Configuration</span>
        </div>
        <button type="button" class="config-dialog-close" aria-label="Close">
          <i class="icon-x"></i>
        </button>
      </div>
      <div class="config-dialog-body">
        <div class="config-dialog-intro">
          <p>Upload a previously exported configuration file. You'll be able to preview and select which sections to import.</p>
        </div>
        <div class="config-import-dropzone" id="importDropzone">
          <div class="config-import-dropzone-content">
            <i class="icon-file-code"></i>
            <span class="config-import-dropzone-title">Drop config file here</span>
            <span class="config-import-dropzone-hint">or click to browse</span>
          </div>
          <input type="file" id="importFileInput" accept=".json,application/json" hidden />
        </div>
        <div class="config-import-file-info" id="importFileInfo" hidden>
          <i class="icon-file-check"></i>
          <span id="importFileName"></span>
          <button type="button" class="config-import-clear-btn" id="importClearBtn">
            <i class="icon-x"></i>
          </button>
        </div>
        <div class="config-import-loading" id="importLoading" hidden>
          <i class="icon-loader spin"></i>
          <span>Analyzing configuration...</span>
        </div>
      </div>
      <div class="config-dialog-footer">
        <div class="config-dialog-footer-info"></div>
        <div class="config-dialog-footer-actions">
          <button type="button" class="config-dialog-btn secondary" data-action="cancel">Cancel</button>
          <button type="button" class="config-dialog-btn primary" data-action="next" id="nextBtn" disabled>
            <i class="icon-arrow-right"></i> Preview
          </button>
        </div>
      </div>
    `;

    this._attachUploadListeners();
  }

  _renderPreviewStep() {
    this.step = "preview";
    const { sections, warnings, total_changes, valid } = this.previewData;

    // Auto-select all valid sections that have data
    this.selectedSections = new Set(
      sections.filter((s) => s.present && s.valid).map((s) => s.name)
    );

    const warningsHtml = warnings.length
      ? `<div class="config-import-warnings">
          <div class="config-import-warnings-header">
            <i class="icon-triangle-alert"></i>
            <span>${warnings.length} Warning${warnings.length === 1 ? "" : "s"}</span>
          </div>
          ${warnings.map((w) => `<div class="config-import-warning-item">${Utils.escapeHtml(w)}</div>`).join("")}
        </div>`
      : "";

    this.element.innerHTML = `
      <div class="config-dialog-header">
        <div class="config-dialog-title">
          <i class="icon-upload"></i>
          <span>Import Configuration</span>
        </div>
        <button type="button" class="config-dialog-close" aria-label="Close">
          <i class="icon-x"></i>
        </button>
      </div>
      <div class="config-dialog-body">
        <div class="config-dialog-intro">
          <p>Review the configuration sections below. Select which sections to import.</p>
        </div>
        ${warningsHtml}
        <div class="config-dialog-sections">
          <div class="config-dialog-sections-header">
            <span class="config-dialog-sections-title">Sections in File</span>
            <div class="config-dialog-sections-actions">
              <button type="button" class="config-dialog-link-btn" data-action="select-all-valid">Select All Valid</button>
              <span class="config-dialog-sep">•</span>
              <button type="button" class="config-dialog-link-btn" data-action="select-none">Select None</button>
            </div>
          </div>
          <div class="config-import-sections-list" id="importSectionsList"></div>
        </div>
        <div class="config-dialog-options">
          <label class="config-dialog-option">
            <input type="checkbox" id="importMergeMode" checked />
            <div class="config-dialog-option-content">
              <span class="config-dialog-option-label">Merge with existing</span>
              <span class="config-dialog-option-hint">Only update fields present in the file. Unchecked = replace entire sections.</span>
            </div>
          </label>
          <label class="config-dialog-option">
            <input type="checkbox" id="importSaveToDisk" checked />
            <div class="config-dialog-option-content">
              <span class="config-dialog-option-label">Save to disk</span>
              <span class="config-dialog-option-hint">Persist changes to config.toml after import</span>
            </div>
          </label>
        </div>
      </div>
      <div class="config-dialog-footer">
        <div class="config-dialog-footer-info">
          <span id="importSummary">${this.selectedSections.size} sections • ${total_changes} changes</span>
        </div>
        <div class="config-dialog-footer-actions">
          <button type="button" class="config-dialog-btn secondary" data-action="back">
            <i class="icon-arrow-left"></i> Back
          </button>
          <button type="button" class="config-dialog-btn primary" data-action="import" id="importBtn" ${!valid || this.selectedSections.size === 0 ? "disabled" : ""}>
            <i class="icon-check"></i> Import Selected
          </button>
        </div>
      </div>
    `;

    this._renderPreviewSections();
    this._attachPreviewListeners();
  }

  _renderPreviewSections() {
    const list = this.element.querySelector("#importSectionsList");
    list.innerHTML = "";

    for (const section of this.previewData.sections) {
      const meta = SECTION_META[section.name] || {
        label: section.label,
        icon: "icon-settings",
        hint: "",
      };
      const isSelected = this.selectedSections.has(section.name);
      const canSelect = section.present && section.valid;

      const item = create("div", {
        className:
          "config-import-section-item" +
          (isSelected ? " selected" : "") +
          (!section.present ? " not-present" : "") +
          (!section.valid ? " invalid" : ""),
      });

      const statusIcon = !section.present
        ? '<i class="icon-circle-minus status-icon not-present" title="Not in file"></i>'
        : !section.valid
          ? '<i class="icon-circle-alert status-icon invalid" title="Invalid configuration"></i>'
          : section.changes.length > 0
            ? `<i class="icon-pencil status-icon changes" title="${section.changes.length} change(s)"></i>`
            : '<i class="icon-circle-check status-icon no-changes" title="No changes"></i>';

      const changesBadge =
        section.present && section.changes.length > 0
          ? `<span class="config-import-changes-badge">${section.changes.length} change${section.changes.length === 1 ? "" : "s"}</span>`
          : "";

      const errorMsg = section.error
        ? `<div class="config-import-section-error">${Utils.escapeHtml(section.error)}</div>`
        : "";

      item.innerHTML = `
        <label class="config-import-section-header">
          <input type="checkbox" value="${section.name}" ${isSelected ? "checked" : ""} ${!canSelect ? "disabled" : ""} />
          <div class="config-import-section-content">
            <div class="config-import-section-icon"><i class="${meta.icon}"></i></div>
            <div class="config-import-section-info">
              <div class="config-import-section-title">
                <span class="config-import-section-label">${Utils.escapeHtml(section.label)}</span>
                ${statusIcon}
                ${changesBadge}
              </div>
              <span class="config-import-section-hint">${section.present ? `${section.field_count} fields` : "Not included in file"}</span>
            </div>
          </div>
        </label>
        ${errorMsg}
        ${section.changes.length > 0 ? `<button type="button" class="config-import-toggle-changes" data-section="${section.name}">
          <i class="icon-chevron-down"></i> Show changes
        </button>
        <div class="config-import-changes-list" id="changes-${section.name}" hidden></div>` : ""}
      `;

      // Checkbox handler
      const checkbox = item.querySelector("input");
      if (checkbox && canSelect) {
        on(checkbox, "change", () => {
          if (checkbox.checked) {
            this.selectedSections.add(section.name);
            item.classList.add("selected");
          } else {
            this.selectedSections.delete(section.name);
            item.classList.remove("selected");
          }
          this._updateImportSummary();
        });
      }

      // Changes toggle
      const toggleBtn = item.querySelector(".config-import-toggle-changes");
      if (toggleBtn) {
        on(toggleBtn, "click", () => {
          const changesList = item.querySelector(`#changes-${section.name}`);
          const isExpanded = !changesList.hidden;
          changesList.hidden = isExpanded;
          toggleBtn.innerHTML = isExpanded
            ? '<i class="icon-chevron-down"></i> Show changes'
            : '<i class="icon-chevron-up"></i> Hide changes';

          if (!isExpanded && changesList.innerHTML === "") {
            this._renderChanges(changesList, section.changes);
          }
        });
      }

      list.appendChild(item);
    }
  }

  _renderChanges(container, changes) {
    container.innerHTML = changes
      .slice(0, 20) // Limit to 20 changes for performance
      .map(
        (change) => `
        <div class="config-import-change-item">
          <span class="config-import-change-field">${Utils.escapeHtml(change.field)}</span>
          <div class="config-import-change-values">
            <span class="config-import-change-current" title="Current value">${this._formatValue(change.current)}</span>
            <i class="icon-arrow-right"></i>
            <span class="config-import-change-imported" title="New value">${this._formatValue(change.imported)}</span>
          </div>
        </div>
      `
      )
      .join("");

    if (changes.length > 20) {
      container.innerHTML += `<div class="config-import-change-more">+${changes.length - 20} more changes</div>`;
    }
  }

  _formatValue(value) {
    if (value === null) return '<span class="null">null</span>';
    if (typeof value === "boolean")
      return `<span class="${value ? "true" : "false"}">${value}</span>`;
    if (typeof value === "number") return `<span class="number">${value}</span>`;
    if (typeof value === "string") {
      const escaped = Utils.escapeHtml(value);
      return escaped.length > 40 ? `"${escaped.substring(0, 40)}…"` : `"${escaped}"`;
    }
    if (Array.isArray(value)) return `[${value.length} items]`;
    if (typeof value === "object") return `{${Object.keys(value).length} keys}`;
    return String(value);
  }

  _updateImportSummary() {
    const summaryEl = this.element.querySelector("#importSummary");
    const importBtn = this.element.querySelector("#importBtn");
    const count = this.selectedSections.size;
    const totalChanges = this.previewData.sections
      .filter((s) => this.selectedSections.has(s.name))
      .reduce((sum, s) => sum + s.changes.length, 0);

    summaryEl.textContent = `${count} section${count === 1 ? "" : "s"} • ${totalChanges} change${totalChanges === 1 ? "" : "s"}`;
    importBtn.disabled = count === 0 || this.isImporting;
  }

  _attachEventListeners() {
    // Close button
    this._closeHandler = () => this._handleCancel();

    // Keyboard
    this._keydownHandler = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this._handleCancel();
      }
    };
    document.addEventListener("keydown", this._keydownHandler);
  }

  _attachUploadListeners() {
    const closeBtn = this.element.querySelector(".config-dialog-close");
    on(closeBtn, "click", this._closeHandler);

    on(this.backdrop, "click", this._closeHandler);

    const cancelBtn = this.element.querySelector('[data-action="cancel"]');
    on(cancelBtn, "click", () => this._handleCancel());

    const dropzone = this.element.querySelector("#importDropzone");
    const fileInput = this.element.querySelector("#importFileInput");

    // Dropzone click
    on(dropzone, "click", () => fileInput.click());

    // File input change
    on(fileInput, "change", (e) => {
      const file = e.target.files?.[0];
      if (file) this._handleFileSelected(file);
    });

    // Drag and drop
    on(dropzone, "dragover", (e) => {
      e.preventDefault();
      dropzone.classList.add("dragover");
    });

    on(dropzone, "dragleave", () => {
      dropzone.classList.remove("dragover");
    });

    on(dropzone, "drop", (e) => {
      e.preventDefault();
      dropzone.classList.remove("dragover");
      const file = e.dataTransfer?.files?.[0];
      if (file) this._handleFileSelected(file);
    });

    // Clear button
    const clearBtn = this.element.querySelector("#importClearBtn");
    if (clearBtn) {
      on(clearBtn, "click", () => {
        this.configData = null;
        this.previewData = null;
        hide(this.element.querySelector("#importFileInfo"));
        show(this.element.querySelector("#importDropzone"));
        this.element.querySelector("#nextBtn").disabled = true;
      });
    }

    // Next button
    const nextBtn = this.element.querySelector("#nextBtn");
    on(nextBtn, "click", () => this._renderPreviewStep());
  }

  _attachPreviewListeners() {
    const closeBtn = this.element.querySelector(".config-dialog-close");
    on(closeBtn, "click", this._closeHandler);

    // Back button
    const backBtn = this.element.querySelector('[data-action="back"]');
    on(backBtn, "click", () => this._renderUploadStep());

    // Import button
    const importBtn = this.element.querySelector('[data-action="import"]');
    on(importBtn, "click", () => this._handleImport());

    // Select all valid / none
    const selectAllBtn = this.element.querySelector('[data-action="select-all-valid"]');
    on(selectAllBtn, "click", () => {
      this.selectedSections = new Set(
        this.previewData.sections.filter((s) => s.present && s.valid).map((s) => s.name)
      );
      this._renderPreviewSections();
      this._updateImportSummary();
    });

    const selectNoneBtn = this.element.querySelector('[data-action="select-none"]');
    on(selectNoneBtn, "click", () => {
      this.selectedSections.clear();
      this._renderPreviewSections();
      this._updateImportSummary();
    });

    // Options checkboxes
    const mergeCheckbox = this.element.querySelector("#importMergeMode");
    on(mergeCheckbox, "change", () => {
      this.mergeMode = mergeCheckbox.checked;
    });

    const saveCheckbox = this.element.querySelector("#importSaveToDisk");
    on(saveCheckbox, "change", () => {
      this.saveToDisk = saveCheckbox.checked;
    });
  }

  async _handleFileSelected(file) {
    const dropzone = this.element.querySelector("#importDropzone");
    const fileInfo = this.element.querySelector("#importFileInfo");
    const fileName = this.element.querySelector("#importFileName");
    const loading = this.element.querySelector("#importLoading");
    const nextBtn = this.element.querySelector("#nextBtn");

    try {
      // Show file name
      hide(dropzone);
      fileName.textContent = file.name;
      show(fileInfo);

      // Parse JSON
      const text = await file.text();
      this.configData = JSON.parse(text);

      // Show loading
      show(loading);
      hide(fileInfo);

      // Preview via API
      const response = await requestManager.fetch("/api/config/import/preview", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ config: this.configData }),
        priority: "high",
      });

      this.previewData = response;

      hide(loading);
      show(fileInfo);
      nextBtn.disabled = false;
    } catch (error) {
      hide(loading);
      show(dropzone);
      hide(fileInfo);
      playError();
      Utils.showToast({
        type: "error",
        title: "Invalid File",
        message: error.message || "Failed to parse configuration file",
      });
    }
  }

  async _handleImport() {
    if (this.isImporting || this.selectedSections.size === 0) return;

    this.isImporting = true;
    const importBtn = this.element.querySelector("#importBtn");
    importBtn.disabled = true;
    importBtn.innerHTML = '<i class="icon-loader spin"></i> Importing...';

    try {
      const response = await requestManager.fetch("/api/config/import", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          config: this.configData,
          sections: Array.from(this.selectedSections),
          merge: this.mergeMode,
          save_to_disk: this.saveToDisk,
        }),
        priority: "high",
      });

      if (!response.success) {
        throw new Error(response.message || "Import failed");
      }

      playSuccess();
      Utils.showToast({
        type: "success",
        title: "Configuration Imported",
        message: response.message || `Imported ${response.imported_sections.length} section(s)`,
      });

      this.destroy();
      this.resolver({ imported: true, sections: response.imported_sections });
    } catch (error) {
      playError();
      Utils.showToast({
        type: "error",
        title: "Import Failed",
        message: error.message || "Failed to import configuration",
      });
      this.isImporting = false;
      importBtn.disabled = false;
      importBtn.innerHTML = '<i class="icon-check"></i> Import Selected';
    }
  }

  _handleCancel() {
    playPanelClose();
    this.destroy();
    this.resolver({ imported: false });
  }

  destroy() {
    if (this._keydownHandler) {
      document.removeEventListener("keydown", this._keydownHandler);
    }

    if (this.element) {
      this.element.classList.remove("visible");
      this.backdrop.classList.remove("visible");

      setTimeout(() => {
        this.element?.remove();
        this.backdrop?.remove();
      }, 200);
    }

    if (ConfigImportDialog.activeDialog === this) {
      ConfigImportDialog.activeDialog = null;
    }
  }
}
