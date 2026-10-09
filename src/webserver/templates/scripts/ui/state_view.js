// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The one loading, empty and error state for a panel whose content cannot render:
 * a heading row (a bare glyph inline with the title), a message, and an optional retry
 * or call to action, centred in the space the content would take. Every dashboard
 * panel, list, table, dialog tab and drawer renders its states through it, so an empty
 * or failed view reads the same everywhere. The glyph never sits alone on a row above
 * its heading; without a title it leads the message.
 */

import { escapeHtml } from "../core/utils.js";

const KINDS = ["loading", "empty", "success", "error"];

/**
 * Markup of a state view.
 * @param {object} [state]
 * @param {"loading"|"empty"|"success"|"error"} [state.kind] - `success` reports a finished
 *   check that found nothing to do
 * @param {string} [state.icon] - a Lucide icon class (`icon-…`)
 * @param {string} [state.title]
 * @param {string} [state.message]
 * @param {string} [state.retry] - the `data-action` of a Retry button; omitted, no button
 * @param {{id: string, label: string, primary?: boolean}} [state.action] - a call to
 *   action: a button with that `data-action` and label; the caller wires its click
 * @param {boolean} [state.compact] - sized to its content instead of the panel's height
 *   (a table body, a drawer list)
 * @returns {string}
 */
export function renderStateView({
  kind = "empty",
  icon = "icon-info",
  title = "",
  message = "",
  retry = "",
  action = null,
  compact = false,
} = {}) {
  const safeKind = KINDS.includes(kind) ? kind : "empty";
  const safeIcon = /^icon-[a-z0-9-]+$/.test(icon) ? icon : "icon-info";
  const role = safeKind === "error" ? 'role="alert"' : 'role="status" aria-live="polite"';
  const size = compact ? " state-view-compact" : "";

  if (safeKind === "loading") {
    return `
      <div class="state-view state-view-loading${size}" ${role}>
        <div class="loading-spinner">${escapeHtml(message || I18n.t("common-loading"))}</div>
      </div>
    `;
  }

  const headingClass = title ? "state-view-title" : "state-view-message";
  const heading = title || message;
  const detail = title ? message : "";
  return `
    <div class="state-view state-view-${safeKind}${size}" ${role}>
      <div class="state-view-heading">
        <i class="state-view-icon ${safeIcon}" aria-hidden="true"></i>
        ${heading ? `<span class="${headingClass}">${escapeHtml(heading)}</span>` : ""}
      </div>
      ${detail ? `<div class="state-view-message">${escapeHtml(detail)}</div>` : ""}
      ${
        retry
          ? `<button type="button" class="btn btn-sm btn-secondary state-view-retry" data-action="${escapeHtml(retry)}">
              <i class="icon-refresh-cw" aria-hidden="true"></i> ${escapeHtml(I18n.t("common-action-retry"))}
            </button>`
          : ""
      }
      ${
        action
          ? `<button type="button" class="btn btn-sm ${action.primary ? "btn-primary" : "btn-secondary"} state-view-action" data-action="${escapeHtml(action.id)}">${escapeHtml(action.label)}</button>`
          : ""
      }
    </div>
  `;
}
