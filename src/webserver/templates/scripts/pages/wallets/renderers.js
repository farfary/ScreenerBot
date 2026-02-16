/**
 * Renderers Module for Wallets
 * Handles all rendering and display logic for wallet panels and data
 */

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
  // =============================================================================
  // Main Render Function
  // =============================================================================

  function renderCurrentPanel() {
    const tab = currentTab();
    if (tab === "main") {
      renderMainWalletPanel();
    } else if (tab === "secondaries") {
      renderSecondariesPanel();
    } else if (tab === "archive") {
      renderArchivePanel();
    }
  }

  // =============================================================================
  // Stats Update
  // =============================================================================

  function updateStats() {
    const wallets = walletsData();
    const tokens = tokenHoldings();
    const mainWallet = wallets.find((w) => w.role === "main");
    const activeWallets = wallets.filter((w) => w.is_active);
    const secondaryWallets = activeWallets.filter((w) => w.role === "secondary");

    // SOL Balance
    const solBalanceEl = $("#stat-sol-balance");
    if (solBalanceEl) {
      solBalanceEl.textContent =
        mainWallet?.balance != null ? Utils.formatSol(mainWallet.balance, { decimals: 4 }) : "—";
    }

    // Token Count
    const tokenCountEl = $("#stat-token-count");
    if (tokenCountEl) {
      tokenCountEl.textContent = tokens.length;
    }

    // Secondary Count
    const secondaryCountEl = $("#stat-secondary-count");
    if (secondaryCountEl) {
      secondaryCountEl.textContent = secondaryWallets.length;
    }

    // Last Activity
    const lastActivityEl = $("#stat-last-activity");
    if (lastActivityEl) {
      const lastUsed = mainWallet?.last_used_at;
      lastActivityEl.textContent = lastUsed
        ? Utils.formatTimestamp(lastUsed, { variant: "relative" })
        : "—";
    }
  }

  // =============================================================================
  // Main Wallet Panel
  // =============================================================================

  function renderMainWalletPanel() {
    const wallets = walletsData();
    const mainWallet = wallets.find((w) => w.role === "main");
    const container = $("#main-wallet-card");

    if (!container) return;

    if (!mainWallet) {
      container.innerHTML = `
        <div class="empty-state">
          <i class="icon-wallet"></i>
          <p>No main wallet configured</p>
          <small>Click "Add Wallet" to create or import your first wallet</small>
        </div>
      `;
      return;
    }

    container.innerHTML = renderMainWalletCard(mainWallet);
    wireMainWalletActions(container);

    // Render token holdings
    renderTokenHoldings();
  }

  function renderMainWalletCard(wallet) {
    const balance = wallet.balance ?? null;
    const balanceDisplay = balance !== null ? Utils.formatSol(balance, { decimals: 4 }) : "—";
    const lastUsed = wallet.last_used_at
      ? Utils.formatTimestamp(wallet.last_used_at, { variant: "relative" })
      : "Never";
    const typeBadge = `<span class="wallet-badge ${wallet.wallet_type}">${capitalizeFirst(wallet.wallet_type)}</span>`;

    return `
      <div class="main-wallet-content">
        <div class="main-wallet-header">
          <div class="main-wallet-identity">
            <div class="main-wallet-icon">
              <i class="icon-star"></i>
            </div>
            <div class="main-wallet-info">
              <div class="main-wallet-name-row">
                <span class="main-wallet-name">${Utils.escapeHtml(wallet.name)}</span>
                <span class="wallet-badge main"><i class="icon-star"></i> Main</span>
                ${typeBadge}
              </div>
              <div class="main-wallet-address">
                <code>${wallet.address}</code>
                <button type="button" class="copy-btn" data-address="${wallet.address}" data-tooltip="Copy address">
                  <i class="icon-copy"></i>
                </button>
              </div>
            </div>
          </div>
          <div class="main-wallet-balance">
            <span class="balance-value">${balanceDisplay}</span>
            <span class="balance-label">SOL</span>
          </div>
        </div>
        <div class="main-wallet-meta">
          <div class="meta-item">
            <i class="icon-clock"></i>
            <span>Last used: ${lastUsed}</span>
          </div>
          <div class="meta-item">
            <i class="icon-calendar"></i>
            <span>Created: ${Utils.formatTimestamp(wallet.created_at, { variant: "short" })}</span>
          </div>
          ${wallet.notes ? `<div class="meta-item notes"><i class="icon-file-text"></i><span>${Utils.escapeHtml(wallet.notes)}</span></div>` : ""}
        </div>
        <div class="main-wallet-actions">
          <button type="button" class="btn" data-action="export" data-id="${wallet.id}">
            <i class="icon-key"></i> Export Key
          </button>
        </div>
      </div>
    `;
  }

  function wireMainWalletActions(container) {
    // Copy button
    container.querySelectorAll(".copy-btn").forEach((btn) => {
      on(btn, "click", (e) => {
        e.stopPropagation();
        const address = btn.dataset.address;
        Utils.copyToClipboard(address);
        Utils.showToast("Address copied!", "success");
      });
    });

    // Export action
    container.querySelectorAll("[data-action='export']").forEach((btn) => {
      on(btn, "click", () => handleWalletAction("export", btn.dataset.id));
    });
  }

  // =============================================================================
  // Token Holdings
  // =============================================================================

  function renderTokenHoldings() {
    const container = $("#token-holdings-table");
    if (!container) return;

    const tokens = tokenHoldings();
    if (tokens.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-state-icon">
            <i class="icon-coins"></i>
          </div>
          <h4 class="empty-state-title">No token holdings</h4>
          <p class="empty-state-description">Tokens in this wallet will appear here</p>
        </div>
      `;
      return;
    }

    // Sort by balance (highest first)
    const sorted = [...tokens].sort((a, b) => (b.ui_amount || 0) - (a.ui_amount || 0));

    const rows = sorted
      .map((token) => {
        const symbol = token.symbol || "Unknown";
        const balance =
          token.ui_amount != null ? Utils.formatNumber(token.ui_amount, { decimals: 4 }) : "—";
        const mint = token.mint || "";
        const shortMint = mint ? `${mint.slice(0, 6)}...${mint.slice(-4)}` : "—";

        return `
          <tr>
            <td class="token-symbol">${Utils.escapeHtml(symbol)}</td>
            <td class="token-balance">${balance}</td>
            <td class="token-mint">
              <code>${shortMint}</code>
              ${mint ? `<button type="button" class="copy-btn-mini" data-address="${mint}" data-tooltip="Copy mint"><i class="icon-copy"></i></button>` : ""}
            </td>
          </tr>
        `;
      })
      .join("");

    container.innerHTML = `
      <table class="holdings-table">
        <thead>
          <tr>
            <th>Token</th>
            <th>Balance</th>
            <th>Mint Address</th>
          </tr>
        </thead>
        <tbody>
          ${rows}
        </tbody>
      </table>
    `;

    // Wire copy buttons
    container.querySelectorAll(".copy-btn-mini").forEach((btn) => {
      on(btn, "click", (e) => {
        e.stopPropagation();
        Utils.copyToClipboard(btn.dataset.address);
        Utils.showToast("Mint address copied!", "success");
      });
    });
  }

  // =============================================================================
  // Secondaries Panel
  // =============================================================================

  function renderSecondariesPanel() {
    const container = $("#secondaries-table-container");
    if (!container) return;

    const wallets = walletsData();
    const secondaryWallets = wallets.filter((w) => w.role === "secondary" && w.is_active);

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
      // Attach click handler to the empty state button
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
    // Copy buttons
    container.querySelectorAll(".copy-btn-mini").forEach((btn) => {
      on(btn, "click", (e) => {
        e.stopPropagation();
        Utils.copyToClipboard(btn.dataset.address);
        Utils.showToast("Address copied!", "success");
      });
    });

    // Action buttons
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
    updateStats,
    renderMainWalletPanel,
    renderTokenHoldings,
    renderSecondariesPanel,
    renderArchivePanel,
  };
}
