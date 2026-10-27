// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The one stat card: a caption, a figure and a detail line stacked from the top edge, with
 * no glyph. The figure is coloured only by the sign of a signed amount (`signedTone`); a
 * ratio, a count or a duration stays neutral. Static page markup that cannot call this
 * spells the same shape (`.metric-card` in components.css).
 */

/** Value class of each `signedTone` result; "neutral" and anything else carry none. */
const SIGN_CLASSES = Object.freeze({ positive: "positive", negative: "negative" });

/**
 * Markup of a stat card.
 * @param {object} card
 * @param {string} card.label - the caption, escaped here
 * @param {string} card.value - the formatted figure, escaped here ("—" when absent)
 * @param {string} [card.detail] - plain text under the figure, escaped here
 * @param {string} [card.detailHtml] - trusted catalog markup (`I18n.markup`), used in place
 *   of `detail`
 * @param {"positive"|"negative"|"neutral"|null} [card.tone] - `Utils.signedTone(amount, decimals)`
 *   of a signed amount, the only source of the figure's colour
 * @param {boolean} [card.fit] - keep the figure on one line, scaled to the card width
 * @param {string} [card.extraHtml] - a trusted block after the detail (a share bar)
 * @param {(text: string) => string} escapeHtml
 * @returns {string}
 */
export function metricCard(
  { label, value, detail = "", detailHtml = "", tone = null, fit = false, extraHtml = "" },
  escapeHtml
) {
  const text = String(value ?? "—");
  const classes = ["metric-value"];
  if (fit) classes.push("metric-value-fit");
  if (Object.hasOwn(SIGN_CLASSES, tone)) classes.push(SIGN_CLASSES[tone]);
  const chars = fit ? ` style="--metric-value-chars:${[...text].length}"` : "";
  const body = detailHtml || escapeHtml(detail);
  return `<div class="metric-card"><div class="metric-label">${escapeHtml(label)}</div><div class="${classes.join(" ")}"${chars}>${escapeHtml(text)}</div><div class="metric-detail">${body}</div>${extraHtml}</div>`;
}
