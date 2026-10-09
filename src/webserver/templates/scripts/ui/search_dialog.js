// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Global Search Dialog — find any token from anywhere in the dashboard.
 *
 * Opened by Cmd/Ctrl+K, `/` outside a field, or the header search button.
 *
 * Before anything is typed the dialog is a starting point: the user's recent
 * searches, then Trending, Positions, Favorites and Boosted lists. Typed results
 * and those lists render through ONE two-line token row and ONE arrow-key list:
 * identity, age, price and 24h change on the first line; the full mint and
 * market cap, liquidity and 24h volume on the second. A mint is never cropped:
 * several tokens share a symbol, and a cropped address cannot be checked
 * against the one the user was given.
 *
 * Each row carries a favorite star and a "more" button that opens the shared
 * token context menu (right-click on a row opens the same menu). Dialogs opened
 * from that menu stack ABOVE the search, which stays open beneath them.
 */

import { $, create } from "../core/dom.js";
import * as AppState from "../core/app_state.js";
import { hasOpenOverlay, pushEscapeHandler } from "../core/escape_stack.js";
import {
  formatCompactFixed,
  formatCurrencyUSD,
  formatPercentValue,
  formatTimeAgo,
  signedTone,
  withUsdSymbol,
} from "../core/format.js";
import { escapeHtml, resolveTokenLogoUrl, showToast } from "../core/utils.js";
import { apiErrorMessage } from "../core/request_manager.js";
import {
  renderAddress,
  renderTokenLogo,
  resolvedTokenName,
  resolvedTokenSymbol,
} from "./token_identity.js";
import "./token_details_dialog.js";

const SEARCH_DEBOUNCE_MS = 250;
const MIN_QUERY_LENGTH = 2;
const SEARCH_LIMIT = 30;
const LIST_SIZE = 30;
const RECENT_KEY = "search.recent";
const RECENT_LIMIT = 8;

/**
 * The lists offered before typing. Trending is a ranking, not a set, so it shows
 * no count; the other three are the user's own sets and show their size.
 */
const LISTS = [
  { id: "trending", counted: false },
  { id: "positions", counted: true },
  { id: "favorites", counted: true },
  { id: "boosted", counted: true },
];

const LIST_LABELS = Object.freeze({
  trending: "tokens-search-tab-trending",
  positions: "tokens-view-positions",
  favorites: "tokens-view-favorites",
  boosted: "tokens-featured-category-boosted",
});

const LIST_EMPTY_LABELS = Object.freeze({
  trending: "tokens-search-empty-trending",
  positions: "tokens-search-empty-positions",
  favorites: "tokens-search-empty-favorites",
  boosted: "tokens-search-empty-boosted",
});

let dialogEl = null;
let releaseEscape = null;
let debounceTimer = null;
/** Incremented per query; a response for an older query is dropped. */
let searchSeq = 0;

const state = {
  open: false,
  query: "",
  tab: "trending",
  active: 0,
  /** Rows of the typed query; null until its first response. */
  results: null,
  searching: false,
  searchError: null,
  /** list id -> { rows, loading, failed } */
  lists: {},
  /** Mints the user has starred. */
  favorites: new Set(),
};

// =============================================================================
// SETUP STATE CHECK
// =============================================================================

/** Search must not open over the splash, setup or onboarding screens. */
function isSetupActive() {
  return ["splash-screen", "setup-screen", "onboarding-screen"].some((id) => {
    const el = document.getElementById(id);
    return el && el.style.display !== "none" && !el.classList.contains("hidden");
  });
}

// =============================================================================
// ROW SHAPE
// =============================================================================

/** A positive finite number, or null: providers send 0 for an unknown price. */
function known(value) {
  const num = Number(value);
  return Number.isFinite(num) && num > 0 ? num : null;
}

/** A signed finite number, or null. */
function signed(value) {
  if (value === null || value === undefined) return null;
  const num = Number(value);
  return Number.isFinite(num) ? num : null;
}

