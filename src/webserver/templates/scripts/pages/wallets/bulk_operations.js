/**
 * Bulk Operations Module for Wallets
 * Handles bulk import/export functionality with multi-step wizards
 */

import { apiErrorMessage } from "../../core/request_manager.js";

// Phrase typed to confirm an export that includes private keys. It is compared
// verbatim and shown to the user in its own element, so it is never translated.
const EXPORT_KEYS_CONFIRMATION = "EXPORT KEYS";

export function createBulkOperations({
  $,
  Utils,
  on,
  showModal,
  hideModal,
  setButtonContent,
  enhanceAllSelects,
  loadAllData,
  walletsData,
}) {
  // =============================================================================
  // Bulk Import State (local to this module)
  // =============================================================================
  let importPreviewData = null;
  let importColumnMapping = {};
  let selectedImportFile = null;

  // =============================================================================
  // Bulk Import Modal Functions
  // =============================================================================

  function setupBulkImportModal() {
    const modal = $("#bulk-import-modal");
    if (!modal) return;

    // Close button
    const closeBtn = $("#bulk-import-modal-close");
    if (closeBtn) {
      on(closeBtn, "click", () => hideBulkImportModal());
    }

    // Backdrop click
    on(modal, "click", (e) => {
      if (e.target === modal) {
        hideBulkImportModal();
      }
    });

    // File dropzone
    setupFileDropzone();

    // Step 1 buttons
    const step1Cancel = $("#import-step1-cancel");
    const step1Next = $("#import-step1-next");
    if (step1Cancel) on(step1Cancel, "click", () => hideBulkImportModal());
    if (step1Next) on(step1Next, "click", () => goToImportStep(2));

    // Step 2 buttons
    const step2Back = $("#import-step2-back");
    const step2Execute = $("#import-step2-execute");
    if (step2Back) on(step2Back, "click", () => goToImportStep(1));
    if (step2Execute) on(step2Execute, "click", () => executeImport());

    // Step 3 buttons
    const step3Done = $("#import-step3-done");
    if (step3Done) {
      on(step3Done, "click", () => {
        hideBulkImportModal();
        loadAllData();
      });
    }
  }

  function setupFileDropzone() {
    const dropzone = $("#import-dropzone");
    const fileInput = $("#import-file-input");
    const clearBtn = $("#clear-file-btn");

    if (!dropzone || !fileInput) return;

    // Click to browse
    on(dropzone, "click", () => fileInput.click());

    // File selected
    on(fileInput, "change", (e) => {
      if (e.target.files && e.target.files[0]) {
        handleFileSelect(e.target.files[0]);
      }
    });

    // Drag and drop
    on(dropzone, "dragover", (e) => {
      e.preventDefault();
      dropzone.classList.add("drag-over");
    });

    on(dropzone, "dragleave", (e) => {
      e.preventDefault();
      dropzone.classList.remove("drag-over");
    });

    on(dropzone, "drop", (e) => {
      e.preventDefault();
      dropzone.classList.remove("drag-over");
      if (e.dataTransfer.files && e.dataTransfer.files[0]) {
        handleFileSelect(e.dataTransfer.files[0]);
      }
    });

    // Clear file
    if (clearBtn) {
      on(clearBtn, "click", (e) => {
        e.stopPropagation();
        clearSelectedFile();
      });
    }
  }

  function handleFileSelect(file) {
    const validExtensions = [".csv", ".xlsx", ".xls"];
    const ext = file.name.toLowerCase().slice(file.name.lastIndexOf("."));

    if (!validExtensions.includes(ext)) {
      Utils.showToast(I18n.t("wallets-bulk-file-invalid"), "error");
      return;
    }

    selectedImportFile = file;

    // Update UI
    const dropzone = $("#import-dropzone");
    const fileInfo = $("#selected-file-info");
    const fileName = $("#selected-file-name");
    const nextBtn = $("#import-step1-next");

    if (dropzone) dropzone.classList.add("hidden");
    if (fileInfo) fileInfo.classList.remove("hidden");
    if (fileName) fileName.textContent = file.name;
    if (nextBtn) nextBtn.disabled = false;
  }

  function clearSelectedFile() {
    selectedImportFile = null;
    importPreviewData = null;

    const dropzone = $("#import-dropzone");
    const fileInfo = $("#selected-file-info");
    const fileInput = $("#import-file-input");
    const nextBtn = $("#import-step1-next");

    if (dropzone) dropzone.classList.remove("hidden");
    if (fileInfo) fileInfo.classList.add("hidden");
    if (fileInput) fileInput.value = "";
    if (nextBtn) nextBtn.disabled = true;
  }

  function showBulkImportModal() {
    resetImportState();
    showModal("bulk-import-modal");
  }

  function hideBulkImportModal() {
    hideModal("bulk-import-modal");
    resetImportState();
  }

  function resetImportState() {
    importPreviewData = null;
    importColumnMapping = {};
    selectedImportFile = null;

    // Reset UI
    clearSelectedFile();
    updateImportStepIndicator(1);

    // Hide all steps except step 1
    const step1 = $("#import-step-1");
    const step2 = $("#import-step-2");
    const step3 = $("#import-step-3");
    if (step1) step1.classList.remove("hidden");
    if (step2) step2.classList.add("hidden");
    if (step3) step3.classList.add("hidden");
  }

  function updateImportStepIndicator(step) {
    const steps = document.querySelectorAll(".import-step");
    steps.forEach((stepEl) => {
      const stepNum = parseInt(stepEl.dataset.step, 10);
      stepEl.classList.remove("active", "completed");
      if (stepNum < step) {
        stepEl.classList.add("completed");
      } else if (stepNum === step) {
        stepEl.classList.add("active");
      }
    });
  }

  async function goToImportStep(step) {
    if (step === 2 && selectedImportFile) {
      // Upload file and get preview
      const success = await uploadFileForPreview();
      if (!success) return;
    }

    updateImportStepIndicator(step);

    const step1 = $("#import-step-1");
    const step2 = $("#import-step-2");
    const step3 = $("#import-step-3");

    if (step1) step1.classList.toggle("hidden", step !== 1);
    if (step2) step2.classList.toggle("hidden", step !== 2);
    if (step3) step3.classList.toggle("hidden", step !== 3);
  }

  async function uploadFileForPreview() {
    if (!selectedImportFile) return false;

    const nextBtn = $("#import-step1-next");
    if (nextBtn) {
      nextBtn.disabled = true;
      setButtonContent(nextBtn, "icon-loader spin", I18n.t("wallets-bulk-preview-busy"));
    }

    try {
      const formData = new FormData();
      formData.append("file", selectedImportFile);

      const response = await fetch("/api/wallets/import/preview", {
        method: "POST",
        body: formData,
      });

      const data = await response.json();
      if (!response.ok) {
        throw new Error(apiErrorMessage(data, I18n.t("wallets-bulk-preview-fallback")));
      }

      importPreviewData = data;
      renderColumnMapping(data);
      renderPreviewTable(data);
      updateImportSummary(data);

      return true;
    } catch (error) {
      console.error("[Wallets] File preview failed:", error);
      Utils.showToast(I18n.t("wallets-bulk-preview-failed", { reason: error.message }), "error");
      return false;
    } finally {
      if (nextBtn) {
        nextBtn.disabled = false;
        setButtonContent(nextBtn, "icon-arrow-right", I18n.t("common-action-next"));
      }
    }
  }

  function renderColumnMapping(preview) {
    const container = $("#column-mapping-grid");
    if (!container) return;

    const columns = preview.columns || [];
    const fields = [
      { id: "name", label: I18n.t("wallets-field-name"), required: true },
      { id: "private_key", label: I18n.t("wallets-field-private-key"), required: true },
      { id: "notes", label: I18n.t("wallets-field-notes"), required: false },
    ];

    const optionsList = columns
      .map((col, idx) => `<option value="${idx}">${Utils.escapeHtml(col)}</option>`)
      .join("");

    container.innerHTML = fields
      .map((field) => {
        const requiredMark = field.required ? '<span class="required">*</span>' : "";
        // Try to auto-detect column
        const autoIdx = autoDetectColumn(field.id, columns);
        if (autoIdx >= 0) {
          importColumnMapping[field.id] = autoIdx;
        }

        return `
          <div class="mapping-field">
            <label>${Utils.escapeHtml(field.label)} ${requiredMark}</label>
            <select data-field="${field.id}" data-custom-select>
              <option value="">${Utils.escapeHtml(I18n.t("wallets-bulk-column-select"))}</option>
              ${optionsList}
            </select>
          </div>
        `;
      })
      .join("");

    // Set auto-detected values and wire events
    container.querySelectorAll("select").forEach((select) => {
      const fieldId = select.dataset.field;
      if (importColumnMapping[fieldId] !== undefined) {
        select.value = importColumnMapping[fieldId];
      }

      on(select, "change", () => {
        const val = select.value;
        if (val === "") {
          delete importColumnMapping[fieldId];
        } else {
          importColumnMapping[fieldId] = parseInt(val, 10);
        }
        updatePreviewWithMapping();
        validateImportMapping();
      });
    });

    validateImportMapping();
  }

  function autoDetectColumn(fieldId, columns) {
    const patterns = {
      name: ["name", "wallet", "label", "title"],
      private_key: ["private", "key", "secret", "privatekey", "private_key"],
      notes: ["note", "notes", "description", "memo", "comment"],
    };

    const fieldPatterns = patterns[fieldId] || [];
    for (let i = 0; i < columns.length; i++) {
      const colLower = columns[i].toLowerCase();
      if (fieldPatterns.some((p) => colLower.includes(p))) {
        return i;
      }
    }
    return -1;
  }

  function renderPreviewTable(preview) {
    const container = $("#preview-table-wrapper");
    if (!container) return;

    const columns = preview.columns || [];
    const rows = preview.rows || [];

    if (rows.length === 0) {
      container.innerHTML = `<p class="info-text">${Utils.escapeHtml(I18n.t("wallets-bulk-preview-empty"))}</p>`;
      return;
    }

    const headerCells = columns.map((col) => `<th>${Utils.escapeHtml(col)}</th>`).join("");

    const bodyRows = rows
      .slice(0, 5)
      .map((row, rowIdx) => {
        const validation = preview.validations?.[rowIdx];
        const statusBadge = renderValidationBadge(validation);
        const cells = row.map((cell) => `<td>${Utils.escapeHtml(cell || "")}</td>`).join("");
        return `<tr>${cells}<td>${statusBadge}</td></tr>`;
      })
      .join("");

    container.innerHTML = `
      <table class="preview-table">
        <thead>
          <tr>${headerCells}<th>${Utils.escapeHtml(I18n.t("wallets-bulk-preview-status"))}</th></tr>
        </thead>
        <tbody>
          ${bodyRows}
        </tbody>
      </table>
    `;
  }

  function renderValidationBadge(validation) {
    if (!validation) return '<span class="validation-badge">—</span>';

    if (validation.status === "valid") {
      return `<span class="validation-badge valid"><i class="icon-check"></i> ${Utils.escapeHtml(I18n.t("wallets-bulk-status-valid"))}</span>`;
    } else if (validation.status === "duplicate") {
      return `<span class="validation-badge duplicate"><i class="icon-copy"></i> ${Utils.escapeHtml(I18n.t("wallets-bulk-status-duplicate"))}</span>`;
    } else {
      // The reason is a per-row validation outcome from the backend, shown as sent.
      return `<span class="validation-badge invalid"><i class="icon-x"></i> ${Utils.escapeHtml(validation.reason || I18n.t("wallets-bulk-status-invalid"))}</span>`; // api-body-ok: per-row validation outcome field
    }
  }

  function updatePreviewWithMapping() {
    // Re-validate with current mapping if needed
    if (importPreviewData) {
      renderPreviewTable(importPreviewData);
    }
  }

  function updateImportSummary(preview) {
    const validations = preview.validations || [];
    let valid = 0,
      invalid = 0,
      duplicate = 0;

    validations.forEach((v) => {
      if (v.status === "valid") valid++;
      else if (v.status === "duplicate") duplicate++;
      else invalid++;
    });

    const validEl = $("#import-summary-valid");
    const invalidEl = $("#import-summary-invalid");
    const duplicateEl = $("#import-summary-duplicate");

    if (validEl) validEl.innerHTML = I18n.markup("wallets-bulk-summary-valid", { count: valid });
    if (invalidEl) {
      invalidEl.innerHTML = I18n.markup("wallets-bulk-summary-invalid", { count: invalid });
    }
    if (duplicateEl) {
      duplicateEl.innerHTML = I18n.markup("wallets-bulk-summary-duplicate", { count: duplicate });
    }
  }

  function validateImportMapping() {
    const executeBtn = $("#import-step2-execute");
    if (!executeBtn) return;

    const hasName = importColumnMapping.name !== undefined;
    const hasKey = importColumnMapping.private_key !== undefined;
    const hasValidRows = importPreviewData?.validations?.some((v) => v.status === "valid");

    executeBtn.disabled = !(hasName && hasKey && hasValidRows);
  }

  async function executeImport() {
    const executeBtn = $("#import-step2-execute");
    if (executeBtn) {
      executeBtn.disabled = true;
      setButtonContent(executeBtn, "icon-loader spin", I18n.t("wallets-bulk-import-busy"));
    }

    try {
      const response = await fetch("/api/wallets/import/execute", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          preview_id: importPreviewData?.preview_id,
          column_mapping: importColumnMapping,
        }),
      });

      const data = await response.json();
      if (!response.ok) {
        throw new Error(apiErrorMessage(data, I18n.t("wallets-import-failed")));
      }

      renderImportResults(data);
      goToImportStep(3);

      const successCount = data.results?.filter((r) => r.success).length || 0;
      Utils.showToast(I18n.t("wallets-bulk-import-toast", { count: successCount }), "success");
    } catch (error) {
      console.error("[Wallets] Import failed:", error);
      Utils.showToast(I18n.t("wallets-bulk-import-error", { reason: error.message }), "error");
    } finally {
      if (executeBtn) {
        executeBtn.disabled = false;
        setButtonContent(executeBtn, "icon-upload", I18n.t("wallets-bulk-import-submit"));
      }
    }
  }

  function renderImportResults(data) {
    const headerEl = $("#import-results-header");
    const tableEl = $("#import-results-table");

    if (!headerEl || !tableEl) return;

    const results = data.results || [];
    const successCount = results.filter((r) => r.success).length;
    const errorCount = results.filter((r) => !r.success).length;

    // Determine result type
    let resultClass, icon, title, subtitle;
    if (errorCount === 0) {
      resultClass = "success";
      icon = "icon-circle-check";
      title = I18n.t("wallets-bulk-result-success-title");
      subtitle = I18n.t("wallets-bulk-result-success-detail", { count: successCount });
    } else if (successCount > 0) {
      resultClass = "partial";
      icon = "icon-triangle-alert";
      title = I18n.t("wallets-bulk-result-partial-title");
      subtitle = I18n.t("wallets-bulk-result-partial-detail", {
        imported: successCount,
        failed: errorCount,
      });
    } else {
      resultClass = "error";
      icon = "icon-circle-x";
      title = I18n.t("wallets-bulk-result-failed-title");
      subtitle = I18n.t("wallets-bulk-result-failed-detail", { count: errorCount });
    }

    headerEl.className = `import-results-header ${resultClass}`;
    headerEl.innerHTML = `
      <div class="results-summary">
        <i class="${icon}"></i>
        <div class="results-text">
          <strong>${Utils.escapeHtml(title)}</strong>
          <span>${Utils.escapeHtml(subtitle)}</span>
        </div>
      </div>
    `;

    // Render results table
    const rows = results
      .map((result) => {
        const rowClass = result.success ? "success-row" : "error-row";
        const statusIcon = result.success
          ? `<span class="result-status success"><i class="icon-check"></i> ${Utils.escapeHtml(I18n.t("wallets-bulk-result-imported"))}</span>`
          : `<span class="result-status error"><i class="icon-x"></i> ${Utils.escapeHtml(result.error || I18n.t("wallets-bulk-result-failed"))}</span>`; // api-body-ok: per-row import outcome field (wallets/bulk/types.rs)

        return `
          <tr class="${rowClass}">
            <td>${Utils.escapeHtml(result.name || "—")}</td>
            <td><code dir="ltr">${result.address ? `${result.address.slice(0, 8)}...${result.address.slice(-6)}` : "—"}</code></td>
            <td>${statusIcon}</td>
          </tr>
        `;
      })
      .join("");

    tableEl.innerHTML = `
      <table class="results-table">
        <thead>
          <tr>
            <th>${Utils.escapeHtml(I18n.t("wallets-list-col-name"))}</th>
            <th>${Utils.escapeHtml(I18n.t("wallets-field-address"))}</th>
            <th>${Utils.escapeHtml(I18n.t("wallets-bulk-preview-status"))}</th>
          </tr>
        </thead>
        <tbody>
          ${rows}
        </tbody>
      </table>
    `;
  }

  // =============================================================================
  // Bulk Export Modal Functions
  // =============================================================================

  function setupBulkExportModal() {
    const modal = $("#bulk-export-modal");
    const confirmModal = $("#export-keys-confirm-modal");

    if (!modal) return;

    // Enhance native selects with custom styling
    enhanceAllSelects(modal);

    // Close buttons
    const closeBtn = $("#bulk-export-modal-close");
    const cancelBtn = $("#bulk-export-cancel-btn");
    if (closeBtn) on(closeBtn, "click", () => hideBulkExportModal());
    if (cancelBtn) on(cancelBtn, "click", () => hideBulkExportModal());

    // Backdrop click
    on(modal, "click", (e) => {
      if (e.target === modal) {
        hideBulkExportModal();
      }
    });

    // Safe export button
    const safeExportBtn = $("#export-safe-btn");
    if (safeExportBtn) {
      on(safeExportBtn, "click", () => exportWallets(false));
    }

    // Dangerous export button - shows confirmation
    const dangerExportBtn = $("#export-with-keys-btn");
    if (dangerExportBtn) {
      on(dangerExportBtn, "click", () => showExportKeysConfirmation());
    }

    // Setup confirmation modal
    if (confirmModal) {
      const confirmClose = $("#export-keys-confirm-close");
      const confirmCancel = $("#export-keys-confirm-cancel");
      const confirmInput = $("#export-confirm-input");
      const confirmBtn = $("#export-keys-confirm-btn");

      if (confirmClose) on(confirmClose, "click", () => hideExportKeysConfirmation());
      if (confirmCancel) on(confirmCancel, "click", () => hideExportKeysConfirmation());

      on(confirmModal, "click", (e) => {
        if (e.target === confirmModal) {
          hideExportKeysConfirmation();
        }
      });

      const tokenEl = $("#export-confirm-token");
      if (tokenEl) tokenEl.textContent = EXPORT_KEYS_CONFIRMATION;

      if (confirmInput && confirmBtn) {
        on(confirmInput, "input", () => {
          confirmBtn.disabled = confirmInput.value !== EXPORT_KEYS_CONFIRMATION;
        });
      }

      if (confirmBtn) {
        on(confirmBtn, "click", () => {
          hideExportKeysConfirmation();
          exportWallets(true);
        });
      }
    }
  }

  function showBulkExportModal() {
    showModal("bulk-export-modal");
  }

  function hideBulkExportModal() {
    hideModal("bulk-export-modal");
  }

  function showExportKeysConfirmation() {
    const activeCount = walletsData().filter((w) => w.is_active).length;
    const includeInactive = $("#export-include-inactive")?.checked;
    const totalCount = includeInactive ? walletsData().length : activeCount;

    const warningEl = $("#export-keys-warning");
    if (warningEl) {
      warningEl.innerHTML = I18n.markup("wallets-bulk-confirm-warning", { count: totalCount });
    }

    const confirmInput = $("#export-confirm-input");
    const confirmBtn = $("#export-keys-confirm-btn");
    if (confirmInput) confirmInput.value = "";
    if (confirmBtn) confirmBtn.disabled = true;

    showModal("export-keys-confirm-modal");
  }

  function hideExportKeysConfirmation() {
    hideModal("export-keys-confirm-modal");
  }

  async function exportWallets(includeKeys) {
    const format = $("#export-format")?.value || "csv";
    const includeInactive = $("#export-include-inactive")?.checked || false;

    const exportBtn = includeKeys ? $("#export-with-keys-btn") : $("#export-safe-btn");
    const originalHtml = exportBtn?.innerHTML;

    if (exportBtn) {
      exportBtn.disabled = true;
      setButtonContent(exportBtn, "icon-loader spin", I18n.t("wallets-bulk-export-busy"));
    }

    try {
      const endpoint = includeKeys ? "/api/wallets/export/with-keys" : "/api/wallets/export";
      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          format,
          include_inactive: includeInactive,
        }),
      });

      if (!response.ok) {
        const data = await response.json();
        throw new Error(apiErrorMessage(data, I18n.t("wallets-bulk-export-fallback")));
      }

      // Get filename from header or generate one
      const contentDisposition = response.headers.get("Content-Disposition");
      let filename = `wallets_export.${format}`;
      if (contentDisposition) {
        const match = contentDisposition.match(/filename="?([^"]+)"?/);
        if (match) filename = match[1];
      }

      // Download the file
      const blob = await response.blob();
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      window.URL.revokeObjectURL(url);

      Utils.showToast(I18n.t("wallets-bulk-export-done", { filename }), "success");
      hideBulkExportModal();
    } catch (error) {
      console.error("[Wallets] Export failed:", error);
      Utils.showToast(I18n.t("wallets-bulk-export-error", { reason: error.message }), "error");
    } finally {
      if (exportBtn) {
        exportBtn.disabled = false;
        exportBtn.innerHTML = originalHtml;
      }
    }
  }

  // =============================================================================
  // Public API
  // =============================================================================

  return {
    setupBulkImportModal,
    setupBulkExportModal,
    showBulkImportModal,
    hideBulkImportModal,
    showBulkExportModal,
    hideBulkExportModal,
  };
}
