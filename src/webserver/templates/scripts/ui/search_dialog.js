/**
 * Global Search Dialog Component
 *
 * Allows users to search for tokens by name, symbol, or mint address.
 * Triggered by Cmd/Ctrl+K keyboard shortcut or via openSearchDialog().
 *
 * Features:
 * - Debounced search as user types
 * - Keyboard navigation (arrow keys, Enter, Escape)
 * - Results show logo, name, symbol, price, market cap
 * - Actions: copy mint, view details
 */

import { $, create, show, hide, on, off } from "../core/dom.js";
import { showToast, notifyCopied } from "../core/utils.js";
import { ConfirmationDialog } from "./confirmation_dialog.js";

const SEARCH_DEBOUNCE_MS = 300;
const MIN_QUERY_LENGTH = 2;

let dialogEl = null;

// =============================================================================
// SETUP STATE CHECK
// =============================================================================

/**
 * Check if setup, onboarding, or splash screens are currently visible.
 * Search dialog should not open during these screens.
 */
function isSetupActive() {
  // Check for splash screen
  const splash = document.getElementById("splash-screen");
  if (splash && splash.style.display !== "none" && !splash.classList.contains("hidden")) {
    return true;
  }

  // Check for setup screen
  const setup = document.getElementById("setup-screen");
  if (setup && setup.style.display !== "none" && !setup.classList.contains("hidden")) {
    return true;
  }

  // Check for onboarding screen
  const onboarding = document.getElementById("onboarding-screen");
  if (
    onboarding &&
    onboarding.style.display !== "none" &&
    !onboarding.classList.contains("hidden")
  ) {
    return true;
  }

  return false;
}
let isOpen = false;
let selectedIndex = 0;
let currentResults = [];
let searchDebounceTimer = null;

// =============================================================================
// UTILITIES
// =============================================================================

/**
 * Format number in compact notation (1.2K, 3.4M, etc.)
 */
function formatCompactNumber(n) {
  if (n === null || n === undefined || !Number.isFinite(n)) return "—";
  if (n >= 1e9) return (n / 1e9).toFixed(2) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(2) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(2) + "K";
  return n.toFixed(2);
}

/**
 * Format currency in USD
 */
function formatCurrencyUSD(value) {
  if (value === null || value === undefined || !Number.isFinite(value)) return "—";
  return (
    "$" +
    value.toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 6,
    })
  );
}

/**
 * Escape HTML to prevent XSS
 */
function escapeHTML(str) {
  if (!str) return "";
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

/**
 * Debounce function execution
 */
function debounce(fn, ms) {
  return function (...args) {
    clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => fn.apply(this, args), ms);
  };
}

// =============================================================================
// DIALOG CREATION
// =============================================================================

/**
 * Create and insert dialog HTML into DOM
 */
function createDialog() {
  if (dialogEl) return dialogEl;

  dialogEl = create("div", { class: "search-dialog-overlay", id: "search-dialog" });
  dialogEl.innerHTML = `
    <div class="search-dialog" role="dialog" aria-modal="true">
      <div class="search-dialog-header">
        <div class="search-input-wrapper">
          <i class="search-icon icon-search" aria-hidden="true"></i>
          <input 
            type="text" 
            id="search-input" 
            class="search-input"
            placeholder="Search name, symbol or mint..." 
            autocomplete="off"
            spellcheck="false"
            aria-label="Search tokens"
          >
        </div>
      </div>
      <div class="search-dialog-body">
        <div id="search-results" class="search-results" role="listbox" aria-label="Search results">
          <div class="search-empty">
            <p class="search-hint">Type token name, symbol or paste mint</p>
          </div>
        </div>
      </div>
      <div class="search-dialog-footer">
        <span class="search-tip"><kbd>↑</kbd><kbd>↓</kbd> nav &nbsp; <kbd>↵</kbd> open &nbsp; <kbd>esc</kbd> close</span>
      </div>
    </div>
  `;

  document.body.appendChild(dialogEl);

  // Event listeners
  on(dialogEl, "click", handleOverlayClick);

  const input = $("#search-input", dialogEl);
  on(input, "input", debounce(handleSearch, SEARCH_DEBOUNCE_MS));
  on(input, "keydown", handleInputKeydown);

  return dialogEl;
}

