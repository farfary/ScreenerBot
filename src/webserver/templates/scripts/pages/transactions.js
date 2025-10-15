import { registerPage } from "../core/lifecycle.js";
import { Poller } from "../core/poller.js";
import { $ } from "../core/dom.js";
import * as Utils from "../core/utils.js";
import * as AppState from "../core/app_state.js";
import { DataTable } from "../ui/data_table.js";

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

function getTypeBadge(txType) {
  const badges = {
    buy: '<span class="badge success">Buy</span>',
    sell: '<span class="badge error">Sell</span>',
    swap: '<span class="badge info">Swap</span>',
    transfer: '<span class="badge secondary">Transfer</span>',
    ata: '<span class="badge secondary">ATA</span>',
    failed: '<span class="badge error">Failed</span>',
    unknown: '<span class="badge secondary">Unknown</span>',
  };
  return badges[txType] || `<span class="badge secondary">${txType}</span>`;
}

function getDirectionBadge(direction) {
  const badges = {
    Incoming: '<span class="badge success">↓ In</span>',
    Outgoing: '<span class="badge error">↑ Out</span>',
    Internal: '<span class="badge secondary">⟲ Internal</span>',
    Unknown: '<span class="badge secondary">? Unknown</span>',
  };
  return (
    badges[direction] || `<span class="badge secondary">${direction}</span>`
  );
}

function getStatusBadge(status) {
  const badges = {
    Pending: '<span class="badge warning">⏳ Pending</span>',
    Confirmed: '<span class="badge success">✓ Confirmed</span>',
    Finalized: '<span class="badge success">✓✓ Finalized</span>',
    Failed: '<span class="badge error">✗ Failed</span>',
  };
  return badges[status] || `<span class="badge secondary">${status}</span>`;
}

// =============================================================================
// STATE & FILTERS
// =============================================================================

// Pagination state
let paginationState = {
  currentCursor: null,
  isLoading: false,
  hasMore: true,
  allLoadedData: [], // Track all loaded rows for client-side operations
};

// Current active filters
let currentFilters = {
  signature: null,
  types: [],
  direction: null,
  status: null,
};

function getFiltersFromUI() {
  const signature = $("#tx-filter-signature")?.value.trim() || "";
  const type = $("#tx-filter-type")?.value || "all";
  const direction = $("#tx-filter-direction")?.value || "all";
  const status = $("#tx-filter-status")?.value || "all";

  return {
    signature: signature || null,
    types: type === "all" ? [] : [type.toLowerCase()],
    direction: direction === "all" ? null : direction,
    status: status === "all" ? null : status,
  };
}

function applyFilters() {
  currentFilters = getFiltersFromUI();
  AppState.save("tx_filters", currentFilters);

  console.log("[TX] Applying filters:", currentFilters);

  // Reset pagination and reload from scratch
  paginationState = {
    currentCursor: null,
    isLoading: false,
    hasMore: true,
    allLoadedData: [],
  };

  if (window.transactionsTable) {
    window.transactionsTable.clearData();
    loadTransactions();
  }
}

function resetFilters() {
  currentFilters = {
    signature: null,
    types: [],
    direction: null,
    status: null,
  };

  // Reset UI
  if ($("#tx-filter-signature")) $("#tx-filter-signature").value = "";
  if ($("#tx-filter-type")) $("#tx-filter-type").value = "all";
  if ($("#tx-filter-direction")) $("#tx-filter-direction").value = "all";
  if ($("#tx-filter-status")) $("#tx-filter-status").value = "all";

  AppState.save("tx_filters", currentFilters);

  console.log("[TX] Resetting filters");

  // Reset pagination and reload
  paginationState = {
    currentCursor: null,
    isLoading: false,
    hasMore: true,
    allLoadedData: [],
  };

  if (window.transactionsTable) {
    window.transactionsTable.clearData();
    loadTransactions();
  }
}

// =============================================================================
// API CALLS
// =============================================================================

async function fetchSummary() {
  try {
    const response = await fetch("/api/transactions/summary", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    return await response.json();
  } catch (err) {
    console.error("Failed to fetch transaction summary:", err);
    return null;
  }
}

async function fetchTransactions(cursor = null, limit = 50) {
  try {
    // Build filters - strip null values for cleaner API call
    const filters = {};
    if (currentFilters.signature) filters.signature = currentFilters.signature;
    if (currentFilters.types && currentFilters.types.length > 0) filters.types = currentFilters.types;
    if (currentFilters.direction) filters.direction = currentFilters.direction;
    if (currentFilters.status) filters.status = currentFilters.status;

    const response = await fetch("/api/transactions/list", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        filters,
        pagination: { cursor, limit },
      }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }

    return await response.json();
  } catch (err) {
    console.error("Failed to fetch transactions:", err);
    return null;
  }
}

