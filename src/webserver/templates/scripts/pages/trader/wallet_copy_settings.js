// Wallet Copy global settings dialog: the copy_trading config section it edits.

export function createWalletCopySettings({ $, Utils, requestManager, reload }) {
  async function open() {
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

  function close() {
    $("#wallet-copy-settings-dialog")?.classList.add("hidden");
  }

  async function save(event) {
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
      await patch(payload);
      close();
      Utils.showToast("Copy trading settings saved", "success");
      await reload();
    } catch (error) {
      console.error("[Trader] Copy settings save failed:", error);
      setText("#wallet-copy-settings-error", "Copy settings could not be saved.");
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  function patch(payload) {
    return requestManager.fetch("/api/config/copy_trading", {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      priority: "high",
      skipDedup: true,
    });
  }

  function setText(selector, value) {
    const node = $(selector);
    if (node) node.textContent = value;
  }

  return { open, close, save, patch };
}
