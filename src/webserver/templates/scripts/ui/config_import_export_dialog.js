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

import { on, create, show, hide, setIconLabel } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { playClick, playSuccess, playError } from "../core/sounds.js";
import { requestManager } from "../core/request_manager.js";
import { hasSectionLabel, sectionLabel } from "../pages/config/field_text.js";

// ============================================================================
// SECTION METADATA
// ============================================================================

const SECTION_ICONS = Object.freeze({
  rpc: "icon-satellite",
  trader: "icon-briefcase",
  positions: "icon-chart-candlestick",
  filtering: "icon-target",
  swaps: "icon-repeat",
  tokens: "icon-coins",
  sol_price: "icon-sun",
  events: "icon-radio",
  services: "icon-wrench",
  monitoring: "icon-trending-up",
  ohlcv: "icon-clock",
  gui: "icon-layout-dashboard",
  telegram: "icon-send",
});

// Ids are the configuration section names the export and import routes use
// (src/webserver/routes/config/import_export.rs).
const SECTION_HINT_LABELS = Object.freeze({
  rpc: "system-config-section-hint-rpc",
  trader: "system-config-section-hint-trader",
  positions: "system-config-section-hint-positions",
  filtering: "system-config-section-hint-filtering",
  swaps: "system-config-section-hint-swaps",
  tokens: "system-config-section-hint-tokens",
  sol_price: "system-config-section-hint-sol-price",
  events: "system-config-section-hint-events",
  services: "system-config-section-hint-services",
  monitoring: "system-config-section-hint-monitoring",
  ohlcv: "system-config-section-hint-ohlcv",
  gui: "system-config-section-hint-gui",
  telegram: "system-config-section-hint-telegram",
});

/** The icon of a section; sections without one share the settings glyph. */
function sectionIcon(sectionId) {
  return SECTION_ICONS[sectionId] ?? "icon-settings";
}

/** The one-line description of a section, or an empty string when it has none. */
function sectionHint(sectionId) {
  return Object.hasOwn(SECTION_HINT_LABELS, sectionId)
    ? I18n.label(SECTION_HINT_LABELS, sectionId)
    : "";
}

/**
 * Display name of a section. The catalog owns the names of configuration
 * sections; an id it does not name is shown as sent.
 */
