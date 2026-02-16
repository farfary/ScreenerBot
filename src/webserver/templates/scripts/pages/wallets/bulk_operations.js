/**
 * Bulk Operations Module for Wallets
 * Handles bulk import/export functionality with multi-step wizards
 */

export function createBulkOperations({ $, Utils, on, showModal, hideModal, enhanceAllSelects, loadAllData, walletsData }) {
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
      Utils.showToast("Invalid file type. Please use CSV or Excel files.", "error");
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
      nextBtn.innerHTML = '<i class="icon-loader spin"></i> Processing...';
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
        throw new Error(data.error || data.message || "Failed to process file");
      }

      importPreviewData = data;
      renderColumnMapping(data);
      renderPreviewTable(data);
      updateImportSummary(data);

      return true;
    } catch (error) {
      console.error("[Wallets] File preview failed:", error);
      Utils.showToast(`Failed to process file: ${error.message}`, "error");
      return false;
    } finally {
      if (nextBtn) {
        nextBtn.disabled = false;
        nextBtn.innerHTML = '<i class="icon-arrow-right"></i> Next';
      }
    }
  }

  function renderColumnMapping(preview) {
    const container = $("#column-mapping-grid");
    if (!container) return;

    const columns = preview.columns || [];
    const fields = [
      { id: "name", label: "Wallet Name", required: true },
      { id: "private_key", label: "Private Key", required: true },
      { id: "notes", label: "Notes", required: false },
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
            <label>${field.label} ${requiredMark}</label>
            <select data-field="${field.id}">
              <option value="">-- Select column --</option>
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
      container.innerHTML = '<p class="info-text">No data rows found in file</p>';
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
          <tr>${headerCells}<th>Status</th></tr>
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
      return '<span class="validation-badge valid"><i class="icon-check"></i> Valid</span>';
    } else if (validation.status === "duplicate") {
      return '<span class="validation-badge duplicate"><i class="icon-copy"></i> Duplicate</span>';
    } else {
      return `<span class="validation-badge invalid"><i class="icon-x"></i> ${Utils.escapeHtml(validation.reason || "Invalid")}</span>`;
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

    const validEl = $("#valid-count");
    const invalidEl = $("#invalid-count");
    const duplicateEl = $("#duplicate-count");

    if (validEl) validEl.textContent = valid;
    if (invalidEl) invalidEl.textContent = invalid;
    if (duplicateEl) duplicateEl.textContent = duplicate;
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
      executeBtn.innerHTML = '<i class="icon-loader spin"></i> Importing...';
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
        throw new Error(data.error || data.message || "Import failed");
      }

      renderImportResults(data);
      goToImportStep(3);

      const successCount = data.results?.filter((r) => r.success).length || 0;
      Utils.showToast(`Imported ${successCount} wallet(s)`, "success");
    } catch (error) {
      console.error("[Wallets] Import failed:", error);
      Utils.showToast(`Import failed: ${error.message}`, "error");
    } finally {
      if (executeBtn) {
        executeBtn.disabled = false;
        executeBtn.innerHTML = '<i class="icon-upload"></i> Import Wallets';
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
      title = "Import Successful";
      subtitle = `All ${successCount} wallet(s) imported successfully`;
    } else if (successCount > 0) {
      resultClass = "partial";
      icon = "icon-triangle-alert";
      title = "Partial Success";
      subtitle = `${successCount} imported, ${errorCount} failed`;
    } else {
      resultClass = "error";
      icon = "icon-circle-x";
      title = "Import Failed";
      subtitle = `All ${errorCount} wallet(s) failed to import`;
    }

    headerEl.className = `import-results-header ${resultClass}`;
    headerEl.innerHTML = `
      <div class="results-summary">
        <i class="${icon}"></i>
        <div class="results-text">
          <strong>${title}</strong>
          <span>${subtitle}</span>
        </div>
      </div>
    `;

    // Render results table
    const rows = results
      .map((result) => {
        const rowClass = result.success ? "success-row" : "error-row";
        const statusIcon = result.success
          ? '<span class="result-status success"><i class="icon-check"></i> Imported</span>'
          : `<span class="result-status error"><i class="icon-x"></i> ${Utils.escapeHtml(result.error || "Failed")}</span>`;

        return `
          <tr class="${rowClass}">
            <td>${Utils.escapeHtml(result.name || "—")}</td>
            <td><code>${result.address ? `${result.address.slice(0, 8)}...${result.address.slice(-6)}` : "—"}</code></td>
            <td>${statusIcon}</td>
          </tr>
        `;
      })
      .join("");

    tableEl.innerHTML = `
      <table class="results-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Address</th>
            <th>Status</th>
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
    const cancelBtn = $("#export-cancel-btn");
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

      if (confirmInput && confirmBtn) {
        on(confirmInput, "input", () => {
          confirmBtn.disabled = confirmInput.value !== "EXPORT KEYS";
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

    const countEl = $("#export-wallet-count");
    if (countEl) countEl.textContent = totalCount;

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
      exportBtn.innerHTML = '<i class="icon-loader spin"></i> Exporting...';
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
        throw new Error(data.error || data.message || "Export failed");
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

      Utils.showToast(`Exported wallets to ${filename}`, "success");
      hideBulkExportModal();
    } catch (error) {
      console.error("[Wallets] Export failed:", error);
      Utils.showToast(`Export failed: ${error.message}`, "error");
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