/**
 * One row from any source: a search hit, a tracked token, a favorite or a
 * boosted card. `volume_24h` is `undefined` for a source that never carries
 * volume, so its column stays empty rather than claiming an unknown per row.
 */
function toRow(source, { createdAt = null, volume } = {}) {
  return {
    mint: source.mint,
    symbol: resolvedTokenSymbol(source.symbol),
    name: resolvedTokenName(source.name),
    logo_url: resolveTokenLogoUrl(source),
    price_usd: known(source.price_usd),
    price_change_h24: signed(source.price_change_h24),
    market_cap: known(source.market_cap),
    fdv: known(source.fdv),
    liquidity_usd: known(source.liquidity_usd),
    volume_24h: volume,
    created_at: createdAt,
  };
}

const fromSearchHit = (hit) =>
  toRow(hit, { createdAt: hit.pair_created_at ?? null, volume: known(hit.volume_24h) });

const fromToken = (token) =>
  toRow(token, { createdAt: token.blockchain_created_at ?? null, volume: known(token.volume_24h) });

const fromBoostCard = (card) => toRow(card);

// =============================================================================
// DATA
// =============================================================================

async function fetchJson(url) {
  const response = await fetch(url);
  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(apiErrorMessage(data, I18n.t("tokens-search-failed")));
  }
  return data;
}

const LIST_LOADERS = {
  trending: async () => {
    const data = await fetchJson(
      `/api/tokens/list?view=pool&sort_by=volume_24h&sort_dir=desc&page_size=${LIST_SIZE}`
    );
    return (data.items || []).map(fromToken);
  },
  positions: async () => {
    const data = await fetchJson(
      `/api/tokens/list?view=positions&sort_by=volume_24h&sort_dir=desc&page_size=${LIST_SIZE}`
    );
    return (data.items || []).map(fromToken);
  },
  favorites: async () => {
    const data = await fetchJson("/api/tokens/favorites");
    const rows = (data.favorites || []).map(fromToken);
    state.favorites = new Set(rows.map((row) => row.mint));
    return rows;
  },
  boosted: async () => {
    const data = await fetchJson("/api/featured");
    return (data.tokens || []).map(fromBoostCard);
  },
};

/** Load every list at once; each repaints only itself when it lands. */
function loadLists() {
  LISTS.forEach(({ id }) => {
    const previous = state.lists[id];
    state.lists[id] = { rows: previous?.rows || [], loading: true, failed: false };
    LIST_LOADERS[id]()
      .then((rows) => {
        state.lists[id] = { rows, loading: false, failed: false };
      })
      .catch(() => {
        state.lists[id] = { rows: previous?.rows || [], loading: false, failed: true };
      })
      .finally(() => {
        if (!state.open) return;
        renderTabs();
        if (id === state.tab || id === "favorites") renderBody();
      });
  });
}

async function runSearch(query) {
  const seq = ++searchSeq;
  state.searching = true;
  state.searchError = null;
  renderBody();

  try {
    const data = await fetchJson(
      `/api/tokens/search?q=${encodeURIComponent(query)}&limit=${SEARCH_LIMIT}`
    );
    if (seq !== searchSeq) return;
    state.results = (data.results || []).map(fromSearchHit);
  } catch (error) {
    if (seq !== searchSeq) return;
    state.results = [];
    state.searchError = error.message;
  }
  state.searching = false;
  state.active = 0;
  renderBody();
}

// =============================================================================
// RECENT SEARCHES
// =============================================================================

function recentEntries() {
  const entries = AppState.load(RECENT_KEY, []);
  return Array.isArray(entries) ? entries.filter((entry) => entry?.mint) : [];
}

function rememberToken(row) {
  const entry = { mint: row.mint, symbol: row.symbol, name: row.name, logo_url: row.logo_url };
  const rest = recentEntries().filter((existing) => existing.mint !== row.mint);
  AppState.save(RECENT_KEY, [entry, ...rest].slice(0, RECENT_LIMIT));
}

function clearRecent() {
  AppState.save(RECENT_KEY, []);
  renderRecent();
}

