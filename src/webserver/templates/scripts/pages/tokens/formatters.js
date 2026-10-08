// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Formatting and display utility functions for tokens page
 * Lines extracted from tokens.js (118-392)
 */

import * as Utils from "../../core/utils.js";
import { boostTierForMint, boostCountForMint, formatBoostCount } from "../../core/boosts.js";
import * as AppState from "../../core/app_state.js";
import * as Hints from "../../core/hints.js";
import { HintTrigger } from "../../ui/hint_popover.js";
import { tokenLogoPlaceholder } from "../../ui/token_identity.js";
import { SORT_KEY_TO_COLUMN, TOKEN_VIEWS } from "./constants.js";

export function findFirstDifferenceIndex(a, b) {
  if (typeof a !== "string" || typeof b !== "string") {
    return -1;
  }

  const minLength = Math.min(a.length, b.length);
  for (let i = 0; i < minLength; i += 1) {
    if (a[i] !== b[i]) {
      return i;
    }
  }

  if (a.length !== b.length) {
    return minLength;
  }

  return -1;
}

export function priceCell(value, row = null) {
  // A price at or below zero is an unquoted token (the API reports 0.0), not a real
  // price: it takes the same placeholder as any other unknown value.
  const price = value === null || value === undefined || value === "" ? Number.NaN : Number(value);
  const known = Number.isFinite(price) && price > 0;
  const formatted = known ? Utils.formatPriceSubscript(price, { fallback: "—" }) : "—";
  const exact = known ? Utils.formatFixed(price, { decimals: 12, trim: true }) : "";
  const baseValue = Utils.escapeHtml(formatted);

  let directionClass = "price-change--neutral";
  let arrowSymbol = "▲";
  let arrowClass = "price-change-arrow price-change-arrow--placeholder";
  let valueHtml = baseValue;

  if (row && row.price_change_meta) {
    const { direction, currentFormatted, changeStartIndex } = row.price_change_meta;
    if (
      direction &&
      typeof changeStartIndex === "number" &&
      currentFormatted &&
      formatted !== "—"
    ) {
      const boundedIndex = Math.max(0, Math.min(changeStartIndex, currentFormatted.length));
      const leadingPart = Utils.escapeHtml(currentFormatted.slice(0, boundedIndex));
      const changedPart = Utils.escapeHtml(currentFormatted.slice(boundedIndex));
      valueHtml = `${leadingPart}<span class="price-change-diff">${changedPart}</span>`;
      directionClass = direction === "up" ? "price-change--up" : "price-change--down";
      arrowSymbol = direction === "up" ? "▲" : "▼";
      arrowClass = "price-change-arrow";
    }
  }

  const titleAttr = exact ? ` title="${Utils.escapeHtml(exact)}"` : "";
  return `<span class="price-change ${directionClass}"${titleAttr}><span class="${arrowClass}" aria-hidden="true">${arrowSymbol}</span><span class="price-change-value">${valueHtml}</span></span>`;
}

export function usdCell(value) {
  return Utils.formatCurrencyUSD(value, { fallback: "—" });
}

export function percentCell(value) {
  if (value === null || value === undefined) return "—";
  const num = Number(value);
  if (!Number.isFinite(num)) return "—";
  const cls = num > 0 ? "value-positive" : num < 0 ? "value-negative" : "";
  const text = Utils.formatPercentValue(num, { includeSign: true, decimals: 2 });
  return `<span class="${cls}">${text}</span>`;
}

export function timeAgoCell(seconds) {
  return Utils.formatTimeAgo(seconds, { fallback: "—" });
}

// Catalog text per rejection code, as sent by the tokens API (`rejection_texts`).
// Merged across responses so rows from earlier pages keep their label.
const rejectionTexts = new Map();

export function setRejectionTexts(texts) {
  if (!texts || typeof texts !== "object") return;
  for (const [code, uiText] of Object.entries(texts)) rejectionTexts.set(code, uiText);
}

export function getRejectionDisplayLabel(reasonCode) {
  if (!reasonCode) return null;
  const uiText = rejectionTexts.get(reasonCode);
  return uiText ? I18n.text(uiText) : reasonCode;
}

