//! Per-card Save / Reset controls for the Auto Trader configuration tabs.
//!
//! Each editable config card gets a Save and a Reset button injected into its
//! header (to the left of the feature on/off toggle). The buttons stay hidden
//! until the card has unsaved changes — i.e. a field differs from the value last
//! loaded from / saved to the backend. Save POSTs the card's fields to
//! /api/config (applied + hot-reloaded by core via the caller's saveConfig).
//! Reset discards the unsaved edits, restoring the last-saved values.

// Declarative spec: which DOM fields belong to which card, their config section
// and key, and how to coerce the input value into the saved payload. Keys/types
// mirror the backend config schema (trader / positions sections).
const CARD_SPECS = [
  {
    section: "trader",
    fields: [
      { id: "stop-loss-enabled", key: "stop_loss_enabled", type: "bool" },
      { id: "stop-loss-threshold", key: "stop_loss_threshold_pct", type: "float" },
      { id: "stop-loss-allow-partial", key: "stop_loss_allow_partial", type: "bool" },
      { id: "stop-loss-min-hold", key: "stop_loss_min_hold_seconds", type: "int" },
    ],
  },
  {
    section: "positions",
    fields: [
      { id: "trailing-enabled", key: "trailing_stop_enabled", type: "bool" },
      { id: "trail-activation", key: "trailing_stop_activation_pct", type: "float" },
      { id: "trail-distance", key: "trailing_stop_distance_pct", type: "float" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "roi-enabled", key: "roi_exit_enabled", type: "bool" },
      { id: "roi-target", key: "roi_target_percent", type: "float" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "time-override-enabled", key: "time_override_enabled", type: "bool" },
      { id: "time-max-hold", key: "time_override_duration", type: "float" },
      { id: "time-unit", key: "time_override_unit", type: "string" },
      { id: "time-loss-threshold", key: "time_override_loss_threshold_percent", type: "float" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "dca-enabled", key: "dca_enabled", type: "bool" },
      { id: "dca-threshold", key: "dca_threshold_pct", type: "float" },
      { id: "dca-max-count", key: "dca_max_count", type: "int" },
      { id: "dca-size", key: "dca_size_percentage", type: "float" },
      { id: "dca-cooldown", key: "dca_cooldown_minutes", type: "int" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "max-positions", key: "max_open_positions", type: "int" },
      { id: "trade-size", key: "trade_size_sol", type: "float" },
      { id: "entry-sizes", key: "entry_sizes", type: "csv-floats" },
    ],
  },
  {
    section: "trader",
    fields: [
      { id: "close-cooldown", key: "close_cooldown_seconds", type: "minutes-to-seconds" },
      { id: "entry-concurrency", key: "entry_monitor_concurrency", type: "int" },
    ],
  },
];

/** Snapshot string used for dirty comparison (independent of value coercion). */
function snapValue(el) {
  return el.type === "checkbox" ? String(el.checked) : el.value;
}

/** Coerce an input into the value sent to the backend for a given field type. */
function readField(el, type) {
  switch (type) {
    case "bool":
      return el.checked;
    case "int": {
      const n = parseInt(el.value, 10);
      return Number.isNaN(n) ? 0 : n;
    }
    case "float": {
      const n = parseFloat(el.value);
      return Number.isNaN(n) ? 0 : n;
    }
    case "csv-floats":
      return el.value
        .split(",")
        .map((s) => parseFloat(s.trim()))
        .filter((n) => !Number.isNaN(n));
    case "minutes-to-seconds": {
      const m = parseInt(el.value, 10);
      return Math.max(0, Number.isNaN(m) ? 0 : m) * 60;
    }
    default:
      return el.value;
  }
}

/**
 * @param {{ saveConfig: (updates: object) => Promise<void> }} deps
 *   saveConfig POSTs the nested config update, reloads, and (via the caller's
 *   loadConfig) calls snapshot() again so the buttons re-hide on success.
 */