// =============================================================================
// RENDERING
// =============================================================================

function discovering() {
  return state.query.trim().length < MIN_QUERY_LENGTH;
}

/** The rows the listbox holds right now. */
function currentRows() {
  if (discovering()) return state.lists[state.tab]?.rows || [];
  return state.results || [];
}

function createDialog() {
  if (dialogEl) return dialogEl;

  dialogEl = create("div", { class: "search-dialog-overlay", id: "search-dialog" });
  // l10n-ignore: keyboard key labels, identical in every locale
  dialogEl.innerHTML = `
    <div class="search-dialog" role="dialog" aria-modal="true" aria-label="${escapeHtml(I18n.t("tokens-search-input-label"))}" data-context="tokens">
      <div class="search-field">
        <i class="search-field-icon icon-search" aria-hidden="true"></i>
        <input
          type="text"
          id="search-input"
          class="search-input"
          placeholder="${escapeHtml(I18n.t("tokens-search-placeholder-dialog"))}"
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          role="combobox"
          aria-autocomplete="list"
          aria-controls="search-results"
          aria-expanded="true"
          aria-label="${escapeHtml(I18n.t("tokens-search-input-label"))}"
        >
        <button type="button" class="search-field-clear" hidden aria-label="${escapeHtml(I18n.attr("tokens-search-clear", "aria-label"))}" title="${escapeHtml(I18n.attr("tokens-search-clear", "title"))}">
          <i class="icon-x" aria-hidden="true"></i>
        </button>
        <kbd class="search-field-esc">esc</kbd>
      </div>
      <div class="search-progress" hidden aria-hidden="true"></div>
      <div class="search-recent" hidden></div>
      <div class="search-tabs" role="tablist" aria-label="${escapeHtml(I18n.t("tokens-search-lists-label"))}"></div>
      <div class="search-summary" aria-live="polite" hidden></div>
      <div id="search-results" class="search-results" role="listbox" aria-label="${escapeHtml(I18n.t("tokens-search-results-label"))}"></div>
      <div class="search-dialog-footer">
        <span><kbd>↑</kbd><kbd>↓</kbd> ${escapeHtml(I18n.t("tokens-search-tip-nav"))}</span>
        <span><kbd>↵</kbd> ${escapeHtml(I18n.t("tokens-search-tip-open"))}</span>
        <span><kbd>esc</kbd> ${escapeHtml(I18n.t("tokens-search-tip-close"))}</span>
      </div>
    </div>
  `;

  document.body.appendChild(dialogEl);

  dialogEl.addEventListener("mousedown", (event) => {
    if (event.target === dialogEl) closeDialog();
  });

  const input = $("#search-input", dialogEl);
  input.addEventListener("input", handleInput);
  input.addEventListener("keydown", handleInputKeydown);

  $(".search-field-clear", dialogEl).addEventListener("click", () => {
    input.value = "";
    handleInput();
    input.focus();
  });

  $(".search-tabs", dialogEl).addEventListener("click", (event) => {
    const tab = event.target.closest("[data-tab]");
    if (!tab) return;
    state.tab = tab.dataset.tab;
    state.active = 0;
    renderTabs();
    renderBody();
    input.focus();
  });

  $(".search-recent", dialogEl).addEventListener("click", (event) => {
    if (event.target.closest(".search-recent-clear")) {
      clearRecent();
      input.focus();
      return;
    }
    const chip = event.target.closest("[data-recent-mint]");
    if (!chip) return;
    const entry = recentEntries().find((item) => item.mint === chip.dataset.recentMint);
    if (entry) openToken(entry);
  });

  const results = $("#search-results", dialogEl);
  results.addEventListener("click", handleResultsClick);
  results.addEventListener("mousemove", (event) => {
    const row = event.target.closest(".search-row");
    if (!row) return;
    const index = Number(row.dataset.index);
    if (index !== state.active) {
      state.active = index;
      updateActive({ scroll: false });
    }
  });

  return dialogEl;
}

