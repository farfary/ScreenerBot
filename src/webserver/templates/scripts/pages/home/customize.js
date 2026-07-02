// Home card customization — reorder (drag), enable/disable, persisted via server UI-state.
import * as AppState from "../../core/app_state.js";

const STATE_KEY = "home.cardLayout";
const DEFAULT_ORDER = ["wallet", "calendar", "positions", "tokens"];

/**
 * Create the home card customization controller.
 * @returns {{ mount:Function, dispose:Function }}
 */
export function createCustomizer() {
  let layout = { order: [...DEFAULT_ORDER], hidden: [] };
  let editing = false;
  let dragId = null;
  const cleanups = [];

  const grid = () => document.getElementById("homeCardGrid");
  const cards = () => Array.from(document.querySelectorAll("#homeCardGrid .dashboard-card"));

  function track(el, evt, handler) {
    if (!el) return;
    el.addEventListener(evt, handler);
    cleanups.push(() => el.removeEventListener(evt, handler));
  }

  function normalizeLayout(raw) {
    const order = Array.isArray(raw?.order) ? raw.order.filter((id) => DEFAULT_ORDER.includes(id)) : [];
    // Append any cards missing from the saved order (e.g. new cards added later).
    for (const id of DEFAULT_ORDER) {
      if (!order.includes(id)) order.push(id);
    }
    const hidden = Array.isArray(raw?.hidden)
      ? raw.hidden.filter((id) => DEFAULT_ORDER.includes(id))
      : [];
    return { order, hidden };
  }

  function persist() {
    AppState.save(STATE_KEY, layout);
  }

  function applyLayout() {
    const gridEl = grid();
    if (!gridEl) return;
    const byId = new Map(cards().map((c) => [c.dataset.cardId, c]));
    // Reorder DOM to match saved order.
    for (const id of layout.order) {
      const el = byId.get(id);
      if (el) gridEl.appendChild(el);
    }
    // Apply enabled/disabled visibility + reflect in toggles.
    for (const el of cards()) {
      const id = el.dataset.cardId;
      const isHidden = layout.hidden.includes(id);
      el.classList.toggle("card-hidden", isHidden && !editing);
      el.classList.toggle("card-disabled", isHidden);
      const toggle = el.querySelector(".card-enable-toggle");
      if (toggle) toggle.checked = !isHidden;
    }
  }

  function setEditing(on) {
    editing = on;
    const gridEl = grid();
    if (gridEl) gridEl.classList.toggle("home-editing", on);

    // Reflect state on every customize trigger (icon + tooltip swap).
    for (const trigger of document.querySelectorAll(".card-customize-trigger")) {
      trigger.classList.toggle("active", on);
      trigger.title = on ? "Done" : "Customize dashboard";
      trigger.setAttribute("aria-label", trigger.title);
      const icon = trigger.querySelector("i");
      if (icon) icon.className = on ? "icon-check" : "icon-sliders-horizontal";
    }

    for (const el of cards()) {
      el.setAttribute("draggable", on ? "true" : "false");
    }
    applyLayout(); // hidden cards become visible (dimmed) in edit mode
  }

  function onDragStart(e) {
    const card = e.target.closest(".dashboard-card");
    if (!card || !editing) return;
    dragId = card.dataset.cardId;
    card.classList.add("dragging");
    e.dataTransfer.effectAllowed = "move";
    try {
      e.dataTransfer.setData("text/plain", dragId);
    } catch {
      /* some browsers require a data payload */
    }
  }

  function onDragOver(e) {
    if (!editing || dragId == null) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    const gridEl = grid();
    const target = e.target.closest(".dashboard-card");
    if (!gridEl || !target || target.dataset.cardId === dragId) return;
    const dragging = gridEl.querySelector(`.dashboard-card[data-card-id="${dragId}"]`);
    if (!dragging) return;
    const rect = target.getBoundingClientRect();
    const after = e.clientY - rect.top > rect.height / 2;
    gridEl.insertBefore(dragging, after ? target.nextSibling : target);
  }

  function onDrop(e) {
    if (!editing) return;
    e.preventDefault();
    commitOrder();
  }

  function onDragEnd() {
    const dragging = grid()?.querySelector(".dashboard-card.dragging");
    if (dragging) dragging.classList.remove("dragging");
    dragId = null;
    commitOrder();
  }

  function commitOrder() {
    const order = cards().map((c) => c.dataset.cardId);
    if (JSON.stringify(order) !== JSON.stringify(layout.order)) {
      layout.order = order;
      persist();
    }
  }

  function onToggleChange(e) {
    const input = e.target.closest(".card-enable-toggle");
    if (!input) return;
    const id = input.dataset.cardId;
    const hidden = new Set(layout.hidden);
    if (input.checked) hidden.delete(id);
    else hidden.add(id);
    layout.hidden = [...hidden];
    persist();
    applyLayout();
  }

  function resetLayout() {
    layout = { order: [...DEFAULT_ORDER], hidden: [] };
    persist();
    applyLayout();
  }

  return {
    async mount() {
      // Adopt persisted layout once the async state cache is ready
      // (never read synchronously at module scope — cache-race pitfall).
      const raw = await AppState.loadAsync(STATE_KEY, null);
      layout = normalizeLayout(raw);
      applyLayout();

      // Delegated clicks: customize triggers live in every card header (hover-revealed).
      track(document, "click", (e) => {
        if (e.target.closest(".card-customize-trigger")) {
          setEditing(!editing);
        } else if (e.target.closest(".card-reset-trigger")) {
          resetLayout();
        }
      });

      const gridEl = grid();
      track(gridEl, "dragstart", onDragStart);
      track(gridEl, "dragover", onDragOver);
      track(gridEl, "drop", onDrop);
      track(gridEl, "dragend", onDragEnd);
      track(gridEl, "change", onToggleChange);
    },

    dispose() {
      if (editing) setEditing(false);
      cleanups.forEach((fn) => fn());
      cleanups.length = 0;
    },
  };
}
