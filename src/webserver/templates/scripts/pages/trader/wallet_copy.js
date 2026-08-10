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
  position_user_only: "Position is managed by the user",
  position_management_mismatch: "Position ownership no longer permits copy sells",
  latency_kill_switch: "Task auto-paused because target activity arrived too late",
  claim_reconciled_abandoned: "Interrupted live submission was closed without retrying",
};

const ENTRY_BLOCK_LABELS = {
  force_stopped: "Trading is force-stopped",
  loss_limit: "Loss limit blocks new entries",
  connectivity: "Required services are unavailable",
  position_limit: "Open-position limit reached",
  already_open: "A position is already open",
  reentry_cooldown: "Token re-entry cooldown is active",
  open_cooldown: "Global entry cooldown is active",
  entry_reserved: "Another entry is processing",
  blacklisted: "Token is blocked by risk controls",
  check_failed: "A safety check could not complete",
};

const POLICY_CONTROLS = [
  ["stop-loss", "stop_loss", "threshold_pct"],
  ["roi", "roi", "target_profit_pct"],
  ["trailing", "trailing", "distance_pct"],
  ["time", "time", "duration_seconds"],
];

const STATE_LABELS = {
  system_paused: "Paused globally",
  force_stopped: "Force stopped",
  paused: "Paused",
  entries_blocked: "Entries blocked",
  live: "Live",
  paper: "Paper",
};