function renderRecent() {
  const el = $(".search-recent", dialogEl);
  const entries = recentEntries();
  if (!discovering() || entries.length === 0) {
    el.hidden = true;
    el.innerHTML = "";
    return;
  }
  el.hidden = false;
  el.innerHTML = `
    <span class="search-section-label">${escapeHtml(I18n.t("tokens-search-recent"))}</span>
    <ul class="search-recent-list" aria-label="${escapeHtml(I18n.t("tokens-search-recent-label"))}">
      ${entries
        .map(
          (entry) => `
        <li>
          <button type="button" class="search-chip" data-recent-mint="${escapeHtml(entry.mint)}" title="${escapeHtml(entry.mint)}">
            ${renderTokenLogo({ mint: entry.mint, symbol: entry.symbol, logoUrl: entry.logo_url }, { size: "xs" })}
            <span class="token-symbol-type">${escapeHtml(entry.symbol || I18n.t("format-unknown"))}</span>
          </button>
        </li>`
        )
        .join("")}
    </ul>
    <button type="button" class="search-recent-clear">${escapeHtml(I18n.t("common-action-clear"))}</button>
  `;
}

function renderTabs() {
  const el = $(".search-tabs", dialogEl);
  if (!discovering()) {
    el.hidden = true;
    return;
  }
  el.hidden = false;
  el.innerHTML = `
    ${LISTS.map(({ id, counted }) => {
      const list = state.lists[id];
      const count = counted && list && !list.loading ? list.rows.length : 0;
      return `
        <button type="button" role="tab" class="search-tab" id="search-tab-${id}" data-tab="${id}"
          aria-selected="${id === state.tab}" aria-controls="search-results">
          ${escapeHtml(I18n.label(LIST_LABELS, id))}
          ${count > 0 ? `<span class="search-tab-count">${escapeHtml(formatCompactFixed(count, { decimals: 0 }))}</span>` : ""}
        </button>`;
    }).join("")}
    <span class="search-tabs-hint">${escapeHtml(I18n.t("tokens-search-kinds"))}</span>
  `;
}

function renderSummary() {
  const el = $(".search-summary", dialogEl);
  if (discovering()) {
    el.hidden = true;
    return;
  }
  el.hidden = false;
  const status =
    state.searchError !== null
      ? I18n.t("tokens-search-failed")
      : state.searching || state.results === null
        ? I18n.t("tokens-search-searching")
        : I18n.t("tokens-search-result-count", { count: state.results.length });
  el.innerHTML = `
    <span>${escapeHtml(status)}</span>
    <span class="search-summary-order">${escapeHtml(I18n.t("tokens-search-order"))}</span>
  `;
}

/** One market figure: its short label, the label's full name and the value. */
function figureHtml(label, title, value) {
  return `
    <span class="search-figure">
      <abbr title="${escapeHtml(title)}">${escapeHtml(label)}</abbr>
      <b>${escapeHtml(value)}</b>
    </span>`;
}

/** Market cap, or the fully diluted valuation when a source has no market cap. */
function capFigureHtml(row) {
  if (row.market_cap === null && row.fdv !== null) {
    return figureHtml(
      I18n.t("tokens-search-metric-fdv"),
      I18n.attr("tokens-search-metric-fdv", "title"),
      compactUsd(row.fdv)
    );
  }
  return figureHtml(
    I18n.t("tokens-search-metric-mc"),
    I18n.attr("tokens-search-metric-mc", "title"),
    compactUsd(row.market_cap)
  );
}

function compactUsd(value) {
  return value === null ? "—" : withUsdSymbol(formatCompactFixed(value));
}

/** The `data-tone` of a 24h change, from its tone as shown by `formatPercentValue`. */
const CHANGE_TONES = { positive: "up", negative: "down", neutral: "flat" };

