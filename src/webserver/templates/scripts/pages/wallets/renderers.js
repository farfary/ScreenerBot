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
import { needsFunding } from "../../ui/funding_notice.js";
import { capabilityAvailable, capabilityReason } from "../../ui/access_gate.js";

// wallet_type ids serialized by `WalletType` (src/wallets/types.rs).
/** The chain capability that reads the balances of wallets other than the trading one. */
const OTHER_BALANCES_CAPABILITY = "other_wallet_balances";

const WALLET_TYPE_LABELS = Object.freeze({
  generated: "wallets-type-generated",
  imported: "wallets-type-imported",
  migrated: "wallets-type-migrated",
});

// The name the vault stores on its trading wallet: `TRADING_WALLET_NAME` in
// src/wallets/types.rs. A row that keeps it after another wallet took the role is a
// replaced trading wallet.
export const TRADING_WALLET_NAME = "Main Wallet";

// The label of a wallet row: the trading wallet by its role, a replaced trading wallet
// as one, a wallet without a name of its own by its type, any other by its name.
export function walletName(wallet) {
  if (!wallet) return "";
  if (wallet.role === "main") return I18n.t("wallets-holdings-main-title");
  if (wallet.name === TRADING_WALLET_NAME) return I18n.t("wallets-list-previous-trading-wallet");
  if (!wallet.name) return I18n.label(WALLET_TYPE_LABELS, wallet.wallet_type);
  return wallet.name;
}

