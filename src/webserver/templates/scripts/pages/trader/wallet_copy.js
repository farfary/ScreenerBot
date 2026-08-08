const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;
const LIVE_ARM_CONFIRMATION = "ARM LIVE COPY TRADING";

const SKIP_LABELS = {
  not_buy_swap: "Target activity was not a buy",
  task_disabled: "Task is paused",
  mode_transition_required: "Execution mode must be changed separately",
  live_confirmation_required: "Live execution needs confirmation",
  unsupported_sizing_mode: "Sizing mode is not supported yet",
  self_copy: "Target belongs to this account",
  target_below_minimum: "Target trade is below the minimum",
  target_above_maximum: "Target trade is above the maximum",
  already_bought: "Buy-once limit reached",
  blacklisted: "Token is blocked by risk controls",
  filter_required: "Token did not pass filtering",
  budget_exhausted: "Task budget is exhausted",
  token_cap_reached: "Per-token limit reached",
  below_minimum_size: "Calculated copy size is too small",
  invalid_sizing: "Task sizing is invalid",
  invalid_slippage: "Task slippage is invalid",
  invalid_exit_policy: "Task exit policy is invalid",
  invalid_price: "No usable market price",
  not_sell_swap: "Target activity was not a sell",
  exit_mode_disabled: "Target sell ignored by task exit mode",
  force_stopped: "Trading is force-stopped",
  copy_position_not_found: "No position owned by this copy task",
  position_user_only: "Position is managed by the user only",
  position_management_mismatch: "Position ownership no longer permits copy sells",
};

const POLICY_CONTROLS = [
  ["stop-loss", "stop_loss", "threshold_pct"],
  ["roi", "roi", "target_profit_pct"],
  ["trailing", "trailing", "distance_pct"],
  ["time", "time", "duration_seconds"],
];

const ENTRY_BLOCK_LABELS = {
  force_stopped: "Trading is force-stopped",
  loss_limit: "The loss limit is blocking new entries",
  connectivity: "Required services are temporarily unavailable",
  position_limit: "Open-position limit reached",
  already_open: "A position is already open",
  reentry_cooldown: "Token re-entry cooldown is active",
  open_cooldown: "Global entry cooldown is active",
  entry_reserved: "Another entry is already processing",
  blacklisted: "Token is blocked by risk controls",
  check_failed: "A safety check could not complete",
};