export function tokenCell(row) {
  const src = row.logo_url || row.image_url;
  const logo = src
    ? `<img class="token-logo token-logo-artwork clickable-logo" alt="" src="${Utils.escapeHtml(src)}" data-logo-url="${Utils.escapeHtml(src)}" data-token-symbol="${Utils.escapeHtml(row.symbol || "")}" data-token-name="${Utils.escapeHtml(row.name || "")}" data-token-mint="${Utils.escapeHtml(row.mint || "")}" title="${Utils.escapeHtml(I18n.t("tokens-cell-logo-enlarge"))}" />`
    : tokenLogoPlaceholder(row.symbol, "token-logo");
  const sym = Utils.escapeHtml(row.symbol || "—");
  const name = row.name ? `<div class="token-name">${Utils.escapeHtml(row.name)}</div>` : "";
  // A boosted token's owner paid for visibility, so the mark rides beside the
  // ticker where the eye already is. Bare gold glyph plus the active count -- it
  // adds information (how strongly), never a second badge saying the same thing.
  const tier = boostTierForMint(row.mint);
  const boostCount = formatBoostCount(boostCountForMint(row.mint));
  const boostMark = tier
    ? `<span class="boost-mark${tier === "golden" ? " golden" : ""}" title="${Utils.escapeHtml(I18n.t("tokens-boost-title", { boosts: boostCount }))}"><i class="icon-zap" aria-hidden="true"></i><span class="boost-mark-count">${boostCount}</span></span>`
    : "";
  const mint = Utils.escapeHtml(row.mint || "");
  const disabledAttr = row.blacklisted ? ' disabled aria-disabled="true"' : "";
  const actionAttrs = (title, label) =>
    `title="${Utils.escapeHtml(title)}" aria-label="${Utils.escapeHtml(label)}"`;
  const tradeActions = row.has_open_position
    ? `
      <button class="btn row-action" data-action="add" data-mint="${mint}" ${actionAttrs(I18n.attr("tokens-cell-action-add", "title"), I18n.attr("tokens-cell-action-add", "aria-label"))}${disabledAttr}><i class="icon-circle-plus"></i></button>
      <button class="btn row-action" data-action="sell" data-mint="${mint}" ${actionAttrs(I18n.attr("tokens-cell-action-sell", "title"), I18n.attr("tokens-cell-action-sell", "aria-label"))}${disabledAttr}><i class="icon-trending-down"></i></button>`
    : `<button class="btn row-action" data-action="buy" data-mint="${mint}" ${actionAttrs(I18n.attr("tokens-cell-action-buy", "title"), I18n.attr("tokens-cell-action-buy", "aria-label"))}${disabledAttr}><i class="icon-shopping-cart"></i></button>`;
  const actionCount = row.has_open_position ? 3 : 2;

  return `<div class="token-cell token-cell--actions-${actionCount}">
    <div class="token-cell__identity">
      ${logo}
      <div class="token-cell__meta"><div class="token-cell__symbol-line"><div class="token-symbol">${sym}</div>${boostMark}</div>${name}</div>
    </div>
    <div class="row-actions token-cell__actions">
      ${tradeActions}
      <button class="btn links-dropdown-trigger" data-mint="${mint}" ${actionAttrs(I18n.attr("tokens-cell-external-links", "title"), I18n.attr("tokens-cell-external-links", "aria-label"))} type="button"><i class="icon-external-link"></i></button>
    </div>
  </div>`;
}

export function normalizeBlacklistReasons(mint, sourcesMap) {
  if (!mint || typeof mint !== "string") return [];
  if (!sourcesMap || typeof sourcesMap !== "object") return [];
  const raw = sourcesMap[mint];
  if (!Array.isArray(raw) || raw.length === 0) return [];
  return raw
    .filter((entry) => entry && typeof entry === "object")
    .map((entry) => {
      const category =
        typeof entry.category === "string" && entry.category.trim().length > 0
          ? entry.category.trim()
          : "unknown";
      const reason =
        typeof entry.reason === "string" && entry.reason.trim().length > 0
          ? entry.reason.trim()
          : "unknown_reason";
      const detail =
        typeof entry.detail === "string" && entry.detail.trim().length > 0
          ? entry.detail.trim()
          : null;
      return { category, reason, detail };
    });
}

export function summarizeBlacklistReasons(sourceList, separator = ", ") {
  if (!Array.isArray(sourceList) || sourceList.length === 0) return "";
  return sourceList
    .map((source) => {
      if (!source || typeof source !== "object") return "unknown";
      const category = source.category || "unknown";
      const reason = source.reason || "unknown_reason";
      const detail = source.detail ? ` (${source.detail})` : "";
      return `${category}:${reason}${detail}`;
    })
    .join(separator);
}

export function resolveSortColumn(sortKey) {
  if (!sortKey) {
    return null;
  }
  // Dynamic keys mapping back to 'updated_at'
  if (
    [
      "pool_price_last_calculated_at",
      "metadata_last_fetched_at",
      "market_data_last_fetched_at",
    ].includes(sortKey)
  ) {
    return "updated_at";
  }
  return SORT_KEY_TO_COLUMN[sortKey] ?? null;
}

export function normalizeSortDirection(direction) {
  return direction === "desc" ? "desc" : "asc";
}

export function loadPersistedSort(stateKey) {
  if (!stateKey) return null;
  const saved = AppState.load(stateKey);
  if (saved && typeof saved === "object" && saved.sortColumn) {
    return {
      column: saved.sortColumn,
      direction: normalizeSortDirection(saved.sortDirection),
    };
  }
  return null;
}

/**
 * Attach hint triggers to tab buttons
 * Must be called after tab bar is rendered
 */
export async function attachHintsToTabs() {
  // Initialize hints system (loads settings and dismissed state)
  await Hints.init();

  if (!Hints.isEnabled()) return;

  const container = document.querySelector("#subTabsContainer");
  if (!container) return;

  // Find all tab buttons and attach hints
  TOKEN_VIEWS.forEach((view) => {
    if (!view.hintKey) return;

    const hint = Hints.getHint(view.hintKey);
    if (!hint || Hints.isDismissed(hint.id)) return;

    const tabButton = container.querySelector(`[data-tab-id="${view.id}"]`);
    if (!tabButton) return;

    // Check if hint already attached
    if (tabButton.querySelector(".hint-trigger")) return;

    // Append hint trigger to tab button (pass hintKey as path)
    const triggerHtml = HintTrigger.render(hint, view.hintKey, { size: "sm" });
    if (triggerHtml) {
      tabButton.insertAdjacentHTML("beforeend", triggerHtml);
    }
  });

  // Initialize hint trigger handlers
  HintTrigger.initAll();
}