// =============================================================================
// SEARCH HANDLING
// =============================================================================

/**
 * Handle search input
 */
async function handleSearch(e) {
  const query = e.target.value.trim();
  const resultsEl = $("#search-results", dialogEl);

  if (query.length < MIN_QUERY_LENGTH) {
    resultsEl.innerHTML = `
      <div class="search-empty">
        <p class="search-hint">Type token name, symbol or paste mint</p>
      </div>
    `;
    currentResults = [];
    return;
  }

  // Show loading — minimal
  resultsEl.innerHTML = `
    <div class="search-loading">
      <i class="icon-loader search-loading-icon"></i>
    </div>
  `;

  try {
    const response = await fetch(`/api/tokens/search?q=${encodeURIComponent(query)}&limit=20`);
    const data = await response.json();

    if (!response.ok) {
      throw new Error(data.error || "Search failed");
    }

    currentResults = data.results || [];
    selectedIndex = 0;
    renderResults();
  } catch (error) {
    resultsEl.innerHTML = `
      <div class="search-error">
        <i class="icon-circle-alert"></i>
        <span>Error: ${escapeHTML(error.message)}</span>
      </div>
    `;
  }
}

/**
 * Render search results
 */
function renderResults() {
  const resultsEl = $("#search-results", dialogEl);

  if (currentResults.length === 0) {
    resultsEl.innerHTML = `
      <div class="search-empty">
        <p class="search-hint">No matches — try different term</p>
      </div>
    `;
    return;
  }

  resultsEl.innerHTML = currentResults
    .map(
      (token, i) => `
    <div class="search-result ${i === selectedIndex ? "selected" : ""}" 
         id="search-result-${i}"
         data-index="${i}" 
         data-mint="${escapeHTML(token.mint)}"
         role="option"
         aria-selected="${i === selectedIndex}">
      <div class="search-result-main">
        ${
          token.logo_url
            ? `<img src="${escapeHTML(token.logo_url)}" class="search-result-logo" alt="" loading="lazy" onerror="this.style.display='none'; this.nextElementSibling.style.display='flex'">`
            : ""
        }
        <div class="search-result-logo-placeholder" ${token.logo_url ? 'style="display:none"' : ""}>
          <i class="icon-coins"></i>
        </div>
        <div class="search-result-info">
          <div class="search-result-name token-name-type">${escapeHTML(token.name || "Unknown")}</div>
          <div class="search-result-symbol token-symbol-type">${escapeHTML(token.symbol || "???")}</div>
        </div>
      </div>
      <div class="search-result-data">
        <div class="search-result-price">${formatCurrencyUSD(token.price_usd)}</div>
        <div class="search-result-mcap">${formatCompactNumber(token.market_cap)}</div>
      </div>
      <div class="search-result-actions">
        <button class="btn-icon btn-icon-sm search-action-btn action-favorite" data-action="favorite" title="Add to Favorites" aria-label="Add to Favorites">
          <i class="icon-star"></i>
        </button>
        <button class="btn-icon btn-icon-sm search-action-btn action-blacklist" data-action="blacklist" title="Add to Blacklist" aria-label="Add to Blacklist">
          <i class="icon-slash"></i>
        </button>
        <button class="btn-icon btn-icon-sm search-action-btn" data-action="copy" title="Copy Mint Address" aria-label="Copy mint address">
          <i class="icon-copy"></i>
        </button>
        <button class="btn-icon btn-icon-sm search-action-btn" data-action="view" title="View on DexScreener" aria-label="View on DexScreener">
          <i class="icon-external-link"></i>
        </button>
      </div>
    </div>
  `
    )
    .join("");

  // Add click handlers (only on full render)
  resultsEl.querySelectorAll(".search-result").forEach((el) => {
    on(el, "click", handleResultClick);
  });

  // Scroll selected item into view
  const selectedEl = resultsEl.querySelector(".search-result.selected");
  if (selectedEl) {
    selectedEl.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }
}

/**
 * Update only the visual selection (no full re-render).
 * Much smoother for arrow key navigation.
 */
