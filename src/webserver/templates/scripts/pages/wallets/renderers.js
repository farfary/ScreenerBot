// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Renderers Module for Wallets
 * Handles all rendering and display logic for wallet panels and data
 */

import { DataTable } from "../../ui/data_table.js";
import {
  addressFloorWidth,
  renderAddress,
  renderNamedAddress,
  renderTokenCell,
  TOKEN_CELL_MIN_WIDTH,
} from "../../ui/token_identity.js";

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
  let secondariesTable = null;
  let archiveTable = null;

  function mainWallet() {
    return walletsData().find((w) => w.role === "main") || null;
  }

  // DataTable sorts and searches `row[column.id]`. The token and wallet cells show
  // a name with the full address beneath it, so their key carries both: sorting
  // follows the name and the table search still finds a pasted address.
  function tokenRows(tokens) {
    return tokens.map((t) => ({ ...t, token: `${t.symbol || ""} ${t.name || ""} ${t.mint}` }));
  }

  function walletAddressHtml(wallet) {
    return wallet?.address ? renderAddress(wallet.address, { explorer: "account" }) : "";
  }

  function walletRows(wallets) {
    return wallets.map((w) => ({ ...w, wallet: `${w.name || ""} ${w.address || ""}` }));
  }

  // Column definitions are stable across renders — define once at closure level
  const TOKEN_COLUMNS = [
    {
      id: "token",
      label: I18n.t("wallets-holdings-col-token"),
      grow: true,
      sortable: true,
      minWidth: TOKEN_CELL_MIN_WIDTH,
      render: (value, row) =>
        renderTokenCell(row.mint, { symbol: row.symbol, name: row.name, logoUrl: row.logo_url }),
    },
    {
      id: "ui_amount",
      label: I18n.t("wallets-holdings-col-balance"),
      type: "number",
      sortable: true,
      // The token's own precision, without padding a whole or short amount with zeros.
      render: (value, row) =>
        Utils.formatNumber(value, { decimals: 0, maxDecimals: row?.decimals ?? 9, fallback: "—" }),
    },
    {
      id: "value_sol",
      label: I18n.t("wallets-holdings-col-value"),
      type: "sol",
      sortable: true,
      render: (value) =>
        value != null ? Utils.formatSol(value, { decimals: 4, suffix: "", trim: false }) : "—",
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
      type: "number",
      sortable: true,
      render: (value) => (value != null ? value : "—"),
    },
  ];

  // Shared column shape for Secondaries/Archive — only the trailing Actions
  // column differs between the two tabs.
  const WALLET_LIST_COLUMNS_BASE = [
    {
      id: "wallet",
      label: I18n.t("wallets-list-col-name"),
      grow: true,
      sortable: true,
      minWidth: addressFloorWidth() + 24,
      render: (value, row) => renderNamedAddress(row.name, row.address),
    },
    {
      id: "balance",
      label: I18n.t("wallets-list-col-balance"),
      type: "sol",
      sortable: true,
      render: (value) =>
        value != null ? Utils.formatSol(value, { decimals: 4, suffix: "", trim: false }) : "—",
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
      render: (value) =>
        Utils.formatTimestamp(value, { includeYear: false, includeSeconds: false, fallback: "—" }),
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
      address: { html: walletAddressHtml(wallet) },
    });

    tokenTable.updateToolbarSummary([
      {
        id: "wt-sol-balance",
        // The chip label names the asset, so the value carries no unit.
        value:
          wallet?.balance != null
            ? Utils.formatSol(wallet.balance, { decimals: 4, suffix: "" })
            : "—",
      },
      { id: "wt-tokens-count", value: String(tokens.length) },
      {
        id: "wt-last-used",
        value: wallet?.last_used_at
          ? Utils.formatTimeAgo(wallet.last_used_at)
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
      tokenTable.setData(tokenRows(tokens));
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
          address: { html: walletAddressHtml(wallet) },
        },
        summary: [
          { id: "wt-sol-balance", label: I18n.t("wallets-summary-native"), value: "—" },
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

    tokenTable.setData(tokenRows(tokens));
    syncMainWalletToolbar(tokens);
    syncLoadingState(tokenTable, loading);
  }

  // =============================================================================
  // Destroy tables (called from wallets.js cleanup / page dispose)
  // =============================================================================

  function destroyTables() {
    if (tokenTable) {
      tokenTable.destroy();
      tokenTable = null;
    }

    if (secondariesTable) {
      secondariesTable.destroy();
      secondariesTable = null;
    }

    if (archiveTable) {
      archiveTable.destroy();
      archiveTable = null;
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
    }

    secondariesTable.setData(walletRows(secondaryWallets));
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
    }

    archiveTable.setData(walletRows(archivedWallets));
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
