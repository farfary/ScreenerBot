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

const SEARCH_DEBOUNCE_MS = 300;
const MIN_QUERY_LENGTH = 2;

let dialogEl = null;
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
  return "$" + value.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 6,
  });
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

/**
 * Show toast notification (uses global toast system if available)
 */
function showToast(message, type = "info") {
  if (window.showToast) {
    window.showToast(message, type);
  } else {
    console.log(`[Toast] ${type}: ${message}`);
  }
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
    <div class="search-dialog" role="dialog" aria-modal="true" aria-labelledby="search-dialog-title">
      <div class="search-dialog-header">
        <div class="search-input-wrapper">
          <i class="search-icon icon-search" aria-hidden="true"></i>
          <input 
            type="text" 
            id="search-input" 
            class="search-input"
            placeholder="Search tokens by name, symbol, or mint..." 
            autocomplete="off"
            spellcheck="false"
            aria-label="Search tokens"
          >
          <kbd class="search-kbd" aria-hidden="true">ESC</kbd>
        </div>
      </div>
      <div class="search-dialog-body">
        <div id="search-results" class="search-results">
          <div class="search-empty">
            <p class="search-empty-title">Search for tokens</p>
            <p class="search-hint">Type a name, symbol, or paste a mint address</p>
          </div>
        </div>
      </div>
      <div class="search-dialog-footer">
        <span class="search-tip"><kbd>↑↓</kbd> Navigate</span>
        <span class="search-tip"><kbd>Enter</kbd> Copy mint</span>
        <span class="search-tip"><kbd>Esc</kbd> Close</span>
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
        <p class="search-empty-title">Search for tokens</p>
        <p class="search-hint">Type a name, symbol, or paste a mint address</p>
      </div>
    `;
    currentResults = [];
    return;
  }

  // Show loading
  resultsEl.innerHTML = `
    <div class="search-loading">
      <i class="icon-loader-2 search-loading-icon"></i>
      <span>Searching...</span>
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
        <i class="icon-alert-circle"></i>
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
        <p class="search-empty-title">No tokens found</p>
        <p class="search-hint">Try a different search term</p>
      </div>
    `;
    return;
  }

  resultsEl.innerHTML = currentResults
    .map(
      (token, i) => `
    <div class="search-result ${i === selectedIndex ? "selected" : ""}" 
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
          <div class="search-result-name">${escapeHTML(token.name || "Unknown")}</div>
          <div class="search-result-symbol">${escapeHTML(token.symbol || "???")}</div>
        </div>
      </div>
      <div class="search-result-data">
        <div class="search-result-price">${formatCurrencyUSD(token.price_usd)}</div>
        <div class="search-result-mcap">MCap: ${formatCompactNumber(token.market_cap)}</div>
      </div>
      <div class="search-result-actions">
        <button class="search-action-btn" data-action="copy" title="Copy Mint Address">
          <i class="icon-copy"></i>
        </button>
        <button class="search-action-btn" data-action="view" title="View on DexScreener">
          <i class="icon-external-link"></i>
        </button>
      </div>
    </div>
  `
    )
    .join("");

  // Add click handlers
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
    }
    return;
  }

  // Default: copy mint and close
  copyMint(token);
  closeDialog();
}

/**
 * Copy mint address to clipboard
 */
async function copyMint(token) {
  try {
    await navigator.clipboard.writeText(token.mint);
    showToast(`Copied: ${token.symbol || token.mint}`, "success");
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
      selectedIndex = Math.min(selectedIndex + 1, currentResults.length - 1);
      renderResults();
      break;
    case "ArrowUp":
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      renderResults();
      break;
    case "Enter":
      e.preventDefault();
      if (currentResults[selectedIndex]) {
        copyMint(currentResults[selectedIndex]);
        closeDialog();
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
      <p class="search-empty-title">Search for tokens</p>
      <p class="search-hint">Type a name, symbol, or paste a mint address</p>
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
