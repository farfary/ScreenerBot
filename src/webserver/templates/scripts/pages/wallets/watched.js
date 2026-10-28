// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/** Wallet observation UI for Wallets > Watched. */

import { DataTable } from "../../ui/data_table.js";
import { openCopyForWallet } from "../../ui/copy_handoff.js";
import { addressFloorWidth, renderNamedAddress } from "../../ui/token_identity.js";

const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

// Second status line per `WatchDisableReason` kind (src/wallets/watch/types.rs).
// `unknown` has no detail line.
const WATCH_DISABLE_DETAIL_LABELS = Object.freeze({
  user: "wallets-watch-reason-user",
  signature_budget: "wallets-watch-reason-signature-budget",
  helius_unavailable: "wallets-watch-reason-helius-unavailable",
  processing_failed: "wallets-watch-reason-processing-failed",
});

// Short form of the same reason, appended to the state label.
const WATCH_DISABLE_SHORT_LABELS = Object.freeze({
  user: "wallets-watch-reason-short-user",
  signature_budget: "wallets-watch-reason-short-signature-budget",
  helius_unavailable: "wallets-watch-reason-short-helius-unavailable",
  processing_failed: "wallets-watch-reason-short-processing-failed",
});

// Display states derived in `render()` from the target and its status.
const WATCH_STATE_LABELS = Object.freeze({
  paused: "wallets-watch-state-paused",
  catching_up: "wallets-watch-state-catching-up",
  watching: "wallets-watch-state-watching",
  streaming: "wallets-watch-state-streaming",
  polling: "wallets-watch-state-polling",
});

function disableShort(reason) {
  return Object.hasOwn(WATCH_DISABLE_SHORT_LABELS, reason?.kind)
    ? I18n.label(WATCH_DISABLE_SHORT_LABELS, reason.kind)
    : "";
}

function disableDetail(reason) {
  return Object.hasOwn(WATCH_DISABLE_DETAIL_LABELS, reason?.kind)
    ? I18n.label(WATCH_DISABLE_DETAIL_LABELS, reason.kind)
    : "";
}