function updateSelectionHighlight() {
  const resultsEl = $("#search-results", dialogEl);
  if (!resultsEl) return;

  const input = $("#search-input", dialogEl);
  const items = resultsEl.querySelectorAll(".search-result");
  let activeId = null;

  items.forEach((el, i) => {
    const isSelected = i === selectedIndex;
    el.classList.toggle("selected", isSelected);
    el.setAttribute("aria-selected", isSelected ? "true" : "false");
    if (isSelected) {
      activeId = el.id || (el.id = `search-result-${i}`);
    }
  });

  // Improve a11y: point input at the active option
  if (input && activeId) {
    input.setAttribute("aria-activedescendant", activeId);
  }

  const selectedEl = items[selectedIndex];
  if (selectedEl) {
    selectedEl.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }
}

/**
 * Handle click on a search result
 */
function handleResultClick(e) {
  const resultEl = e.currentTarget;
  const actionBtn = e.target.closest(".search-action-btn");
  const index = parseInt(resultEl.dataset.index, 10);
  const token = currentResults[index];

  if (actionBtn) {
    e.stopPropagation();
    const action = actionBtn.dataset.action;
    if (action === "copy") {
      copyMint(token);
    } else if (action === "view") {
      viewOnDexScreener(token);
    } else if (action === "favorite") {
      addToFavorites(token, actionBtn);
    } else if (action === "blacklist") {
      addToBlacklist(token, actionBtn);
    }
    return;
  }

  // Default: open the token details dialog and close the search.
  openTokenDetails(token);
}

/**
 * Open the global token details dialog for a search result.
 * Reuses the shared `screenerbot:open-token-details` event so we stay decoupled
 * from the dialog implementation (same path the context menu uses).
 */
async function openTokenDetails(token) {
  if (!token?.mint) {
    showToast("Token has no mint address", "warning");
    return;
  }
  closeDialog();
  // The token details dialog registers the global `screenerbot:open-token-details`
  // listener on import. It is a heavy module (charts, tabs) loaded per-page, and
  // the search dialog is global, so load it lazily here — only when the user
  // actually opens details — then fire the event the listener handles.
  try {
    await import("./token_details_dialog.js");
  } catch {
    showToast("Failed to open token details", "error");
    return;
  }
  window.dispatchEvent(
    new CustomEvent("screenerbot:open-token-details", {
      detail: { mint: token.mint, symbol: token.symbol || "" },
    })
  );
}

/**
 * Copy mint address to clipboard
 */
async function copyMint(token) {
  try {
    await navigator.clipboard.writeText(token.mint);
    notifyCopied(token.symbol || "Mint address");
  } catch {
    showToast("Failed to copy to clipboard", "error");
  }
}

/**
 * Open token on DexScreener
 */
function viewOnDexScreener(token) {
  window.open(`https://dexscreener.com/solana/${token.mint}`, "_blank", "noopener,noreferrer");
}

/**
 * Add token to favorites
 */
async function addToFavorites(token, btn) {
  if (btn) {
    btn.disabled = true;
    btn.classList.add("loading");
  }

  try {
    const response = await fetch("/api/tokens/favorites", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        mint: token.mint,
        name: token.name,
        symbol: token.symbol,
        logo_url: token.logo_url,
      }),
    });

    const data = await response.json();

    if (response.ok && data.success) {
      showToast(`Added ${token.symbol || token.mint} to favorites`, "success");
      if (btn) {
        btn.classList.add("active");
        btn.title = "Already in Favorites";
      }
    } else {
      throw new Error(data.error || "Failed to add to favorites");
    }
  } catch (error) {
    showToast(`Error: ${error.message}`, "error");
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.classList.remove("loading");
    }
  }
}

/**
 * Add token to blacklist
 */
