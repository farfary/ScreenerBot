/**
 * Hint Popover Component
 *
 * Provides contextual help hints that appear inline with UI elements.
 * Consists of:
 * - HintTrigger: Small "?" button that opens the popover
 * - HintPopover: The actual popover with hint content
 */

import * as Hints from "../core/hints.js";

// Currently open popover instance
let activePopover = null;

// Click outside handler reference
let outsideClickHandler = null;

/**
 * HintTrigger - Creates the small help icon button
 */
export class HintTrigger {
  /**
   * Render a hint trigger button as HTML string
   * @param {Object} hint - Hint definition from HINTS registry
   * @param {string} hintPath - The path used to look up the hint (e.g., "tokens.poolService")
   * @param {Object} options - Rendering options
   * @returns {string} HTML string for the trigger button
   */
  static render(hint, hintPath, options = {}) {
    if (!hint || !hint.id) return "";
    if (!Hints.isEnabled()) return "";
    if (Hints.isDismissed(hint.id)) return "";

    const size = options.size || "sm"; // sm, md, lg
    const position = options.position || "right"; // Position hint for popover

    return `<button
      class="hint-trigger hint-trigger--${size}"
      data-hint-id="${hint.id}"
      data-hint-path="${hintPath}"
      data-hint-position="${position}"
      type="button"
      aria-label="Help: ${hint.title}"
      title="${hint.title}"
    ><i class="icon-circle-question-mark"></i></button>`;
  }

  /**
   * Programmatically attach a hint trigger to an element
   * @param {HTMLElement} container - Container to append trigger to
   * @param {Object} hint - Hint definition
   * @param {string} hintPath - The path used to look up the hint
   * @param {Object} options - Rendering options
   */
  static attach(container, hint, hintPath, options = {}) {
    if (!container || !hint) return null;
    if (!Hints.isEnabled()) return null;
    if (Hints.isDismissed(hint.id)) return null;

    const html = this.render(hint, hintPath, options);
    if (!html) return null;

    const wrapper = document.createElement("span");
    wrapper.innerHTML = html;
    const trigger = wrapper.firstElementChild;

    container.appendChild(trigger);
    return trigger;
  }

  /**
   * Initialize all hint triggers on the page
   * Call this after page content is rendered
   */
  static initAll() {
    // Set up delegated click handler for all hint triggers
    document.removeEventListener("click", handleTriggerClick);
    document.addEventListener("click", handleTriggerClick);

    // Listen for hints toggle events
    document.removeEventListener("hints:toggle", handleHintsToggle);
    document.addEventListener("hints:toggle", handleHintsToggle);

    // Listen for escape key to close popover
    document.removeEventListener("keydown", handleKeyDown);
    document.addEventListener("keydown", handleKeyDown);
  }
}

/**
 * HintPopover - The popover component showing hint content
 */
export class HintPopover {
  constructor(hint, triggerEl) {
    this.hint = hint;
    this.triggerEl = triggerEl;
    this.el = null;
  }

  /**
   * Show the popover
   */
  show() {
    // Close any existing popover
    if (activePopover) {
      activePopover.close();
    }

    this._create();
    this._position();
    this._attachHandlers();

    activePopover = this;

    // Animate in
    requestAnimationFrame(() => {
      if (this.el) {
        this.el.classList.add("hint-popover--visible");
      }
    });
  }

  /**
   * Close the popover
   */
  close() {
    if (!this.el) return;

    this.el.classList.remove("hint-popover--visible");

    // Remove after animation
    setTimeout(() => {
      if (this.el && this.el.parentNode) {
        this.el.parentNode.removeChild(this.el);
      }
      this.el = null;
    }, 200);

    if (activePopover === this) {
      activePopover = null;
    }

    // Remove outside click handler
    if (outsideClickHandler) {
      setTimeout(() => {
        document.removeEventListener("click", outsideClickHandler);
        outsideClickHandler = null;
      }, 10);
    }
  }

