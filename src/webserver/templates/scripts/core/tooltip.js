/**
 * Global hover tooltip
 *
 * A single, reusable floating tooltip shared across the whole dashboard.
 * Any element carrying a `data-tooltip="..."` attribute shows its full text
 * on hover / keyboard focus — used by table cells that clamp long content
 * (blacklist reasons, service dependencies, rejection reasons) but generic
 * enough for any future "truncated value, full value on hover" need.
 *
 * Behaviour:
 *  - Plain `data-tooltip` always shows the tooltip.
 *  - `data-tooltip-truncated` only shows it when the element is actually
 *    clipped (scroll size exceeds client size), so short values stay quiet.
 *  - Content is rendered as text with `white-space: pre-wrap`, so multi-line
 *    values (newline-separated) format nicely. Never HTML — no injection.
 *
 * The module self-initialises on import via delegated document listeners,
 * mirroring connectivity_watcher.js / menu_manager.js.
 */

const VIEWPORT_MARGIN = 8;
const SHOW_DELAY_MS = 120;
const GAP = 8;

let tooltipEl = null;
let currentTarget = null;
let showTimer = null;

function ensureElement() {
  if (tooltipEl) return tooltipEl;
  tooltipEl = document.createElement("div");
  tooltipEl.className = "app-tooltip";
  tooltipEl.setAttribute("role", "tooltip");
  tooltipEl.hidden = true;
  document.body.appendChild(tooltipEl);
  return tooltipEl;
}

function isTruncated(el) {
  return el.scrollHeight > el.clientHeight + 1 || el.scrollWidth > el.clientWidth + 1;
}

function resolveTooltipTarget(start) {
  if (!start || typeof start.closest !== "function") return null;
  return start.closest("[data-tooltip]");
}

function position(target) {
  const el = ensureElement();
  const trigger = target.getBoundingClientRect();
  const tip = el.getBoundingClientRect();
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  // Prefer below the trigger; flip above when there isn't room.
  let top = trigger.bottom + GAP;
  if (top + tip.height > vh - VIEWPORT_MARGIN && trigger.top - GAP - tip.height > VIEWPORT_MARGIN) {
    top = trigger.top - GAP - tip.height;
  }
  top = Math.max(VIEWPORT_MARGIN, Math.min(top, vh - tip.height - VIEWPORT_MARGIN));

  // Align left edges, then clamp horizontally inside the viewport.
  let left = trigger.left;
  left = Math.max(VIEWPORT_MARGIN, Math.min(left, vw - tip.width - VIEWPORT_MARGIN));

  el.style.top = `${Math.round(top)}px`;
  el.style.left = `${Math.round(left)}px`;
}

function show(target) {
  const text = target.getAttribute("data-tooltip");
  if (!text) return;
  if (target.hasAttribute("data-tooltip-truncated") && !isTruncated(target)) return;

  const el = ensureElement();
  el.textContent = text;
  el.hidden = false;
  // Measure with content present, then place and reveal.
  el.classList.remove("app-tooltip--visible");
  position(target);
  // Force reflow so the transition runs from the hidden state.
  void el.offsetWidth;
  el.classList.add("app-tooltip--visible");
}

function hide() {
  currentTarget = null;
  if (showTimer) {
    clearTimeout(showTimer);
    showTimer = null;
  }
  if (!tooltipEl) return;
  tooltipEl.classList.remove("app-tooltip--visible");
  tooltipEl.hidden = true;
}

function scheduleShow(target) {
  currentTarget = target;
  if (showTimer) clearTimeout(showTimer);
  showTimer = window.setTimeout(() => {
    showTimer = null;
    if (currentTarget === target && document.body.contains(target)) {
      show(target);
    }
  }, SHOW_DELAY_MS);
}

function onPointerOver(event) {
  const target = resolveTooltipTarget(event.target);
  if (!target || target === currentTarget) return;
  scheduleShow(target);
}

function onPointerOut(event) {
  if (!currentTarget) return;
  // Ignore moves that stay inside the same trigger.
  const next = resolveTooltipTarget(event.relatedTarget);
  if (next === currentTarget) return;
  hide();
}

function onFocusIn(event) {
  const target = resolveTooltipTarget(event.target);
  if (target) scheduleShow(target);
}

function init() {
  if (window.__SB_TOOLTIP_READY__) return;
  window.__SB_TOOLTIP_READY__ = true;

  document.addEventListener("pointerover", onPointerOver, { passive: true });
  document.addEventListener("pointerout", onPointerOut, { passive: true });
  document.addEventListener("focusin", onFocusIn, { passive: true });
  document.addEventListener("focusout", hide, { passive: true });
  // Any scroll or wheel hides the tooltip so it never floats detached.
  document.addEventListener("scroll", hide, { passive: true, capture: true });
  window.addEventListener("resize", hide, { passive: true });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") hide();
  });
}

init();

export { hide as hideTooltip };