// =============================================================================
// RENDERING
// =============================================================================

function renderSummary(summary) {
  const grid = $("#tx-summary-grid");
  if (!grid || !summary) return;

  grid.innerHTML = `
    <div class="transactions-summary-card">
      <div class="summary-label">Total Transactions</div>
      <div class="summary-value">${Utils.formatNumber(summary.total)}</div>
    </div>
    <div class="transactions-summary-card">
      <div class="summary-label">Success / Failed</div>
      <div class="summary-value">
        ${Utils.formatNumber(summary.success_count)} / ${Utils.formatNumber(
    summary.failed_count
  )}
      </div>
      <div class="summary-subvalue">
        <span class="value-positive">${summary.success_rate.toFixed(1)}%</span>
        <span class="value-negative">${summary.failure_rate.toFixed(1)}%</span>
      </div>
    </div>
    <div class="transactions-summary-card">
      <div class="summary-label">Pending</div>
      <div class="summary-value">${Utils.formatNumber(
        summary.pending_global
      )}</div>
      ${
        summary.deferred_count > 0
          ? `<div class="summary-subvalue">${summary.deferred_count} deferred</div>`
          : ""
      }
    </div>
    <div class="transactions-summary-card">
      <div class="summary-label">Database</div>
      <div class="summary-value">${summary.db_size_mb.toFixed(2)} MB</div>
      <div class="summary-subvalue">Schema v${summary.db_schema_version}</div>
    </div>
  `;
}

function updateLastUpdated() {
  const el = $("#tx-last-updated");
  if (el) {
    el.textContent = `Last update: ${new Date().toLocaleTimeString()}`;
  }
}

/**
 * Load transactions with pagination support
 * @param {boolean} append - If true, append to existing data; if false, replace
 */
async function loadTransactions(append = false) {
  console.log("[TX] loadTransactions called, append:", append);
  
  if (!window.transactionsTable) {
    console.error("[TX] transactionsTable not initialized!");
    return;
  }

  // Prevent duplicate loads
  if (paginationState.isLoading) {
    console.log("[TX] Already loading, skipping");
    return;
  }

  // Check if we have more data to load
  if (append && !paginationState.hasMore) {
    console.log("[TX] No more data to load");
    return;
  }

  paginationState.isLoading = true;

  const cursor = append ? paginationState.currentCursor : null;
  const result = await fetchTransactions(cursor, 50);
  
  paginationState.isLoading = false;

  console.log("[TX] fetchTransactions result:", result);
  if (!result) return;

  // Update pagination state
  paginationState.currentCursor = result.next_cursor;
  paginationState.hasMore = result.next_cursor !== null && result.next_cursor !== undefined;

  console.log("[TX] Next cursor:", paginationState.currentCursor);
  console.log("[TX] Has more:", paginationState.hasMore);

  if (append) {
    // Append to existing data
    paginationState.allLoadedData.push(...result.items);
    console.log("[TX] Appending items, new total:", paginationState.allLoadedData.length);
    window.transactionsTable.setData(paginationState.allLoadedData);
  } else {
    // Replace data (initial load or filter change)
    paginationState.allLoadedData = result.items;
    console.log("[TX] Setting initial data, items count:", result.items?.length);
    window.transactionsTable.setData(result.items);
  }

  // Update summary meta
  const summaryEl = $("#tx-summary");
  if (summaryEl && result.total_estimate !== undefined) {
    summaryEl.textContent = `${Utils.formatNumber(
      result.total_estimate
    )} total`;
  }

  // Update loaded count indicator
  updateLoadedCountIndicator();
}

/**
 * Update the indicator showing how many transactions are loaded
 */
function updateLoadedCountIndicator() {
  const summaryEl = $("#tx-summary");
  if (!summaryEl) return;

  const loaded = paginationState.allLoadedData.length;
  const hasMore = paginationState.hasMore;
  const suffix = hasMore ? " (scroll for more)" : " (all loaded)";
  
  summaryEl.textContent = `${Utils.formatNumber(loaded)} loaded ${suffix}`;
}