  /**
   * Create the popover DOM
   */
  _create() {
    const content = this._formatContent(this.hint.content);

    this.el = document.createElement("div");
    this.el.className = "hint-popover";
    this.el.setAttribute("role", "tooltip");
    this.el.setAttribute("aria-live", "polite");

    this.el.innerHTML = `
      <div class="hint-popover__arrow"></div>
      <div class="hint-popover__header">
        <div class="hint-popover__icon">
          <i class="icon-info"></i>
        </div>
        <h4 class="hint-popover__title">${escapeHtml(this.hint.title)}</h4>
        <button class="hint-popover__close" type="button" aria-label="Close">
          <i class="icon-x"></i>
        </button>
      </div>
      <div class="hint-popover__content">
        ${content}
      </div>
      ${this._renderFooter()}
    `;

    document.body.appendChild(this.el);
  }

  /**
   * Format hint content with markdown-like syntax
   */
  _formatContent(text) {
    if (!text) return "";

    // Escape HTML first
    let html = escapeHtml(text);

    // Bold: **text**
    html = html.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");

    // Italic: *text*
    html = html.replace(/\*([^*]+)\*/g, "<em>$1</em>");

    // Code: `text`
    html = html.replace(/`([^`]+)`/g, "<code>$1</code>");

    // Lists: lines starting with •
    html = html.replace(/^• (.+)$/gm, '<li class="hint-popover__list-item">$1</li>');

    // Wrap consecutive list items in ul
    html = html.replace(
      /(<li class="hint-popover__list-item">.*<\/li>\n?)+/g,
      '<ul class="hint-popover__list">$&</ul>'
    );

    // Paragraphs: double newlines
    html = html
      .split(/\n\n+/)
      .map((p) => {
        p = p.trim();
        if (!p) return "";
        if (p.startsWith("<ul") || p.startsWith("<li")) return p;
        return `<p>${p}</p>`;
      })
      .join("");

    // Single newlines within paragraphs
    html = html.replace(/([^>])\n([^<])/g, "$1<br>$2");

    return html;
  }

  /**
   * Render footer with learn more link and dismiss checkbox
   */
  _renderFooter() {
    const hasLearnMore = !!this.hint.learnMoreUrl;

    return `
      <div class="hint-popover__footer">
        ${
          hasLearnMore
            ? `<a href="${escapeHtml(this.hint.learnMoreUrl)}" target="_blank" rel="noopener noreferrer" class="hint-popover__learn-more">
            <i class="icon-external-link"></i>
            Learn more
          </a>`
            : "<span></span>"
        }
        <label class="hint-popover__dismiss">
          <input type="checkbox" class="hint-popover__dismiss-checkbox">
          <span>Don't show again</span>
        </label>
      </div>
    `;
  }

  /**
   * Position the popover relative to trigger
   */
  _position() {
    if (!this.el || !this.triggerEl) return;

    const triggerRect = this.triggerEl.getBoundingClientRect();
    const popoverRect = this.el.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const margin = 12;
    const arrowSize = 8;

    // Preferred position from data attribute
    const preferred = this.triggerEl.dataset.hintPosition || "right";

    // Calculate available space in each direction
    const spaceRight = viewportWidth - triggerRect.right;
    const spaceLeft = triggerRect.left;
    const spaceBottom = viewportHeight - triggerRect.bottom;
    const spaceTop = triggerRect.top;

    // Determine best position
    let position = preferred;
    const popoverWidth = Math.min(popoverRect.width, 360);
    const popoverHeight = popoverRect.height;

    // Check if preferred position has enough space
    if (position === "right" && spaceRight < popoverWidth + margin) {
      position = spaceLeft > spaceRight ? "left" : "bottom";
    } else if (position === "left" && spaceLeft < popoverWidth + margin) {
      position = spaceRight > spaceLeft ? "right" : "bottom";
    } else if (position === "bottom" && spaceBottom < popoverHeight + margin) {
      position = spaceTop > spaceBottom ? "top" : "right";
    } else if (position === "top" && spaceTop < popoverHeight + margin) {
      position = spaceBottom > spaceTop ? "bottom" : "right";
    }

    // Calculate position coordinates
    let top, left;
    const arrow = this.el.querySelector(".hint-popover__arrow");

    switch (position) {
      case "right":
        top = triggerRect.top + triggerRect.height / 2 - popoverHeight / 2;
        left = triggerRect.right + margin;
        this.el.classList.add("hint-popover--right");
        break;

      case "left":
        top = triggerRect.top + triggerRect.height / 2 - popoverHeight / 2;
        left = triggerRect.left - popoverWidth - margin;
        this.el.classList.add("hint-popover--left");
        break;

      case "bottom":
        top = triggerRect.bottom + margin;
        left = triggerRect.left + triggerRect.width / 2 - popoverWidth / 2;
        this.el.classList.add("hint-popover--bottom");
        break;

      case "top":
        top = triggerRect.top - popoverHeight - margin;
        left = triggerRect.left + triggerRect.width / 2 - popoverWidth / 2;
        this.el.classList.add("hint-popover--top");
        break;
    }

    // Clamp to viewport
    top = Math.max(margin, Math.min(top, viewportHeight - popoverHeight - margin));
    left = Math.max(margin, Math.min(left, viewportWidth - popoverWidth - margin));

    this.el.style.top = `${top}px`;
    this.el.style.left = `${left}px`;
    this.el.style.maxWidth = `${popoverWidth}px`;
  }

  /**
   * Attach event handlers
   */
  _attachHandlers() {
    // Close button
    const closeBtn = this.el.querySelector(".hint-popover__close");
    if (closeBtn) {
      closeBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        this.close();
      });
    }

    // Dismiss checkbox
    const dismissCheckbox = this.el.querySelector(".hint-popover__dismiss-checkbox");
    if (dismissCheckbox) {
      dismissCheckbox.addEventListener("change", async (e) => {
        if (e.target.checked) {
          await Hints.dismissHint(this.hint.id);
          // Remove the trigger button as well
          if (this.triggerEl && this.triggerEl.parentNode) {
            this.triggerEl.parentNode.removeChild(this.triggerEl);
          }
          this.close();
        }
      });
    }

    // Click outside to close (delayed to avoid immediate close)
    setTimeout(() => {
      outsideClickHandler = (e) => {
        if (this.el && !this.el.contains(e.target) && !this.triggerEl.contains(e.target)) {
          this.close();
        }
      };
      document.addEventListener("click", outsideClickHandler);
    }, 100);
  }
}

/**
 * Handle click on hint trigger buttons
 */
function handleTriggerClick(e) {
  const trigger = e.target.closest(".hint-trigger");
  if (!trigger) return;

  e.preventDefault();
  e.stopPropagation();

  const hintPath = trigger.dataset.hintPath;
  const hintId = trigger.dataset.hintId;
  if (!hintPath || !hintId) return;

  // Get hint from registry using path
  const hint = Hints.getHint(hintPath);
  if (!hint) {
    console.warn(`[HintPopover] Hint not found: ${hintPath}`);
    return;
  }

  // Toggle if clicking same trigger
  if (activePopover && activePopover.hint.id === hintId) {
    activePopover.close();
    return;
  }

  // Show popover
  const popover = new HintPopover(hint, trigger);
  popover.show();
}

/**
 * Handle hints toggle event
 */
function handleHintsToggle(e) {
  const { enabled } = e.detail;

  if (!enabled) {
    // Close active popover
    if (activePopover) {
      activePopover.close();
    }

    // Hide all triggers
    document.querySelectorAll(".hint-trigger").forEach((trigger) => {
      trigger.style.display = "none";
    });
  } else {
    // Show all triggers (except dismissed)
    document.querySelectorAll(".hint-trigger").forEach((trigger) => {
      const hintId = trigger.dataset.hintId;
      if (!Hints.isDismissed(hintId)) {
        trigger.style.display = "";
      }
    });
  }
}

/**
 * Handle keyboard events
 */
function handleKeyDown(e) {
  if (e.key === "Escape" && activePopover) {
    activePopover.close();
  }
}

/**
 * Escape HTML special characters
 */
function escapeHtml(text) {
  if (!text) return "";
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

/**
 * Close any active popover
 */
export function closeActivePopover() {
  if (activePopover) {
    activePopover.close();
  }
}

/**
 * Check if a popover is currently open
 */
export function isPopoverOpen() {
  return activePopover !== null;
}