export function createWalletCopy({ $, Utils, requestManager, ConfirmationDialog }) {
  let tasks = [];
  let selectedId = null;
  let setupDone = false;
  let defaultSlippage = 2;
  let lastTaskRenderKey = "";
  let lastActivityRenderKey = "";

  function setup(on) {
    if (setupDone) return;
    setupDone = true;
    on($("#wallet-copy-new-task"), "click", () => editTask(null));
    on($("#wallet-copy-cancel"), "click", closeEditor);
    on($("#wallet-copy-form"), "submit", saveTask);
    on($("#wallet-copy-delete"), "click", deleteTask);
    on($("#wallet-copy-mode-action"), "click", changeTaskMode);
    on($("#wallet-copy-task-list"), "click", (event) => {
      const button = event.target.closest("button[data-copy-task-id]");
      if (button) editTask(Number(button.dataset.copyTaskId));
    });
    on($("#wallet-copy-sizing"), "change", updateSizeLabel);
    POLICY_CONTROLS.forEach(([control]) => {
      on($(`#wallet-copy-${control}-mode`), "change", () => updatePolicyControl(control));
    });
    on($("#wallet-copy-global-toggle"), "change", toggleGlobal);
    on($("#stats-wallet-copy-toggle"), "change", toggleGlobal);
  }

  async function load() {
    try {
      const [taskData, activityData, statusData] = await Promise.all([
        requestManager.fetch("/api/copy-trading/tasks"),
        requestManager.fetch("/api/copy-trading/activity?limit=30"),
        requestManager.fetch("/api/copy-trading/status"),
      ]);
      tasks = taskData.tasks || taskData || [];
      renderTasks();
      renderActivity(activityData.activity || activityData.items || activityData || []);
      applyStatus(statusData);
      if (selectedId && !tasks.some((task) => task.id === selectedId)) closeEditor();
    } catch (error) {
      console.error("[Trader] Wallet copy load failed:", error);
      renderLoadError();
    }
  }

  async function loadStatus() {
    try {
      applyStatus(await requestManager.fetch("/api/copy-trading/status"));
    } catch (error) {
      console.error("[Trader] Wallet copy status failed:", error);
    }
  }

  function applyStatus(status) {
    defaultSlippage = Number(status.default_slippage_pct) || 2;
    [$("#wallet-copy-global-toggle"), $("#stats-wallet-copy-toggle")].forEach((toggle) => {
      if (toggle) toggle.checked = Boolean(status.enabled);
    });
    const blockedLabels = {
      force_stop: "Blocked by Force Stop",
      loss_limit: "Entries blocked by Loss Limit · exits active",
    };
    const modeSummary = status.live_tasks
      ? `${status.live_tasks} live · ${status.paper_tasks || 0} paper`
      : `${status.paper_tasks || 0} paper`;
    const statusText = !status.enabled
      ? "Paused"
      : status.blocked_reason
        ? blockedLabels[status.blocked_reason] || "Entries blocked"
        : `Active · ${modeSummary}`;
    const globalStatus = $("#wallet-copy-global-status");
    const statsStatus = $("#stats-wallet-copy-status");
    if (globalStatus) globalStatus.textContent = statusText;
    if (statsStatus) statsStatus.textContent = statusText;
  }

  async function toggleGlobal(event) {
    const enabled = event.currentTarget.checked;
    const toggles = [$("#wallet-copy-global-toggle"), $("#stats-wallet-copy-toggle")].filter(
      Boolean
    );
    toggles.forEach((toggle) => {
      toggle.disabled = true;
      toggle.checked = enabled;
    });
    try {
      await requestManager.fetch("/api/config/copy_trading", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
        priority: "high",
        skipDedup: true,
      });
      Utils.showToast(enabled ? "Wallet copy resumed" : "Wallet copy paused", "success");
      await loadStatus();
    } catch {
      toggles.forEach((toggle) => {
        toggle.checked = !enabled;
      });
      Utils.showToast("Wallet copy status could not be changed", "error");
    } finally {
      toggles.forEach((toggle) => {
        toggle.disabled = false;
      });
    }
  }

  function editTask(id) {
    selectedId = id;
    const task = tasks.find((item) => item.id === id) || null;
    $("#wallet-copy-editor-empty")?.setAttribute("hidden", "");
    const form = $("#wallet-copy-form");
    if (!form) return;
    form.hidden = false;
    $("#wallet-copy-form-title").textContent = task ? task.label || "Copy task" : "New copy task";
    $("#wallet-copy-task-id").value = task?.id || "";
    $("#wallet-copy-delete").hidden = !task;
    $("#wallet-copy-address").value = task?.target_address || "";
    $("#wallet-copy-label").value = task?.label || "";
    $("#wallet-copy-enabled").checked = task?.enabled ?? true;
    const sizingKind = task?.sizing?.kind || "fixed";
    $("#wallet-copy-sizing").value = sizingKind;
    $("#wallet-copy-size").value = task
      ? sizingKind === "fixed"
        ? task.sizing.sol
        : task.sizing.pct
      : "0.05";
    $("#wallet-copy-max-trade").value = task?.max_sol_per_trade ?? "0.1";
    $("#wallet-copy-max-token").value = task?.max_sol_per_token ?? "0.5";
    $("#wallet-copy-budget").value = task?.total_budget_sol ?? "2";
    $("#wallet-copy-slippage").value = task?.slippage_pct ?? String(defaultSlippage);
    $("#wallet-copy-exit-mode").value = task?.exit_mode || "buy_only";
    POLICY_CONTROLS.forEach(([control, group, field]) => {
      const override = task?.exit_policy_overrides?.[group] || {};
      const mode =
        override.enabled === true ? "enabled" : override.enabled === false ? "disabled" : "inherit";
      $(`#wallet-copy-${control}-mode`).value = mode;
      const storedValue = override[field];
      $(`#wallet-copy-${control}-value`).value =
        storedValue == null
          ? ""
          : control === "time"
            ? String(Number(storedValue) / 3600)
            : String(storedValue);
      updatePolicyControl(control);
    });
    const formError = $("#wallet-copy-form-error");
    if (formError) formError.textContent = "";
    const modeAction = $("#wallet-copy-mode-action");
    if (modeAction) {
      modeAction.hidden = !task;
      modeAction.dataset.mode = task?.mode || "paper";
      modeAction.textContent = task?.mode === "live" ? "Return to Paper" : "Arm Live";
      modeAction.classList.toggle("btn-danger", task?.mode !== "live");
    }
    const modeState = $("#wallet-copy-task-mode");
    if (modeState)
      modeState.textContent = task?.mode === "live" ? "Live execution" : "Paper execution";
    updateSizeLabel();
    renderTasks();
  }

  async function changeTaskMode(event) {
    const task = tasks.find((item) => item.id === selectedId);
    if (!task) return;
    const requestedMode = task.mode === "live" ? "paper" : "live";
    if (requestedMode === "live") {
      const name = task.label || task.target_address;
      const { confirmed } = await ConfirmationDialog.show({
        title: "Arm Live Copy Trading",
        message: `Allow “${name}” to submit real swaps from your wallet? Per-trade and total-budget limits remain enforced.`,
        confirmLabel: "Arm Live",
        cancelLabel: "Keep Paper",
        variant: "danger",
      });
      if (!confirmed) return;
    }
    event.currentTarget.disabled = true;
    try {
      await requestManager.fetch(`/api/copy-trading/tasks/${task.id}/mode`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mode: requestedMode,
          ...(requestedMode === "live" ? { confirmation: LIVE_ARM_CONFIRMATION } : {}),
        }),
        priority: "high",
        skipDedup: true,
      });
      Utils.showToast(
        requestedMode === "live" ? "Live copy armed" : "Task returned to paper",
        "success"
      );
      await load();
      editTask(task.id);
    } catch (error) {
      console.error("[Trader] Wallet copy mode change failed:", error);
      Utils.showToast("Execution mode could not be changed", "error");
    } finally {
      event.currentTarget.disabled = false;
    }
  }

  function closeEditor() {
    selectedId = null;
    const form = $("#wallet-copy-form");
    if (form) form.hidden = true;
    const formError = $("#wallet-copy-form-error");
    if (formError) formError.textContent = "";
    $("#wallet-copy-editor-empty")?.removeAttribute("hidden");
    renderTasks();
  }

  function updateSizeLabel() {
    const ratio = $("#wallet-copy-sizing")?.value === "ratio_of_target";
    const label = $("#wallet-copy-size-label");
    if (label) label.textContent = ratio ? "Target ratio (%)" : "Size (SOL)";
  }

  function updatePolicyControl(control) {
    const mode = $(`#wallet-copy-${control}-mode`)?.value || "inherit";
    const input = $(`#wallet-copy-${control}-value`);
    if (!input) return;
    input.hidden = mode !== "enabled";
    input.required = mode === "enabled";
  }

  function readPolicyOverrides(currentTask) {
    const current = currentTask?.exit_policy_overrides || {};
    const result = {
      stop_loss: { ...(current.stop_loss || {}) },
      trailing: { ...(current.trailing || {}) },
      roi: { ...(current.roi || {}) },
      time: { ...(current.time || {}) },
    };
    for (const [control, group, field] of POLICY_CONTROLS) {
      const mode = $(`#wallet-copy-${control}-mode`).value;
      result[group].enabled = mode === "inherit" ? null : mode === "enabled";
      if (mode === "enabled") {
        const value = Number($(`#wallet-copy-${control}-value`).value);
        if (!Number.isFinite(value) || value <= 0) return null;
        result[group][field] = control === "time" ? value * 3600 : value;
      } else {
        result[group][field] = null;
      }
    }
    return result;
  }

  async function saveTask(event) {
    event.preventDefault();
    const address = $("#wallet-copy-address")?.value.trim() || "";
    const error = $("#wallet-copy-address-error");
    if (!SOLANA_ADDRESS_RE.test(address)) {
      if (error) error.textContent = "Enter a valid Solana wallet address.";
      return;
    }
    if (error) error.textContent = "";
    const sizingKind = $("#wallet-copy-sizing").value;
    const size = Number($("#wallet-copy-size").value);
    const maxTrade = Number($("#wallet-copy-max-trade").value);
    const maxToken = Number($("#wallet-copy-max-token").value);
    const budget = Number($("#wallet-copy-budget").value);
    const slippage = Number($("#wallet-copy-slippage").value);
    const currentTask = tasks.find((task) => task.id === selectedId);
    const exitPolicyOverrides = readPolicyOverrides(currentTask);
    const formError = $("#wallet-copy-form-error");
    if (
      ![size, maxTrade, maxToken, budget, slippage].every(
        (value) => Number.isFinite(value) && value > 0
      ) ||
      maxTrade > maxToken ||
      maxToken > budget ||
      slippage > 50 ||
      !exitPolicyOverrides
    ) {
      if (formError) {
        formError.textContent =
          "Use positive values, keep per-trade ≤ per-token ≤ total budget, and slippage ≤ 50%.";
      }
      return;
    }
    if (formError) formError.textContent = "";
    const payload = {
      target_address: address,
      label: $("#wallet-copy-label").value.trim() || null,
      enabled: $("#wallet-copy-enabled").checked,
      mode: tasks.find((task) => task.id === selectedId)?.mode || "paper",
      sizing:
        sizingKind === "fixed"
          ? { kind: "fixed", sol: size }
          : { kind: "ratio_of_target", pct: size },
      exit_mode: $("#wallet-copy-exit-mode").value,
      exit_policy_overrides: exitPolicyOverrides,
      max_sol_per_trade: maxTrade,
      max_sol_per_token: maxToken,
      total_budget_sol: budget,
      min_target_trade_sol: null,
      max_target_trade_sol: null,
      buy_once_per_token: true,
      slippage_pct: slippage,
    };
    const submit = event.currentTarget.querySelector('button[type="submit"]');
    if (submit) submit.disabled = true;
    try {
      const id = $("#wallet-copy-task-id").value;
      await requestManager.fetch(id ? `/api/copy-trading/tasks/${id}` : "/api/copy-trading/tasks", {
        method: id ? "PATCH" : "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
        priority: "high",
        skipDedup: true,
      });
      Utils.showToast(id ? "Copy task updated" : "Paper task created", "success");
      closeEditor();
      await load();
    } catch (loadError) {
      console.error("[Trader] Wallet copy save failed:", loadError);
      Utils.showToast("Copy task could not be saved", "error");
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  async function deleteTask(event) {
    const task = tasks.find((item) => item.id === selectedId);
    if (!task) return;
    const { confirmed } = await ConfirmationDialog.show({
      title: "Delete Copy Task",
      message: `Delete copy task "${task.label || task.target_address}"? Its recorded activity will also be removed.`,
      confirmLabel: "Delete",
      cancelLabel: "Cancel",
      variant: "danger",
    });
    if (!confirmed) return;

    event.currentTarget.disabled = true;
    try {
      await requestManager.fetch(`/api/copy-trading/tasks/${task.id}`, {
        method: "DELETE",
        priority: "high",
        skipDedup: true,
      });
      closeEditor();
      await load();
      Utils.showToast("Copy task deleted", "success");
    } catch (deleteError) {
      console.error("[Trader] Wallet copy delete failed:", deleteError);
      Utils.showToast("Copy task could not be deleted", "error");
    } finally {
      event.currentTarget.disabled = false;
    }
  }

  function renderTasks() {
    const root = $("#wallet-copy-task-list");
    const count = $("#wallet-copy-task-count");
    if (count) count.textContent = `${tasks.filter((task) => task.enabled).length} active`;
    if (!root) return;
    const renderKey = JSON.stringify([tasks, selectedId]);
    if (renderKey === lastTaskRenderKey) return;
    lastTaskRenderKey = renderKey;
    if (!tasks.length) {
      root.innerHTML =
        '<div class="wallet-copy-empty"><i class="icon-copy"></i><strong>No copy tasks</strong><span>Create a paper task to measure it safely before going live.</span></div>';
      return;
    }
    root.innerHTML = tasks
      .map((task) => {
        const short = `${task.target_address.slice(0, 6)}…${task.target_address.slice(-4)}`;
        const mode = task.mode === "live" ? "Live" : "Paper";
        const exit = {
          buy_only: "Own exits",
          mirror: "Mirror sells",
          hybrid: "Hybrid exits",
        }[task.exit_mode];
        return `<button type="button" class="wallet-copy-task${task.id === selectedId ? " is-active" : ""}" data-copy-task-id="${task.id}"><strong>${Utils.escapeHtml(task.label || "Unlabelled wallet")}</strong><span>${Utils.escapeHtml(short)}</span><span class="wallet-copy-task-meta"><span>${task.enabled ? "Active" : "Paused"}</span><span>${mode} · ${exit || "Own exits"}</span></span></button>`;
      })
      .join("");
  }

  function renderActivity(items) {
    const root = $("#wallet-copy-activity-list");
    if (!root) return;
    const renderKey = JSON.stringify(items);
    if (renderKey === lastActivityRenderKey) return;
    lastActivityRenderKey = renderKey;
    if (!items.length) {
      root.innerHTML =
        '<div class="wallet-copy-empty"><strong>No decisions yet</strong><span>Paper fills, live submissions and skips will appear here.</span></div>';
      return;
    }
    root.innerHTML = items
      .map((item) => {
        const outcome = item.outcome || {};
        const titles = {
          paper_filled: "Paper fill",
          live_submitted: "Live submitted",
          live_confirmed: "Live confirmed",
          live_failed: "Live failed",
          paper_sell_observed: "Paper sell observed",
          live_sell_submitted: "Copy sell submitted",
          live_sell_failed: "Copy sell failed",
          skipped: "Skipped",
        };
        const title = titles[outcome.outcome] || "Decision";
        const isSkip = outcome.outcome === "skipped";
        const blockKind = outcome.reason?.block?.kind;
        const isSell = outcome.outcome?.includes("sell");
        const isPaperSell = outcome.outcome === "paper_sell_observed";
        const detail = isSkip
          ? blockKind
            ? ENTRY_BLOCK_LABELS[blockKind] || "Entry blocked"
            : SKIP_LABELS[outcome.reason?.kind] || "Policy skip"
          : outcome.error ||
            (isSell
              ? isPaperSell
                ? `Target sold ${outcome.target_token_amount ?? "—"} tokens · observation only`
                : `Full close · target sold ${outcome.target_token_amount ?? "—"} tokens`
              : `${outcome.sized_sol ?? "—"} SOL`);
        const telemetry = outcome.telemetry;
        const arrivalMs =
          telemetry?.target_block_time && telemetry?.detected_at
            ? new Date(telemetry.detected_at).getTime() - Number(telemetry.target_block_time) * 1000
            : null;
        const arrival =
          Number.isFinite(arrivalMs) && arrivalMs >= 0
            ? ` · ${(arrivalMs / 1000).toFixed(1)}s arrival`
            : "";
        const timestamp = telemetry?.decided_at || outcome.decided_at;
        return `<div class="wallet-copy-task"><strong>${title}</strong><span>${Utils.escapeHtml(outcome.mint || outcome.signature || "")}</span><span class="wallet-copy-task-meta"><span>${Utils.escapeHtml(`${detail}${arrival}`)}</span><span>${Utils.escapeHtml(timestamp || item.created_at || "")}</span></span></div>`;
      })
      .join("");
  }

  function renderLoadError() {
    const root = $("#wallet-copy-task-list");
    if (root)
      root.innerHTML =
        '<div class="wallet-copy-empty"><i class="icon-circle-alert"></i><strong>Copy tasks unavailable</strong><span>Try again in a moment.</span></div>';
  }

  function reset() {
    tasks = [];
    selectedId = null;
    setupDone = false;
    lastTaskRenderKey = "";
    lastActivityRenderKey = "";
  }

  return { setup, load, loadStatus, reset };
}
