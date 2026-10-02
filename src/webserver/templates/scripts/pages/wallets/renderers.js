// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Renderers Module for Wallets
 * Handles all rendering and display logic for wallet panels and data
 */

import { DataTable } from "../../ui/data_table.js";

// wallet_type ids serialized by `WalletType` (src/wallets/types.rs).
const WALLET_TYPE_LABELS = Object.freeze({
  generated: "wallets-type-generated",
  imported: "wallets-type-imported",
  migrated: "wallets-type-migrated",
});

export function createWalletRenderers({
  walletsData,
  tokenHoldings,
  currentTab,
  $,
  Utils,
  handleWalletAction,
  onRefresh,
  onAddWallet,
  onImportWallets,
  onExportWallets,
}) {
  // DataTable instances — created once, updated via setData() on every poll
  let tokenTable = null;
  let tokenTableClickHandler = null;
  let secondariesTable = null;
  let secondariesClickHandler = null;
  let archiveTable = null;
  let archiveClickHandler = null;

  function mainWallet() {
    return walletsData().find((w) => w.role === "main") || null;
  }

  function solscanAccountUrl(address) {
    return address ? `https://solscan.io/account/${encodeURIComponent(address)}` : "";
  }

  // Address cell shared by the Secondaries/Archive tables — same short-address +
  // copy + Solscan-link treatment as the main wallet's token Mint column.
  function _addressCellHtml(address) {
    if (!address) return "—";
    const escaped = Utils.escapeHtml(address);
    const short = `${address.slice(0, 6)}...${address.slice(-4)}`;
    const url = `https://solscan.io/account/${encodeURIComponent(address)}`;
    const copyTitle = Utils.escapeHtml(I18n.t("wallets-address-copy"));
    const linkTitle = Utils.escapeHtml(I18n.t("links-view-solscan"));
    return `<div class="wt-mint-cell"><span class="wt-mint-addr" dir="ltr">${short}</span><button type="button" class="copy-btn-mini" data-copy-address="${escaped}" title="${copyTitle}"><i class="icon-copy"></i></button><a class="wt-mint-link" href="${url}" target="_blank" rel="noopener" title="${linkTitle}"><i class="icon-external-link"></i></a></div>`;
  }

  // Delegated click handler for the address copy button — mirrors the mint-copy
  // wiring on the tokens table (one listener per table root, not per row).
  function _wireAddressCopy(rootEl) {
    const handler = (e) => {
      const btn = e.target.closest("[data-copy-address]");
      if (!btn) return;
      e.stopPropagation();
      Utils.copyToClipboard(btn.dataset.copyAddress);
      Utils.notifyCopied(I18n.t("wallets-copied-address"));
    };
    rootEl.addEventListener("click", handler);
    return handler;
  }

  // Column definitions are stable across renders — define once at closure level
  const TOKEN_COLUMNS = [
    {
      id: "symbol",
      label: I18n.t("wallets-holdings-col-token"),
      sortable: true,
      render: (value, row) => {
        const sym = Utils.escapeHtml(row.symbol || I18n.t("format-unknown"));
        const name = row.name ? Utils.escapeHtml(row.name) : null;
        const logo = row.logo_url || "";
        const logoHtml = logo
          ? `<img class="token-logo token-logo-artwork" src="${Utils.escapeHtml(logo)}" alt="${sym}" loading="lazy"/>`
          : '<i class="token-logo icon-coins"></i>';
        return `<div class="wt-token-cell">${logoHtml}<div class="wt-token-meta"><span class="wt-symbol">${sym}</span>${name ? `<span class="wt-name">${name}</span>` : ""}</div></div>`;
      },
    },
    {
      id: "ui_amount",
      label: I18n.t("wallets-holdings-col-balance"),
      sortable: true,
      render: (value) => (value != null ? Utils.formatNumber(value, { decimals: 4 }) : "—"),
    },
    {
      id: "value_sol",
      label: I18n.t("wallets-holdings-col-value"),
      sortable: true,
      render: (value) => (value != null ? Utils.formatSol(value, { decimals: 4 }) : "—"),
    },
    {
      id: "is_token_2022",
      label: I18n.t("wallets-holdings-col-type"),
      sortable: true,
      // l10n-ignore: token program standard names
      value: (row) => (row.is_token_2022 ? "Token-2022" : "SPL"),
      render: (value, row) =>
        row.is_token_2022
          ? // l10n-ignore: token program standard name
            '<span class="wt-type-badge token2022">Token-2022</span>'
          : // l10n-ignore: token program standard name
            '<span class="wt-type-badge spl">SPL</span>',
    },
    {
      id: "decimals",
      label: I18n.t("wallets-holdings-col-decimals"),
      sortable: true,
      render: (value) => (value != null ? value : "—"),
    },
    {
      id: "mint",
      label: I18n.t("wallets-holdings-col-mint"),
      sortable: false,
      render: (value) => {
        if (!value) return "—";
        const escaped = Utils.escapeHtml(value);
        const short = `${value.slice(0, 6)}...${value.slice(-4)}`;
        const url = `https://solscan.io/token/${encodeURIComponent(value)}`;
        const copyTitle = Utils.escapeHtml(I18n.attr("links-copy-mint", "title"));
        const linkTitle = Utils.escapeHtml(I18n.t("links-view-solscan"));
        return `<div class="wt-mint-cell"><span class="wt-mint-addr" dir="ltr">${short}</span><button type="button" class="copy-btn-mini" data-copy-mint="${escaped}" title="${copyTitle}"><i class="icon-copy"></i></button><a class="wt-mint-link" href="${url}" target="_blank" rel="noopener" title="${linkTitle}"><i class="icon-external-link"></i></a></div>`;
      },
    },
  ];

  // Shared column shape for Secondaries/Archive — only the trailing Actions
  // column differs between the two tabs.
  const WALLET_LIST_COLUMNS_BASE = [
    {
      id: "name",
      label: I18n.t("wallets-list-col-name"),
      sortable: true,
      className: "wallet-name-cell",
      render: (value) => Utils.escapeHtml(value || "—"),
    },
    {
      id: "address",
      label: I18n.t("wallets-field-address"),
      sortable: false,
      render: (value, row) => _addressCellHtml(row.address),
    },
    {
      id: "balance",
      label: I18n.t("wallets-list-col-balance"),
      sortable: true,
      className: "wallet-balance-cell",
      render: (value) => (value != null ? Utils.formatSol(value, { decimals: 4 }) : "—"),
    },
    {
      id: "wallet_type",
      label: I18n.t("wallets-list-col-type"),
      sortable: true,
      render: (value, row) =>
        `<span class="wallet-badge ${row.wallet_type}">${Utils.escapeHtml(I18n.label(WALLET_TYPE_LABELS, row.wallet_type))}</span>`,
    },
    {
      id: "created_at",
      label: I18n.t("wallets-list-col-created"),
      sortable: true,
      render: (value) => (value ? Utils.formatTimestamp(value, { variant: "short" }) : "—"),
    },
  ];

  const SECONDARY_COLUMNS = [
    ...WALLET_LIST_COLUMNS_BASE,
    {
      id: "actions",
      label: I18n.t("wallets-list-col-actions"),
      type: "actions",
      sortable: false,
      actions: {
        buttons: [
          {
            id: "export",
            icon: '<i class="icon-key"></i>',
            tooltip: I18n.t("wallets-list-action-export"),
            size: "sm",
            onClick: (row) => handleWalletAction("export", row.id),
          },
          {
            id: "archive",
            icon: '<i class="icon-archive"></i>',
            tooltip: I18n.t("wallets-list-action-archive"),
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
      label: I18n.t("wallets-list-col-actions"),
      type: "actions",
      sortable: false,
      actions: {
        buttons: [
          {
            id: "restore",
            icon: '<i class="icon-archive-restore"></i>',
            tooltip: I18n.t("wallets-list-action-restore"),
            variant: "success",
            size: "sm",
            onClick: (row) => handleWalletAction("restore", row.id),
          },
          {
            id: "export",
            icon: '<i class="icon-key"></i>',
            tooltip: I18n.t("wallets-list-action-export"),
            size: "sm",
            onClick: (row) => handleWalletAction("export", row.id),
          },
          {
            id: "delete",
            icon: '<i class="icon-trash-2"></i>',
            tooltip: I18n.t("wallets-list-action-delete"),
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

  function renderCurrentPanel({ loading = false } = {}) {
    const tab = currentTab();
    if (tab === "main") renderMainWalletPanel({ loading });
    else if (tab === "secondaries") renderSecondariesPanel({ loading });
    else if (tab === "archive") renderArchivePanel({ loading });
  }

  function syncLoadingState(table, loading) {
    if (!table) return;
    if (loading) {
      table.showBlockingState?.({
        variant: "loading",
        title: I18n.t("wallets-list-loading-title"),
        description: I18n.t("wallets-list-loading-description"),
      });
    } else {
      table.hideBlockingState?.();
    }
  }

  // =============================================================================
  // Main Wallet Panel
  // =============================================================================

  // The main wallet IS the subject of its token-holdings table, so its name,
  // address, balances and actions live in that table's toolbar identity/stats —
  // there is no separate wallet info bar above the table.
  function renderMainWalletPanel(options = {}) {
    renderTokenHoldingsTable(options);
  }

  // =============================================================================
  // Token Holdings DataTable
  // =============================================================================

  // In-place toolbar refresh on every poll — identity and stats only, so an open
  // search box, menu or settings dialog is never torn down.
  function syncMainWalletToolbar(tokens) {
    if (!tokenTable) return;
    const wallet = mainWallet();

    tokenTable.setToolbarIdentity({
      title: wallet?.name || I18n.t("wallets-holdings-no-main"),
      tag: wallet ? I18n.t("wallets-holdings-main-tag") : "",
      address: {
        value: wallet?.address || "—",
        href: solscanAccountUrl(wallet?.address) || "#",
      },
    });

    tokenTable.updateToolbarSummary([
      {
        id: "wt-sol-balance",
        value: wallet?.balance != null ? Utils.formatSol(wallet.balance, { decimals: 4 }) : "—",
      },
      { id: "wt-tokens-count", value: String(tokens.length) },
      {
        id: "wt-last-used",
        value: wallet?.last_used_at
          ? Utils.formatTimestamp(wallet.last_used_at, { variant: "relative" })
          : I18n.t("wallets-holdings-never"),
      },
    ]);

    tokenTable.setToolbarItem("wt-export-key", { hidden: !wallet });
  }

  function renderTokenHoldingsTable({ loading = false } = {}) {
    const dtRoot = document.querySelector("#tokens-datatable-root");
    if (!dtRoot) return;

    const tokens = tokenHoldings();
    const wallet = mainWallet();

    if (tokenTable) {
      // Silent data refresh — no DOM teardown, no visual flash, settings dialog stays open
      tokenTable.setData(tokens);
      syncMainWalletToolbar(tokens);
      syncLoadingState(tokenTable, loading);
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
      emptyTitle: I18n.t("wallets-holdings-empty-title"),
      emptyMessage: I18n.t("wallets-holdings-empty-message"),
      toolbar: {
        // Identity nodes are always rendered (even before the wallet loads) so
        // `setToolbarIdentity` can fill them in place on the first poll.
        identity: {
          icon: "icon-wallet",
          title: wallet?.name || I18n.t("wallets-holdings-main-title"),
          tag: I18n.t("wallets-holdings-main-tag"),
          address: {
            value: wallet?.address || "—",
            href: solscanAccountUrl(wallet?.address) || "#",
            linkTooltip: I18n.t("links-view-solscan"),
          },
        },
        summary: [
          { id: "wt-sol-balance", label: I18n.t("wallets-summary-sol"), value: "—" },
          {
            id: "wt-tokens-count",
            label: I18n.t("wallets-holdings-tokens"),
            value: "0",
            variant: "secondary",
          },
          {
            id: "wt-last-used",
            label: I18n.t("wallets-holdings-last-used"),
            value: "—",
            variant: "secondary",
          },
        ],
        search: {
          enabled: true,
          mode: "client",
          placeholder: I18n.attr("wallets-holdings-search", "placeholder"),
        },
        buttons: [
          {
            id: "wt-export-key",
            label: I18n.t("wallets-holdings-export"),
            icon: "icon-key",
            tooltip: I18n.t("wallets-holdings-export-tooltip"),
            onClick: () => {
              const current = mainWallet();
              if (current) handleWalletAction("export", current.id);
            },
          },
          {
            id: "wt-refresh",
            icon: "icon-refresh-cw",
            tooltip: I18n.t("common-action-refresh"),
            onClick: (btn) => onRefresh?.(btn),
          },
        ],
      },
    });

    tokenTable.setData(tokens);
    syncMainWalletToolbar(tokens);
    syncLoadingState(tokenTable, loading);

    // Event delegation for copy buttons inside DataTable cells — wired once
    tokenTableClickHandler = (e) => {
      const btn = e.target.closest("[data-copy-mint]");
      if (btn) {
        e.stopPropagation();
        Utils.copyToClipboard(btn.dataset.copyMint);
        Utils.notifyCopied(I18n.t("wallets-copied-mint"));
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
  }

  // =============================================================================
  // Secondaries Panel
  // =============================================================================

  function renderSecondariesPanel({ loading = false } = {}) {
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
        emptyTitle: I18n.t("wallets-secondaries-empty-title"),
        emptyMessage: I18n.t("wallets-secondaries-empty-message"),
        toolbar: {
          summary: [
            {
              id: "secondaries-count",
              label: I18n.t("wallets-list-count"),
              value: "0",
              variant: "secondary",
            },
          ],
          search: {
            enabled: true,
            mode: "client",
            placeholder: I18n.attr("wallets-list-search", "placeholder"),
          },
          buttons: [
            {
              id: "secondaries-add",
              label: I18n.t("wallets-secondaries-add"),
              icon: "icon-plus",
              variant: "primary",
              onClick: () => onAddWallet?.(),
            },
            // Bulk transfers are secondary to the primary create action, so they
            // collapse into the overflow menu rather than widening the bar.
            {
              id: "secondaries-import",
              label: I18n.t("common-action-import"),
              icon: "icon-upload",
              overflow: true,
              onClick: () => onImportWallets?.(),
            },
            {
              id: "secondaries-export",
              label: I18n.t("common-action-export"),
              icon: "icon-download",
              overflow: true,
              onClick: () => onExportWallets?.(),
            },
            {
              id: "secondaries-refresh",
              icon: "icon-refresh-cw",
              tooltip: I18n.t("common-action-refresh"),
              onClick: (btn) => onRefresh?.(btn),
            },
          ],
        },
      });
      secondariesClickHandler = _wireAddressCopy(container);
    }

    secondariesTable.setData(secondaryWallets);
    syncLoadingState(secondariesTable, loading);
    secondariesTable.updateToolbarSummary?.([
      { id: "secondaries-count", value: String(secondaryWallets.length) },
    ]);
  }

  // =============================================================================
  // Archive Panel
  // =============================================================================

  function renderArchivePanel({ loading = false } = {}) {
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
        emptyTitle: I18n.t("wallets-archive-empty-title"),
        emptyMessage: I18n.t("wallets-archive-empty-message"),
        toolbar: {
          summary: [
            {
              id: "archive-count",
              label: I18n.t("wallets-list-count"),
              value: "0",
              variant: "secondary",
            },
          ],
          search: {
            enabled: true,
            mode: "client",
            placeholder: I18n.attr("wallets-list-search", "placeholder"),
          },
          buttons: [
            {
              id: "archive-refresh",
              icon: "icon-refresh-cw",
              tooltip: I18n.t("common-action-refresh"),
              onClick: (btn) => onRefresh?.(btn),
            },
          ],
        },
      });
      archiveClickHandler = _wireAddressCopy(container);
    }

    archiveTable.setData(archivedWallets);
    syncLoadingState(archiveTable, loading);
    archiveTable.updateToolbarSummary?.([
      { id: "archive-count", value: String(archivedWallets.length) },
    ]);
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
