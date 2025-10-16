import { registerPage } from "../core/lifecycle.js";
// Poller no longer used for page-level summary; keep import if needed later
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
const DEFAULT_FILTERS = {
  signature: "",
  type: "all",
  direction: "all",
  status: "all",
};

let pendingFilters = { ...DEFAULT_FILTERS };
let currentFilters = {
  signature: null,
  types: [],
  direction: null,
  status: null,
};

function normalizeFilters(raw) {
  return {
    signature: raw.signature ? raw.signature.trim() || null : null,
    types:
      raw.type && raw.type !== "all" ? [raw.type.toLowerCase()] : [],
    direction:
      raw.direction && raw.direction !== "all" ? raw.direction : null,
    status: raw.status && raw.status !== "all" ? raw.status : null,
  };
}

function loadSavedFilters() {
  const saved = AppState.load("tx_filters");
  if (saved && typeof saved === "object") {
    pendingFilters = { ...DEFAULT_FILTERS };
    if (typeof saved.signature === "string") {
      pendingFilters.signature = saved.signature;
    }
    if (Array.isArray(saved.types) && saved.types.length > 0) {
      pendingFilters.type = saved.types[0] || "all";
    } else if (typeof saved.type === "string") {
      pendingFilters.type = saved.type || "all";
    }
    if (typeof saved.direction === "string") {
      pendingFilters.direction = saved.direction || "all";
    }
    if (typeof saved.status === "string") {
      pendingFilters.status = saved.status || "all";
    }
  } else {
    pendingFilters = { ...DEFAULT_FILTERS };
  }
  currentFilters = normalizeFilters(pendingFilters);
}

function persistPendingFilters() {
  AppState.save("tx_filters", pendingFilters);
}

function resetPendingFilters(table) {
  pendingFilters = { ...DEFAULT_FILTERS };
  currentFilters = normalizeFilters(pendingFilters);
  persistPendingFilters();
  if (table) {
    table.setToolbarSearchValue("", { apply: false });
    table.setToolbarFilterValue("type", "all", { apply: false });
    table.setToolbarFilterValue("direction", "all", { apply: false });
    table.setToolbarFilterValue("status", "all", { apply: false });
  }
  paginationState = {
    currentCursor: null,
    isLoading: false,
    hasMore: true,
    allLoadedData: [],
  };
  if (table) {
    table.clearData();
  }
  loadTransactions();
}

function applyPendingFilters(table, options = {}) {
  currentFilters = normalizeFilters(pendingFilters);
  persistPendingFilters();

  console.log("[TX] Applying filters:", currentFilters);

  paginationState = {
    currentCursor: null,
    isLoading: false,
    hasMore: true,
    allLoadedData: [],
  };

  if (table) {
    table.clearData();
  }
  loadTransactions(options.append === true);
}

function syncToolbarFromPending(table) {
  if (!table) {
    return;
  }
  table.setToolbarSearchValue(pendingFilters.signature || "", { apply: false });
  table.setToolbarFilterValue("type", pendingFilters.type || "all", {
    apply: false,
  });
  table.setToolbarFilterValue(
    "direction",
    pendingFilters.direction || "all",
    { apply: false }
  );
  table.setToolbarFilterValue("status", pendingFilters.status || "all", {
    apply: false,
  });
}

// =============================================================================
// API CALLS
// =============================================================================


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


function updateLastUpdated() {
  if (!window.transactionsTable) {
    return;
  }
  window.transactionsTable.updateToolbarMeta([
    { id: "lastUpdate", text: `Last update ${new Date().toLocaleTimeString()}` },
  ]);
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
  paginationState.hasMore =
    result.next_cursor !== null && result.next_cursor !== undefined;

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

  updateToolbarStats(result.total_estimate);
  updateLastUpdated();
}

function updateToolbarStats(totalEstimate) {
  if (!window.transactionsTable) {
    return;
  }

  const loaded = paginationState.allLoadedData.length;
  const hasMore = paginationState.hasMore;

  const summaryItems = [
    {
      id: "loaded",
      label: "Loaded",
      value: Utils.formatNumber(loaded),
      tooltip: hasMore ? "Scroll for more" : "All results loaded",
    },
    {
      id: "total",
      label: "Total",
      value:
        totalEstimate !== undefined && totalEstimate !== null
          ? Utils.formatNumber(totalEstimate)
          : "—",
    },
  ];

  window.transactionsTable.updateToolbarSummary(summaryItems);
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
    toolbar: {
      title: {
        icon: "💱",
        text: "Transactions",
        meta: [{ id: "lastUpdate", text: "Last update —" }],
      },
      summary: [
        { id: "loaded", label: "Loaded", value: "0" },
        { id: "total", label: "Total", value: "—" },
      ],
      search: {
        enabled: true,
        placeholder: "Search by signature...",
        onChange: (value) => {
          pendingFilters.signature = value ?? "";
        },
        onSubmit: () => {
          applyPendingFilters(window.transactionsTable);
        },
      },
      filters: [
        {
          id: "type",
          label: "Type",
          defaultValue: "all",
          minWidth: "140px",
          autoApply: false,
          options: [
            { value: "all", label: "All types" },
            { value: "buy", label: "Buy" },
            { value: "sell", label: "Sell" },
            { value: "swap", label: "Swap" },
            { value: "transfer", label: "Transfer" },
            { value: "ata", label: "ATA" },
            { value: "failed", label: "Failed" },
            { value: "unknown", label: "Unknown" },
          ],
          onChange: (value) => {
            pendingFilters.type = value || "all";
          },
        },
        {
          id: "direction",
          label: "Direction",
          defaultValue: "all",
          minWidth: "140px",
          autoApply: false,
          options: [
            { value: "all", label: "All directions" },
            { value: "Incoming", label: "Incoming" },
            { value: "Outgoing", label: "Outgoing" },
            { value: "Internal", label: "Internal" },
            { value: "Unknown", label: "Unknown" },
          ],
          onChange: (value) => {
            pendingFilters.direction = value || "all";
          },
        },
        {
          id: "status",
          label: "Status",
          defaultValue: "all",
          minWidth: "140px",
          autoApply: false,
          options: [
            { value: "all", label: "All statuses" },
            { value: "Pending", label: "Pending" },
            { value: "Confirmed", label: "Confirmed" },
            { value: "Finalized", label: "Finalized" },
            { value: "Failed", label: "Failed" },
          ],
          onChange: (value) => {
            pendingFilters.status = value || "all";
          },
        },
      ],
      buttons: [
        {
          id: "reset",
          label: "Reset",
          onClick: (_btn, table) => {
            resetPendingFilters(table);
          },
        },
        {
          id: "apply",
          label: "Apply Filters",
          variant: "primary",
          onClick: (_btn, table) => {
            applyPendingFilters(table);
          },
        },
      ],
    },
  });

  syncToolbarFromPending(window.transactionsTable);

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
  return {
    init(_ctx) {
      loadSavedFilters();
      paginationState = {
        currentCursor: null,
        isLoading: false,
        hasMore: true,
        allLoadedData: [],
      };
      createTable();
    },

    activate(_ctx) {
      // Initial load
      loadTransactions();
    },

    deactivate() {
      // Poller auto-paused by lifecycle context
    },

    dispose() {
      // Cleanup
      if (window.transactionsTable) {
        window.transactionsTable = null;
      }
    },
  };
}

registerPage("transactions", createLifecycle());
