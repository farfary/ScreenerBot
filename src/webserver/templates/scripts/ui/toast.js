/**
 * Toast view — renders one notice and nothing else.
 *
 * It owns no lifecycle: `core/toast.js` decides when a toast appears, changes
 * or leaves, and hands this class callbacks. Keeping the two apart is what lets
 * one toast be updated in place through the steps of a trade instead of being
 * torn down and re-created (which is what made them stack).
 */

const CLOSE_ICON = `<svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
  <path d="M1 1L11 11M1 11L11 1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
</svg>`;

export class Toast {
  /**
   * @param {Object} config normalized config from the manager
   * @param {number} repeat how many times this notice has fired
   * @param {{onClose:Function,onHoverChange:Function}} callbacks
   */
  constructor(config, repeat, { onClose, onHoverChange }) {
    this.element = document.createElement("div");
    this.element.className = "toast";
    this.element.setAttribute("role", "status");
    this.element.setAttribute("aria-atomic", "true");

    this.element.innerHTML = `
      <div class="toast__row">
        <i class="toast__icon" aria-hidden="true"></i>
        <div class="toast__body">
          <p class="toast__title"></p>
          <p class="toast__message"></p>
        </div>
        <span class="toast__repeat" aria-hidden="true"></span>
        <button class="toast__close" type="button" aria-label="Dismiss">${CLOSE_ICON}</button>
      </div>
      <div class="toast__progress" role="progressbar" aria-valuemin="0" aria-valuemax="100" hidden>
        <span class="toast__progress-bar"></span>
      </div>`;

    this.iconEl = this.element.querySelector(".toast__icon");
    this.titleEl = this.element.querySelector(".toast__title");
    this.messageEl = this.element.querySelector(".toast__message");
    this.repeatEl = this.element.querySelector(".toast__repeat");
    this.progressEl = this.element.querySelector(".toast__progress");
    this.progressBarEl = this.element.querySelector(".toast__progress-bar");

    this.element.querySelector(".toast__close").addEventListener("click", onClose);
    this.element.addEventListener("mouseenter", () => onHoverChange(true));
    this.element.addEventListener("mouseleave", () => onHoverChange(false));

    this.update(config, repeat);
  }

  /** Paint `config` onto the existing element — never rebuilds it. */
  update(config, repeat) {
    this.element.dataset.type = config.type;
    // An error is the one notice worth interrupting a screen reader for.
    this.element.setAttribute("aria-live", config.type === "error" ? "assertive" : "polite");

    this.iconEl.className = `toast__icon ${config.icon}`;
    this.titleEl.textContent = config.title;

    this.messageEl.textContent = config.message || "";
    this.messageEl.hidden = !config.message;

    this.repeatEl.textContent = repeat > 1 ? `${repeat}` : "";
    this.repeatEl.hidden = repeat <= 1;

    const hasProgress = config.progress !== null;
    this.progressEl.hidden = !hasProgress;
    if (hasProgress) {
      this.progressBarEl.style.width = `${config.progress}%`;
      this.progressEl.setAttribute("aria-valuenow", String(config.progress));
    }
  }

  enter() {
    requestAnimationFrame(() => this.element.classList.add("toast--visible"));
  }

  startExit() {
    this.element.classList.add("toast--exiting");
  }

  destroy() {
    this.element.remove();
  }
}