function sectionName(sectionId) {
  if (sectionId === "gui") return I18n.t("system-config-section-gui");
  return hasSectionLabel(sectionId) ? sectionLabel(sectionId) : sectionId;
}

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
          <span data-l10n-id="system-config-export-dialog-title"></span>
        </div>
        <button type="button" class="config-dialog-close" data-l10n-id="system-config-dialog-close">
          <i class="icon-x"></i>
        </button>
      </div>
      <div class="config-dialog-body">
        <div class="config-dialog-intro">
          <p data-l10n-id="system-config-export-intro"></p>
        </div>
        <div class="config-dialog-sections">
          <div class="config-dialog-sections-header">
            <span class="config-dialog-sections-title" data-l10n-id="system-config-export-sections"></span>
            <div class="config-dialog-sections-actions">
              <button type="button" class="config-dialog-link-btn" data-action="select-all" data-l10n-id="common-action-select-all"></button>
              <span class="config-dialog-sep">•</span>
              <button type="button" class="config-dialog-link-btn" data-action="select-none" data-l10n-id="system-config-select-none"></button>
            </div>
          </div>
          <div class="config-dialog-sections-grid" id="exportSectionsGrid"></div>
        </div>
        <div class="config-dialog-options">
          <label class="config-dialog-option">
            <input type="checkbox" id="exportIncludeMetadata" checked />
            <span data-l10n-id="system-config-export-timestamp"></span>
          </label>
        </div>
      </div>
      <div class="config-dialog-footer">
        <div class="config-dialog-footer-info">
          <span id="exportSelectionCount"></span>
        </div>
        <div class="config-dialog-footer-actions">
          <button type="button" class="config-dialog-btn secondary" data-action="cancel" data-l10n-id="common-action-cancel"></button>
          <button type="button" class="config-dialog-btn primary" data-action="export" id="exportBtn">
            <i class="icon-download"></i> <span data-l10n-id="common-action-export"></span>
          </button>
        </div>
      </div>
    `;
    I18n.localizeTree(this.element);
    this.element.querySelector("#exportSelectionCount").textContent = I18n.t(
      "system-config-sections-selected",
      { count: this.selectedSections.size }
    );

    this._renderSections();
    this._attachEventListeners();

    document.body.appendChild(this.backdrop);
    document.body.appendChild(this.element);

    requestAnimationFrame(() => {
      this.backdrop.classList.add("visible");
      this.element.classList.add("visible");
    });
  }

  _renderSections() {
    const grid = this.element.querySelector("#exportSectionsGrid");
    grid.innerHTML = "";

    for (const sectionId of SECTION_ORDER) {
      const isSelected = this.selectedSections.has(sectionId);

      const item = create("label", {
        className: "config-section-item" + (isSelected ? " selected" : ""),
      });
      item.innerHTML = `
        <input type="checkbox" value="${Utils.escapeHtml(sectionId)}" ${isSelected ? "checked" : ""} />
        <div class="config-section-item-content">
          <div class="config-section-item-icon"><i class="${sectionIcon(sectionId)}"></i></div>
          <div class="config-section-item-info">
            <span class="config-section-item-label">${Utils.escapeHtml(sectionName(sectionId))}</span>
            <span class="config-section-item-hint">${Utils.escapeHtml(sectionHint(sectionId))}</span>
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

    countEl.textContent = I18n.t("system-config-sections-selected", { count });
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
    setIconLabel(exportBtn, "icon-loader spin", I18n.t("system-config-exporting"));

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
        throw new Error(I18n.t("system-config-export-invalid-response"));
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
        title: I18n.t("system-config-exported-title"),
        message: I18n.t("system-config-exported-message", { count: response.sections.length }),
      });

      this.destroy();
      this.resolver({ exported: true, sections: response.sections });
    } catch (error) {
      playError();
      Utils.showToast({
        type: "error",
        title: I18n.t("system-config-export-failed-title"),
        message: error.message || I18n.t("system-config-export-failed"),
      });
      this.isExporting = false;
      exportBtn.disabled = false;
      setIconLabel(exportBtn, "icon-download", I18n.t("common-action-export"));
    }
  }

  _handleCancel() {
    playClick();
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
    });
  }

  _renderUploadStep() {
    this.step = "upload";
    this.element.innerHTML = `
      <div class="config-dialog-header">
        <div class="config-dialog-title">
          <i class="icon-upload"></i>
          <span data-l10n-id="system-config-import-dialog-title"></span>
        </div>
        <button type="button" class="config-dialog-close" data-l10n-id="system-config-dialog-close">
          <i class="icon-x"></i>
        </button>
      </div>
      <div class="config-dialog-body">
        <div class="config-dialog-intro">
          <p data-l10n-id="system-config-import-upload-intro"></p>
        </div>
        <div class="config-import-dropzone" id="importDropzone">
          <div class="config-import-dropzone-content">
            <i class="icon-file-code"></i>
            <span class="config-import-dropzone-title" data-l10n-id="system-config-import-dropzone-title"></span>
            <span class="config-import-dropzone-hint" data-l10n-id="system-config-import-dropzone-hint"></span>
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
          <span data-l10n-id="system-config-import-analyzing"></span>
        </div>
      </div>
      <div class="config-dialog-footer">
        <div class="config-dialog-footer-info"></div>
        <div class="config-dialog-footer-actions">
          <button type="button" class="config-dialog-btn secondary" data-action="cancel" data-l10n-id="common-action-cancel"></button>
          <button type="button" class="config-dialog-btn primary" data-action="next" id="nextBtn" disabled>
            <i class="icon-arrow-right"></i> <span data-l10n-id="system-config-import-preview"></span>
          </button>
        </div>
      </div>
    `;
    I18n.localizeTree(this.element);

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
            <span>${Utils.escapeHtml(I18n.t("system-config-import-warnings", { count: warnings.length }))}</span>
          </div>
          ${warnings.map((w) => `<div class="config-import-warning-item">${Utils.escapeHtml(I18n.text(w))}</div>`).join("")}
        </div>`
      : "";

    this.element.innerHTML = `
      <div class="config-dialog-header">
        <div class="config-dialog-title">
          <i class="icon-upload"></i>
          <span data-l10n-id="system-config-import-dialog-title"></span>
        </div>
        <button type="button" class="config-dialog-close" data-l10n-id="system-config-dialog-close">
          <i class="icon-x"></i>
        </button>
      </div>
      <div class="config-dialog-body">
        <div class="config-dialog-intro">
          <p data-l10n-id="system-config-import-preview-intro"></p>
        </div>
        ${warningsHtml}
        <div class="config-dialog-sections">
          <div class="config-dialog-sections-header">
            <span class="config-dialog-sections-title" data-l10n-id="system-config-import-sections"></span>
            <div class="config-dialog-sections-actions">
              <button type="button" class="config-dialog-link-btn" data-action="select-all-valid" data-l10n-id="system-config-import-select-valid"></button>
              <span class="config-dialog-sep">•</span>
              <button type="button" class="config-dialog-link-btn" data-action="select-none" data-l10n-id="system-config-select-none"></button>
            </div>
          </div>
          <div class="config-import-sections-list" id="importSectionsList"></div>
        </div>
        <div class="config-dialog-options">
          <label class="config-dialog-option">
            <input type="checkbox" id="importMergeMode" checked />
            <div class="config-dialog-option-content">
              <span class="config-dialog-option-label" data-l10n-id="system-config-import-merge-label"></span>
              <span class="config-dialog-option-hint" data-l10n-id="system-config-import-merge-hint"></span>
            </div>
          </label>
          <label class="config-dialog-option">
            <input type="checkbox" id="importSaveToDisk" checked />
            <div class="config-dialog-option-content">
              <span class="config-dialog-option-label" data-l10n-id="system-config-import-save-label"></span>
              <span class="config-dialog-option-hint" data-l10n-id="system-config-import-save-hint"></span>
            </div>
          </label>
        </div>
      </div>
      <div class="config-dialog-footer">
        <div class="config-dialog-footer-info">
          <span id="importSummary"></span>
        </div>
        <div class="config-dialog-footer-actions">
          <button type="button" class="config-dialog-btn secondary" data-action="back">
            <i class="icon-arrow-left"></i> <span data-l10n-id="common-action-back"></span>
          </button>
          <button type="button" class="config-dialog-btn primary" data-action="import" id="importBtn" ${!valid || this.selectedSections.size === 0 ? "disabled" : ""}>
            <i class="icon-check"></i> <span data-l10n-id="system-config-import-selected"></span>
          </button>
        </div>
      </div>
    `;
    I18n.localizeTree(this.element);
    this.element.querySelector("#importSummary").textContent = this._summaryText(total_changes);

    this._renderPreviewSections();
    this._attachPreviewListeners();
  }

  _renderPreviewSections() {
    const list = this.element.querySelector("#importSectionsList");
    list.innerHTML = "";

    for (const section of this.previewData.sections) {
      const isSelected = this.selectedSections.has(section.name);
      const canSelect = section.present && section.valid;

      const item = create("div", {
        className:
          "config-import-section-item" +
          (isSelected ? " selected" : "") +
          (!section.present ? " not-present" : "") +
          (!section.valid ? " invalid" : ""),
      });

      const changeCount = section.changes.length;
      const changesText = I18n.t("system-config-changes-count", { count: changeCount });
      const statusIcon = !section.present
        ? `<i class="icon-circle-minus status-icon not-present" title="${Utils.escapeHtml(I18n.t("system-config-import-status-absent"))}"></i>`
        : !section.valid
          ? `<i class="icon-circle-alert status-icon invalid" title="${Utils.escapeHtml(I18n.t("system-config-import-status-invalid"))}"></i>`
          : changeCount > 0
            ? `<i class="icon-pencil status-icon changes" title="${Utils.escapeHtml(changesText)}"></i>`
            : `<i class="icon-circle-check status-icon no-changes" title="${Utils.escapeHtml(I18n.t("system-config-import-status-unchanged"))}"></i>`;

      const changesBadge =
        section.present && changeCount > 0
          ? `<span class="config-import-changes-badge">${Utils.escapeHtml(changesText)}</span>`
          : "";

      const errorMsg = section.error
        ? `<div class="config-import-section-error">${Utils.escapeHtml(I18n.text(section.error))}</div>`
        : "";

      const sectionHintText = section.present
        ? I18n.t("system-config-fields-count", { count: section.field_count })
        : I18n.t("system-config-import-not-included");

      item.innerHTML = `
        <label class="config-import-section-header">
          <input type="checkbox" value="${Utils.escapeHtml(section.name)}" ${isSelected ? "checked" : ""} ${!canSelect ? "disabled" : ""} />
          <div class="config-import-section-content">
            <div class="config-import-section-icon"><i class="${sectionIcon(section.name)}"></i></div>
            <div class="config-import-section-info">
              <div class="config-import-section-title">
                <span class="config-import-section-label">${Utils.escapeHtml(sectionName(section.name))}</span>
                ${statusIcon}
                ${changesBadge}
              </div>
              <span class="config-import-section-hint">${Utils.escapeHtml(sectionHintText)}</span>
            </div>
          </div>
        </label>
        ${errorMsg}
        ${
          changeCount > 0
            ? `<button type="button" class="config-import-toggle-changes">
          <i class="icon-chevron-down"></i> <span data-l10n-id="system-config-import-show-changes"></span>
        </button>
        <div class="config-import-changes-list" hidden></div>`
            : ""
        }
      `;
      I18n.localizeTree(item);

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
          const changesList = item.querySelector(".config-import-changes-list");
          const isExpanded = !changesList.hidden;
          changesList.hidden = isExpanded;
          setIconLabel(
            toggleBtn,
            isExpanded ? "icon-chevron-down" : "icon-chevron-up",
            isExpanded
              ? I18n.t("system-config-import-show-changes")
              : I18n.t("system-config-import-hide-changes")
          );

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
            <span class="config-import-change-current" title="${Utils.escapeHtml(I18n.t("system-config-import-value-current"))}">${this._formatValue(change.current)}</span>
            <i class="icon-arrow-right"></i>
            <span class="config-import-change-imported" title="${Utils.escapeHtml(I18n.t("system-config-import-value-new"))}">${this._formatValue(change.imported)}</span>
          </div>
        </div>
      `
      )
      .join("");

    if (changes.length > 20) {
      container.innerHTML += `<div class="config-import-change-more">${Utils.escapeHtml(I18n.t("system-config-import-more-changes", { count: changes.length - 20 }))}</div>`;
    }
  }

  _formatValue(value) {
    // l10n-ignore: the JSON literal
    if (value === null) return '<span class="null">null</span>';
    if (typeof value === "boolean")
      return `<span class="${value ? "true" : "false"}">${value}</span>`;
    if (typeof value === "number") return `<span class="number">${value}</span>`;
    if (typeof value === "string") {
      const shown = value.length > 40 ? `${value.substring(0, 40)}…` : value;
      return `"${Utils.escapeHtml(shown)}"`;
    }
    if (Array.isArray(value)) {
      return Utils.escapeHtml(I18n.t("system-config-import-value-items", { count: value.length }));
    }
    if (typeof value === "object") {
      return Utils.escapeHtml(
        I18n.t("system-config-import-value-keys", { count: Object.keys(value).length })
      );
    }
    return Utils.escapeHtml(String(value));
  }

  /** "N sections • M changes". */
  _summaryText(changeCount) {
    return I18n.t("system-config-import-summary", {
      sections: I18n.t("system-config-sections-count", { count: this.selectedSections.size }),
      changes: I18n.t("system-config-changes-count", { count: changeCount }),
    });
  }

  _updateImportSummary() {
    const summaryEl = this.element.querySelector("#importSummary");
    const importBtn = this.element.querySelector("#importBtn");
    const count = this.selectedSections.size;
    const totalChanges = this.previewData.sections
      .filter((s) => this.selectedSections.has(s.name))
      .reduce((sum, s) => sum + s.changes.length, 0);

    summaryEl.textContent = this._summaryText(totalChanges);
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
        title: I18n.t("system-config-import-invalid-file-title"),
        message: error.message || I18n.t("system-config-import-invalid-file"),
      });
    }
  }

  async _handleImport() {
    if (this.isImporting || this.selectedSections.size === 0) return;

    this.isImporting = true;
    const importBtn = this.element.querySelector("#importBtn");
    importBtn.disabled = true;
    setIconLabel(importBtn, "icon-loader spin", I18n.t("system-config-importing"));

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
        throw new Error(I18n.t("system-config-import-failed"));
      }

      playSuccess();
      Utils.showToast({
        type: "success",
        title: I18n.t("system-config-imported-title"),
        message:
          I18n.text(response.text) ||
          I18n.t("system-config-imported-message", { count: response.imported_sections.length }),
      });

      this.destroy();
      this.resolver({ imported: true, sections: response.imported_sections });
    } catch (error) {
      playError();
      Utils.showToast({
        type: "error",
        title: I18n.t("system-config-import-failed-title"),
        message: error.message || I18n.t("system-config-import-failed-message"),
      });
      this.isImporting = false;
      importBtn.disabled = false;
      setIconLabel(importBtn, "icon-check", I18n.t("system-config-import-selected"));
    }
  }

  _handleCancel() {
    playClick();
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