function rowHtml(row, index) {
  const change = row.price_change_h24;
  const tone = CHANGE_TONES[signedTone(change, 2)];
  const starred = state.favorites.has(row.mint);
  const starLabel = starred ? I18n.t("menu-favorite-remove") : I18n.t("menu-favorite-add");
  const age = row.created_at ? formatTimeAgo(row.created_at, { fallback: "" }) : "";

  return `
    <div class="search-row" role="option" id="search-option-${index}" data-index="${index}"
      data-row-id="${escapeHtml(row.mint)}" aria-selected="${index === state.active}">
      <span class="search-row-logo" data-field="logo">
        ${renderTokenLogo({ mint: row.mint, symbol: row.symbol, logoUrl: row.logo_url }, { size: "md" })}
      </span>
      <span class="search-row-top">
        <span class="search-row-name">
          <span class="search-row-symbol token-symbol-type" data-field="symbol">${escapeHtml(row.symbol || I18n.t("format-unknown"))}</span>
          ${row.name ? `<span class="search-row-fullname token-name-type" data-field="name">${escapeHtml(row.name)}</span>` : ""}
          ${age ? `<span class="search-row-age">${escapeHtml(age)}</span>` : ""}
        </span>
        <span class="search-row-price">${escapeHtml(formatCurrencyUSD(row.price_usd))}</span>
        <span class="search-row-change" data-tone="${tone}">${escapeHtml(formatPercentValue(change))}</span>
      </span>
      <span class="search-row-bottom">
        ${renderAddress(row.mint, { plain: true })}
        <span class="search-row-figures">
          ${capFigureHtml(row)}
          ${figureHtml(
            I18n.t("tokens-search-metric-liq"),
            I18n.attr("tokens-search-metric-liq", "title"),
            compactUsd(row.liquidity_usd)
          )}
          ${
            row.volume_24h === undefined
              ? "<span></span>"
              : figureHtml(
                  I18n.t("tokens-search-metric-vol"),
                  I18n.attr("tokens-search-metric-vol", "title"),
                  compactUsd(row.volume_24h)
                )
          }
        </span>
      </span>
      <span class="search-row-actions">
        <button type="button" class="search-row-action search-row-star" data-action="star" tabindex="-1"
          aria-pressed="${starred}" title="${escapeHtml(starLabel)}" aria-label="${escapeHtml(starLabel)}">
          <i class="icon-star" aria-hidden="true"></i>
        </button>
        <button type="button" class="search-row-action" data-action="more" tabindex="-1"
          title="${escapeHtml(I18n.attr("tokens-search-more", "title"))}" aria-label="${escapeHtml(I18n.attr("tokens-search-more", "aria-label"))}">
          <i class="icon-ellipsis" aria-hidden="true"></i>
        </button>
      </span>
    </div>`;
}

function messageHtml(text, tone = "") {
  return `<p class="search-message${tone ? ` search-message-${tone}` : ""}">${escapeHtml(text)}</p>`;
}

function skeletonHtml() {
  return `<div class="search-skeleton" role="status" aria-label="${escapeHtml(I18n.t("tokens-search-searching"))}">
    ${Array.from({ length: 5 }, () => '<span class="search-skeleton-row"><span></span><span></span><span></span></span>').join("")}
  </div>`;
}

function renderBody() {
  if (!dialogEl) return;
  renderRecent();
  renderTabs();
  renderSummary();

  const resultsEl = $("#search-results", dialogEl);
  const input = $("#search-input", dialogEl);
  const rows = currentRows();
  state.active = Math.min(state.active, Math.max(rows.length - 1, 0));

  $(".search-progress", dialogEl).hidden = !(state.searching && !discovering());

  if (discovering()) {
    resultsEl.setAttribute("aria-labelledby", `search-tab-${state.tab}`);
  } else {
    resultsEl.removeAttribute("aria-labelledby");
  }

  if (rows.length > 0) {
    resultsEl.innerHTML = rows.map(rowHtml).join("");
    updateActive({ scroll: false });
    return;
  }

  input.removeAttribute("aria-activedescendant");
  if (discovering()) {
    const list = state.lists[state.tab];
    resultsEl.innerHTML =
      !list || list.loading
        ? skeletonHtml()
        : list.failed
          ? messageHtml(I18n.t("tokens-search-list-failed"), "error")
          : messageHtml(I18n.label(LIST_EMPTY_LABELS, state.tab));
    return;
  }
  resultsEl.innerHTML =
    state.searching || state.results === null
      ? skeletonHtml()
      : state.searchError !== null
        ? messageHtml(I18n.t("tokens-search-error", { message: state.searchError }), "error")
        : messageHtml(I18n.t("tokens-search-no-match", { query: state.query.trim() }));
}

