/**
 * Renderers Module for Wallets
 * Handles all rendering and display logic for wallet panels and data
 */

import { DataTable } from "../../ui/data_table.js";

export function createWalletRenderers({
  walletsData,
  tokenHoldings,
  currentTab,
  $,
  Utils,
  on,
  capitalizeFirst,
  handleWalletAction,
}) {
  // DataTable instances — created once, updated via setData() on every poll
  let tokenTable = null;
  let tokenTableClickHandler = null;
  let secondariesTable = null;
  let secondariesClickHandler = null;
  let archiveTable = null;
  let archiveClickHandler = null;

  // Track which wallet is currently rendered in the toolbar so we can do
  // fast in-place updates instead of rebuilding the entire info bar each poll.
  let lastRenderedWalletId = null;

  // Address cell shared by the Secondaries/Archive tables — same short-address +
  // copy + Solscan-link treatment as the main wallet's token Mint column.
  function _addressCellHtml(address) {
    if (!address) return "—";
    const escaped = Utils.escapeHtml(address);
    const short = `${address.slice(0, 6)}...${address.slice(-4)}`;
    const url = `https://solscan.io/account/${encodeURIComponent(address)}`;
    return `<div class="wt-mint-cell"><span class="wt-mint-addr">${short}</span><button type="button" class="copy-btn-mini" data-copy-address="${escaped}" title="Copy address"><i class="icon-copy"></i></button><a class="wt-mint-link" href="${url}" target="_blank" rel="noopener" title="View on Solscan"><i class="icon-external-link"></i></a></div>`;
  }

  // Delegated click handler for the address copy button — mirrors the mint-copy
  // wiring on the tokens table (one listener per table root, not per row).
  function _wireAddressCopy(rootEl) {
    const handler = (e) => {
      const btn = e.target.closest("[data-copy-address]");
      if (!btn) return;
      e.stopPropagation();
      Utils.copyToClipboard(btn.dataset.copyAddress);
      Utils.showToast("Address copied!", "success");
    };
    rootEl.addEventListener("click", handler);
    return handler;
  }

  // Column definitions are stable across renders — define once at closure level
  const TOKEN_COLUMNS = [
    {
      id: "symbol",
      label: "Token",
      sortable: true,
      render: (value, row) => {
        const sym = Utils.escapeHtml(row.symbol || "Unknown");
        const name = row.name ? Utils.escapeHtml(row.name) : null;
        const logo = row.logo_url || "";
        const logoHtml = logo
          ? `<img class="token-logo" src="${Utils.escapeHtml(logo)}" alt="${sym}" loading="lazy"/>`
          : '<i class="token-logo icon-coins"></i>';
        return `<div class="wt-token-cell">${logoHtml}<div class="wt-token-meta"><span class="wt-symbol">${sym}</span>${name ? `<span class="wt-name">${name}</span>` : ""}</div></div>`;
      },
    },
    {
      id: "ui_amount",
      label: "Balance",
      sortable: true,
      render: (value) => (value != null ? Utils.formatNumber(value, { decimals: 4 }) : "—"),
    },
    {
      id: "value_sol",
      label: "Value (SOL)",
      sortable: true,
      render: (value) => (value != null ? Utils.formatSol(value, { decimals: 4 }) : "—"),
    },
    {
      id: "is_token_2022",
      label: "Type",
      sortable: true,
      value: (row) => (row.is_token_2022 ? "Token-2022" : "SPL"),
      render: (value, row) =>
        row.is_token_2022
          ? "<span class=\"wt-type-badge token2022\">Token-2022</span>"
          : "<span class=\"wt-type-badge spl\">SPL</span>",
    },
    {
      id: "decimals",
      label: "Decimals",
      sortable: true,
      render: (value) => (value != null ? value : "—"),
    },
    {
      id: "mint",
      label: "Mint",
      sortable: false,
      render: (value) => {
        if (!value) return "—";
        const escaped = Utils.escapeHtml(value);
        const short = `${value.slice(0, 6)}...${value.slice(-4)}`;
        const url = `https://solscan.io/token/${encodeURIComponent(value)}`;
        return `<div class="wt-mint-cell"><span class="wt-mint-addr">${short}</span><button type="button" class="copy-btn-mini" data-copy-mint="${escaped}" title="Copy mint"><i class="icon-copy"></i></button><a class="wt-mint-link" href="${url}" target="_blank" rel="noopener" title="View on Solscan"><i class="icon-external-link"></i></a></div>`;
      },
    },
  ];

  // Shared column shape for Secondaries/Archive — only the trailing Actions
  // column differs between the two tabs.
  const WALLET_LIST_COLUMNS_BASE = [
    {
      id: "name",
      label: "Name",
      sortable: true,
      className: "wallet-name-cell",
      render: (value) => Utils.escapeHtml(value || "—"),
    },
    {
      id: "address",
      label: "Address",
      sortable: false,
      render: (value, row) => _addressCellHtml(row.address),
    },
    {
      id: "balance",
      label: "Balance (SOL)",
      sortable: true,
      className: "wallet-balance-cell",
      render: (value) => (value != null ? Utils.formatSol(value, { decimals: 4 }) : "—"),
    },
    {
      id: "wallet_type",
      label: "Type",
      sortable: true,
      render: (value, row) =>
        `<span class="wallet-badge ${row.wallet_type}">${capitalizeFirst(row.wallet_type)}</span>`,
    },
    {
      id: "created_at",
      label: "Created",
      sortable: true,
      render: (value) => (value ? Utils.formatTimestamp(value, { variant: "short" }) : "—"),
    },
  ];

  const SECONDARY_COLUMNS = [
    ...WALLET_LIST_COLUMNS_BASE,
    {
      id: "actions",
      label: "Actions",
      type: "actions",
      sortable: false,
      actions: {
        buttons: [
          {
            id: "export",
            icon: '<i class="icon-key"></i>',
            tooltip: "Export private key",
            size: "sm",
            onClick: (row) => handleWalletAction("export", row.id),
          },
          {
            id: "archive",
            icon: '<i class="icon-archive"></i>',
            tooltip: "Archive wallet",
            size: "sm",
            onClick: (row) => handleWalletAction("archive", row.id),
          },
        ],
      },
    },
  ];

  const ARCHIVE_COLUMNS = [
    ...WALLET_LIST_COLUMNS_BASE,
    {
      id: "actions",
      label: "Actions",
      type: "actions",
      sortable: false,
      actions: {
        buttons: [
          {
            id: "restore",
            icon: '<i class="icon-archive-restore"></i>',
            tooltip: "Restore wallet",
            variant: "success",
            size: "sm",
            onClick: (row) => handleWalletAction("restore", row.id),
          },
          {
            id: "export",
            icon: '<i class="icon-key"></i>',
            tooltip: "Export private key",
            size: "sm",
            onClick: (row) => handleWalletAction("export", row.id),
          },
          {
            id: "delete",
            icon: '<i class="icon-trash-2"></i>',
            tooltip: "Delete permanently",
            variant: "danger",
            size: "sm",
            onClick: (row) => handleWalletAction("delete", row.id),
          },
        ],
      },
    },
  ];

  // =============================================================================
  // Main Render Function
  // =============================================================================

  function renderCurrentPanel() {
    const tab = currentTab();
    if (tab === "main") {
      renderMainWalletPanel();
    } else {
      // Clear toolbar inline slot and reset tracked wallet so the next visit
      // to the main tab performs a full structural render.
      const inline = document.querySelector("#wt-info-inline");
      if (inline) inline.innerHTML = "";
      lastRenderedWalletId = null;
      if (tab === "secondaries") renderSecondariesPanel();
      else if (tab === "archive") renderArchivePanel();
    }
  }

  // =============================================================================
  // Main Wallet Panel
  // =============================================================================

  function renderMainWalletPanel() {
    const wallets = walletsData();
    const mainWallet = wallets.find((w) => w.role === "main");
    const container = document.querySelector("#wt-info-inline");

    if (!container) return;

    if (!mainWallet) {
      if (lastRenderedWalletId !== null) {
        container.innerHTML = "<span class=\"wt-no-wallet\">No main wallet configured</span>";
        lastRenderedWalletId = null;
      }
      renderTokenHoldingsTable();
      return;
    }

    const balance =
      mainWallet.balance != null ? Utils.formatSol(mainWallet.balance, { decimals: 4 }) : "—";
    const lastUsed = mainWallet.last_used_at
      ? Utils.formatTimestamp(mainWallet.last_used_at, { variant: "relative" })
      : "Never";

    if (lastRenderedWalletId === mainWallet.id) {
      // Silent in-place update — only values that can change between polls
      const balanceEl = document.querySelector("#wt-balance-value");
      if (balanceEl) balanceEl.textContent = balance;
      const lastUsedEl = document.querySelector("#wt-last-used-value");
      if (lastUsedEl) lastUsedEl.textContent = lastUsed;
      // Token count is updated by renderTokenHoldingsTable via #wt-token-count
      renderTokenHoldingsTable();
      return;
    }

    // Full structural render — first time or wallet changed
    lastRenderedWalletId = mainWallet.id;

    const fullAddress = mainWallet.address || "—";
    const solscanUrl = mainWallet.address
      ? `https://solscan.io/account/${encodeURIComponent(mainWallet.address)}`
      : "#";

    container.innerHTML = `
      <div class="wt-info-identity">
        <span class="wt-info-name">${Utils.escapeHtml(mainWallet.name)}</span>
      </div>
      <div class="wt-info-divider"></div>
      <div class="wt-info-address-group">
        <code class="wt-info-address">${Utils.escapeHtml(fullAddress)}</code>
        <button type="button" class="copy-btn" data-address="${mainWallet.address}" title="Copy address">
          <i class="icon-copy"></i>
        </button>
        <a class="copy-btn" href="${solscanUrl}" target="_blank" rel="noopener" title="View on Solscan">
          <i class="icon-external-link"></i>
        </a>
      </div>
      <div class="wt-info-divider"></div>
      <div class="wt-info-stat">
        <span class="label">SOL Balance</span>
        <span class="value" id="wt-balance-value">${balance}</span>
      </div>
      <div class="wt-info-stat">
        <span class="label">Tokens</span>
        <span class="value" id="wt-token-count">—</span>
      </div>
      <div class="wt-info-stat">
        <span class="label">Last Used</span>
        <span class="value" id="wt-last-used-value">${lastUsed}</span>
      </div>
      <div class="wt-info-actions">
        <button type="button" class="btn" data-action="export" data-id="${mainWallet.id}">
          <i class="icon-key"></i> Export Key
        </button>
      </div>
    `;

    // Wire copy button on address (scope to data-address so the adjacent
    // Solscan link — which also carries .copy-btn for styling — is skipped).
    container.querySelectorAll(".copy-btn[data-address]").forEach((btn) => {
      on(btn, "click", (e) => {
        e.stopPropagation();
        Utils.copyToClipboard(btn.dataset.address);
        Utils.showToast("Address copied!", "success");
      });
    });

    // Wire export key action
    container.querySelectorAll("[data-action='export']").forEach((btn) => {
      on(btn, "click", () => handleWalletAction("export", btn.dataset.id));
    });

    // Render the DataTable for token holdings (first time creates it)
    renderTokenHoldingsTable();
  }

  // =============================================================================
  // Token Holdings DataTable
  // =============================================================================

  function renderTokenHoldingsTable() {
    const dtRoot = document.querySelector("#tokens-datatable-root");
    if (!dtRoot) return;

    const tokens = tokenHoldings();

    // Update token count in the info bar
    const countEl = document.querySelector("#wt-token-count");
    if (countEl) countEl.textContent = tokens.length;

    if (tokenTable) {
      // Silent data refresh — no DOM teardown, no visual flash, settings dialog stays open
      tokenTable.setData(tokens);
      tokenTable.updateToolbarSummary?.([{ id: "wt-tokens-count", value: String(tokens.length) }]);
      return;
    }

    // First-time creation only
    tokenTable = new DataTable({
      container: "#tokens-datatable-root",
      columns: TOKEN_COLUMNS,
      rowIdField: "mint",
      stateKey: "wallets.tokens-table",
      compact: true,
      stickyHeader: true,
      zebra: true,
      fitToContainer: true,
      sorting: {
        mode: "client",
        column: "ui_amount",
        direction: "desc",
      },
      toolbar: {
        summary: [{ id: "wt-tokens-count", label: "Tokens", value: "0", variant: "secondary" }],
        search: {
          enabled: true,
          mode: "client",
          placeholder: "Search by symbol or mint...",
        },
      },
    });

    tokenTable.setData(tokens);
    tokenTable.updateToolbarSummary?.([{ id: "wt-tokens-count", value: String(tokens.length) }]);

    // Event delegation for copy buttons inside DataTable cells — wired once
    tokenTableClickHandler = (e) => {
      const btn = e.target.closest("[data-copy-mint]");
      if (btn) {
        e.stopPropagation();
        Utils.copyToClipboard(btn.dataset.copyMint);
        Utils.showToast("Mint copied!", "success");
      }
    };
    dtRoot.addEventListener("click", tokenTableClickHandler);
  }

  // =============================================================================
  // Destroy tables (called from wallets.js cleanup / page dispose)
  // =============================================================================

  function destroyTables() {
    if (tokenTable) {
      tokenTable.destroy();
      tokenTable = null;
    }
    const dtRoot = document.querySelector("#tokens-datatable-root");
    if (dtRoot && tokenTableClickHandler) {
      dtRoot.removeEventListener("click", tokenTableClickHandler);
      tokenTableClickHandler = null;
    }

    if (secondariesTable) {
      secondariesTable.destroy();
      secondariesTable = null;
    }
    const secRoot = document.querySelector("#secondaries-table-container");
    if (secRoot && secondariesClickHandler) {
      secRoot.removeEventListener("click", secondariesClickHandler);
      secondariesClickHandler = null;
    }

    if (archiveTable) {
      archiveTable.destroy();
      archiveTable = null;
    }
    const archRoot = document.querySelector("#archive-table-container");
    if (archRoot && archiveClickHandler) {
      archRoot.removeEventListener("click", archiveClickHandler);
      archiveClickHandler = null;
    }

    lastRenderedWalletId = null;
  }

  // =============================================================================
  // Secondaries Panel
  // =============================================================================

  function renderSecondariesPanel() {
    const container = $("#secondaries-table-container");
    if (!container) return;

    const wallets = walletsData();
    const secondaryWallets = wallets.filter((w) => w.role === "secondary" && w.is_active);

    if (!secondariesTable) {
      secondariesTable = new DataTable({
        container: "#secondaries-table-container",
        columns: SECONDARY_COLUMNS,
        rowIdField: "id",
        stateKey: "wallets.secondaries-table",
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        sorting: { mode: "client", column: "created_at", direction: "desc" },
        emptyTitle: "No secondary wallets",
        emptyMessage:
          "Create additional wallets to organize your trading activities across multiple accounts.",
        toolbar: {
          summary: [{ id: "secondaries-count", label: "Wallets", value: "0", variant: "secondary" }],
          search: { enabled: true, mode: "client", placeholder: "Search by name or address..." },
        },
      });
      secondariesClickHandler = _wireAddressCopy(container);
    }

    secondariesTable.setData(secondaryWallets);
    secondariesTable.updateToolbarSummary?.([
      { id: "secondaries-count", value: String(secondaryWallets.length) },
    ]);
  }

  // =============================================================================
  // Archive Panel
  // =============================================================================

  function renderArchivePanel() {
    const container = $("#archive-table-container");
    if (!container) return;

    const wallets = walletsData();
    const archivedWallets = wallets.filter((w) => w.role === "archive" || !w.is_active);

    if (!archiveTable) {
      archiveTable = new DataTable({
        container: "#archive-table-container",
        columns: ARCHIVE_COLUMNS,
        rowIdField: "id",
        stateKey: "wallets.archive-table",
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        sorting: { mode: "client", column: "created_at", direction: "desc" },
        emptyTitle: "No archived wallets",
        emptyMessage: "Wallets you archive will be safely stored here for future reference.",
        toolbar: {
          summary: [{ id: "archive-count", label: "Wallets", value: "0", variant: "secondary" }],
          search: { enabled: true, mode: "client", placeholder: "Search by name or address..." },
        },
      });
      archiveClickHandler = _wireAddressCopy(container);
    }

    archiveTable.setData(archivedWallets);
    archiveTable.updateToolbarSummary?.([{ id: "archive-count", value: String(archivedWallets.length) }]);
  }

  // =============================================================================
  // Public API
  // =============================================================================

  return {
    renderCurrentPanel,
    renderMainWalletPanel,
    renderSecondariesPanel,
    renderArchivePanel,
    destroyTables,
  };
}