// =============================================================================
// TABLE CONFIGURATION
// =============================================================================

function createTable() {
  const columns = [
    {
      id: "timestamp",
      label: "Date / Time",
      minWidth: 140,
      render: (val) => {
        if (!val) return '<span class="text-muted">—</span>';
        const date = new Date(val);
        const dateStr = date.toLocaleDateString();
        const timeStr = date.toLocaleTimeString();
        return `<div style="font-size: 0.85em;">${dateStr}<br/>${timeStr}</div>`;
      },
    },
    {
      id: "signature",
      label: "Signature",
      minWidth: 200,
      wrap: false,
      render: (val) => {
        if (!val) return '<span class="text-muted">—</span>';
        const short = Utils.formatSignatureCompact(val);
        return `<a href="https://solscan.io/tx/${val}" target="_blank" rel="noopener" class="link-signature">${short}</a>`;
      },
    },
    {
      id: "transaction_type",
      label: "Type",
      minWidth: 80,
      render: (val) => getTypeBadge(val || "unknown"),
    },
    {
      id: "direction",
      label: "Direction",
      minWidth: 80,
      render: (val) => getDirectionBadge(val || "Unknown"),
    },
    {
      id: "status",
      label: "Status",
      minWidth: 90,
      render: (val) => getStatusBadge(val || "Unknown"),
    },
    {
      id: "token_mint",
      label: "Mint",
      minWidth: 140,
      render: (val) => {
        if (!val) return '<span class="text-muted">—</span>';
        const short = Utils.formatAddressCompact(val);
        return `<a href="https://solscan.io/token/${val}" target="_blank" rel="noopener" class="link-signature" title="${val}">${short}</a>`;
      },
    },
    {
      id: "router",
      label: "Router",
      minWidth: 100,
      render: (val) => val || '<span class="text-muted">—</span>',
    },
    {
      id: "sol_delta",
      label: "SOL Δ",
      minWidth: 90,
      sortable: true,
      render: (val) => {
        if (val === null || val === undefined) return "—";
        const formatted = Utils.formatSol(Math.abs(val), { suffix: "" });
        const className = val >= 0 ? "value-positive" : "value-negative";
        const sign = val >= 0 ? "+" : "";
        return `<span class="${className}">${sign}${formatted}</span>`;
      },
    },
    {
      id: "fee_sol",
      label: "Fee",
      minWidth: 80,
      render: (val) => (val ? Utils.formatSol(val, { suffix: "" }) : "—"),
    },
    {
      id: "ata_rents",
      label: "ATA Rent",
      minWidth: 80,
      render: (val) => (val ? Utils.formatSol(val, { suffix: "" }) : "—"),
    },
    {
      id: "instructions_count",
      label: "Instr",
      minWidth: 48,
      render: (val) => (val !== null && val !== undefined ? val : "—"),
    },
  ];

  window.transactionsTable = new DataTable({
    container: "#transactions-root",
    columns,
    rowIdField: "signature",
    emptyMessage: "No transactions found",
    loadingMessage: "Loading transactions...",
    stateKey: "transactions-table",
    compact: true,
    stickyHeader: true,
    zebra: true,
    fitToContainer: true,
    autoSizeColumns: true,
    autoSizeSample: 40,
    autoSizePadding: 20,
    // Disable built-in toolbar since we use custom filters
    toolbar: {},
  });

  // Setup scroll pagination
  setupScrollPagination();
}

/**
 * Setup infinite scroll pagination
 */
