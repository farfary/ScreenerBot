// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Events dialog - modal EventDetailsDialog showing the message, key-value field list, JSON payload section and copy-to-clipboard export of one event.

import { on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import { createFocusTrap } from "../core/utils.js";
import { eventCategoryLabel, eventSubtypeLabel, severityBadge } from "./event_labels.js";
import { renderAddress } from "./token_identity.js";

/** Display text of an event: catalog text when the row carries it, else the stored message. */
export function eventMessageText(event) {
  return event.text ? I18n.text(event.text) : event.message;
}

// EventDetailsDialog renders a modal overlay for inspecting full event data.
const notAvailable = () => I18n.t("events-dialog-not-available");

/** Longest message the dialog title carries; a longer one is shown in the body instead. */
const TITLE_MESSAGE_MAX_CHARS = 140;

/** The event message when it fits the title, else an empty string. */
function titleMessage(event) {
  const message = coerceText(eventMessageText(event)).trim();
  return message.length <= TITLE_MESSAGE_MAX_CHARS ? message : "";
}

/** Event time to the second, as the dialog and the copy export show it. */
function formatEventTimestamp(value) {
  return Utils.formatTimestamp(value, { includeSeconds: true, fallback: notAvailable() });
}

function formatMintDisplay(mint) {
  if (!mint) {
    return "—";
  }
  const trimmed = String(mint).trim();
  if (!trimmed) {
    return "—";
  }
  return renderAddress(trimmed);
}

function coerceText(value) {
  if (value === null || value === undefined) {
    return "";
  }
  return String(value);
}

function safeText(value) {
  const text = coerceText(value).trim();
  if (!text) {
    return "—";
  }
  return Utils.escapeHtml(text);
}

export class EventDetailsDialog {
  constructor() {
    this.root = null;
    this.dialog = null;
    this.titleEl = null;
    this.subtitleEl = null;
    this.messageEl = null;
    this.fieldsEl = null;
    this.payloadSection = null;
    this.payloadCodeEl = null;
    this.closeButtons = [];
    this.copyButton = null;

    this._isOpen = false;
    this._previousActiveElement = null;
    this._currentEvent = null;

    this._overlayListener = this._handleOverlayClick.bind(this);
    this._closeListener = this._handleCloseClick.bind(this);
    this._keyListener = this._handleKeyDown.bind(this);
    this._copyListener = this._handleCopyClick.bind(this);
    this._focusTrap = null;

    this._ensureElements();
  }

  _ensureElements() {
    if (this.root) {
      return;
    }

    const overlay = document.createElement("div");
    overlay.className = "events-dialog-overlay";
    overlay.setAttribute("role", "presentation");
    overlay.setAttribute("aria-hidden", "true");

    overlay.innerHTML = `
      <div class="events-dialog" role="dialog" aria-modal="true" aria-labelledby="events-dialog-title" tabindex="-1">
        <header class="events-dialog-header">
          <div class="events-dialog-heading">
            <h2 id="events-dialog-title" class="events-dialog-title" data-l10n-id="events-dialog-title"></h2>
            <div class="events-dialog-subtitle"></div>
          </div>
          <button type="button" class="modal-close" data-action="close" data-l10n-id="events-dialog-close">
            <i class="icon-x" aria-hidden="true"></i>
          </button>
        </header>
        <div class="events-dialog-body">
          <div class="events-dialog-message" data-visible="false"></div>
          <dl class="events-dialog-fields"></dl>
          <section class="events-dialog-payload" data-visible="false">
            <h3 class="events-dialog-section-title" data-l10n-id="events-dialog-payload"></h3>
            <pre class="events-dialog-payload-code"><code></code></pre>
          </section>
        </div>
        <footer class="events-dialog-footer">
          <button type="button" class="btn btn-secondary events-dialog-copy" data-action="copy" data-l10n-id="events-dialog-copy-title">
            <i class="icon-copy" aria-hidden="true"></i>
            <span data-l10n-id="events-dialog-copy"></span>
          </button>
        </footer>
      </div>
    `;

    I18n.localizeTree(overlay);
    document.body.appendChild(overlay);

    this.root = overlay;
    this.dialog = overlay.querySelector(".events-dialog");
    this.titleEl = overlay.querySelector(".events-dialog-title");
    this.subtitleEl = overlay.querySelector(".events-dialog-subtitle");
    this.messageEl = overlay.querySelector(".events-dialog-message");
    this.fieldsEl = overlay.querySelector(".events-dialog-fields");
    this.payloadSection = overlay.querySelector(".events-dialog-payload");
    this.payloadCodeEl = overlay.querySelector(".events-dialog-payload-code code");
    this.closeButtons = Array.from(overlay.querySelectorAll('[data-action="close"]'));
    this.copyButton = overlay.querySelector('[data-action="copy"]');

    on(overlay, "click", this._overlayListener);
    this.closeButtons.forEach((button) => on(button, "click", this._closeListener));
    if (this.copyButton) {
      on(this.copyButton, "click", this._copyListener);
    }
  }

  open(event) {
    if (!event) {
      return;
    }

    // Guard against multiple simultaneous opens
    if (this._isOpen) {
      console.warn("[EventsDialog] Dialog already open, ignoring duplicate request");
      return;
    }

    this._ensureElements();

    this._previousActiveElement =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;

    this._currentEvent = event;
    this._render(event);

    this.root.classList.add("is-visible");
    this.root.setAttribute("aria-hidden", "false");
    document.body.classList.add("events-dialog-open");
    this._isOpen = true;

    document.addEventListener("keydown", this._keyListener, true);

    // Activate focus trap
    this._focusTrap = createFocusTrap(this.dialog);
    this._focusTrap.activate();

    requestAnimationFrame(() => {
      if (!this._isOpen) {
        return;
      }
      this.dialog?.focus();
      const firstCloseBtn = this.closeButtons[0];
      if (firstCloseBtn) {
        firstCloseBtn.focus();
      }
    });
  }

  close({ restoreFocus = true } = {}) {
    if (!this._isOpen) {
      return;
    }

    this.root.classList.remove("is-visible");
    this.root.setAttribute("aria-hidden", "true");
    document.body.classList.remove("events-dialog-open");
    this._isOpen = false;

    document.removeEventListener("keydown", this._keyListener, true);

    // Deactivate focus trap
    if (this._focusTrap) {
      this._focusTrap.deactivate();
      this._focusTrap = null;
    }

    if (
      restoreFocus &&
      this._previousActiveElement &&
      typeof this._previousActiveElement.focus === "function"
    ) {
      try {
        this._previousActiveElement.focus();
      } catch {
        // Ignore focus errors silently
      }
    }
    this._previousActiveElement = null;
    this._currentEvent = null;
  }

  destroy() {
    this.close({ restoreFocus: false });
    if (!this.root) {
      return;
    }

    off(this.root, "click", this._overlayListener);
    this.closeButtons.forEach((button) => off(button, "click", this._closeListener));
    if (this.copyButton) {
      off(this.copyButton, "click", this._copyListener);
    }

    if (this.root.parentNode) {
      this.root.parentNode.removeChild(this.root);
    }

    this.root = null;
    this.dialog = null;
    this.titleEl = null;
    this.subtitleEl = null;
    this.messageEl = null;
    this.fieldsEl = null;
    this.payloadSection = null;
    this.payloadCodeEl = null;
    this.closeButtons = [];
    this.copyButton = null;
    this._currentEvent = null;
  }

  _render(event) {
    this._renderHeader(event);
    this._renderMessage(event);
    this._renderFields(event);
    this._renderPayload(event.payload);
  }

  _renderHeader(event) {
    if (!this.titleEl || !this.subtitleEl || !this.dialog) {
      return;
    }

    const message = titleMessage(event);
    const heading =
      message ||
      (event.category
        ? I18n.t("events-dialog-category-event", { category: eventCategoryLabel(event.category) })
        : I18n.t("events-dialog-title"));

    this.titleEl.textContent = heading;
    this.titleEl.title = heading;
    this.dialog.setAttribute("aria-label", heading);

    const pieces = [];
    const badge = severityBadge(event.severity);
    if (badge) {
      pieces.push(badge);
    }
    const metaParts = [];
    if (event.category && message) {
      metaParts.push(Utils.escapeHtml(eventCategoryLabel(event.category)));
    }
    if (event.subtype) {
      metaParts.push(Utils.escapeHtml(eventSubtypeLabel(event.subtype)));
    }
    if (metaParts.length > 0) {
      pieces.push(`<span class="events-dialog-subtitle-meta">${metaParts.join(" &bull; ")}</span>`);
    }
    this.subtitleEl.innerHTML = pieces.join(" ");
  }

  /** Shows the message in the body only when it is too long for the title. */
  _renderMessage(event) {
    if (!this.messageEl) {
      return;
    }

    const message = coerceText(eventMessageText(event)).trim();
    const inBody = Boolean(message) && !titleMessage(event);
    this.messageEl.textContent = inBody ? message : "";
    this.messageEl.setAttribute("data-visible", inBody ? "true" : "false");
  }

  /**
   * Key-value list of what the header does not already show: the id, token,
   * reference and one time with its age. The creation time is listed only when it
   * differs from the event time at the shown precision.
   */
  _renderFields(event) {
    if (!this.fieldsEl) {
      return;
    }

    const fields = [{ label: I18n.t("events-dialog-field-id"), value: safeText(event.id) }];
    if (event.mint) {
      fields.push({
        label: I18n.t("events-dialog-field-mint"),
        value: formatMintDisplay(event.mint),
      });
    }
    if (event.reference_id) {
      fields.push({
        label: I18n.t("events-dialog-field-reference"),
        value: safeText(event.reference_id),
      });
    }
    const eventTime = event.event_time ? formatEventTimestamp(event.event_time) : "";
    if (eventTime) {
      const age = Utils.formatTimeAgo(event.event_time, { fallback: "" });
      fields.push({
        label: I18n.t("events-dialog-field-time"),
        value: age
          ? `${safeText(eventTime)} <span class="events-dialog-field-aside">${safeText(age)}</span>`
          : safeText(eventTime),
      });
    }
    if (event.created_at) {
      const created = formatEventTimestamp(event.created_at);
      if (created !== eventTime) {
        fields.push({ label: I18n.t("events-dialog-field-created"), value: safeText(created) });
      }
    }

    this.fieldsEl.innerHTML = fields.map((field) => this._renderField(field)).join("");
  }

  /** One key-value row; `value` is already escaped markup. */
  _renderField(field) {
    return `
      <div class="events-dialog-field">
        <dt class="events-dialog-field-label">${Utils.escapeHtml(field.label)}</dt>
        <dd class="events-dialog-field-value">${field.value}</dd>
      </div>
    `;
  }

  _renderPayload(payload) {
    if (!this.payloadSection || !this.payloadCodeEl) {
      return;
    }

    if (payload && typeof payload === "object") {
      try {
        this.payloadCodeEl.textContent = JSON.stringify(payload, null, 2);
      } catch {
        this.payloadCodeEl.textContent = coerceText(payload);
      }
      this.payloadSection.setAttribute("data-visible", "true");
      return;
    }

    if (payload !== null && payload !== undefined) {
      this.payloadCodeEl.textContent = coerceText(payload);
      this.payloadSection.setAttribute("data-visible", "true");
      return;
    }

    this.payloadCodeEl.textContent = "";
    this.payloadSection.setAttribute("data-visible", "false");
  }

  _handleOverlayClick(event) {
    if (event.target === this.root) {
      this.close();
    }
  }

  _handleCloseClick(event) {
    event.preventDefault();
    this.close();
  }

  _handleKeyDown(event) {
    if (!this._isOpen) {
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      this.close();
    }
  }

  _handleCopyClick(event) {
    event.preventDefault();
    if (!this._currentEvent) {
      return;
    }

    const textToCopy = this._formatEventForCopy(this._currentEvent);

    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard
        .writeText(textToCopy)
        .then(() => {
          this._showCopyFeedback(true);
        })
        .catch(() => {
          this._showCopyFeedback(false);
        });
    } else {
      // Fallback for older browsers
      const textarea = document.createElement("textarea");
      textarea.value = textToCopy;
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.appendChild(textarea);
      textarea.select();
      try {
        document.execCommand("copy");
        this._showCopyFeedback(true);
      } catch {
        this._showCopyFeedback(false);
      }
      document.body.removeChild(textarea);
    }
  }

  _formatEventForCopy(event) {
    const lines = [];

    lines.push("=".repeat(60));
    lines.push(I18n.t("events-dialog-export-heading"));
    lines.push("=".repeat(60));
    lines.push("");

    // Basic info
    const line = (label, value) =>
      lines.push(I18n.t("events-dialog-export-line", { label, value }));
    line(I18n.t("events-dialog-field-id"), event.id || notAvailable());
    line(I18n.t("events-dialog-field-severity"), event.severity || notAvailable());
    line(I18n.t("events-dialog-field-category"), event.category || notAvailable());
    line(I18n.t("events-dialog-field-subtype"), event.subtype || notAvailable());

    if (event.event_time) {
      line(I18n.t("events-dialog-field-time"), formatEventTimestamp(event.event_time));
      line(
        I18n.t("events-dialog-field-age"),
        Utils.formatTimeAgo(event.event_time, { fallback: "-" })
      );
    }

    if (event.created_at) {
      line(I18n.t("events-dialog-field-created"), formatEventTimestamp(event.created_at));
    }

    if (event.mint) {
      line(I18n.t("events-dialog-field-mint"), event.mint);
    }

    if (event.reference_id) {
      line(I18n.t("events-dialog-field-reference"), event.reference_id);
    }

    // Message
    if (event.message) {
      lines.push("");
      lines.push("-".repeat(60));
      lines.push(I18n.t("events-dialog-export-message"));
      lines.push("-".repeat(60));
      lines.push(event.message);
    }

    // Payload
    if (event.payload && typeof event.payload === "object") {
      lines.push("");
      lines.push("-".repeat(60));
      lines.push(I18n.t("events-dialog-export-payload"));
      lines.push("-".repeat(60));
      try {
        lines.push(JSON.stringify(event.payload, null, 2));
      } catch {
        lines.push(String(event.payload));
      }
    } else if (event.payload !== null && event.payload !== undefined) {
      lines.push("");
      lines.push("-".repeat(60));
      lines.push(I18n.t("events-dialog-export-payload"));
      lines.push("-".repeat(60));
      lines.push(String(event.payload));
    }

    lines.push("");
    lines.push("=".repeat(60));

    return lines.join("\n");
  }

  _showCopyFeedback(success) {
    if (!this.copyButton) {
      return;
    }

    const originalContent = this.copyButton.innerHTML;
    const icon = `<i class="${success ? "icon-check" : "icon-x"}" aria-hidden="true"></i>`;
    const text = success ? I18n.t("events-dialog-copy-done") : I18n.t("events-dialog-copy-failed");

    this.copyButton.innerHTML = `${icon}<span>${Utils.escapeHtml(text)}</span>`;
    this.copyButton.classList.add(success ? "success" : "error");
    this.copyButton.disabled = true;

    setTimeout(() => {
      if (!this.copyButton) {
        return;
      }
      this.copyButton.innerHTML = originalContent;
      this.copyButton.classList.remove("success", "error");
      this.copyButton.disabled = false;
    }, 2000);
  }
}
