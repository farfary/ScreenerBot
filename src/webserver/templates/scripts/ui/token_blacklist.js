// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Token blacklist, as the dashboard sees it: reads a token's entry, names who filed it,
 * and removes it at the user's request. Every surface that offers a removal (the Tokens
 * rows, the right-click menu, the token details badge, the trade dialog) goes through
 * here, so the confirmation, the request and the change event have one owner.
 *
 * A token the user removes stays removed: no automatic rule files it again. Only a manual
 * blacklist puts it back, which is why the confirmation says so.
 */

import { requestManager, errorMessageOf } from "../core/request_manager.js";
import { escapeHtml, showToast } from "../core/utils.js";
import { formatTimeAgo } from "../core/format.js";
import { ConfirmationDialog } from "./confirmation_dialog.js";

/** Fired on `window` after a token is blacklisted or removed; `detail` is `{ mint, blacklisted }`. */
export const BLACKLIST_CHANGED_EVENT = "screenerbot:blacklist-changed";

// Who filed an entry, keyed by the source code the server stores
// (`BlacklistSource::code`, src/tokens/database/blacklist.rs).
export const BLACKLIST_SOURCE_LABELS = Object.freeze({
  manual: "tokens-blacklist-source-manual",
  auto_cleanup: "tokens-blacklist-source-auto-cleanup",
  auto_loss: "tokens-blacklist-source-auto-loss",
  auto_llm_analysis: "tokens-blacklist-source-auto-llm-analysis",
  auto_swap_route: "tokens-blacklist-source-auto-swap-route",
});

// The right-click menu and the trade dialog wait for the entry before they render, so a slow
// read gives up and shows them without it (the trade API still refuses a blacklisted buy).
const ENTRY_READ_TIMEOUT_MS = 3000;

function endpoint(mint) {
  return `/api/tokens/${encodeURIComponent(mint)}/blacklist`;
}

function announce(mint, blacklisted) {
  window.dispatchEvent(new CustomEvent(BLACKLIST_CHANGED_EVENT, { detail: { mint, blacklisted } }));
}

/**
 * The token's blacklist entry, or null when it has none or the read failed (an unread
 * entry never blocks a menu or a trade dialog).
 * @param {string} mint
 * @returns {Promise<{reason: string, source: string, addedAt: number|null}|null>}
 */
export async function fetchBlacklistEntry(mint) {
  if (!mint) return null;
  try {
    const data = await requestManager.fetch(endpoint(mint), {
      priority: "high",
      timeout: ENTRY_READ_TIMEOUT_MS,
    });
    if (!data?.is_blacklisted) return null;
    return {
      reason: data.reason ?? "",
      source: data.source ?? "",
      addedAt: Number.isFinite(data.added_at) ? data.added_at : null,
    };
  } catch {
    return null;
  }
}

/** The facts a removal confirms: why the token is listed, who listed it and when. */
function renderFacts(entry, fallbackReason) {
  const facts = [];
  const reason = entry?.reason || fallbackReason;
  if (reason) facts.push([I18n.t("tokens-blacklist-fact-reason"), reason]);
  if (entry?.source) {
    facts.push([
      I18n.t("tokens-blacklist-fact-source"),
      Object.hasOwn(BLACKLIST_SOURCE_LABELS, entry.source)
        ? I18n.label(BLACKLIST_SOURCE_LABELS, entry.source)
        : entry.source,
    ]);
  }
  if (entry?.addedAt) {
    facts.push([I18n.t("tokens-blacklist-fact-added"), formatTimeAgo(entry.addedAt)]);
  }
  if (facts.length === 0) return "";
  const rows = facts
    .map(([label, value]) => `<dt>${escapeHtml(label)}</dt><dd>${escapeHtml(value)}</dd>`)
    .join("");
  return `<dl class="confirmation-dialog__facts">${rows}</dl>`;
}

/**
 * Remove a token from the blacklist, with no confirmation. The server deletes the entry,
 * remembers the removal and clears the token's pool blacklist rows.
 * @param {string} mint
 * @param {string} [symbol] - named in the toast
 * @returns {Promise<boolean>} true when the token left the blacklist
 */
export async function removeFromBlacklist(mint, symbol = "") {
  try {
    await requestManager.fetch(endpoint(mint), { method: "DELETE", priority: "high" });
  } catch (error) {
    showToast({
      type: "error",
      title: I18n.t("tokens-blacklist-remove-failed"),
      message: await errorMessageOf(error, error?.message || null),
    });
    return false;
  }
  showToast({
    type: "success",
    title: I18n.t("tokens-blacklist-removed", { symbol: symbol || mint }),
  });
  announce(mint, false);
  return true;
}

/**
 * Ask the user to confirm the removal of a token from the blacklist, then remove it.
 * The dialog shows the entry's reason, who filed it and when, and says the bot will not
 * file the token again on its own.
 * @param {Object} options
 * @param {string} options.mint
 * @param {string} [options.symbol]
 * @param {{reason: string, source: string, addedAt: number|null}|null} [options.entry] -
 *   the entry when the caller already read it; read here otherwise
 * @param {string} [options.fallbackReason] - shown when the token has no entry of its own
 *   (it is listed through its pools)
 * @returns {Promise<boolean>} true when the token was removed
 */
export async function confirmRemoveFromBlacklist({
  mint,
  symbol = "",
  entry,
  fallbackReason = "",
}) {
  const resolved = entry === undefined ? await fetchBlacklistEntry(mint) : entry;
  const { confirmed } = await ConfirmationDialog.show({
    title: I18n.t("tokens-blacklist-remove-title"),
    message: I18n.t("tokens-blacklist-remove-message", { symbol: symbol || mint }),
    body: renderFacts(resolved, fallbackReason),
    confirmLabel: I18n.t("common-action-remove"),
    cancelLabel: I18n.t("common-action-cancel"),
    variant: "warning",
  });
  if (!confirmed) return false;
  return removeFromBlacklist(mint, symbol);
}

/**
 * Blacklist a token as a manual decision. The request carries no reason, so the server
 * records its default one.
 * @param {string} mint
 * @param {string} [symbol] - named in the toast
 * @returns {Promise<boolean>} true when the token is on the blacklist
 */
export async function addToBlacklist(mint, symbol = "") {
  try {
    await requestManager.fetch(endpoint(mint), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
      priority: "high",
    });
  } catch (error) {
    showToast({
      type: "error",
      title: I18n.t("menu-blacklist-failed"),
      message: await errorMessageOf(error, error?.message || null),
    });
    return false;
  }
  showToast({ type: "success", title: I18n.t("menu-blacklist-done", { symbol: symbol || mint }) });
  announce(mint, true);
  return true;
}
