/**
 * Position Remove Dialog
 *
 * Lets the user remove a position from the dashboard in one of two modes:
 *   - Archive (DEFAULT): reversible. Hides it from Open/Closed into the Archived
 *     tab. Nothing is sold; trades/transactions are kept.
 *   - Permanent delete: irreversible hard-delete of the position and its history.
 *
 * For a position that is still OPEN, an extra warning explains that removing it
 * frees the trade slot and stops tracking but does NOT sell the token.
 *
 * Follows the structure/animation of confirmation_dialog.js and the visual
 * language of the trade dialog. Resolves to { confirmed, mode } where mode is
 * "archive" | "delete". Cancel/backdrop/Esc resolve { confirmed: false }.
 */

import { playPanelOpen, playPanelClose, playSuccess } from "../core/sounds.js";

const escapeHTML = (str) => {
  const div = document.createElement("div");
  div.textContent = str == null ? "" : String(str);
  return div.innerHTML;
};

const shortMint = (mint) =>
  mint && mint.length > 10 ? `${mint.slice(0, 4)}…${mint.slice(-4)}` : mint || "";

class PositionRemoveDialog {
  static activeDialog = null;

  /**
   * @param {Object} config
   * @param {string} config.symbol - Token symbol
   * @param {string} config.mint - Token mint
   * @param {boolean} [config.isOpen=false] - Whether the position is still open
   * @returns {Promise<{confirmed: boolean, mode: ('archive'|'delete')}>}
   */
  static async show(config) {
    if (PositionRemoveDialog.activeDialog) {
      PositionRemoveDialog.activeDialog.destroy();
    }
    return new Promise((resolve) => {
      const dialog = new PositionRemoveDialog(config, resolve);
      PositionRemoveDialog.activeDialog = dialog;
      dialog.render();
    });
  }

  constructor(config, resolver) {
    this.config = {
      symbol: config.symbol || "?",
      mint: config.mint || "",
      isOpen: config.isOpen === true,
    };
    this.resolver = resolver;
    this.element = null;
    this.backdrop = null;
    // Archive is the safe default.
    this.mode = "archive";
  }

  render() {
    this.backdrop = document.createElement("div");
    this.backdrop.className = "position-remove-backdrop";
    this.backdrop.setAttribute("role", "presentation");

    this.element = document.createElement("div");
    this.element.className = "position-remove-dialog";
    this.element.dataset.mode = this.mode;
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-modal", "true");
    this.element.setAttribute("aria-labelledby", "position-remove-title");

    const sym = escapeHTML(this.config.symbol);
    const mintShort = escapeHTML(shortMint(this.config.mint));

    const openWarning = this.config.isOpen
      ? `<div class="position-remove-open-warning" role="note">
           <i class="icon-triangle-alert" aria-hidden="true"></i>
           <div>
             <strong>This position is still open.</strong>
             The bot is holding this token. Removing it frees the trade slot and
             stops tracking — but it does <strong>not</strong> sell. Sell first if
             you want your SOL back.
           </div>
         </div>`
      : "";

    this.element.innerHTML = `
      <div class="position-remove-header">
        <h3 class="position-remove-title" id="position-remove-title">Remove position</h3>
        <p class="position-remove-subtitle">${sym} <span class="position-remove-mint">${mintShort}</span></p>
      </div>

      ${openWarning}

      <div class="position-remove-choices" role="radiogroup" aria-label="Removal mode">
        <button type="button" class="position-remove-choice" data-mode="archive" role="radio" aria-checked="true">
          <span class="position-remove-choice-icon"><i class="icon-archive" aria-hidden="true"></i></span>
          <span class="position-remove-choice-body">
            <span class="position-remove-choice-title">Archive <span class="position-remove-badge">Recommended</span></span>
            <span class="position-remove-choice-desc">Hide it into the Archived tab. Reversible anytime — nothing is sold and all trades stay on record.</span>
          </span>
          <span class="position-remove-choice-check" aria-hidden="true"><i class="icon-check"></i></span>
        </button>

        <button type="button" class="position-remove-choice" data-mode="delete" role="radio" aria-checked="false">
          <span class="position-remove-choice-icon"><i class="icon-trash-2" aria-hidden="true"></i></span>
          <span class="position-remove-choice-body">
            <span class="position-remove-choice-title">Delete permanently</span>
            <span class="position-remove-choice-desc">Erase this position and its full history from the database.</span>
          </span>
          <span class="position-remove-choice-check" aria-hidden="true"><i class="icon-check"></i></span>
        </button>
      </div>

      <div class="position-remove-danger" data-visible="false" role="alert">
        <i class="icon-triangle-alert" aria-hidden="true"></i>
        <span>This permanently removes the position and its history. <strong>This cannot be undone.</strong> Your transactions and token data are not affected.</span>
      </div>

      <div class="position-remove-footer">
        <button type="button" class="position-remove-btn position-remove-btn--cancel" data-action="cancel">Cancel</button>
        <button type="button" class="position-remove-btn position-remove-btn--confirm" data-action="confirm">
          <span class="position-remove-confirm-label">Archive position</span>
        </button>
      </div>
    `;

    this._attachEventListeners();

    document.body.appendChild(this.backdrop);
    document.body.appendChild(this.element);

    requestAnimationFrame(() => {
      this.backdrop.classList.add("position-remove-backdrop--visible");
      this.element.classList.add("position-remove-dialog--visible");
      playPanelOpen();
    });

    setTimeout(() => {
      this.element.querySelector('[data-action="confirm"]')?.focus();
    }, 100);

    this._trapFocus();
    this._syncMode();
  }

