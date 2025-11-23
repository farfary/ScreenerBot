import { on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";

/**
 * TradeActionDialog - Compact modal for buy/add/sell actions with preset buttons
 *
 * Features:
 * - Quick-action preset buttons (25%, 50%, 75%, 100% for sell, etc.)
 * - Custom amount input with validation
 * - Promise-based return value
 * - Keyboard navigation (Enter, Escape)
 * - Focus management and ARIA compliance
 */

const ACTION_CONFIG = {
  buy: {
    title: "<i class='icon-shopping-cart'></i> Buy Token",
    confirmLabel: "Confirm Buy",
    inputLabel: "Or Custom Amount (SOL):",
    inputHint: "Leave empty for config default",
    presets: [
      { label: "0.005 SOL", value: 0.005, type: "amount" },
      { label: "0.01 SOL", value: 0.01, type: "amount" },
      { label: "0.02 SOL", value: 0.02, type: "amount" },
    ],
  },
  sell: {
    title: '<i class="icon-dollar-sign"></i> Sell Position',
    confirmLabel: "Confirm Sell",
    inputLabel: "Or Custom Percentage:",
    inputHint: "Enter value between 1-100",
    presets: [
      { label: "25%", value: 25, type: "percentage" },
      { label: "50%", value: 50, type: "percentage" },
      { label: "75%", value: 75, type: "percentage" },
      { label: "100%", value: 100, type: "percentage", default: true },
    ],
  },
  add: {
    title: "<i class='icon-plus-circle'></i> Add to Position",
    confirmLabel: "Confirm Add",
    inputLabel: "Or Custom Amount (SOL):",
    inputHint: "Leave empty for default (50%)",
    presets: [], // Dynamic, built from context
  },
};

export class TradeActionDialog {
  constructor() {
    this.root = null;
    this.dialog = null;
    this.titleEl = null;
    this.contextEl = null;
    this.presetContainers = [];
    this.inputField = null;
    this.errorEl = null;
    this.confirmBtn = null;
    this.cancelBtn = null;
    this.closeBtn = null;
    this._presetButtons = []; // Track preset buttons for cleanup

    this._isOpen = false;
    this._previousActiveElement = null;
    this._resolveOpen = null;

    this.currentAction = null;
    this.currentContext = null;
    this._selectedPreset = null;

    this._overlayListener = this._handleOverlayClick.bind(this);
    this._closeListener = this._handleCloseClick.bind(this);
    this._cancelListener = this._handleCancelClick.bind(this);
    this._confirmListener = this._handleConfirmClick.bind(this);
    this._keyListener = this._handleKeyDown.bind(this);
    this._presetClickListener = this._handlePresetClick.bind(this);
    this._inputChangeListener = this._handleInputChange.bind(this);

    this._ensureElements();
  }

  _ensureElements() {
    if (this.root) {
      return;
    }

    const overlay = document.createElement("div");
    overlay.className = "trade-action-dialog-overlay";
    overlay.setAttribute("role", "presentation");
    overlay.setAttribute("aria-hidden", "true");

    overlay.innerHTML = `
      <div class="trade-action-dialog" role="dialog" aria-modal="true" aria-labelledby="trade-action-title" tabindex="-1">
        <header class="trade-action-header">
          <h2 id="trade-action-title" class="trade-action-title"></h2>
          <button type="button" class="trade-action-close" data-action="close" aria-label="Close dialog">&times;</button>
        </header>
        <div class="trade-action-body">
          <div class="trade-action-context"></div>
          <div class="trade-action-presets"></div>
          <div class="trade-action-input-section">
            <label class="trade-action-input-label" for="trade-action-input"></label>
            <input type="number" id="trade-action-input" class="trade-action-input" step="any" min="0" />
            <div class="trade-action-input-hint"></div>
            <div class="trade-action-error-msg" data-visible="false"></div>
          </div>
        </div>
        <footer class="trade-action-footer">
          <button type="button" class="trade-action-btn trade-action-btn-cancel" data-action="cancel">Cancel</button>
          <button type="button" class="trade-action-btn trade-action-btn-confirm" data-action="confirm" disabled>Confirm</button>
        </footer>
      </div>
    `;

    document.body.appendChild(overlay);

    this.root = overlay;
    this.dialog = overlay.querySelector(".trade-action-dialog");
    this.titleEl = overlay.querySelector(".trade-action-title");
    this.contextEl = overlay.querySelector(".trade-action-context");
    this.presetsContainer = overlay.querySelector(".trade-action-presets");
    this.inputField = overlay.querySelector(".trade-action-input");
    this.inputLabelEl = overlay.querySelector(".trade-action-input-label");
    this.inputHintEl = overlay.querySelector(".trade-action-input-hint");
    this.errorEl = overlay.querySelector(".trade-action-error-msg");
    this.confirmBtn = overlay.querySelector('[data-action="confirm"]');
    this.cancelBtn = overlay.querySelector('[data-action="cancel"]');
    this.closeBtn = overlay.querySelector('[data-action="close"]');

    on(overlay, "click", this._overlayListener);
    on(this.closeBtn, "click", this._closeListener);
    on(this.cancelBtn, "click", this._cancelListener);
    on(this.confirmBtn, "click", this._confirmListener);
    on(this.inputField, "input", this._inputChangeListener);
  }

  /**
   * Open the dialog and return a promise that resolves with the selected value or null
   * @param {Object} options - Dialog options
   * @param {string} options.action - 'buy' | 'add' | 'sell'
   * @param {string} options.symbol - Token symbol for display
   * @param {Object} options.context - Contextual data (balance, entrySize, entrySizes, holdings)
   * @returns {Promise<Object|null>} - Resolves with { amount: number } or { percentage: number } or null if cancelled
   */
  async open({ action, symbol, context = {} }) {
    if (!action || !ACTION_CONFIG[action]) {
      throw new Error(`Invalid action: ${action}`);
    }

    // Guard against multiple simultaneous opens
    if (this._isOpen) {
      console.warn("[TradeActionDialog] Dialog already open, ignoring duplicate request");
      return null;
    }

    this._ensureElements();

    this._previousActiveElement =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;

    this.currentAction = action;
    this.currentContext = context;
    this._selectedPreset = null;

    // Create promise before rendering
    const resultPromise = new Promise((resolve) => {
      this._resolveOpen = resolve;
    });

    // Render content
    this._render(action, symbol, context);

    // Show dialog
    this.root.classList.add("is-visible");
    this.root.setAttribute("aria-hidden", "false");
    document.body.classList.add("trade-action-dialog-open");
    this._isOpen = true;

    document.addEventListener("keydown", this._keyListener, true);

    requestAnimationFrame(() => {
      if (!this._isOpen) {
        return;
      }
      this.dialog?.focus();

      // Auto-select default preset if exists
      const defaultPreset = this.presetsContainer.querySelector(
        ".trade-action-preset-btn[data-default='true']"
      );
      if (defaultPreset) {
        defaultPreset.click();
      }
    });

    return resultPromise;
  }

  close({ restoreFocus = true } = {}) {
    if (!this._isOpen) {
      return;
    }

    this.root.classList.remove("is-visible");
    this.root.setAttribute("aria-hidden", "true");
    document.body.classList.remove("trade-action-dialog-open");
    this._isOpen = false;

    document.removeEventListener("keydown", this._keyListener, true);

    if (
      restoreFocus &&
      this._previousActiveElement &&
      typeof this._previousActiveElement.focus === "function"
    ) {
      try {
        this._previousActiveElement.focus();
      } catch {
        // Ignore focus errors
      }
    }
    this._previousActiveElement = null;
  }

  destroy() {
    this.close({ restoreFocus: false });

    // Resolve pending promise to prevent hanging
    if (this._resolveOpen) {
      this._resolveOpen(null);
      this._resolveOpen = null;
    }

    if (!this.root) {
      return;
    }

    // Clean up preset button listeners
    this._presetButtons.forEach((btn) => {
      off(btn, "click", this._presetClickListener);
    });
    this._presetButtons = [];

    off(this.root, "click", this._overlayListener);
    off(this.closeBtn, "click", this._closeListener);
    off(this.cancelBtn, "click", this._cancelListener);
    off(this.confirmBtn, "click", this._confirmListener);
    off(this.inputField, "input", this._inputChangeListener);

    if (this.root.parentNode) {
      this.root.parentNode.removeChild(this.root);
    }

    this.root = null;
    this.dialog = null;
    this.titleEl = null;
    this.contextEl = null;
    this.presetsContainer = null;
    this.inputField = null;
    this.errorEl = null;
    this.confirmBtn = null;
    this.cancelBtn = null;
    this.closeBtn = null;
  }

  _render(action, symbol, context) {
    const config = ACTION_CONFIG[action];

    // Set title
    this.titleEl.textContent = config.title;

    // Render context info
    this._renderContext(action, symbol, context);

    // Build and render presets
    const presets = this._buildPresets(action, context);
    this._renderPresets(presets);

    // Set input labels
    this.inputLabelEl.textContent = config.inputLabel;
    this.inputHintEl.textContent = config.inputHint;
    this.inputField.value = "";
    this.inputField.placeholder =
      action === "sell" ? "1-100" : action === "buy" ? "e.g. 0.01" : "e.g. 0.005";

    // Set confirm button label
    this.confirmBtn.textContent = config.confirmLabel;
    this.confirmBtn.disabled = true;

    // Clear error
    this._clearError();
  }

  _renderContext(action, symbol, context) {
    const rows = [];

    rows.push(`
      <div class="trade-action-context-row">
        <span class="trade-action-context-label">Token:</span>
        <span class="trade-action-context-value">${Utils.escapeHtml(symbol || "?")}</span>
      </div>
    `);

    if (action === "buy" || action === "add") {
      const balance = context.balance != null ? context.balance.toFixed(4) : "—";
      rows.push(`
        <div class="trade-action-context-row">
          <span class="trade-action-context-label">Balance:</span>
          <span class="trade-action-context-value">${Utils.escapeHtml(balance)} SOL</span>
        </div>
      `);
    }

    if (action === "add" && context.currentSize != null) {
      rows.push(`
        <div class="trade-action-context-row">
          <span class="trade-action-context-label">Current Size:</span>
          <span class="trade-action-context-value">${context.currentSize.toFixed(4)} SOL</span>
        </div>
      `);
    }

    if (action === "sell" && context.holdings != null) {
      const formatted = Utils.formatCompactNumber(context.holdings, 2);
      rows.push(`
        <div class="trade-action-context-row">
          <span class="trade-action-context-label">Holdings:</span>
          <span class="trade-action-context-value">${Utils.escapeHtml(formatted)}</span>
        </div>
      `);
    }

    this.contextEl.innerHTML = rows.join("");
  }

  _buildPresets(action, context) {
    const config = ACTION_CONFIG[action];

    if (action === "buy" || action === "sell") {
      return config.presets;
    }

    if (action === "add") {
      const presets = [];

      // Multipliers based on entry size
      if (context.entrySize != null && context.entrySize > 0) {
        presets.push(
          {
            label: "1.0×",
            value: context.entrySize,
            preview: `${context.entrySize.toFixed(3)} SOL`,
            type: "amount",
            group: "multiplier",
          },
          {
            label: "1.5×",
            value: context.entrySize * 1.5,
            preview: `${(context.entrySize * 1.5).toFixed(3)} SOL`,
            type: "amount",
            group: "multiplier",
          },
          {
            label: "2.0×",
            value: context.entrySize * 2.0,
            preview: `${(context.entrySize * 2.0).toFixed(3)} SOL`,
            type: "amount",
            group: "multiplier",
          }
        );
      }

      // Entry sizes from config
      if (Array.isArray(context.entrySizes) && context.entrySizes.length > 0) {
        context.entrySizes.forEach((size) => {
          presets.push({
            label: `${size} SOL`,
            value: size,
            type: "amount",
            group: "entry",
          });
        });
      }

      return presets;
    }

    return [];
  }

  _renderPresets(presets) {
    if (!presets || presets.length === 0) {
      this.presetsContainer.innerHTML = "";
      return;
    }

    // Group presets if needed
    const groups = {};
    presets.forEach((preset) => {
      const group = preset.group || "default";
      if (!groups[group]) {
        groups[group] = [];
      }
      groups[group].push(preset);
    });

    const sections = [];

    Object.entries(groups).forEach(([groupName, groupPresets]) => {
      const gridClass =
        groupPresets.length === 4
          ? "trade-action-preset-grid--four"
          : groupPresets.length === 3
            ? "trade-action-preset-grid--three"
            : "";

      const label =
        groupName === "multiplier"
          ? "Match Entry Amounts:"
          : groupName === "entry"
            ? "Position Entry Sizes:"
            : "Quick Actions:";

      const buttons = groupPresets
        .map(
          (preset) => `
        <button 
          type="button" 
          class="trade-action-preset-btn" 
          data-value="${preset.value}"
          data-type="${preset.type}"
          ${preset.default ? 'data-default="true"' : ""}
          aria-label="Select ${Utils.escapeHtml(preset.label)}"
        >
          <span>${Utils.escapeHtml(preset.label)}</span>
          ${preset.preview ? `<span class="trade-action-preset-preview">${Utils.escapeHtml(preset.preview)}</span>` : ""}
        </button>
      `
        )
        .join("");

      sections.push(`
        <div class="trade-action-preset-section">
          <div class="trade-action-section-label">${label}</div>
          <div class="trade-action-preset-grid ${gridClass}">
            ${buttons}
          </div>
        </div>
      `);
    });

    this.presetsContainer.innerHTML = sections.join("");

    // Clear old preset button references
    this._presetButtons.forEach((btn) => {
      off(btn, "click", this._presetClickListener);
    });
    this._presetButtons = [];

    // Attach click listeners to all preset buttons and track them
    this.presetsContainer.querySelectorAll(".trade-action-preset-btn").forEach((btn) => {
      on(btn, "click", this._presetClickListener);
      this._presetButtons.push(btn);
    });
  }

  _handlePresetClick(e) {
    const btn = e.target.closest(".trade-action-preset-btn");
    if (!btn) {
      return;
    }

    const value = parseFloat(btn.getAttribute("data-value"));
    const type = btn.getAttribute("data-type");

    // Clear all selections
    this.presetsContainer.querySelectorAll(".trade-action-preset-btn").forEach((b) => {
      b.classList.remove("selected");
    });

    // Select this one
    btn.classList.add("selected");
    this._selectedPreset = { value, type };

    // Fill input
    this.inputField.value = value;

    // Clear error and validate
    this._clearError();
    this._updateConfirmButton();
  }

  _handleInputChange() {
    // Clear preset selection when user types
    this.presetsContainer.querySelectorAll(".trade-action-preset-btn").forEach((b) => {
      b.classList.remove("selected");
    });
    this._selectedPreset = null;

    // Clear error and validate
    this._clearError();
    this._updateConfirmButton();
  }

  _updateConfirmButton() {
    const value = this._getInputValue();

    if (value === null || value === "") {
      // Empty input - allow if default behavior is acceptable
      if (this.currentAction === "buy" || this.currentAction === "add") {
        this.confirmBtn.disabled = false; // Allow empty for default
      } else {
        this.confirmBtn.disabled = true;
      }
      return;
    }

    const error = this._validateInput(this.currentAction, value, this.currentContext);
    this.confirmBtn.disabled = error !== null;
  }

  _getInputValue() {
    const raw = this.inputField.value.trim();
    if (raw === "") {
      return "";
    }
    const num = parseFloat(raw);
    return Number.isFinite(num) ? num : null;
  }

  _validateInput(action, value, context) {
    if (value === "") {
      return null; // Allow empty for default behavior
    }

    if (value === null) {
      return "Invalid number";
    }

    if (action === "sell") {
      if (value <= 0 || value > 100) {
        return "Percentage must be between 1 and 100";
      }
    }

    if (action === "buy" || action === "add") {
      if (value <= 0) {
        return "Amount must be greater than 0";
      }
      if (value < 0.001) {
        return "Minimum: 0.001 SOL";
      }
      if (context.balance != null && value > context.balance) {
        return `Insufficient balance (need ${value.toFixed(4)}, have ${context.balance.toFixed(4)})`;
      }
    }

    return null;
  }

  _showError(message) {
    this.errorEl.textContent = message;
    this.errorEl.setAttribute("data-visible", "true");
    this.inputField.classList.add("error");
    this.confirmBtn.disabled = true;
  }

  _clearError() {
    this.errorEl.setAttribute("data-visible", "false");
    this.inputField.classList.remove("error");
  }

  _handleConfirmClick() {
    const value = this._getInputValue();

    // Validate
    if (value !== "" && value !== null) {
      const error = this._validateInput(this.currentAction, value, this.currentContext);
      if (error) {
        this._showError(error);
        return;
      }
    }

    // Build result
    let result;
    if (this.currentAction === "sell") {
      result = {
        percentage: value === "" ? 100 : value,
      };
    } else {
      // buy or add
      result = value === "" ? {} : { amount: value };
    }

    // Resolve promise
    if (this._resolveOpen) {
      this._resolveOpen(result);
      this._resolveOpen = null;
    }

    this.close({ restoreFocus: true });
  }

  _handleCancelClick() {
    if (this._resolveOpen) {
      this._resolveOpen(null); // null = cancelled
      this._resolveOpen = null;
    }
    this.close({ restoreFocus: true });
  }

  _handleCloseClick() {
    this._handleCancelClick();
  }

  _handleOverlayClick(event) {
    if (event.target === this.root) {
      this._handleCancelClick();
    }
  }

  _handleKeyDown(event) {
    if (!this._isOpen) {
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      this._handleCancelClick();
    }

    if (event.key === "Enter" && !this.confirmBtn.disabled) {
      event.preventDefault();
      event.stopPropagation();
      this._handleConfirmClick();
    }
  }
}