export function createWalletRenderers({
  walletsData,
  walletBackups,
  tokenHoldings,
  holdingsReadError,
  currentTab,
  $,
  Utils,
  handleWalletAction,
  onRefresh,
}) {
  // DataTable instances — created once, updated via setData() on every poll
  let tokenTable = null;
  let otherWalletsTable = null;
  // The read failure and funding state the holdings empty state last showed.
  let renderedEmptyState = null;

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

  // The newest backup a replacement of `address` as trading wallet moved its stores to.
  function replacementBackup(address) {
    return (
      walletBackups().find(
        (backup) => backup.reason === "wallet_replaced" && backup.previous_wallet === address
      ) || null
    );
  }

  function walletRows(wallets) {
    return wallets.map((w) => ({
      ...w,
      wallet: `${walletName(w)} ${w.address || ""}`,
      replaced_at: replacementBackup(w.address)?.created_at_ms ?? null,
    }));
  }

  function renderReplacedCell(row) {
    const backup = replacementBackup(row.address);
    if (!backup) return "—";
    const date = Utils.formatTimestamp(new Date(backup.created_at_ms).toISOString(), {
      includeYear: false,
      includeSeconds: false,
      fallback: "—",
    });
    const path = Utils.escapeHtml(backup.directory);
    return `<div class="wallet-replaced">
        <span>${Utils.escapeHtml(I18n.t("wallets-list-replaced-on", { date }))}</span>
        <span class="wallet-replaced-path" title="${path}">${path}</span>
      </div>`;
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
        value != null
          ? Utils.formatSol(value, { decimals: 4, suffix: "", trim: false, subscript: true })
          : "—",
    },
    {
      id: "is_token_2022",
      label: I18n.t("wallets-holdings-col-type"),
      sortable: true,
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

  // Other Wallets lists every wallet besides the trading wallet: replaced trading
  // wallets, keys a replacement added and wallets from earlier versions. Its one action
  // is the private-key export.
  const WALLET_LIST_COLUMNS = [
    {
      id: "wallet",
      label: I18n.t("wallets-list-col-name"),
      grow: true,
      sortable: true,
      minWidth: addressFloorWidth() + 24,
      render: (value, row) => renderNamedAddress(walletName(row), row.address),
    },
    {
      id: "balance",
      label: I18n.t("wallets-list-col-balance"),
      type: "sol",
      sortable: true,
      render: (value, row) => {
        if (value != null) {
          return Utils.formatSol(value, { decimals: 4, suffix: "", trim: false, subscript: true });
        }
        // Another wallet's balance is unread without the capability: say why.
        const reason =
          row.role === "main" || capabilityAvailable(OTHER_BALANCES_CAPABILITY)
            ? ""
            : capabilityReason(OTHER_BALANCES_CAPABILITY);
        return reason ? `<span title="${Utils.escapeHtml(reason)}">—</span>` : "—";
      },
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
      minWidth: "content",
      wrap: false,
      render: (value) =>
        Utils.formatTimestamp(value, { includeYear: false, includeSeconds: false, fallback: "—" }),
    },
    {
      id: "replaced_at",
      label: I18n.t("wallets-list-col-replaced"),
      sortable: true,
      render: (value, row) => renderReplacedCell(row),
    },
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
    else if (tab === "other") renderOtherWalletsPanel({ loading });
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

  // A main wallet known to hold no SOL cannot trade. The table says so itself: its empty
  // state when nothing is held, and the SOL figure's tone and tooltip when tokens are.
  function walletNeedsFunding() {
    const wallet = mainWallet();
    return Boolean(wallet?.address) && needsFunding(wallet.balance);
  }

  // What the holdings empty state shows; a change repaints it even when the rows are unchanged.
  function emptyStateKey() {
    return `${holdingsReadError() || ""}|${walletNeedsFunding()}`;
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
      title: wallet ? walletName(wallet) : I18n.t("wallets-holdings-no-main"),
      address: { html: walletAddressHtml(wallet) },
    });

    const funding = walletNeedsFunding();
    tokenTable.updateToolbarSummary([
      {
        id: "wt-sol-balance",
        // The chip label names the asset, so the value carries no unit.
        value:
          wallet?.balance != null
            ? Utils.formatSol(wallet.balance, { decimals: 4, suffix: "" })
            : "—",
        variant: funding ? "warning" : "",
        tooltip: funding ? I18n.t("wallets-funding-message") : "",
      },
      { id: "wt-tokens-count", value: holdingsReadError() ? "—" : String(tokens.length) },
      {
        id: "wt-last-used",
        value: wallet?.last_used_at
          ? Utils.formatTimeAgo(wallet.last_used_at)
          : I18n.t("wallets-holdings-never"),
      },
    ]);

    tokenTable.setToolbarItem("wt-export-key", { hidden: !wallet });
    tokenTable.setToolbarItem("wt-replace", { hidden: !wallet });
  }

  function renderTokenHoldingsTable({ loading = false } = {}) {
    const dtRoot = document.querySelector("#tokens-datatable-root");
    if (!dtRoot) return;

    const tokens = tokenHoldings();
    const wallet = mainWallet();

    if (tokenTable) {
      // Silent data refresh — no DOM teardown, no visual flash, settings dialog stays open.
      const emptyState = emptyStateKey();
      tokenTable.setData(tokenRows(tokens), { forceRender: emptyState !== renderedEmptyState });
      renderedEmptyState = emptyState;
      syncMainWalletToolbar(tokens);
      syncLoadingState(tokenTable, loading);
      return;
    }

    // First-time creation only
    tokenTable = new DataTable({
      container: "#tokens-datatable-root",
      columns: TOKEN_COLUMNS,
      uniformRowHeight: 2,
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
      // Unknown holdings name the failed read instead of reading as an empty wallet; a wallet
      // without SOL says how to fund it where its holdings would be.
      emptyIcon: () => (!holdingsReadError() && walletNeedsFunding() ? "icon-wallet" : null),
      emptyTitle: () => {
        if (holdingsReadError()) return I18n.t("wallets-holdings-unreadable-title");
        return walletNeedsFunding()
          ? I18n.t("wallets-funding-title")
          : I18n.t("wallets-holdings-empty-title");
      },
      emptyMessage: () => {
        if (holdingsReadError()) return holdingsReadError();
        return walletNeedsFunding()
          ? I18n.t("wallets-funding-message")
          : I18n.t("wallets-holdings-empty-message");
      },
      toolbar: {
        // Identity nodes are always rendered (even before the wallet loads) so
        // `setToolbarIdentity` can fill them in place on the first poll.
        identity: {
          icon: "icon-wallet",
          title: I18n.t("wallets-holdings-main-title"),
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
            id: "wt-replace",
            icon: "icon-replace",
            tooltip: I18n.t("wallets-holdings-replace-tooltip"),
            onClick: () => handleWalletAction("replace"),
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

    renderedEmptyState = emptyStateKey();
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

    if (otherWalletsTable) {
      otherWalletsTable.destroy();
      otherWalletsTable = null;
    }
  }

  // =============================================================================
  // Other Wallets Panel
  // =============================================================================

  function renderOtherWalletsPanel({ loading = false } = {}) {
    const container = $("#other-wallets-table-container");
    if (!container) return;

    const trading = mainWallet();
    const otherWallets = walletsData().filter((w) => w.id !== trading?.id);

    if (!otherWalletsTable) {
      otherWalletsTable = new DataTable({
        container: "#other-wallets-table-container",
        columns: WALLET_LIST_COLUMNS,
        uniformRowHeight: 2,
        rowIdField: "id",
        stateKey: "wallets.other-table",
        compact: true,
        stickyHeader: true,
        zebra: true,
        fitToContainer: true,
        sorting: { mode: "client", column: "created_at", direction: "desc" },
        emptyTitle: I18n.t("wallets-other-empty-title"),
        emptyMessage: I18n.t("wallets-list-kept-empty-message"),
        toolbar: {
          summary: [
            {
              id: "other-count",
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
              id: "other-refresh",
              icon: "icon-refresh-cw",
              tooltip: I18n.t("common-action-refresh"),
              onClick: (btn) => onRefresh?.(btn),
            },
          ],
        },
      });
    }

    otherWalletsTable.setData(walletRows(otherWallets));
    syncLoadingState(otherWalletsTable, loading);
    otherWalletsTable.updateToolbarSummary?.([
      { id: "other-count", value: String(otherWallets.length) },
    ]);
  }

  // =============================================================================
  // Public API
  // =============================================================================

  return {
    renderCurrentPanel,
    renderMainWalletPanel,
    renderOtherWalletsPanel,
    destroyTables,
  };
}
