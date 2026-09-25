/** Wallet observation UI for Wallets > Watched. */

import { DataTable } from "../../ui/data_table.js";
import { openCopyForWallet } from "../../ui/copy_handoff.js";

const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

export function createWatchedWallets({
  $,
  on,
  Utils,
  requestManager,
  showModal,
  hideModal,
  onRefresh,
}) {
  let targets = [];
  let statuses = new Map();
  let loading = false;
  let hasLoadedOnce = false;
  let table = null;
  let copyClickHandler = null;
  let budgetTarget = null;
  let savingBudget = false;

  const COLUMNS = [
    {
      id: "label",
      label: "Wallet",
      sortable: true,
      minWidth: 160,
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
      minWidth: 230,
      render: (value, row) =>
        `<span class="watched-wallet-state ${row._stateClass}"><i class="icon-activity"></i><span>${value}</span></span>${row._reason ? `<small class="watched-wallet-reason">${Utils.escapeHtml(row._reason)}</small>` : ""}`,
    },
    {
      id: "_lastActivity",
      label: "Last Sync",
      sortable: true,
      minWidth: 140,
      render: (value) => (value ? formatTime(value) : "No activity yet"),
    },
    {
      id: "actions",
      label: "",
      sortable: false,
      minWidth: 240,
      render: (value, row) => `
        <div class="watched-wallet-actions">
          <button class="btn" type="button" data-watch-action="copy" data-watch-id="${row.id}" title="Open this wallet in Copy Trading">Copy trade</button>
          <button class="btn" type="button" data-watch-action="budget" data-watch-id="${row.id}">${row.disable_reason?.kind === "signature_budget" ? "Resume watch" : "Watch limit"}</button>
          ${row.disable_reason?.kind === "signature_budget" ? "" : `<button class="btn" type="button" data-watch-action="toggle" data-watch-id="${row.id}">${row.enabled ? "Pause" : "Enable"}</button>`}
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

    const budgetForm = $("#watch-budget-form");
    if (budgetForm) on(budgetForm, "submit", saveBudget);
    on($("#watch-page-budget"), "input", syncBudgetSubmit);
    on($("#watch-budget-ack"), "change", syncBudgetSubmit);
    ["#watch-budget-close", "#watch-budget-cancel"].forEach((selector) => {
      const button = $(selector);
      if (button) on(button, "click", hideBudgetModal);
    });
    const budgetModal = $("#watch-budget-modal");
    if (budgetModal) on(budgetModal, "click", (event) => {
      if (event.target === budgetModal) hideBudgetModal();
    });

    // The table is created lazily by load(), after the parent has made the
    // restored Watched panel visible. Constructing it here would measure a
    // display:none ancestor whenever another wallet subtab is active.
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

  function showBudgetModal(target) {
    budgetTarget = target;
    const modal = $("#watch-budget-modal");
    const pausedForBudget = target.disable_reason?.kind === "signature_budget";
    const currentLimit = (target.page_budget || 5) * 100;
    const input = $("#watch-page-budget");
    if (input)
      input.value = String(pausedForBudget ? Math.min(5000, currentLimit + 500) : currentLimit);
    const title = $("#watch-budget-title");
    if (title) title.textContent = pausedForBudget ? "Resume wallet watch" : "Wallet watch limit";
    const hint = $("#watch-budget-hint");
    if (hint)
      hint.textContent = `Current limit: ${currentLimit.toLocaleString()}. Choose 500–5,000 signatures per check in steps of 100. A higher limit uses more RPC credits and may still fall behind.`;
    $("#watch-budget-resume-notice")?.classList.toggle("hidden", !pausedForBudget);
    const ack = $("#watch-budget-ack");
    if (ack) ack.checked = false;
    const button = $("#watch-budget-save");
    if (button) button.textContent = pausedForBudget ? "Resume from now" : "Save limit";
    const error = $("#watch-budget-error");
    if (error) {
      error.textContent = "";
      error.classList.add("hidden");
    }
    syncBudgetSubmit();
    showModal("watch-budget-modal");
    modal?.querySelector("#watch-page-budget")?.focus();
  }

  function hideBudgetModal() {
    hideModal("watch-budget-modal");
    budgetTarget = null;
  }

  function syncBudgetSubmit() {
    const button = $("#watch-budget-save");
    if (!button) return;
    const input = $("#watch-page-budget");
    const signatures = Number(input?.value);
    const needsAck = budgetTarget?.disable_reason?.kind === "signature_budget";
    button.disabled =
      savingBudget ||
      !budgetTarget ||
      !input?.value ||
      !Number.isInteger(signatures / 100) ||
      signatures < 500 ||
      signatures > 5000 ||
      (needsAck && !$("#watch-budget-ack")?.checked);
  }

  async function saveBudget(event) {
    event.preventDefault();
    const target = budgetTarget;
    if (!target) return;
    const signatureLimit = Number($("#watch-page-budget")?.value);
    const pageBudget = signatureLimit / 100;
    const pausedForBudget = target.disable_reason?.kind === "signature_budget";
    const ack = $("#watch-budget-ack");
    const error = $("#watch-budget-error");
    if (!Number.isInteger(pageBudget) || pageBudget < 5 || pageBudget > 50) {
      if (error) {
        error.textContent =
          "Choose between 500 and 5,000 signatures per poll in 100-signature steps.";
        error.classList.remove("hidden");
      }
      return;
    }
    if (pausedForBudget && !ack?.checked) {
      if (error) {
        error.textContent =
          "Acknowledge that signatures since the last completed check will be skipped.";
        error.classList.remove("hidden");
      }
      return;
    }
    savingBudget = true;
    syncBudgetSubmit();
    try {
      await requestManager.fetch(
        `/api/wallets/watch/${target.id}/${pausedForBudget ? "resume" : "budget"}`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(
            pausedForBudget
              ? { page_budget: pageBudget, acknowledge_missed_activity: true }
              : { page_budget: pageBudget }
          ),
          priority: "high",
          skipDedup: true,
        }
      );
      hideBudgetModal();
      Utils.showToast(
        pausedForBudget
          ? "Watch resumed from the current wallet head"
          : "Wallet watch limit updated",
        "success"
      );
      await load({ force: true });
    } catch (requestError) {
      if (error) {
        error.textContent =
          requestError.detail || requestError.message || "Watch limit could not be saved.";
        error.classList.remove("hidden");
      }
    } finally {
      savingBudget = false;
      syncBudgetSubmit();
    }
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
        summary: [{ id: "watched-count", label: "Watched", value: "0", variant: "secondary" }],
        search: { enabled: true, mode: "client", placeholder: "Search watched wallets..." },
        buttons: [
          {
            id: "watched-add",
            label: "Watch Wallet",
            icon: "icon-plus",
            variant: "primary",
            onClick: () => showAddModal(),
          },
          {
            id: "watched-refresh",
            icon: "icon-refresh-cw",
            tooltip: "Refresh watched wallets",
            onClick: (btn) => onRefresh?.(btn),
          },
        ],
      },
    });
    on(root, "click", handleListAction);
    copyClickHandler = (e) => {
      const btn = e.target.closest("[data-copy-address]");
      if (!btn) return;
      e.stopPropagation();
      Utils.copyToClipboard(btn.dataset.copyAddress);
      Utils.notifyCopied("Address");
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
      const data = await requestManager.fetch("/api/wallets/watch", {
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
      await requestManager.fetch("/api/wallets/watch", {
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
    if (action === "copy") {
      openCopyForWallet(target.address, target.label || null);
      return;
    }
    if (action === "budget" || (action === "toggle" && !target.enabled && target.disable_reason?.kind === "signature_budget")) {
      showBudgetModal(target);
      return;
    }
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
        _reason: target.disable_reason?.kind === "signature_budget"
          ? `Reached the ${(Number(target.disable_reason.page_budget) || 5) * 100}-signature check limit before catching up.`
          : target.disable_reason?.kind === "user" ? "Paused by you." : "",
        _lastActivity: status?.last_activity_at || null,
      };
    });
    t.setData(rows);
    t.updateToolbarSummary([{ id: "watched-count", value: String(rows.length) }]);
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