export function createWatchedWallets({
  $,
  on,
  Utils,
  requestManager,
  showModal,
  hideModal,
  confirm,
  onRefresh,
}) {
  let targets = [];
  let statuses = new Map();
  let loading = false;
  let hasLoadedOnce = false;
  let table = null;
  let budgetTarget = null;
  let savingBudget = false;

  const COLUMNS = [
    {
      id: "label",
      label: I18n.t("wallets-watched-col-wallet"),
      grow: true,
      sortable: true,
      minWidth: addressFloorWidth() + 24,
      render: (value, row) =>
        renderNamedAddress(row.label || I18n.t("wallets-watched-unlabelled"), row.address),
    },
    {
      id: "_state",
      label: I18n.t("wallets-watched-col-status"),
      sortable: true,
      minWidth: 290,
      render: (value, row) => {
        const state = row._reasonShort
          ? I18n.t("wallets-watch-state-reason", { state: value, reason: row._reasonShort })
          : value;
        const title = row._reason ? ` title="${Utils.escapeHtml(row._reason)}"` : "";
        const detail = row._detail
          ? `<small class="watched-wallet-reason">${Utils.escapeHtml(row._detail)}</small>`
          : "";
        return `<span class="watched-wallet-state ${row._stateClass}"${title}><i class="icon-activity"></i><span>${Utils.escapeHtml(state)}</span></span>${detail}`;
      },
    },
    {
      id: "_lastActivity",
      label: I18n.t("wallets-watched-col-progress"),
      sortable: true,
      minWidth: "content",
      wrap: false,
      render: (value) => (value ? formatTime(value) : I18n.t("wallets-watched-not-synced")),
    },
    {
      id: "_lastCheck",
      label: I18n.t("wallets-watched-col-last-check"),
      sortable: true,
      minWidth: "content",
      wrap: false,
      render: (value) => (value ? formatTime(value) : I18n.t("wallets-watched-not-checked")),
    },
    {
      id: "actions",
      label: I18n.t("wallets-list-col-actions"),
      type: "actions",
      sortable: false,
      actions: { buttons: watchActions },
    },
  ];

  /** The row actions for one watched wallet: icon buttons named by their tooltips. */
  function watchActions(row) {
    const restore = row.disable_reason?.kind === "signature_budget";
    const retry = ["helius_unavailable", "processing_failed"].includes(row.disable_reason?.kind);
    const run = (action) => (target, event) =>
      runWatchAction(action, target, event.target.closest("button"));
    const actions = [
      {
        id: "copy",
        icon: '<i class="icon-copy-plus"></i>',
        tooltip: I18n.attr("wallets-watched-action-copy", "title"),
        ariaLabel: I18n.t("wallets-watched-action-copy"),
        size: "sm",
        onClick: run("copy"),
      },
      {
        id: "budget",
        icon: restore
          ? '<i class="icon-rotate-ccw"></i>'
          : '<i class="icon-sliders-horizontal"></i>',
        tooltip: restore
          ? I18n.t("wallets-watched-action-restore")
          : I18n.t("wallets-watched-action-options"),
        size: "sm",
        onClick: run("budget"),
      },
    ];
    if (retry) {
      actions.push({
        id: "retry",
        icon: '<i class="icon-rotate-cw"></i>',
        tooltip: I18n.t("wallets-watched-action-retry"),
        variant: "primary",
        size: "sm",
        onClick: run("retry"),
      });
    } else if (!restore) {
      actions.push({
        id: "toggle",
        icon: row.enabled ? '<i class="icon-pause"></i>' : '<i class="icon-play"></i>',
        tooltip: row.enabled
          ? I18n.t("wallets-watched-action-pause")
          : I18n.t("wallets-watched-action-enable"),
        size: "sm",
        onClick: run("toggle"),
      });
    }
    actions.push({
      id: "delete",
      icon: '<i class="icon-trash-2"></i>',
      tooltip: I18n.attr("wallets-watched-action-remove", "title"),
      ariaLabel: I18n.attr("wallets-watched-action-remove", "aria-label", {
        name: row.label || I18n.t("wallets-watched-generic-name"),
      }),
      variant: "danger",
      size: "sm",
      onClick: run("delete"),
    });
    return actions;
  }

  function setup() {
    const form = $("#watched-wallet-form");
    if (form) on(form, "submit", addTarget);

    const budgetForm = $("#watch-budget-form");
    if (budgetForm) on(budgetForm, "submit", saveBudget);
    on($("#watch-helius-action"), "click", () => {
      if (budgetTarget) void setHeliusApproval(budgetTarget, $("#watch-helius-action"));
    });
    on($("#watch-page-budget"), "input", syncBudgetSubmit);
    on($("#watch-budget-ack"), "change", syncBudgetSubmit);

    // The table is created lazily by load(), after the parent has made the
    // restored Watched panel visible. Constructing it here would measure a
    // display:none ancestor whenever another wallet subtab is active.
  }

  function showAddModal() {
    resetForm();
    showModal("watch-wallet-modal", { onClose: resetForm });
    $("#watched-wallet-address")?.focus();
  }

  function hideAddModal() {
    hideModal("watch-wallet-modal");
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
    if (title)
      title.textContent = pausedForBudget
        ? I18n.t("wallets-watch-budget-title-restore")
        : I18n.t("wallets-watch-budget-title-options");
    const status = statuses.get(target.id);
    const helius = status?.catch_up_options?.find((option) => option.provider === "helius");
    const highActivity = status?.mode === "helius_high_activity";
    const label = $("#watch-page-budget-label");
    if (label)
      label.textContent = highActivity
        ? I18n.t("wallets-watch-budget-label-transactions")
        : I18n.t("wallets-watch-budget-label-signatures");
    const hint = $("#watch-budget-hint");
    const formattedLimit = Utils.formatNumber(currentLimit, 0);
    if (hint)
      hint.textContent = highActivity
        ? I18n.t("wallets-watch-budget-hint-transactions", { limit: formattedLimit })
        : I18n.t("wallets-watch-budget-hint-signatures", { limit: formattedLimit });
    const heliusDescription = $("#watch-helius-description");
    const heliusAction = $("#watch-helius-action");
    if (heliusDescription && heliusAction) {
      heliusAction.hidden = !target.high_activity_approved && !helius?.available;
      heliusAction.textContent = target.high_activity_approved
        ? I18n.t("wallets-watch-helius-stop")
        : pausedForBudget
          ? I18n.t("wallets-watch-helius-try")
          : I18n.t("wallets-watch-helius-allow");
      heliusDescription.textContent = target.high_activity_approved
        ? I18n.t("wallets-watch-helius-description-approved")
        : helius?.available
          ? I18n.t("wallets-watch-helius-description-available")
          : helius
            ? I18n.t("wallets-watch-helius-description-unavailable")
            : I18n.t("wallets-watch-helius-description-unsupported");
    }
    const resumeNotice = $("#watch-budget-resume-notice");
    if (resumeNotice) resumeNotice.hidden = !pausedForBudget;
    const ack = $("#watch-budget-ack");
    if (ack) ack.checked = false;
    const button = $("#watch-budget-save");
    if (button) {
      button.textContent = pausedForBudget
        ? I18n.t("wallets-watch-budget-resume")
        : I18n.t("wallets-watch-budget-save");
    }
    const error = $("#watch-budget-error");
    if (error) {
      error.textContent = "";
      error.hidden = true;
    }
    syncBudgetSubmit();
    showModal("watch-budget-modal", {
      onClose: () => {
        budgetTarget = null;
      },
    });
    modal?.querySelector("#watch-page-budget")?.focus();
  }

  function hideBudgetModal() {
    hideModal("watch-budget-modal");
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
        error.textContent = I18n.t("wallets-watch-budget-error-range");
        error.hidden = false;
      }
      return;
    }
    if (pausedForBudget && !ack?.checked) {
      if (error) {
        error.textContent = I18n.t("wallets-watch-budget-error-ack");
        error.hidden = false;
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
          ? I18n.t("wallets-watch-budget-resumed")
          : I18n.t("wallets-watch-budget-updated"),
        "success"
      );
      await load({ force: true });
    } catch (requestError) {
      if (error) {
        error.textContent =
          requestError.detail || requestError.message || I18n.t("wallets-watch-budget-save-failed");
        error.hidden = false;
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
      errorEl.hidden = !message;
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
      uniformRowHeight: 2,
      rowIdField: "id",
      stateKey: "wallets.watched-table",
      compact: true,
      stickyHeader: true,
      zebra: true,
      fitToContainer: true,
      sorting: { mode: "client", column: "label", direction: "asc" },
      emptyTitle: I18n.t("wallets-watched-empty-title"),
      emptyMessage: I18n.t("wallets-watched-empty-message"),
      toolbar: {
        summary: [
          {
            id: "watched-count",
            label: I18n.t("wallets-watched-count"),
            value: "0",
            variant: "secondary",
          },
        ],
        search: {
          enabled: true,
          mode: "client",
          placeholder: I18n.attr("wallets-watched-search", "placeholder"),
        },
        buttons: [
          {
            id: "watched-add",
            label: I18n.t("wallets-watched-add"),
            icon: "icon-plus",
            variant: "primary",
            onClick: () => showAddModal(),
          },
          {
            id: "watched-refresh",
            icon: "icon-refresh-cw",
            tooltip: I18n.t("wallets-watched-refresh"),
            onClick: (btn) => onRefresh?.(btn),
          },
        ],
      },
    });
    return table;
  }

  async function load({ force = false } = {}) {
    if (loading && !force) return;
    loading = true;
    const t = ensureTable();
    if (!hasLoadedOnce && t?.showBlockingState) {
      t.showBlockingState({
        variant: "loading",
        title: I18n.t("wallets-watched-loading-title"),
        description: I18n.t("wallets-watched-loading-description"),
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
          title: I18n.t("wallets-watched-load-error-title"),
          description: I18n.t("wallets-watched-load-error-description"),
        });
      } else {
        Utils.showToast(I18n.t("wallets-watched-load-error-title"), "error");
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
      setAddressError(I18n.t("wallets-watched-address-invalid"));
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
      Utils.showToast(I18n.t("wallets-watched-added"), "success");
      await load({ force: true });
    } catch (error) {
      const message =
        error.status === 409
          ? I18n.t("wallets-watched-duplicate")
          : I18n.t("wallets-watched-add-failed");
      setAddressError(message);
      Utils.showToast(message, "error");
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  async function setHeliusApproval(target, button) {
    const approved = !target.high_activity_approved;
    const status = statuses.get(target.id);
    const helius = status?.catch_up_options?.find((option) => option.provider === "helius");
    if (approved && !helius?.available) return;
    const result = await confirm(
      approved
        ? {
            title: I18n.t("wallets-watch-helius-allow-title"),
            message: I18n.t("wallets-watch-helius-allow-message"),
            confirmLabel: I18n.t("wallets-watch-helius-allow-confirm"),
            cancelLabel: I18n.t("common-action-cancel"),
            variant: "warning",
          }
        : {
            title: I18n.t("wallets-watch-helius-stop"),
            message: I18n.t("wallets-watch-helius-stop-message"),
            confirmLabel: I18n.t("wallets-watch-helius-stop-confirm"),
            cancelLabel: I18n.t("wallets-watch-helius-stop-keep"),
            variant: "warning",
          }
    );
    if (!result.confirmed) return;
    button.disabled = true;
    try {
      await requestManager.fetch(`/api/wallets/watch/${target.id}/high-activity-approval`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ approved, acknowledge_provider_usage: approved }),
        priority: "high",
        skipDedup: true,
      });
      hideBudgetModal();
      Utils.showToast(
        approved
          ? target.disable_reason?.kind === "signature_budget"
            ? I18n.t("wallets-watch-helius-restored")
            : I18n.t("wallets-watch-helius-allowed")
          : I18n.t("wallets-watch-helius-stopped"),
        "success"
      );
      await load({ force: true });
    } catch (error) {
      Utils.showToast(
        error.detail || error.message || I18n.t("wallets-watch-helius-update-failed"),
        "error"
      );
      button.disabled = false;
    }
  }

  async function runWatchAction(action, row, button) {
    const target = targets.find((item) => item.id === row.id);
    if (!target) return;
    const id = target.id;
    if (action === "copy") {
      openCopyForWallet(target.address, target.label || null);
      return;
    }
    if (
      action === "budget" ||
      (action === "toggle" && !target.enabled && target.disable_reason?.kind === "signature_budget")
    ) {
      showBudgetModal(target);
      return;
    }
    button.disabled = true;
    try {
      if (action === "toggle" || action === "retry") {
        await requestManager.fetch(`/api/wallets/watch/${id}/enabled`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: action === "retry" ? true : !target.enabled }),
          priority: "high",
          skipDedup: true,
        });
        Utils.showToast(
          action === "retry"
            ? I18n.t("wallets-watched-retried")
            : target.enabled
              ? I18n.t("wallets-watched-paused")
              : I18n.t("wallets-watched-enabled"),
          "success"
        );
      } else if (action === "delete") {
        await requestManager.fetch(`/api/wallets/watch/${id}`, {
          method: "DELETE",
          priority: "high",
          skipDedup: true,
        });
        Utils.showToast(I18n.t("wallets-watched-removed"), "success");
      }
      await load({ force: true });
    } catch (error) {
      console.error("[Wallets] Watch action failed:", error);
      Utils.showToast(
        error.detail || error.message || I18n.t("wallets-watched-update-failed"),
        "error"
      );
      button.disabled = false;
    }
  }

  function render() {
    const t = ensureTable();
    if (!t) return;

    const rows = targets.map((target) => {
      const status = statuses.get(target.id);
      const highActivity = status?.mode === "helius_high_activity";
      const state = !target.enabled
        ? "paused"
        : status?.catching_up
          ? "catching_up"
          : highActivity
            ? "watching"
            : status?.subscribed
              ? "streaming"
              : "polling";
      const stateClass = !target.enabled
        ? "is-paused"
        : status?.catching_up
          ? "is-polling"
          : highActivity || status?.subscribed
            ? "is-streaming"
            : "is-polling";
      return {
        ...target,
        _state: I18n.label(WATCH_STATE_LABELS, state),
        _stateClass: stateClass,
        _reason: disableDetail(target.disable_reason),
        _reasonShort: disableShort(target.disable_reason),
        _detail: target.enabled
          ? (status?.last_error ? I18n.text(status.last_error) : "") ||
            (highActivity ? I18n.t("wallets-watched-detail-helius") : "")
          : "",
        _lastActivity: status?.last_activity_at || null,
        _lastCheck: status?.last_checked_at || null,
      };
    });
    t.setData(rows);
    t.updateToolbarSummary([{ id: "watched-count", value: String(rows.length) }]);
  }

  function formatTime(value) {
    return Utils.formatTimestamp(value, {
      includeYear: false,
      includeSeconds: false,
      fallback: I18n.t("format-unknown"),
    });
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
  }

  return { setup, load, reset };
}
