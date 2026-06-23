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
  // DataTable instance — created once, updated via setData() on every poll
  let tokenTable = null;
  let tokenTableClickHandler = null;

  // Track which wallet is currently rendered in the toolbar so we can do
  // fast in-place updates instead of rebuilding the entire info bar each poll.
  let lastRenderedWalletId = null;

  // Fingerprints for secondaries/archive tables — skip re-render when data unchanged
  let lastSecondariesHash = null;
  let lastArchiveHash = null;

  // Build a compact fingerprint for a wallet row covering all rendered fields
  function _walletHash(w) {
    return `${w.id}|${w.name}|${w.address}|${w.balance}|${w.wallet_type}|${w.role}|${w.is_active}|${w.created_at}`;
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
        return `<div class="wt-token-cell"><span class="wt-symbol">${sym}</span>${name ? `<span class="wt-name">${name}</span>` : ""}</div>`;
      },
    },
    {
      id: "ui_amount",
      label: "Balance",
      sortable: true,
      render: (value) => (value != null ? Utils.formatNumber(value, { decimals: 4 }) : "—"),
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
        return `<div class="wt-mint-cell"><span class="wt-mint-addr">${short}</span><button type="button" class="copy-btn-mini" data-copy-mint="${escaped}" data-tooltip="Copy mint"><i class="icon-copy"></i></button><a class="wt-mint-link" href="${url}" target="_blank" rel="noopener" data-tooltip="View on Solscan"><i class="icon-external-link"></i></a></div>`;
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

    const shortAddress = mainWallet.address
      ? `${mainWallet.address.slice(0, 6)}...${mainWallet.address.slice(-4)}`
      : "—";
    const typeLabel = capitalizeFirst(mainWallet.wallet_type || "");

    container.innerHTML = `
      <div class="wt-info-identity">
        <span class="wt-info-name">${Utils.escapeHtml(mainWallet.name)}</span>
        <span class="wallet-badge main"><i class="icon-star"></i> Main</span>
        <span class="wallet-badge ${mainWallet.wallet_type}">${typeLabel}</span>
      </div>
      <div class="wt-info-divider"></div>
      <div class="wt-info-address-group">
        <code class="wt-info-address">${shortAddress}</code>
        <button type="button" class="copy-btn" data-address="${mainWallet.address}" data-tooltip="Copy address">
          <i class="icon-copy"></i>
        </button>
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

    // Wire copy button on address
    container.querySelectorAll(".copy-btn").forEach((btn) => {
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
  // Destroy token table (called from wallets.js cleanup / page dispose)
  // =============================================================================

  function destroyTokenTable() {
    if (tokenTable) {
      tokenTable.destroy();
      tokenTable = null;
    }
    const dtRoot = document.querySelector("#tokens-datatable-root");
    if (dtRoot && tokenTableClickHandler) {
      dtRoot.removeEventListener("click", tokenTableClickHandler);
      tokenTableClickHandler = null;
    }
    lastRenderedWalletId = null;
    lastSecondariesHash = null;
    lastArchiveHash = null;
  }

  // =============================================================================
  // Secondaries Panel
  // =============================================================================

  function renderSecondariesPanel() {
    const container = $("#secondaries-table-container");
    if (!container) return;

    const wallets = walletsData();
    const secondaryWallets = wallets.filter((w) => w.role === "secondary" && w.is_active);

    const hash = secondaryWallets.map(_walletHash).join(";");
    if (hash === lastSecondariesHash) return;
    lastSecondariesHash = hash;

    if (secondaryWallets.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-state-icon">
            <i class="icon-wallet"></i>
          </div>
          <h4 class="empty-state-title">No secondary wallets</h4>
          <p class="empty-state-description">Create additional wallets to organize your trading activities across multiple accounts</p>
          <div class="empty-state-action">
            <button type="button" class="btn primary" id="add-wallet-empty-btn">
              <i class="icon-plus"></i> Add Wallet
            </button>
          </div>
          <div class="empty-state-hint">
            <i class="icon-lightbulb"></i>
            <span>Tip: Use separate wallets for different trading strategies</span>
          </div>
        </div>
      `;
      const emptyBtn = container.querySelector("#add-wallet-empty-btn");
      if (emptyBtn) {
        emptyBtn.addEventListener("click", () => {
          const modal = $("#add-wallet-modal");
          if (modal) modal.classList.remove("hidden");
        });
      }
      return;
    }

    const rows = secondaryWallets
      .map((wallet) => {
        const balance =
          wallet.balance != null ? Utils.formatSol(wallet.balance, { decimals: 4 }) : "—";
        const shortAddress = `${wallet.address.slice(0, 6)}...${wallet.address.slice(-4)}`;
        const created = Utils.formatTimestamp(wallet.created_at, { variant: "short" });

        return `
          <tr data-id="${wallet.id}">
            <td class="wallet-name-cell">${Utils.escapeHtml(wallet.name)}</td>
            <td class="wallet-address-cell">
              <code>${shortAddress}</code>
              <button type="button" class="copy-btn-mini" data-address="${wallet.address}" data-tooltip="Copy address">
                <i class="icon-copy"></i>
              </button>
            </td>
            <td class="wallet-balance-cell">${balance}</td>
            <td class="wallet-type-cell">
              <span class="wallet-badge ${wallet.wallet_type}">${capitalizeFirst(wallet.wallet_type)}</span>
            </td>
            <td class="wallet-created-cell">${created}</td>
            <td class="wallet-actions-cell">
              <div class="table-actions">
                <button type="button" class="btn btn-sm" data-action="set-main" data-id="${wallet.id}" data-tooltip="Set as main wallet">
                  <i class="icon-star"></i>
                </button>
                <button type="button" class="btn btn-sm" data-action="export" data-id="${wallet.id}" data-tooltip="Export private key">
                  <i class="icon-key"></i>
                </button>
                <button type="button" class="btn btn-sm" data-action="archive" data-id="${wallet.id}" data-tooltip="Archive wallet">
                  <i class="icon-archive"></i>
                </button>
              </div>
            </td>
          </tr>
        `;
      })
      .join("");

    container.innerHTML = `
      <table class="wallets-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Address</th>
            <th>Balance</th>
            <th>Type</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          ${rows}
        </tbody>
      </table>
    `;

    wireTableActions(container);
  }

  // =============================================================================
  // Archive Panel
  // =============================================================================

  function renderArchivePanel() {
    const container = $("#archive-table-container");
    if (!container) return;

    const wallets = walletsData();
    const archivedWallets = wallets.filter((w) => w.role === "archive" || !w.is_active);

    const hash = archivedWallets.map(_walletHash).join(";");
    if (hash === lastArchiveHash) return;
    lastArchiveHash = hash;

    if (archivedWallets.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-state-icon">
            <i class="icon-archive"></i>
          </div>
          <h4 class="empty-state-title">No archived wallets</h4>
          <p class="empty-state-description">Wallets you archive will be safely stored here for future reference</p>
          <div class="empty-state-hint">
            <i class="icon-info"></i>
            <span>Archived wallets retain their keys and can be restored anytime</span>
          </div>
        </div>
      `;
      return;
    }

    const rows = archivedWallets
      .map((wallet) => {
        const balance =
          wallet.balance != null ? Utils.formatSol(wallet.balance, { decimals: 4 }) : "—";
        const shortAddress = `${wallet.address.slice(0, 6)}...${wallet.address.slice(-4)}`;
        const created = Utils.formatTimestamp(wallet.created_at, { variant: "short" });

        return `
          <tr data-id="${wallet.id}">
            <td class="wallet-name-cell">${Utils.escapeHtml(wallet.name)}</td>
            <td class="wallet-address-cell">
              <code>${shortAddress}</code>
              <button type="button" class="copy-btn-mini" data-address="${wallet.address}" data-tooltip="Copy address">
                <i class="icon-copy"></i>
              </button>
            </td>
            <td class="wallet-balance-cell">${balance}</td>
            <td class="wallet-type-cell">
              <span class="wallet-badge ${wallet.wallet_type}">${capitalizeFirst(wallet.wallet_type)}</span>
            </td>
            <td class="wallet-created-cell">${created}</td>
            <td class="wallet-actions-cell">
              <div class="table-actions">
                <button type="button" class="btn btn-sm success" data-action="restore" data-id="${wallet.id}" data-tooltip="Restore wallet">
                  <i class="icon-archive-restore"></i>
                </button>
                <button type="button" class="btn btn-sm" data-action="export" data-id="${wallet.id}" data-tooltip="Export private key">
                  <i class="icon-key"></i>
                </button>
                <button type="button" class="btn btn-sm danger" data-action="delete" data-id="${wallet.id}" data-tooltip="Delete permanently">
                  <i class="icon-trash-2"></i>
                </button>
              </div>
            </td>
          </tr>
        `;
      })
      .join("");

    container.innerHTML = `
      <table class="wallets-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Address</th>
            <th>Balance</th>
            <th>Type</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          ${rows}
        </tbody>
      </table>
    `;

    wireTableActions(container);
  }

  // =============================================================================
  // Wire Table Actions
  // =============================================================================

  function wireTableActions(container) {
    container.querySelectorAll(".copy-btn-mini").forEach((btn) => {
      on(btn, "click", (e) => {
        e.stopPropagation();
        Utils.copyToClipboard(btn.dataset.address);
        Utils.showToast("Address copied!", "success");
      });
    });

    container.querySelectorAll("[data-action]").forEach((btn) => {
      on(btn, "click", (e) => {
        e.stopPropagation();
        handleWalletAction(btn.dataset.action, btn.dataset.id);
      });
    });
  }

  // =============================================================================
  // Public API
  // =============================================================================

  return {
    renderCurrentPanel,
    renderMainWalletPanel,
    renderSecondariesPanel,
    renderArchivePanel,
    destroyTokenTable,
  };
}