export function createTraderConfigCards({ saveConfig }) {
  /** @type {Array<{spec:object, el:HTMLElement, saved:object, saveBtn:HTMLButtonElement, resetBtn:HTMLButtonElement, cleanups:Array<Function>}>} */
  const cards = [];

  function isDirty(card) {
    return card.spec.fields.some((f) => {
      const el = document.getElementById(f.id);
      return el && snapValue(el) !== card.saved[f.id];
    });
  }

  function evaluate(card) {
    const dirty = isDirty(card);
    card.saveBtn.hidden = !dirty;
    card.resetBtn.hidden = !dirty;
  }

  function hasDirtyCards() {
    return cards.some((card) => isDirty(card));
  }

  async function save(card) {
    const payload = { [card.spec.section]: {} };
    card.spec.fields.forEach((f) => {
      const el = document.getElementById(f.id);
      if (el) payload[card.spec.section][f.key] = readField(el, f.type);
    });
    card.saveBtn.disabled = true;
    card.resetBtn.disabled = true;
    try {
      await saveConfig(payload);
    } catch {
      // saveConfig already surfaced an error toast; keep the buttons visible so
      // the user can retry.
    } finally {
      card.saveBtn.disabled = false;
      card.resetBtn.disabled = false;
    }
  }

  function reset(card) {
    card.spec.fields.forEach((f) => {
      const el = document.getElementById(f.id);
      if (!el) return;
      const saved = card.saved[f.id];
      if (el.type === "checkbox") {
        el.checked = saved === "true";
      } else if (el._customSelectInstance) {
        el._customSelectInstance.setValue(saved);
      } else {
        el.value = saved;
      }
      el.dispatchEvent(new Event("input", { bubbles: true }));
    });
    evaluate(card);
  }

  function buildButtons(card) {
    const header = card.el.querySelector(".card-header");
    if (!header) return false;

    const actions = document.createElement("div");
    actions.className = "card-header-actions";

    const resetBtn = document.createElement("button");
    resetBtn.type = "button";
    resetBtn.className = "btn btn-sm btn-ghost config-card-reset";
    resetBtn.innerHTML = '<i class="icon-rotate-ccw"></i><span>Reset</span>';
    resetBtn.hidden = true;
    resetBtn.addEventListener("click", () => reset(card));

    const saveBtn = document.createElement("button");
    saveBtn.type = "button";
    saveBtn.className = "btn btn-sm btn-primary config-card-save";
    saveBtn.innerHTML = '<i class="icon-save"></i><span>Save</span>';
    saveBtn.hidden = true;
    saveBtn.addEventListener("click", () => save(card));

    actions.append(resetBtn, saveBtn);

    // Move the feature on/off toggle (if any) into the actions group so the
    // Save/Reset buttons sit to its left, sharing one right-aligned cluster.
    const toggle = header.querySelector(":scope > .toggle");
    header.appendChild(actions);
    if (toggle) actions.appendChild(toggle);

    card.saveBtn = saveBtn;
    card.resetBtn = resetBtn;
    return true;
  }

  function setup() {
    CARD_SPECS.forEach((spec) => {
      const first = document.getElementById(spec.fields[0].id);
      const el = first?.closest(".config-card");
      if (!el) return;

      const card = { spec, el, saved: {}, saveBtn: null, resetBtn: null, cleanups: [] };
      if (!buildButtons(card)) return;

      spec.fields.forEach((f) => {
        const fe = document.getElementById(f.id);
        if (!fe) return;
        const evt = fe.tagName === "SELECT" || fe.type === "checkbox" ? "change" : "input";
        const handler = () => evaluate(card);
        fe.addEventListener(evt, handler);
        card.cleanups.push(() => fe.removeEventListener(evt, handler));
      });

      cards.push(card);
    });
    snapshot();
  }

  // Record the current field values as the saved baseline and re-hide buttons.
  // Called after every config load/save so "dirty" means "differs from saved".
  function snapshot() {
    cards.forEach((card) => {
      card.spec.fields.forEach((f) => {
        const el = document.getElementById(f.id);
        if (el) card.saved[f.id] = snapValue(el);
      });
      evaluate(card);
    });
  }

  function dispose() {
    cards.forEach((card) => {
      card.cleanups.forEach((c) => c());
      const actions = card.el.querySelector(".card-header-actions");
      if (!actions) return;
      // The page DOM persists across visits, so move the feature toggle back out
      // to the header before discarding the actions cluster — otherwise the
      // toggle would be destroyed and gone on the next visit.
      const header = actions.parentElement;
      const toggle = actions.querySelector(":scope > .toggle");
      if (toggle && header) header.appendChild(toggle);
      actions.remove();
    });
    cards.length = 0;
  }

  return { setup, snapshot, dispose, hasDirtyCards };
}
