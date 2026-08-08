const SOLANA_ADDRESS_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

export function createWalletCopy({ $, Utils, requestManager, ConfirmationDialog }) {
  let tasks = [];
  let selectedId = null;
  let setupDone = false;
  let defaultSlippage = 2;

  function setup(on) {
    if (setupDone) return;
    setupDone = true;
    on($("#wallet-copy-new-task"), "click", () => editTask(null));
    on($("#wallet-copy-cancel"), "click", closeEditor);
    on($("#wallet-copy-form"), "submit", saveTask);
    on($("#wallet-copy-delete"), "click", deleteTask);
    on($("#wallet-copy-task-list"), "click", (event) => {
      const button = event.target.closest("button[data-copy-task-id]");
      if (button) editTask(Number(button.dataset.copyTaskId));
    });
    on($("#wallet-copy-sizing"), "change", updateSizeLabel);
    on($("#wallet-copy-global-toggle"), "change", toggleGlobal);
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
      const globalToggle = $("#wallet-copy-global-toggle");
      if (globalToggle) globalToggle.checked = Boolean(statusData.enabled);
      defaultSlippage = Number(statusData.default_slippage_pct) || 2;
      const globalStatus = $("#wallet-copy-global-status");
      if (globalStatus) globalStatus.textContent = statusData.enabled ? "Paper active" : "Paper paused";
      if (selectedId && !tasks.some((task) => task.id === selectedId)) closeEditor();
    } catch (error) {
      console.error("[Trader] Wallet copy load failed:", error);
      renderLoadError();
    }
  }

  async function toggleGlobal(event) {
    const enabled = event.currentTarget.checked;
    event.currentTarget.disabled = true;
    try {
      await requestManager.fetch("/api/config/copy_trading", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled }),
        priority: "high",
        skipDedup: true,
      });
      Utils.showToast(enabled ? "Paper copying resumed" : "Paper copying paused", "success");
      await load();
    } catch {
      event.currentTarget.checked = !enabled;
      Utils.showToast("Paper copy status could not be changed", "error");
    } finally {
      event.currentTarget.disabled = false;
    }
  }

  function editTask(id) {
    selectedId = id;
    const task = tasks.find((item) => item.id === id) || null;
    $("#wallet-copy-editor-empty")?.setAttribute("hidden", "");
    const form = $("#wallet-copy-form");
    if (!form) return;
    form.hidden = false;
    $("#wallet-copy-form-title").textContent = task ? task.label || "Paper task" : "New paper task";
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
    updateSizeLabel();
    renderTasks();
  }

  function closeEditor() {
    selectedId = null;
    const form = $("#wallet-copy-form");
    if (form) form.hidden = true;
    $("#wallet-copy-editor-empty")?.removeAttribute("hidden");
    renderTasks();
  }

  function updateSizeLabel() {
    const ratio = $("#wallet-copy-sizing")?.value === "ratio_of_target";
    const label = $("#wallet-copy-size-label");
    if (label) label.textContent = ratio ? "Target ratio (%)" : "Size (SOL)";
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
    const payload = {
      target_address: address,
      label: $("#wallet-copy-label").value.trim() || null,
      enabled: $("#wallet-copy-enabled").checked,
      mode: "paper",
      sizing: sizingKind === "fixed" ? { kind: "fixed", sol: size } : { kind: "ratio_of_target", pct: size },
      exit_mode: "buy_only",
      max_sol_per_trade: Number($("#wallet-copy-max-trade").value),
      max_sol_per_token: Number($("#wallet-copy-max-token").value),
      total_budget_sol: Number($("#wallet-copy-budget").value),
      min_target_trade_sol: null,
      max_target_trade_sol: null,
      buy_once_per_token: true,
      slippage_pct: Number($("#wallet-copy-slippage").value),
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
      Utils.showToast(id ? "Paper task updated" : "Paper task created", "success");
      closeEditor();
      await load();
    } catch (loadError) {
      console.error("[Trader] Wallet copy save failed:", loadError);
      Utils.showToast("Paper task could not be saved", "error");
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  async function deleteTask(event) {
    const task = tasks.find((item) => item.id === selectedId);
    if (!task) return;
    const { confirmed } = await ConfirmationDialog.show({
      title: "Delete Copy Task",
      message: `Delete paper task "${task.label || task.target_address}"? Its recorded activity will also be removed.`,
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
      Utils.showToast("Paper task deleted", "success");
    } catch (deleteError) {
      console.error("[Trader] Wallet copy delete failed:", deleteError);
      Utils.showToast("Paper task could not be deleted", "error");
    } finally {
      event.currentTarget.disabled = false;
    }
  }

  function renderTasks() {
    const root = $("#wallet-copy-task-list");
    const count = $("#wallet-copy-task-count");
    if (count) count.textContent = `${tasks.filter((task) => task.enabled).length} active`;
    if (!root) return;
    if (!tasks.length) {
      root.innerHTML = '<div class="wallet-copy-empty"><i class="icon-copy"></i><strong>No copy tasks</strong><span>Create a paper task to begin measuring decisions.</span></div>';
      return;
    }
    root.innerHTML = tasks
      .map((task) => {
        const short = `${task.target_address.slice(0, 6)}…${task.target_address.slice(-4)}`;
        return `<button type="button" class="wallet-copy-task${task.id === selectedId ? " is-active" : ""}" data-copy-task-id="${task.id}"><strong>${Utils.escapeHtml(task.label || "Unlabelled wallet")}</strong><span>${Utils.escapeHtml(short)}</span><span class="wallet-copy-task-meta"><span>${task.enabled ? "Active" : "Paused"}</span><span>Paper</span></span></button>`;
      })
      .join("");
  }

  function renderActivity(items) {
    const root = $("#wallet-copy-activity-list");
    if (!root) return;
    if (!items.length) {
      root.innerHTML = '<div class="wallet-copy-empty"><strong>No decisions yet</strong><span>Observed paper fills and skips will appear here.</span></div>';
      return;
    }
    root.innerHTML = items
      .map((item) => {
        const outcome = item.outcome || {};
        const filled = outcome.outcome === "paper_filled";
        const title = filled ? "Paper fill" : "Skipped";
        const detail = filled ? `${outcome.sized_sol ?? "—"} SOL` : outcome.reason?.kind || "Policy skip";
        const timestamp = filled ? outcome.telemetry?.decided_at : outcome.decided_at;
        return `<div class="wallet-copy-task"><strong>${title}</strong><span>${Utils.escapeHtml(outcome.mint || outcome.signature || "")}</span><span class="wallet-copy-task-meta"><span>${Utils.escapeHtml(detail)}</span><span>${Utils.escapeHtml(timestamp || item.created_at || "")}</span></span></div>`;
      })
      .join("");
  }

  function renderLoadError() {
    const root = $("#wallet-copy-task-list");
    if (root) root.innerHTML = '<div class="wallet-copy-empty"><i class="icon-circle-alert"></i><strong>Copy tasks unavailable</strong><span>Try again in a moment.</span></div>';
  }

  function reset() {
    tasks = [];
    selectedId = null;
    setupDone = false;
  }

  return { setup, load, reset };
}