/** Move the highlight without rebuilding the rows. */
function updateActive({ scroll = true } = {}) {
  const input = $("#search-input", dialogEl);
  const rows = dialogEl.querySelectorAll(".search-row");
  rows.forEach((el, index) => el.setAttribute("aria-selected", String(index === state.active)));
  const activeEl = rows[state.active];
  if (activeEl) {
    input.setAttribute("aria-activedescendant", activeEl.id);
    if (scroll) activeEl.scrollIntoView({ block: "nearest" });
  } else {
    input.removeAttribute("aria-activedescendant");
  }
}

// =============================================================================
// ACTIONS
// =============================================================================

/** Open the token details dialog for a row and remember it. */
function openToken(row) {
  if (!row?.mint) return;
  rememberToken(row);
  closeDialog();
  window.dispatchEvent(
    new CustomEvent("screenerbot:open-token-details", {
      detail: {
        mint: row.mint,
        symbol: row.symbol || "",
        name: row.name || "",
        logo_url: row.logo_url || null,
      },
    })
  );
}

async function toggleFavorite(row, button) {
  const starred = state.favorites.has(row.mint);
  const symbol = row.symbol || I18n.t("menu-token-fallback");
  button.disabled = true;
  try {
    const response = starred
      ? await fetch(`/api/tokens/favorites/${encodeURIComponent(row.mint)}`, { method: "DELETE" })
      : await fetch("/api/tokens/favorites", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            mint: row.mint,
            symbol: row.symbol,
            name: row.name,
            logo_url: row.logo_url,
          }),
        });
    if (!response.ok) {
      throw new Error(
        starred ? I18n.t("menu-favorite-remove-failed") : I18n.t("menu-favorite-add-failed")
      );
    }
    showToast(
      starred
        ? I18n.t("menu-favorite-removed", { symbol })
        : I18n.t("menu-favorite-added", { symbol }),
      "success"
    );
    window.dispatchEvent(
      new CustomEvent("screenerbot:favorites-changed", {
        detail: { mint: row.mint, isFavorite: !starred },
      })
    );
  } catch (error) {
    showToast(error.message || I18n.t("menu-favorite-update-failed"), "error");
  } finally {
    button.disabled = false;
  }
}

/** Keep the stars and the Favorites list in step with a toggle made anywhere. */
function handleFavoritesChanged(event) {
  const { mint, isFavorite } = event.detail || {};
  if (!mint) return;
  if (isFavorite) state.favorites.add(mint);
  else state.favorites.delete(mint);
  if (!state.open) return;
  LIST_LOADERS.favorites()
    .then((rows) => {
      state.lists.favorites = { rows, loading: false, failed: false };
    })
    .catch(() => {})
    .finally(() => {
      if (state.open) renderBody();
    });
  renderBody();
}

/** Open the shared token context menu at a row's "more" button. */
function openRowMenu(button) {
  const rect = button.getBoundingClientRect();
  button.dispatchEvent(
    new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      clientX: rect.left,
      clientY: rect.bottom + 4,
    })
  );
}

function handleResultsClick(event) {
  const row = event.target.closest(".search-row");
  if (!row) return;
  const data = currentRows()[Number(row.dataset.index)];
  if (!data) return;

  const action = event.target.closest("[data-action]");
  if (action?.dataset.action === "star") {
    event.stopPropagation();
    toggleFavorite(data, action);
    return;
  }
  if (action?.dataset.action === "more") {
    event.stopPropagation();
    openRowMenu(action);
    return;
  }
  openToken(data);
}

