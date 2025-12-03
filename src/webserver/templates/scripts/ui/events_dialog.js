import { on, off } from "../core/dom.js";
import * as Utils from "../core/utils.js";

// EventDetailsDialog renders a modal overlay for inspecting full event data.
const SEVERITY_BADGES = {
  info: '<span class="badge"><i class="icon-info"></i> Info</span>',
  warn: '<span class="badge warning"><i class="icon-triangle-alert"></i> Warning</span>',
  warning: '<span class="badge warning"><i class="icon-triangle-alert"></i> Warning</span>',
  error: '<span class="badge error"><i class="icon-x"></i> Error</span>',
  critical: '<span class="badge error"><i class="icon-circle-alert"></i> Critical</span>',
  debug: '<span class="badge secondary"><i class="icon-bug"></i> Debug</span>',
};

function formatSeverityBadge(value) {
  if (!value) {
    return "";
  }
  const key = String(value).toLowerCase();
  if (SEVERITY_BADGES[key]) {
    return SEVERITY_BADGES[key];
  }
  return `<span class="badge">${Utils.escapeHtml(String(value))}</span>`;
}

function formatMintDisplay(mint) {
  if (!mint) {
    return "—";
  }
  const trimmed = String(mint).trim();
  if (!trimmed) {
    return "—";
  }
  const short = `${trimmed.slice(0, 4)}...${trimmed.slice(-4)}`;
  const safeFull = Utils.escapeHtml(trimmed);
  const safeShort = Utils.escapeHtml(short);
  return `<code class="mono-text" title="${safeFull}">${safeShort}</code>`;
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
            <h2 id="events-dialog-title" class="events-dialog-title">Event details</h2>
            <div class="events-dialog-subtitle"></div>
          </div>
          <button type="button" class="events-dialog-close" data-action="close" aria-label="Close dialog">&times;</button>
    </header>
        <div class="events-dialog-body">
          <div class="events-dialog-message" data-visible="false"></div>
          <div class="events-dialog-fields"></div>
          <section class="events-dialog-payload" data-visible="false">
            <h3 class="events-dialog-section-title">Payload</h3>
            <pre class="events-dialog-payload-code"><code></code></pre>
          </section>
        </div>
        <footer class="events-dialog-footer">
          <button type="button" class="events-dialog-copy" data-action="copy" title="Copy all event details">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="5" y="5" width="9" height="9" rx="1.5"></rect>
              <path d="M3 10V3a1.5 1.5 0 0 1 1.5-1.5H10"></path>
            </svg>
            <span>Copy Details</span>
          </button>
          <button type="button" class="events-dialog-dismiss" data-action="close">Close</button>
        </footer>
      </div>
    `;

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

    if (
      restoreFocus &&
      this._previousActiveElement &&
      typeof this._previousActiveElement.focus === "function"
    ) {
      try {
        this._previousActiveElement.focus();
      } catch (_error) {
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

    const message = coerceText(event.message).trim();
    const fallback = event.category ? `${event.category} event` : "Event details";
    const heading = message
      ? message.length > 140
        ? `${message.slice(0, 140)}...`
        : message
      : fallback;

    this.titleEl.textContent = heading || "Event details";
    this.titleEl.title = message || fallback;
    this.dialog.setAttribute("aria-label", this.titleEl.textContent);

    const severityBadge = formatSeverityBadge(event.severity);
    const metaParts = [];
    if (event.category) {
      metaParts.push(Utils.escapeHtml(String(event.category)));
    }
    if (event.subtype) {
      metaParts.push(Utils.escapeHtml(String(event.subtype)));
    }
    if (event.event_time) {
      const formatted = Utils.formatTimestamp(event.event_time, {
        includeSeconds: true,
        fallback: "N/A",
      });
      metaParts.push(Utils.escapeHtml(formatted));
    }

    const metaHtml =
      metaParts.length > 0
        ? `<span class="events-dialog-subtitle-meta">${metaParts.join(" &bull; ")}</span>`
        : "";
    const pieces = [];
    if (severityBadge) {
      pieces.push(severityBadge);
    }
    if (metaHtml) {
      pieces.push(metaHtml);
    }
    this.subtitleEl.innerHTML = pieces.join(" ");
  }

  _renderMessage(event) {
    if (!this.messageEl) {
      return;
    }

    const message = coerceText(event.message).trim();
    if (message) {
      this.messageEl.textContent = message;
      this.messageEl.setAttribute("data-visible", "true");
      this.messageEl.title = message;
    } else {
      this.messageEl.textContent = "";
      this.messageEl.setAttribute("data-visible", "false");
      this.messageEl.removeAttribute("title");
    }
  }

  _renderFields(event) {
    if (!this.fieldsEl) {
      return;
    }

    const fields = [];

    fields.push({ label: "Event ID", value: event.id });
    if (event.severity) {
      fields.push({ label: "Severity", value: formatSeverityBadge(event.severity), isHtml: true });
    }
    if (event.category) {
      fields.push({ label: "Category", value: event.category });
    }
    if (event.subtype) {
      fields.push({ label: "Subtype", value: event.subtype });
    }
    if (event.mint) {
      fields.push({ label: "Token Mint", value: formatMintDisplay(event.mint), isHtml: true });
    }
    if (event.reference_id) {
      fields.push({ label: "Reference", value: event.reference_id });
    }
    if (event.event_time) {
      fields.push({
        label: "Event Time",
        value: Utils.formatTimestamp(event.event_time, {
          includeSeconds: true,
          fallback: "N/A",
        }),
      });
      fields.push({
        label: "Age",
        value: Utils.formatTimeAgo(event.event_time, { fallback: "-" }),
      });
    }
    if (event.created_at) {
      fields.push({
        label: "Created",
        value: Utils.formatTimestamp(event.created_at, {
          includeSeconds: true,
          fallback: "N/A",
        }),
      });
    }

    const html = fields.map((field) => this._renderField(field)).join("");

    this.fieldsEl.innerHTML = html;
  }

  _renderField(field) {
    const classes = ["events-dialog-field"];
    if (field.wide) {
      classes.push("events-dialog-field--wide");
    }
    const label = Utils.escapeHtml(field.label || "");
    const value = field.isHtml ? field.value : safeText(field.value);
    return `
      <div class="${classes.join(" ")}">
        <span class="events-dialog-field-label">${label}</span>
        <span class="events-dialog-field-value">${value}</span>
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
      } catch (_error) {
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
      } catch (_error) {
        this._showCopyFeedback(false);
      }
      document.body.removeChild(textarea);
    }
  }

  _formatEventForCopy(event) {
    const lines = [];

    lines.push("=".repeat(60));
    lines.push("EVENT DETAILS");
    lines.push("=".repeat(60));
    lines.push("");

    // Basic info
    lines.push(`Event ID: ${event.id || "N/A"}`);
    lines.push(`Severity: ${event.severity || "N/A"}`);
    lines.push(`Category: ${event.category || "N/A"}`);
    lines.push(`Subtype: ${event.subtype || "N/A"}`);

    if (event.event_time) {
      const formatted = Utils.formatTimestamp(event.event_time, {
        includeSeconds: true,
        fallback: "N/A",
      });
      lines.push(`Event Time: ${formatted}`);
      lines.push(`Age: ${Utils.formatTimeAgo(event.event_time, { fallback: "-" })}`);
    }

    if (event.created_at) {
      const formatted = Utils.formatTimestamp(event.created_at, {
        includeSeconds: true,
        fallback: "N/A",
      });
      lines.push(`Created: ${formatted}`);
    }

    if (event.mint) {
      lines.push(`Token Mint: ${event.mint}`);
    }

    if (event.reference_id) {
      lines.push(`Reference: ${event.reference_id}`);
    }

    // Message
    if (event.message) {
      lines.push("");
      lines.push("-".repeat(60));
      lines.push("MESSAGE");
      lines.push("-".repeat(60));
      lines.push(event.message);
    }

    // Payload
    if (event.payload && typeof event.payload === "object") {
      lines.push("");
      lines.push("-".repeat(60));
      lines.push("PAYLOAD");
      lines.push("-".repeat(60));
      try {
        lines.push(JSON.stringify(event.payload, null, 2));
      } catch (_error) {
        lines.push(String(event.payload));
      }
    } else if (event.payload !== null && event.payload !== undefined) {
      lines.push("");
      lines.push("-".repeat(60));
      lines.push("PAYLOAD");
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
    const icon = success
      ? '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 8l3 3 7-7"></path></svg>'
      : '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4l8 8M12 4l-8 8"></path></svg>';
    const text = success ? "Copied!" : "Failed";

    this.copyButton.innerHTML = `${icon}<span>${text}</span>`;
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
