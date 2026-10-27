// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The one notice: guidance, a caution or a failure shown beside content that still
 * renders, as opposed to the state view that replaces content which cannot. A heading
 * row (a bare glyph inline with the title), a message, an optional trusted extra block
 * and at most one action, on a surface toned by its variant. Without a title the glyph
 * leads the message. Static page markup that cannot call this spells the same shape.
 */

import { escapeHtml } from "../core/utils.js";

/** Default glyph per variant; a caller may name another `icon-…` class. */
export const NOTICE_ICONS = Object.freeze({
  info: "icon-info",
  warning: "icon-triangle-alert",
  error: "icon-octagon-alert",
  success: "icon-circle-check",
});

const ACTION_ID = /^[a-z0-9-]+$/;
const ICON_CLASS = /^icon-[a-z0-9-]+$/;

/**
 * Markup of a notice.
 * @param {object} [notice]
 * @param {"info"|"warning"|"error"|"success"} [notice.variant]
 * @param {string} [notice.icon] - a Lucide icon class (`icon-…`); the variant's glyph when omitted
 * @param {string} [notice.title]
 * @param {string} [notice.message] - plain text, escaped here
 * @param {string} [notice.messageHtml] - trusted catalog markup (`I18n.markup`), used in
 *   place of `message`
 * @param {string} [notice.extraHtml] - a trusted block rendered under the message (an
 *   address with its copy control)
 * @param {{id: string, label: string}} [notice.action] - a button with that `data-action`
 *   and label; the caller wires its click
 * @returns {string}
 */
export function renderNotice({
  variant = "info",
  icon = "",
  title = "",
  message = "",
  messageHtml = "",
  extraHtml = "",
  action = null,
} = {}) {
  const safeVariant = Object.hasOwn(NOTICE_ICONS, variant) ? variant : "info";
  const safeIcon = ICON_CLASS.test(icon) ? icon : NOTICE_ICONS[safeVariant];
  const role = safeVariant === "error" ? "alert" : "note";
  const body = messageHtml || (message ? escapeHtml(message) : "");
  const glyph = `<i class="notice-icon ${safeIcon}" aria-hidden="true"></i>`;

  const heading = title
    ? `<div class="notice-heading">${glyph}<span class="notice-title">${escapeHtml(title)}</span></div>${
        body ? `<div class="notice-message">${body}</div>` : ""
      }`
    : `<div class="notice-heading">${glyph}<span class="notice-message">${body}</span></div>`;
  const extra = extraHtml ? `<div class="notice-extra">${extraHtml}</div>` : "";
  const button =
    action && ACTION_ID.test(action.id)
      ? `<div class="notice-actions"><button type="button" class="btn btn-outline btn-sm" data-action="${action.id}">${escapeHtml(action.label)}</button></div>`
      : "";

  return `<div class="notice" data-variant="${safeVariant}" role="${role}">${heading}${extra}${button}</div>`;
}