export function createWalletCopy({ $, Utils, requestManager, ConfirmationDialog }) {
  let tasks = [];
  let activity = [];
  let status = {};
  let selectedId = null;
  let editingId = null;
  let setupDone = false;
  let defaultSlippage = 2;
  let lastRenderKey = "";

  function setup(on) {
    if (setupDone) return;
    setupDone = true;
    [$("#wallet-copy-new-task"), $("#wallet-copy-onboarding-add")]
      .filter(Boolean)
      .forEach((button) => {
        on(button, "click", () => openEditor(null));
      });
    on($("#wallet-copy-global-action"), "click", toggleGlobal);
    on($("#wallet-copy-settings"), "click", openSettings);
    on($("#wallet-copy-settings-close"), "click", closeSettings);
    on($("#wallet-copy-settings-cancel"), "click", closeSettings);
    on($("#wallet-copy-settings-form"), "submit", saveSettings);
    on($("#wallet-copy-editor-close"), "click", closeEditor);
    on($("#wallet-copy-cancel"), "click", closeEditor);
    on($("#wallet-copy-form"), "submit", saveTask);
    on($("#wallet-copy-edit-task"), "click", () => openEditor(selectedId));
    on($("#wallet-copy-task-toggle"), "click", toggleTask);
    on($("#wallet-copy-mode-action"), "click", changeTaskMode);
    on($("#wallet-copy-task-list"), "click", (event) => {
      const button = event.target.closest("button[data-copy-task-id]");
      if (button) selectTask(Number(button.dataset.copyTaskId));
    });
    on($("#wallet-copy-sizing"), "change", updateSizeLabel);
    POLICY_CONTROLS.forEach(([control]) => {
      on($(`#wallet-copy-${control}-mode`), "change", () => updatePolicyControl(control));
    });
    [$("#wallet-copy-editor-dialog"), $("#wallet-copy-settings-dialog")]
      .filter(Boolean)
      .forEach((overlay) =>
        on(overlay, "click", (event) => {
          if (event.target !== overlay) return;
          if (overlay.id === "wallet-copy-editor-dialog") closeEditor();
          else closeSettings();
        })
      );
    on(document, "keydown", (event) => {
      if (event.key !== "Escape") return;
      if (!$("#wallet-copy-settings-dialog")?.classList.contains("hidden")) closeSettings();
      else if (!$("#wallet-copy-editor-dialog")?.classList.contains("hidden")) closeEditor();
    });
  }

  async function load() {
    try {
      const overview = await requestManager.fetch("/api/copy-trading/overview");
      tasks = overview.tasks || [];
      activity = overview.activity || [];
      status = overview.status || {};
      defaultSlippage = Number(status.default_slippage_pct) || 2;
      if (!selectedId || !tasks.some((task) => task.id === selectedId)) {
        selectedId = tasks[0]?.id || null;
      }
      render();
    } catch (error) {
      console.error("[Trader] Copy trading overview failed:", error);
      renderLoadError();
    }
  }

  function render() {
    const renderKey = JSON.stringify([tasks, activity, status, selectedId]);
    if (renderKey === lastRenderKey) return;
    lastRenderKey = renderKey;
    const hasTasks = tasks.length > 0;
    $(".wallet-copy-detail")?.removeAttribute("aria-busy");
    $("#wallet-copy-onboarding")?.toggleAttribute("hidden", hasTasks);
    $("#wallet-copy-operations")?.toggleAttribute("hidden", !hasTasks);
    renderSystemState();
    if (!hasTasks) return;
    renderSummary();
    renderTasks();
    renderSelectedTask();
  }

  function renderSystemState() {
    const label = !status.enabled
      ? "Paused globally"
      : status.blocked_reason === "force_stop"
        ? "Force stopped"
        : status.blocked_reason === "loss_limit"
          ? "Entries blocked · exits active"
          : status.live_tasks
            ? `${status.live_tasks} Live · ${status.paper_tasks || 0} Paper`
            : `${status.paper_tasks || 0} Paper`;
    const root = $("#wallet-copy-global-status");
    if (root) root.textContent = label;
    const action = $("#wallet-copy-global-action");
    if (action) {
      action.textContent = status.enabled ? "Pause all" : "Resume processing";
      action.dataset.enabled = String(Boolean(status.enabled));
      action.disabled = !tasks.length;
    }
  }

  function renderSummary() {
    const active = tasks.filter((task) => task.enabled).length;
    const spent = tasks.reduce((sum, task) => sum + number(task.spent_sol), 0);
    const budget = tasks.reduce((sum, task) => sum + number(task.total_budget_sol), 0);
    const positions = tasks.reduce((sum, task) => sum + number(task.stats?.open_positions), 0);
    const pnl = tasks.reduce(
      (sum, task) =>
        sum + number(task.stats?.realized_pnl_sol) + number(task.stats?.unrealized_pnl_sol),
      0
    );
    const latencies = tasks
      .map((task) => task.stats?.arrival_distance?.p95_ms)
      .filter((value) => Number.isFinite(Number(value)))
      .map(Number);
    setText("#wallet-copy-summary-tasks", `${active} active · ${tasks.length - active} paused`);
    setText(
      "#wallet-copy-summary-mode",
      `${status.live_tasks || 0} Live · ${status.paper_tasks || 0} Paper`
    );
    setText("#wallet-copy-summary-budget", `${spent.toFixed(2)} / ${budget.toFixed(2)} SOL`);
    setText("#wallet-copy-summary-positions", `${positions} open`);
    setText("#wallet-copy-summary-pnl", signedSol(pnl));
    setText(
      "#wallet-copy-summary-latency",
      latencies.length ? `${(Math.max(...latencies) / 1000).toFixed(1)}s worst p95` : "No samples"
    );
  }

  function renderTasks() {
    const root = $("#wallet-copy-task-list");
    if (!root) return;
    const active = tasks.filter((task) => task.enabled).length;
    setText("#wallet-copy-task-count", `${active} active · ${tasks.length} total`);
    root.innerHTML = tasks
      .map((task) => {
        const short = `${task.target_address.slice(0, 6)}…${task.target_address.slice(-4)}`;
        const pnl = number(task.stats?.realized_pnl_sol) + number(task.stats?.unrealized_pnl_sol);
        const budgetPct =
          task.total_budget_sol > 0
            ? Math.min(100, (number(task.spent_sol) / number(task.total_budget_sol)) * 100)
            : 0;
        return `<button type="button" class="wallet-copy-task${task.id === selectedId ? " is-active" : ""}" data-copy-task-id="${task.id}" aria-pressed="${task.id === selectedId}"><span class="wallet-copy-task-name">${Utils.escapeHtml(task.label || "Unnamed wallet")}</span><span class="wallet-copy-state wallet-copy-state-${Utils.escapeHtml(task.effective_state)}">${Utils.escapeHtml(STATE_LABELS[task.effective_state] || "Unknown")}</span><span class="wallet-copy-task-address">${Utils.escapeHtml(short)}</span><span class="wallet-copy-task-pnl">${Utils.escapeHtml(signedSol(pnl))}</span><span class="wallet-copy-task-budget"><span style="width:${budgetPct.toFixed(1)}%"></span></span><span class="wallet-copy-task-meta">${number(task.stats?.open_positions)} open · ${number(task.spent_sol).toFixed(2)} / ${number(task.total_budget_sol).toFixed(2)} SOL</span></button>`;
      })
      .join("");
  }

  function selectTask(id) {
    if (!tasks.some((task) => task.id === id)) return;
    selectedId = id;
    lastRenderKey = "";
    render();
  }

  function renderSelectedTask() {
    const task = currentTask();
    if (!task) return;
    setText("#wallet-copy-detail-state", STATE_LABELS[task.effective_state] || "Unknown");
    $("#wallet-copy-detail-state").className =
      `wallet-copy-state wallet-copy-state-${task.effective_state}`;
    setText("#wallet-copy-detail-title", task.label || "Unnamed wallet");
    setText("#wallet-copy-detail-address", task.target_address);
    const pnl = number(task.stats?.realized_pnl_sol) + number(task.stats?.unrealized_pnl_sol);
    setText("#wallet-copy-stats-pnl", signedSol(pnl));
    setText(
      "#wallet-copy-stats-arrival",
      task.stats?.arrival_distance?.p95_ms == null
        ? "No samples"
        : `${(number(task.stats.arrival_distance.p95_ms) / 1000).toFixed(1)}s`
    );
    setText(
      "#wallet-copy-stats-positions",
      `${number(task.stats?.open_positions)} open · ${number(task.stats?.closed_positions)} closed`
    );
    setText("#wallet-copy-stats-decisions", String(number(task.stats?.decisions)));
    const spent = number(task.spent_sol);
    const budget = number(task.total_budget_sol);
    const budgetPct = budget > 0 ? Math.min(100, (spent / budget) * 100) : 0;
    setText(
      "#wallet-copy-budget-caption",
      `${spent.toFixed(3)} spent · ${number(task.remaining_budget_sol).toFixed(3)} SOL remaining`
    );
    const fill = $("#wallet-copy-budget-fill");
    if (fill) fill.style.width = `${budgetPct}%`;
    $("#wallet-copy-sizing-summary").innerHTML = definitionRows([
      ["Sizing", formatSizing(task.sizing)],
      ["Per trade", `≤ ${number(task.max_sol_per_trade).toFixed(3)} SOL`],
      ["Per token", `≤ ${number(task.max_sol_per_token).toFixed(3)} SOL`],
      ["Total budget", `${budget.toFixed(3)} SOL`],
    ]);
    $("#wallet-copy-rules-summary").innerHTML = definitionRows([
      ["Exit ownership", formatExitMode(task.exit_mode)],
      ["Stop loss", formatPolicy(task.exit_policy_overrides?.stop_loss, "threshold_pct", "%")],
      ["Take profit", formatPolicy(task.exit_policy_overrides?.roi, "target_profit_pct", "%")],
      ["Trailing stop", formatPolicy(task.exit_policy_overrides?.trailing, "distance_pct", "%")],
      ["Maximum hold", formatTimePolicy(task.exit_policy_overrides?.time)],
    ]);
    const taskToggle = $("#wallet-copy-task-toggle");
    if (taskToggle) taskToggle.textContent = task.enabled ? "Pause task" : "Resume task";
    const modeAction = $("#wallet-copy-mode-action");
    if (modeAction) modeAction.textContent = task.mode === "live" ? "Return to Paper" : "Arm Live";
    renderActivity(activity.filter((item) => item.task_id === task.id));
  }

  async function toggleGlobal() {
    const enabled = !status.enabled;
    if (enabled && number(status.live_tasks) > 0) {
      const result = await ConfirmationDialog.show({
        title: "Resume Copy Trading",
        message: `Resume ${status.live_tasks} Live and ${status.paper_tasks || 0} Paper tasks? Live tasks may submit swaps when target wallets trade.`,
        confirmLabel: "Resume Processing",
        cancelLabel: "Keep Paused",
        variant: "danger",
      });
      if (!result.confirmed) return;
    }
    const button = $("#wallet-copy-global-action");
    if (button) button.disabled = true;
    try {
      await patchGlobalSettings({ enabled });
      Utils.showToast(
        enabled ? "Copy processing resumed" : "All copy processing paused",
        "success"
      );
      await load();
    } catch (error) {
      console.error("[Trader] Copy processing change failed:", error);
      Utils.showToast("Copy processing could not be changed", "error");
    } finally {
      if (button) button.disabled = false;
    }
  }

  async function toggleTask(event) {
    const task = currentTask();
    if (!task) return;
    event.currentTarget.disabled = true;
    try {
      await updateTask(task, { enabled: !task.enabled });
      Utils.showToast(task.enabled ? "Copy task paused" : "Copy task resumed", "success");
      await load();
    } catch (error) {
      console.error("[Trader] Copy task state failed:", error);
      Utils.showToast("Copy task state could not be changed", "error");
    } finally {
      event.currentTarget.disabled = false;
    }
  }

  async function changeTaskMode(event) {
    const task = currentTask();
    if (!task) return;
    const requestedMode = task.mode === "live" ? "paper" : "live";
    if (requestedMode === "live") {
      const result = await ConfirmationDialog.show({
        title: "Arm Live Copy Trading",
        message: `Allow “${task.label || task.target_address}” to submit real swaps? Task budgets and trading safety remain enforced.`,
        confirmLabel: "Arm Live",
        cancelLabel: "Keep Paper",
        variant: "danger",
      });
      if (!result.confirmed) return;
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
        requestedMode === "live" ? "Live copy armed" : "Task returned to Paper",
        "success"
      );
      await load();
    } catch (error) {
      console.error("[Trader] Copy mode change failed:", error);
      Utils.showToast("Execution mode could not be changed", "error");
    } finally {
      event.currentTarget.disabled = false;
    }
  }

  function openEditor(id) {
    editingId = id;
    const task = tasks.find((item) => item.id === id) || null;
    setText("#wallet-copy-form-title", task ? task.label || "Edit wallet" : "Add wallet");
    setText("#wallet-copy-task-mode", task?.mode === "live" ? "Live mode" : "Paper mode");
    $("#wallet-copy-task-id").value = task?.id || "";
    $("#wallet-copy-address").value = task?.target_address || "";
    $("#wallet-copy-label").value = task?.label || "";
    $("#wallet-copy-enabled").checked = task?.enabled ?? true;
    const sizingKind = task?.sizing?.kind || "fixed";
    setSelectValue("#wallet-copy-sizing", sizingKind);
    $("#wallet-copy-size").value = task
      ? sizingKind === "fixed"
        ? task.sizing.sol
        : task.sizing.pct
      : "0.05";
    $("#wallet-copy-max-trade").value = task?.max_sol_per_trade ?? "0.1";
    $("#wallet-copy-max-token").value = task?.max_sol_per_token ?? "0.5";
    $("#wallet-copy-budget").value = task?.total_budget_sol ?? "2";
    $("#wallet-copy-slippage").value = task?.slippage_pct ?? String(defaultSlippage);
    setSelectValue("#wallet-copy-exit-mode", task?.exit_mode || "buy_only");
    POLICY_CONTROLS.forEach(([control, group, field]) => {
      const override = task?.exit_policy_overrides?.[group] || {};
      setSelectValue(
        "#wallet-copy-" + control + "-mode",
        override.enabled === true ? "enabled" : override.enabled === false ? "disabled" : "inherit"
      );
      const storedValue = override[field];
      $("#wallet-copy-" + control + "-value").value =
        storedValue == null
          ? ""
          : control === "time"
            ? String(number(storedValue) / 3600)
            : String(storedValue);
      updatePolicyControl(control);
    });
    setText("#wallet-copy-address-error", "");
    setText("#wallet-copy-form-error", "");
    updateSizeLabel();
    $("#wallet-copy-editor-dialog")?.classList.remove("hidden");
    $("#wallet-copy-address")?.focus();
  }

  function closeEditor() {
    editingId = null;
    $("#wallet-copy-editor-dialog")?.classList.add("hidden");
  }

  function updateSizeLabel() {
    setText(
      "#wallet-copy-size-label",
      $("#wallet-copy-sizing")?.value === "ratio_of_target"
        ? "Wallet trade share (%)"
        : "Copy amount (SOL)"
    );
  }

  function updatePolicyControl(control) {
    const input = $("#wallet-copy-" + control + "-value");
    if (!input) return;
    const enabled = $("#wallet-copy-" + control + "-mode")?.value === "enabled";
    input.hidden = !enabled;
    input.required = enabled;
  }

  function readPolicyOverrides(task) {
    const current = task?.exit_policy_overrides || {};
    const result = {
      stop_loss: { ...(current.stop_loss || {}) },
      trailing: { ...(current.trailing || {}) },
      roi: { ...(current.roi || {}) },
      time: { ...(current.time || {}) },
    };
    for (const [control, group, field] of POLICY_CONTROLS) {
      const mode = $("#wallet-copy-" + control + "-mode").value;
      result[group].enabled = mode === "inherit" ? null : mode === "enabled";
      if (mode === "enabled") {
        const value = Number($("#wallet-copy-" + control + "-value").value);
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
    if (!SOLANA_ADDRESS_RE.test(address)) {
      setText("#wallet-copy-address-error", "Enter a valid Solana wallet address.");
      return;
    }
    setText("#wallet-copy-address-error", "");
    const current = tasks.find((task) => task.id === editingId);
    const size = Number($("#wallet-copy-size").value);
    const maxTrade = Number($("#wallet-copy-max-trade").value);
    const maxToken = Number($("#wallet-copy-max-token").value);
    const budget = Number($("#wallet-copy-budget").value);
    const slippage = Number($("#wallet-copy-slippage").value);
    const exitPolicyOverrides = readPolicyOverrides(current);
    if (
      ![size, maxTrade, maxToken, budget, slippage].every(
        (value) => Number.isFinite(value) && value > 0
      ) ||
      maxTrade > maxToken ||
      maxToken > budget ||
      slippage > 50 ||
      !exitPolicyOverrides
    ) {
      setText(
        "#wallet-copy-form-error",
        "Use positive values, keep per-trade ≤ per-token ≤ total budget, and slippage ≤ 50%."
      );
      return;
    }
    setText("#wallet-copy-form-error", "");
    const sizingKind = $("#wallet-copy-sizing").value;
    const payload = {
      target_address: address,
      label: $("#wallet-copy-label").value.trim() || null,
      enabled: $("#wallet-copy-enabled").checked,
      mode: current?.mode || "paper",
      sizing:
        sizingKind === "fixed"
          ? { kind: "fixed", sol: size }
          : { kind: "ratio_of_target", pct: size },
      exit_mode: $("#wallet-copy-exit-mode").value,
      exit_policy_overrides: exitPolicyOverrides,
      max_sol_per_trade: maxTrade,
      max_sol_per_token: maxToken,
      total_budget_sol: budget,
      min_target_trade_sol: current?.min_target_trade_sol ?? null,
      max_target_trade_sol: current?.max_target_trade_sol ?? null,
      buy_once_per_token: current?.buy_once_per_token ?? true,
      slippage_pct: slippage,
    };
    const isEditing = Boolean(editingId);
    const submit = event.currentTarget.querySelector('button[type="submit"]');
    if (submit) submit.disabled = true;
    try {
      const response = await requestManager.fetch(
        editingId ? `/api/copy-trading/tasks/${editingId}` : "/api/copy-trading/tasks",
        {
          method: editingId ? "PATCH" : "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
          priority: "high",
          skipDedup: true,
        }
      );
      selectedId = response.task?.id || editingId || selectedId;
      closeEditor();
      Utils.showToast(isEditing ? "Copy task updated" : "Paper task created", "success");
      await load();
    } catch (error) {
      console.error("[Trader] Copy task save failed:", error);
      setText("#wallet-copy-form-error", "Copy task could not be saved.");
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  async function updateTask(task, overrides) {
    return requestManager.fetch(`/api/copy-trading/tasks/${task.id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        target_address: task.target_address,
        label: task.label,
        enabled: task.enabled,
        mode: task.mode,
        sizing: task.sizing,
        exit_mode: task.exit_mode,
        exit_policy_overrides: task.exit_policy_overrides,
        max_sol_per_trade: task.max_sol_per_trade,
        max_sol_per_token: task.max_sol_per_token,
        total_budget_sol: task.total_budget_sol,
        min_target_trade_sol: task.min_target_trade_sol,
        max_target_trade_sol: task.max_target_trade_sol,
        buy_once_per_token: task.buy_once_per_token,
        slippage_pct: task.slippage_pct,
        ...overrides,
      }),
      priority: "high",
      skipDedup: true,
    });
  }

  async function openSettings() {
    const dialog = $("#wallet-copy-settings-dialog");
    if (!dialog) return;
    try {
      const response = await requestManager.fetch("/api/config/copy_trading");
      const config =
        response.copy_trading || response.data?.copy_trading || response.data || response;
      $("#wallet-copy-require-filter").checked = Boolean(config.require_filter_pass);
      $("#wallet-copy-latency-enabled").checked = Boolean(config.latency_kill_switch_enabled);
      $("#wallet-copy-default-slippage").value = config.default_slippage_pct;
      $("#wallet-copy-max-tasks").value = config.max_active_tasks;
      $("#wallet-copy-max-delay").value = config.max_arrival_distance_ms;
      $("#wallet-copy-latency-window").value = config.latency_window_size;
      setText("#wallet-copy-settings-error", "");
      dialog.classList.remove("hidden");
    } catch (error) {
      console.error("[Trader] Copy settings load failed:", error);
      Utils.showToast("Copy settings could not be loaded", "error");
    }
  }

  function closeSettings() {
    $("#wallet-copy-settings-dialog")?.classList.add("hidden");
  }

  async function saveSettings(event) {
    event.preventDefault();
    const payload = {
      require_filter_pass: $("#wallet-copy-require-filter").checked,
      latency_kill_switch_enabled: $("#wallet-copy-latency-enabled").checked,
      default_slippage_pct: Number($("#wallet-copy-default-slippage").value),
      max_active_tasks: Number($("#wallet-copy-max-tasks").value),
      max_arrival_distance_ms: Number($("#wallet-copy-max-delay").value),
      latency_window_size: Number($("#wallet-copy-latency-window").value),
    };
    const submit = event.currentTarget.querySelector('button[type="submit"]');
    if (submit) submit.disabled = true;
    try {
      await patchGlobalSettings(payload);
      closeSettings();
      Utils.showToast("Copy trading settings saved", "success");
      await load();
    } catch (error) {
      console.error("[Trader] Copy settings save failed:", error);
      setText("#wallet-copy-settings-error", "Copy settings could not be saved.");
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  function patchGlobalSettings(payload) {
    return requestManager.fetch("/api/config/copy_trading", {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      priority: "high",
      skipDedup: true,
    });
  }

  function renderActivity(items) {
    const root = $("#wallet-copy-activity-list");
    const columns = $("#wallet-copy-activity-columns");
    if (!root) return;
    if (columns) columns.hidden = !items.length;
    if (!items.length) {
      root.innerHTML =
        '<div class="wallet-copy-activity-empty"><strong>No decisions yet</strong><span>Paper fills, live submissions and policy skips will appear here.</span></div>';
      return;
    }
    root.innerHTML = items.slice(0, 30).map(renderActivityRow).join("");
  }

  function renderActivityRow(item) {
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
    const isSkip = outcome.outcome === "skipped";
    const blockKind = outcome.reason?.block?.kind;
    const isSell = outcome.outcome?.includes("sell");
    const detail = isSkip
      ? blockKind
        ? ENTRY_BLOCK_LABELS[blockKind] || "Entry blocked"
        : SKIP_LABELS[outcome.reason?.kind] || "Policy skip"
      : outcome.error ||
        (isSell
          ? outcome.outcome === "paper_sell_observed"
            ? `Target sold ${outcome.target_token_amount ?? "—"} tokens · observation only`
            : `${outcome.exit_percentage == null ? "Full close" : `${number(outcome.exit_percentage).toFixed(1)}% exit`} · target sold ${outcome.target_token_amount ?? "—"} tokens`
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
    const identity = outcome.mint || outcome.signature || "—";
    const timestamp = telemetry?.decided_at || outcome.decided_at || item.created_at;
    return `<div class="wallet-copy-activity-row"><strong>${Utils.escapeHtml(titles[outcome.outcome] || "Decision")}</strong><span class="wallet-copy-activity-detail"><span class="wallet-copy-activity-mint" title="${Utils.escapeHtml(identity)}">${Utils.escapeHtml(identity)}</span><span class="wallet-copy-activity-result">${Utils.escapeHtml(detail + arrival)}</span></span><time class="wallet-copy-activity-time">${Utils.escapeHtml(formatActivityTime(timestamp))}</time></div>`;
  }

  function renderLoadError() {
    $("#wallet-copy-onboarding")?.setAttribute("hidden", "");
    const operations = $("#wallet-copy-operations");
    if (operations) operations.hidden = false;
    const taskList = $("#wallet-copy-task-list");
    if (taskList) taskList.innerHTML = "";
    const detail = $(".wallet-copy-detail");
    if (detail) detail.setAttribute("aria-busy", "true");
    setText("#wallet-copy-detail-title", "Copy trading is unavailable");
    setText("#wallet-copy-detail-address", "Task and execution status could not be loaded.");
  }

  function reset() {
    tasks = [];
    activity = [];
    status = {};
    selectedId = null;
    editingId = null;
    setupDone = false;
    lastRenderKey = "";
  }

  function currentTask() {
    return tasks.find((task) => task.id === selectedId) || null;
  }

  function setText(selector, value) {
    const node = $(selector);
    if (node) node.textContent = value;
  }

  function setSelectValue(selector, value) {
    const select = $(selector);
    if (!select) return;
    select.value = value;
    select._customSelectInstance?.setValue(String(value));
  }

  function number(value) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : 0;
  }

  function signedSol(value) {
    return `${value >= 0 ? "+" : ""}${number(value).toFixed(4)} SOL`;
  }

  function formatSizing(sizing) {
    if (sizing?.kind === "ratio_of_target")
      return `${number(sizing.pct).toFixed(1)}% of target trade`;
    return `${number(sizing?.sol).toFixed(3)} SOL fixed`;
  }

  function formatExitMode(mode) {
    return (
      {
        buy_only: "Use my exit rules",
        mirror: "Mirror wallet sells",
        hybrid: "Wallet sells + my rules",
      }[mode] || "Use my exit rules"
    );
  }

  function formatPolicy(policy, field, unit) {
    if (policy?.enabled === false) return "Disabled for this task";
    if (policy?.enabled === true) return `${number(policy[field]).toFixed(1)}${unit}`;
    return "Use Trader default";
  }

  function formatTimePolicy(policy) {
    if (policy?.enabled === false) return "Disabled for this task";
    if (policy?.enabled === true)
      return `${(number(policy.duration_seconds) / 3600).toFixed(1)} hours`;
    return "Use Trader default";
  }

  function definitionRows(rows) {
    return rows
      .map(
        ([label, value]) =>
          `<div><dt>${Utils.escapeHtml(label)}</dt><dd>${Utils.escapeHtml(value)}</dd></div>`
      )
      .join("");
  }

  function formatActivityTime(value) {
    if (!value) return "—";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return String(value);
    return date.toLocaleString([], {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  return { setup, load, reset };
}