async function addToBlacklist(token, btn) {
  // Confirm before blacklisting
  const result = await ConfirmationDialog.show({
    title: "Blacklist Token",
    message: `Blacklist ${token.symbol || token.mint}? This token will be excluded from trading.`,
    confirmLabel: "Blacklist",
    variant: "warning",
  });
  if (!result.confirmed) {
    return;
  }

  if (btn) {
    btn.disabled = true;
    btn.classList.add("loading");
  }

  try {
    const response = await fetch(`/api/tokens/${token.mint}/blacklist`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        mint: token.mint,
        reason: "Manual blacklist via search",
      }),
    });

    const data = await response.json();

    if (response.ok && data.success) {
      showToast(`Blacklisted ${token.symbol || token.mint}`, "success");
      if (btn) {
        btn.classList.add("active");
        btn.title = "Blacklisted";
      }
    } else {
      throw new Error(data.error || "Failed to blacklist token");
    }
  } catch (error) {
    showToast(`Error: ${error.message}`, "error");
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.classList.remove("loading");
    }
  }
}

// =============================================================================
// EVENT HANDLERS
// =============================================================================

/**
 * Handle overlay click (close on backdrop click)
 */
function handleOverlayClick(e) {
  if (e.target === dialogEl) {
    closeDialog();
  }
}

/**
 * Handle keyboard navigation within input
 */
function handleInputKeydown(e) {
  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      if (currentResults.length > 0) {
        selectedIndex = Math.min(selectedIndex + 1, currentResults.length - 1);
        updateSelectionHighlight();
      }
      break;
    case "ArrowUp":
      e.preventDefault();
      if (currentResults.length > 0) {
        selectedIndex = Math.max(selectedIndex - 1, 0);
        updateSelectionHighlight();
      }
      break;
    case "Enter":
      e.preventDefault();
      if (currentResults[selectedIndex]) {
        openTokenDetails(currentResults[selectedIndex]);
      }
      break;
    case "Escape":
      e.preventDefault();
      closeDialog();
      break;
  }
}

/**
 * Global keydown handler for Cmd/Ctrl+K shortcut
 */
function handleGlobalKeydown(e) {
  // Cmd/Ctrl+K to toggle search
  if ((e.metaKey || e.ctrlKey) && e.key === "k") {
    e.preventDefault();
    // Don't open during setup screens
    if (isSetupActive()) {
      return;
    }
    if (isOpen) {
      closeDialog();
    } else {
      openDialog();
    }
    return;
  }

  // Escape to close (if open)
  if (isOpen && e.key === "Escape") {
    e.preventDefault();
    closeDialog();
  }
}

// =============================================================================
// PUBLIC API
// =============================================================================

/**
 * Open the search dialog
 */
export function openDialog() {
  // Don't open during setup, onboarding, or splash screens
  if (isSetupActive()) {
    return;
  }

  createDialog();
  show(dialogEl);
  dialogEl.classList.add("visible");
  isOpen = true;

  const input = $("#search-input", dialogEl);
  input.value = "";
  input.focus();

  currentResults = [];
  selectedIndex = 0;

  $("#search-results", dialogEl).innerHTML = `
    <div class="search-empty">
      <p class="search-hint">Type token name, symbol or paste mint</p>
    </div>
  `;

  // Prevent body scroll
  document.body.style.overflow = "hidden";
}

/**
 * Close the search dialog
 */
export function closeDialog() {
  if (dialogEl) {
    dialogEl.classList.remove("visible");
    hide(dialogEl);
  }
  isOpen = false;
  currentResults = [];
  selectedIndex = 0;

  // Restore body scroll
  document.body.style.overflow = "";
}

/**
 * Check if dialog is currently open
 */
export function isDialogOpen() {
  return isOpen;
}

/**
 * Initialize the search dialog (sets up global keyboard shortcut)
 */
export function initSearchDialog() {
  on(document, "keydown", handleGlobalKeydown);
}

/**
 * Dispose the search dialog (cleanup)
 */
export function disposeSearchDialog() {
  off(document, "keydown", handleGlobalKeydown);
  if (dialogEl) {
    dialogEl.remove();
    dialogEl = null;
  }
  isOpen = false;
  currentResults = [];
}

// Export for global access
export const searchDialog = {
  open: openDialog,
  close: closeDialog,
  isOpen: isDialogOpen,
  init: initSearchDialog,
  dispose: disposeSearchDialog,
};

// Make available globally for header button
window.openSearchDialog = openDialog;
