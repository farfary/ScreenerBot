/** Wallet observation UI for Wallets > Watched. */

import { DataTable } from "../../ui/data_table.js";

const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

export function createWatchedWallets({ $, on, Utils, requestManager, showModal, hideModal }) {
  let targets = [];
  let statuses = new Map();
  let loading = false;
  let hasLoadedOnce = false;
  let table = null;
  let copyClickHandler = null;

  const COLUMNS = [
    {
      id: "label",
      label: "Wallet",
      sortable: true,
      render: (value, row) => {
        const label = Utils.escapeHtml(row.label || "Unlabelled wallet");
        const address = row.address || "";
        const short = address ? `${address.slice(0, 6)}...${address.slice(-4)}` : "—";
        const copyBtn = address
          ? `<button type="button" class="copy-btn-mini" data-copy-address="${Utils.escapeHtml(address)}" title="Copy address"><i class="icon-copy"></i></button>`
          : "";
        return `<div class="wt-token-meta"><span class="wt-symbol">${label}</span><span class="wt-name wt-mint-cell"><span class="wt-mint-addr">${short}</span>${copyBtn}</span></div>`;
      },
    },
    {
      id: "_state",
      label: "Status",
      sortable: true,
      render: (value, row) =>
        `<span class="watched-wallet-state ${row._stateClass}"><i class="icon-activity"></i><span>${value}</span></span>`,
    },
    {
      id: "_lastActivity",
      label: "Last Sync",
      sortable: true,
      render: (value) => (value ? formatTime(value) : "No activity yet"),
    },
    {
      id: "actions",
      label: "",
      sortable: false,
      render: (value, row) => `
        <div class="watched-wallet-actions">
          <button class="btn" type="button" data-watch-action="toggle" data-watch-id="${row.id}">${row.enabled ? "Pause" : "Enable"}</button>
          <button class="btn-icon danger" type="button" data-watch-action="delete" data-watch-id="${row.id}" title="Remove" aria-label="Remove ${Utils.escapeHtml(row.label || "wallet")}"><i class="icon-trash-2"></i></button>
        </div>`,
    },
  ];

  function setup() {
    const form = $("#watched-wallet-form");
    if (form) on(form, "submit", addTarget);

    const modal = $("#watch-wallet-modal");
    if (modal) {
      on(modal, "click", (event) => {
        if (event.target === modal) hideAddModal();
      });
    }
    const closeBtn = $("#watch-modal-close");
    if (closeBtn) on(closeBtn, "click", () => hideAddModal());
    const cancelBtn = $("#watch-cancel-btn");
    if (cancelBtn) on(cancelBtn, "click", () => hideAddModal());

    ensureTable();
  }

  function showAddModal() {
    resetForm();
    showModal("watch-wallet-modal");
    $("#watched-wallet-address")?.focus();
  }

  function hideAddModal() {
    hideModal("watch-wallet-modal");
    resetForm();
  }

  function resetForm() {
    $("#watched-wallet-form")?.reset();
    setAddressError("");
  }

  function setAddressError(message) {
    const errorEl = $("#watched-wallet-address-error");
    if (errorEl) {
      errorEl.textContent = message;
      errorEl.classList.toggle("hidden", !message);
    }
    const addressInput = $("#watched-wallet-address");
    if (message) addressInput?.setAttribute("aria-invalid", "true");
    else addressInput?.removeAttribute("aria-invalid");
  }

  function ensureTable() {
    if (table) return table;
    const root = $("#watched-wallets-root");
    if (!root) return null;
    table = new DataTable({
      container: "#watched-wallets-root",
      columns: COLUMNS,
      rowIdField: "id",
      stateKey: "wallets.watched-table",
      compact: true,
      stickyHeader: true,
      zebra: true,
      fitToContainer: true,
      sorting: { mode: "client", column: "label", direction: "asc" },
      emptyTitle: "No watched addresses",
      emptyMessage: "Use Watch Wallet to record a public wallet's on-chain activity.",
      toolbar: {
        search: { enabled: true, mode: "client", placeholder: "Search watched wallets..." },
      },
    });
    on(root, "click", handleListAction);
    copyClickHandler = (e) => {
      const btn = e.target.closest("[data-copy-address]");
      if (!btn) return;
      e.stopPropagation();
      Utils.copyToClipboard(btn.dataset.copyAddress);
      Utils.showToast("Address copied!", "success");
    };
    root.addEventListener("click", copyClickHandler);
    return table;
  }

  async function load({ force = false } = {}) {
    if (loading && !force) return;
    loading = true;
    const t = ensureTable();
    if (!hasLoadedOnce && t?.showBlockingState) {
      t.showBlockingState({
        variant: "loading",
        title: "Loading watched wallets...",
        description: "Fetching observation targets.",
      });
    }
    try {
      const data = await requestManager.fetch("/api/wallets/watch/", {
        priority: force ? "high" : "normal",
        skipDedup: force,
      });
      targets = data.targets || [];
      const statusEntries = await Promise.all(
        targets.map(async (target) => {
          try {
            const status = await requestManager.fetch(`/api/wallets/watch/${target.id}/status`);
            return [target.id, status];
          } catch {
            return [target.id, null];
          }
        })
      );
      statuses = new Map(statusEntries);
      hasLoadedOnce = true;
      t?.hideBlockingState?.();
      render();
    } catch (error) {
      console.error("[Wallets] Failed to load watched addresses:", error);
      if (!hasLoadedOnce) {
        t?.showBlockingState?.({
          variant: "error",
          title: "Watched addresses could not be loaded",
          description: "Use refresh to try again.",
        });
      } else {
        Utils.showToast("Watched addresses could not be loaded", "error");
      }
    } finally {
      loading = false;
    }
  }

  async function addTarget(event) {
    event.preventDefault();
    const addressInput = $("#watched-wallet-address");
    const labelInput = $("#watched-wallet-label");
    const submit = event.currentTarget.querySelector('button[type="submit"]');
    const address = addressInput?.value.trim() || "";
    if (!SOLANA_ADDRESS_RE.test(address)) {
      setAddressError("Enter a valid Solana wallet address.");
      addressInput?.focus();
      return;
    }
    setAddressError("");
    if (submit) submit.disabled = true;
    try {
      await requestManager.fetch("/api/wallets/watch/", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ address, label: labelInput?.value.trim() || null }),
        priority: "high",
        skipDedup: true,
      });
      hideAddModal();
      Utils.showToast("Wallet watch added", "success");
      await load({ force: true });
    } catch (error) {
      const message =
        error.status === 409
          ? "That wallet is already watched."
          : "Wallet watch could not be added.";
      setAddressError(message);
      Utils.showToast(message, "error");
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  async function handleListAction(event) {
    const button = event.target.closest("button[data-watch-action]");
    if (!button) return;
    const id = Number(button.dataset.watchId);
    const action = button.dataset.watchAction;
    const target = targets.find((item) => item.id === id);
    if (!target) return;
    button.disabled = true;
    try {
      if (action === "toggle") {
        await requestManager.fetch(`/api/wallets/watch/${id}/enabled`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: !target.enabled }),
          priority: "high",
          skipDedup: true,
        });
        Utils.showToast(target.enabled ? "Wallet watch paused" : "Wallet watch enabled", "success");
      } else if (action === "delete") {
        await requestManager.fetch(`/api/wallets/watch/${id}`, {
          method: "DELETE",
          priority: "high",
          skipDedup: true,
        });
        Utils.showToast("Wallet watch removed", "success");
      }
      await load({ force: true });
    } catch (error) {
      console.error("[Wallets] Watch action failed:", error);
      Utils.showToast("Wallet watch could not be updated", "error");
      button.disabled = false;
    }
  }

  function render() {
    const t = ensureTable();
    if (!t) return;

    const rows = targets.map((target) => {
      const status = statuses.get(target.id);
      const state = !target.enabled ? "Paused" : status?.subscribed ? "Streaming" : "Polling";
      const stateClass = !target.enabled
        ? "is-paused"
        : status?.subscribed
          ? "is-streaming"
          : "is-polling";
      return {
        ...target,
        _state: state,
        _stateClass: stateClass,
        _lastActivity: status?.last_activity_at || null,
      };
    });
    t.setData(rows);
  }

  function formatTime(value) {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? "Unknown" : date.toLocaleString();
  }

  function reset() {
    targets = [];
    statuses = new Map();
    loading = false;
    hasLoadedOnce = false;
    if (table) {
      table.destroy();
      table = null;
    }
    const root = $("#watched-wallets-root");
    if (root && copyClickHandler) {
      root.removeEventListener("click", copyClickHandler);
      copyClickHandler = null;
    }
  }

  return { setup, load, reset, showAddModal, hideAddModal };
}