function handleInput() {
  const input = $("#search-input", dialogEl);
  state.query = input.value;
  state.active = 0;
  $(".search-field-clear", dialogEl).hidden = input.value === "";

  clearTimeout(debounceTimer);
  if (discovering()) {
    searchSeq++;
    state.results = null;
    state.searching = false;
    state.searchError = null;
    renderBody();
    return;
  }
  state.searching = true;
  renderBody();
  const query = state.query.trim();
  debounceTimer = setTimeout(() => runSearch(query), SEARCH_DEBOUNCE_MS);
}

/** A context menu opened from a row owns the keyboard until it closes. */
function contextMenuOpen() {
  return document.querySelector(".context-menu") !== null;
}

function handleInputKeydown(event) {
  if (contextMenuOpen()) return;
  const rows = currentRows();
  const count = rows.length;
  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      if (count > 0) {
        state.active = (state.active + 1) % count;
        updateActive();
      }
      break;
    case "ArrowUp":
      event.preventDefault();
      if (count > 0) {
        state.active = (state.active - 1 + count) % count;
        updateActive();
      }
      break;
    case "Enter":
      event.preventDefault();
      if (rows[state.active]) openToken(rows[state.active]);
      break;
    case "Tab":
      // Tab walks the lists before typing, so the keyboard reaches every tab.
      if (discovering()) {
        event.preventDefault();
        const order = LISTS.map(({ id }) => id);
        const step = event.shiftKey ? -1 : 1;
        state.tab = order[(order.indexOf(state.tab) + step + order.length) % order.length];
        state.active = 0;
        renderBody();
      }
      break;
  }
}

/** Cmd/Ctrl+K toggles; a bare `/` opens where it is not a character being typed. */
function handleGlobalKeydown(event) {
  const typing = event.target.matches?.('input, textarea, select, [contenteditable="true"]');
  const commandK = (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k";
  const slash = event.key === "/" && !typing && !event.metaKey && !event.ctrlKey && !event.altKey;
  if (!commandK && !slash) return;
  if (state.open) {
    if (commandK) {
      event.preventDefault();
      closeDialog();
    }
    return;
  }
  if (!canOpen()) return;
  event.preventDefault();
  openDialog();
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Search opens only over the page itself. It sits below the dialog layer, so
 * opening it under an open dialog would take the keyboard while staying hidden.
 */
function canOpen() {
  return !isSetupActive() && !hasOpenOverlay();
}

export function openDialog() {
  if (state.open || !canOpen()) return;

  createDialog();
  state.open = true;
  state.query = "";
  state.active = 0;
  state.results = null;
  state.searching = false;
  state.searchError = null;

  const input = $("#search-input", dialogEl);
  input.value = "";
  $(".search-field-clear", dialogEl).hidden = true;

  dialogEl.classList.add("visible");
  document.body.style.overflow = "hidden";
  releaseEscape = pushEscapeHandler(closeDialog);

  loadLists();
  renderBody();
  input.focus();
}

export function closeDialog() {
  if (!state.open) return;
  state.open = false;
  clearTimeout(debounceTimer);
  searchSeq++;
  dialogEl?.classList.remove("visible");
  document.body.style.overflow = "";
  releaseEscape?.();
  releaseEscape = null;
  document.getElementById("searchBtn")?.focus();
}

export function isDialogOpen() {
  return state.open;
}

/** Set up the global shortcuts (called once from the shell). */
export function initSearchDialog() {
  document.addEventListener("keydown", handleGlobalKeydown);
  window.addEventListener("screenerbot:favorites-changed", handleFavoritesChanged);
}

export function disposeSearchDialog() {
  closeDialog();
  document.removeEventListener("keydown", handleGlobalKeydown);
  window.removeEventListener("screenerbot:favorites-changed", handleFavoritesChanged);
  dialogEl?.remove();
  dialogEl = null;
}

export const searchDialog = {
  open: openDialog,
  close: closeDialog,
  isOpen: isDialogOpen,
  init: initSearchDialog,
  dispose: disposeSearchDialog,
};

window.openSearchDialog = openDialog;
