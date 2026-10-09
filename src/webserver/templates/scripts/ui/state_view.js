// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The one loading, empty and error state for a panel whose content cannot render:
 * a bare glyph, a title, a message and an optional retry action, centred in the
 * space the content would take. Token Details tabs and Settings sections render
 * their states through it, so a failed load reads the same everywhere.
 */

import { escapeHtml } from "../core/utils.js";

const KINDS = ["loading", "empty", "error"];

/**
 * Markup of a state view.
 * @param {object} [state]
 * @param {"loading"|"empty"|"error"} [state.kind]
 * @param {string} [state.icon] - a Lucide icon class (`icon-…`)
 * @param {string} [state.title]
 * @param {string} [state.message]
 * @param {string} [state.retry] - the `data-action` of a Retry button; omitted, no button
 * @returns {string}
 */
export function renderStateView({
  kind = "empty",
  icon = "icon-info",
  title = "",
  message = "",
  retry = "",
} = {}) {
  const safeKind = KINDS.includes(kind) ? kind : "empty";
  const safeIcon = /^icon-[a-z0-9-]+$/.test(icon) ? icon : "icon-info";
  const role = safeKind === "error" ? 'role="alert"' : 'role="status" aria-live="polite"';

  if (safeKind === "loading") {
    return `
      <div class="state-view state-view-loading" ${role}>
        <div class="loading-spinner">${escapeHtml(message || I18n.t("common-loading"))}</div>
      </div>
    `;
  }

  return `
    <div class="state-view state-view-${safeKind}" ${role}>
      <i class="state-view-icon ${safeIcon}" aria-hidden="true"></i>
      ${title ? `<div class="state-view-title">${escapeHtml(title)}</div>` : ""}
      ${message ? `<div class="state-view-message">${escapeHtml(message)}</div>` : ""}
      ${
        retry
          ? `<button type="button" class="btn btn-sm btn-secondary state-view-retry" data-action="${escapeHtml(retry)}">
              <i class="icon-refresh-cw" aria-hidden="true"></i> ${escapeHtml(I18n.t("common-action-retry"))}
            </button>`
          : ""
      }
    </div>
  `;
}