function setupScrollPagination() {
  // Wait for table to be rendered
  setTimeout(() => {
    const scrollContainer = document.querySelector(
      "#transactions-root .data-table-scroll-container"
    );

    if (!scrollContainer) {
      console.warn("[TX] Scroll container not found for pagination");
      return;
    }

    console.log("[TX] Scroll container found:", {
      scrollHeight: scrollContainer.scrollHeight,
      clientHeight: scrollContainer.clientHeight,
      isScrollable: scrollContainer.scrollHeight > scrollContainer.clientHeight,
    });

    // Runtime safeguard: if CSS max-height didn't apply and container isn't scrollable,
    // forcibly set a sensible max-height so infinite scroll can trigger.
    try {
      const computed = window.getComputedStyle(scrollContainer);
      const computedMaxHeight = computed?.maxHeight || "";
      if (
        scrollContainer.scrollHeight === scrollContainer.clientHeight ||
        computedMaxHeight === "none"
      ) {
        console.warn(
          "[TX] Scroll container not scrollable (or max-height none); applying runtime max-height fallback"
        );
        // Apply fallback styles
        scrollContainer.style.maxHeight = "600px"; // matches CSS default
        scrollContainer.style.overflowY = "auto";
        // Re-evaluate after style application
        setTimeout(() => {
          console.log("[TX] Post-fallback scroll container:", {
            scrollHeight: scrollContainer.scrollHeight,
            clientHeight: scrollContainer.clientHeight,
            isScrollable:
              scrollContainer.scrollHeight > scrollContainer.clientHeight,
            computedMaxHeight:
              window.getComputedStyle(scrollContainer)?.maxHeight || "",
          });
        }, 0);
      }
    } catch (e) {
      console.warn("[TX] Failed to inspect/apply fallback styles:", e);
    }

    let scrollThrottle = null;

    scrollContainer.addEventListener("scroll", () => {
      // Throttle scroll events
      if (scrollThrottle) {
        window.clearTimeout(scrollThrottle);
      }

      scrollThrottle = window.setTimeout(() => {
        const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
        const scrollPercentage = (scrollTop + clientHeight) / scrollHeight;

        console.log("[TX] Scroll event:", {
          scrollTop,
          scrollHeight,
          clientHeight,
          percentage: (scrollPercentage * 100).toFixed(1) + "%",
          isLoading: paginationState.isLoading,
          hasMore: paginationState.hasMore
        });

        // Load more when scrolled to 80% of the way down
        if (scrollPercentage > 0.8 && !paginationState.isLoading && paginationState.hasMore) {
          console.log("[TX] Scroll threshold reached (>80%), loading more data...");
          loadTransactions(true); // append = true
        }
      }, 150); // 150ms throttle
    });

    console.log("[TX] Scroll pagination setup complete");
  }, 500); // Wait for DataTable to fully render
}

// =============================================================================
// LIFECYCLE
// =============================================================================

function createLifecycle() {
  let summaryPoller = null;

  return {
    init(ctx) {
      // Restore filters from state
      const savedFilters = AppState.load("tx_filters");
      if (savedFilters) {
        currentFilters = savedFilters;

        // Apply to UI
        if (savedFilters.signature && $("#tx-filter-signature")) {
          $("#tx-filter-signature").value = savedFilters.signature;
        }
        if (savedFilters.types && savedFilters.types.length > 0 && $("#tx-filter-type")) {
          $("#tx-filter-type").value = savedFilters.types[0];
        }
        if (savedFilters.direction && $("#tx-filter-direction")) {
          $("#tx-filter-direction").value = savedFilters.direction;
        }
        if (savedFilters.status && $("#tx-filter-status")) {
          $("#tx-filter-status").value = savedFilters.status;
        }
      }

      // Create table
      createTable();

      // Bind filter controls
      const applyBtn = $("#tx-apply-filters");
      const resetBtn = $("#tx-reset-filters");
      const signatureInput = $("#tx-filter-signature");
      const clearBtn = $("#tx-clear-signature");

      if (applyBtn) {
        applyBtn.addEventListener("click", applyFilters);
      }

      if (resetBtn) {
        resetBtn.addEventListener("click", resetFilters);
      }

      if (signatureInput && clearBtn) {
        signatureInput.addEventListener("input", () => {
          clearBtn.hidden = !signatureInput.value;
        });

        signatureInput.addEventListener("keydown", (e) => {
          if (e.key === "Enter") {
            applyFilters();
          }
        });

        clearBtn.addEventListener("click", () => {
          signatureInput.value = "";
          clearBtn.hidden = true;
          signatureInput.focus();
        });
      }
    },

    activate(ctx) {
      // Initial load
      loadTransactions();

      // Start summary poller
      summaryPoller = ctx.managePoller(
        new Poller(async () => {
          const summary = await fetchSummary();
          if (summary) {
            renderSummary(summary);
            updateLastUpdated();
          }
        }, 10000)
      ); // Poll every 10s

      summaryPoller.start();
    },

    deactivate() {
      // Poller auto-paused by lifecycle context
    },

    dispose() {
      // Cleanup
      summaryPoller = null;
      if (window.transactionsTable) {
        window.transactionsTable = null;
      }
    },
  };
}

registerPage("transactions", createLifecycle());
