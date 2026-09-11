// Global copy-trading settings: the copy_trading config section. The arrival
// limit is edited in seconds and stored in milliseconds.
export function createSettings(page) {
  const { $, api, on, toast, dialogs } = page;

  function setup() {
    on($("#copy-settings-form"), "submit", save);
    on($("#copy-settings-filter"), "change", syncWarning);
    on($("#copy-settings-latency"), "change", syncLatency);
  }

  function syncWarning() {
    const warning = $("#copy-settings-filter-warning");
    if (warning) warning.hidden = !$("#copy-settings-filter")?.checked;
  }

  /** The arrival limit and its window only act while the kill switch is on. */
  function syncLatency() {
    const off = !$("#copy-settings-latency")?.checked;
    ["#copy-settings-delay", "#copy-settings-window"].forEach((selector) => {
      const input = $(selector);
      if (input) input.disabled = off;
    });
  }

  function setError(text) {
    const node = $("#copy-settings-error");
    if (node) node.textContent = text;
  }

  async function open() {
    setError("");
    try {
      const config = await api.config();
      $("#copy-settings-filter").checked = Boolean(config.require_filter_pass);
      $("#copy-settings-latency").checked = Boolean(config.latency_kill_switch_enabled);
      $("#copy-settings-delay").value = String(Number(config.max_arrival_distance_ms) / 1000);
      $("#copy-settings-window").value = String(config.latency_window_size);
      $("#copy-settings-slippage").value = String(config.default_slippage_pct);
      $("#copy-settings-max-tasks").value = String(config.max_active_tasks);
      $("#copy-settings-readiness").value = String(config.readiness_min_closed_rounds);
      syncWarning();
      syncLatency();
      dialogs.show("copy-settings");
      $("#copy-settings-delay")?.focus();
    } catch (error) {
      toast("error", "Copy settings could not be loaded", error.detail);
    }
  }

  async function save(event) {
    event.preventDefault();
    const form = event.currentTarget;
    if (!form.reportValidity()) return;
    const number = (selector) => Number($(selector).value);
    const payload = {
      require_filter_pass: $("#copy-settings-filter").checked,
      latency_kill_switch_enabled: $("#copy-settings-latency").checked,
      max_arrival_distance_ms: Math.round(number("#copy-settings-delay") * 1000),
      latency_window_size: number("#copy-settings-window"),
      default_slippage_pct: number("#copy-settings-slippage"),
      max_active_tasks: number("#copy-settings-max-tasks"),
      readiness_min_closed_rounds: number("#copy-settings-readiness"),
    };
    const submit = form.querySelector('button[type="submit"]');
    if (submit) submit.disabled = true;
    try {
      await api.patchConfig(payload);
      dialogs.hide("copy-settings");
      toast("success", "Copy trading settings saved");
      await Promise.all([page.reload(), page.reloadDefaults()]);
    } catch (error) {
      setError(error.detail);
    } finally {
      if (submit) submit.disabled = false;
    }
  }

  return { setup, open };
}