  _attachEventListeners() {
    this._clickHandler = (e) => {
      const choice = e.target.closest(".position-remove-choice");
      if (choice) {
        this.mode = choice.dataset.mode === "delete" ? "delete" : "archive";
        this._syncMode();
        return;
      }
      const actionBtn = e.target.closest("[data-action]");
      if (actionBtn) {
        if (actionBtn.dataset.action === "confirm") this._handleConfirm();
        else this._handleCancel();
      }
    };
    this.element.addEventListener("click", this._clickHandler);

    this._backdropHandler = () => this._handleCancel();
    this.backdrop.addEventListener("click", this._backdropHandler);

    this._keydownHandler = (e) => {
      if (e.key === "Escape") {
        e.preventDefault();
        this._handleCancel();
      } else if (e.key === "Enter") {
        e.preventDefault();
        this._handleConfirm();
      }
    };
    document.addEventListener("keydown", this._keydownHandler);
  }

  /** Reflect the selected mode in the choices, danger banner, and confirm button. */
  _syncMode() {
    if (!this.element) return;
    this.element.dataset.mode = this.mode;

    this.element.querySelectorAll(".position-remove-choice").forEach((el) => {
      const active = el.dataset.mode === this.mode;
      el.classList.toggle("is-selected", active);
      el.setAttribute("aria-checked", active ? "true" : "false");
    });

    const danger = this.element.querySelector(".position-remove-danger");
    if (danger) danger.dataset.visible = this.mode === "delete" ? "true" : "false";

    const confirmBtn = this.element.querySelector(".position-remove-btn--confirm");
    const label = this.element.querySelector(".position-remove-confirm-label");
    if (confirmBtn && label) {
      const isDelete = this.mode === "delete";
      confirmBtn.classList.toggle("is-danger", isDelete);
      label.textContent = isDelete ? "Delete permanently" : "Archive position";
    }
  }

  _handleConfirm() {
    playSuccess();
    this._resolve({ confirmed: true, mode: this.mode });
    this.destroy();
  }

  _handleCancel() {
    playPanelClose();
    this._resolve({ confirmed: false, mode: this.mode });
    this.destroy();
  }

  _resolve(result) {
    if (this.resolver) {
      this.resolver(result);
      this.resolver = null;
    }
  }

  destroy() {
    // Resolve as cancelled if still pending (closed without an explicit action).
    this._resolve({ confirmed: false, mode: this.mode, cancelled: true });

    if (this._keydownHandler) {
      document.removeEventListener("keydown", this._keydownHandler);
      this._keydownHandler = null;
    }
    if (this._clickHandler && this.element) {
      this.element.removeEventListener("click", this._clickHandler);
      this._clickHandler = null;
    }
    if (this._backdropHandler && this.backdrop) {
      this.backdrop.removeEventListener("click", this._backdropHandler);
      this._backdropHandler = null;
    }
    if (this._trapFocusHandler && this.element) {
      this.element.removeEventListener("keydown", this._trapFocusHandler);
      this._trapFocusHandler = null;
    }

    this.backdrop?.classList.remove("position-remove-backdrop--visible");
    this.element?.classList.remove("position-remove-dialog--visible");

    const backdrop = this.backdrop;
    const element = this.element;
    setTimeout(() => {
      backdrop?.parentNode && backdrop.remove();
      element?.parentNode && element.remove();
      if (PositionRemoveDialog.activeDialog === this) {
        PositionRemoveDialog.activeDialog = null;
      }
    }, 250);
  }

  _trapFocus() {
    const focusable = this.element.querySelectorAll(
      'button, [href], input, [tabindex]:not([tabindex="-1"])'
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    this._trapFocusHandler = (e) => {
      if (e.key !== "Tab") return;
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };
    this.element.addEventListener("keydown", this._trapFocusHandler);
  }
}

export { PositionRemoveDialog };
