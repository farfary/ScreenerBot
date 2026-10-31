// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// macOS window controls: tells the Electron shell when a modal scrim covers the title bar, so the native traffic lights dim with the rest of the window.

/*
 * The traffic lights are native buttons drawn above the page, so no scrim can darken
 * them. Every layer that dims the dashboard (dialogs, search, the actions drawer,
 * the assistant chat) is a full-window element with a translucent background, while
 * the full-screen views (Featured, token, transaction and position details) are
 * opaque. So instead of each dialog reporting itself, this module looks at what
 * actually covers the window's top-left corner, where the buttons sit:
 *
 * - the header itself: nothing covers the title bar;
 * - a translucent surface: a scrim covers it, so the buttons dim;
 * - an opaque surface: a full-screen view owns the title bar, so they stay as they are.
 *
 * Closed overlays never answer the hit test (they are hidden, `pointer-events: none`
 * or `visibility: hidden`), so the check follows the moment an overlay opens or
 * starts to close. It runs at most once per frame after a DOM change or the end of a
 * transition, and only in the macOS Electron shell.
 */

/** CSS px from the window's top-left corner: inside the title bar at every zoom level. */
const PROBE_POINT = 8;

const OBSERVED_ATTRIBUTES = ["class", "hidden", "style", "open", "aria-hidden", "inert", "data-state"];

/** Alpha of a computed `background-color`; anything not parsed as translucent counts as opaque. */
function backgroundAlpha(color) {
  if (!color || color === "transparent") return 0;
  const slash = color.match(/\/\s*([\d.]+)(%?)\s*\)$/);
  if (slash) return slash[2] ? Number(slash[1]) / 100 : Number(slash[1]);
  const legacy = color.match(/^rgba\((?:[^,]+,){3}\s*([\d.]+)\s*\)$/);
  return legacy ? Number(legacy[1]) : 1;
}

/** Whether a translucent layer covers the title bar (see the module comment). */
function titleBarUnderScrim() {
  for (
    let node = document.elementFromPoint(PROBE_POINT, PROBE_POINT);
    node && node !== document.body && node !== document.documentElement;
    node = node.parentElement
  ) {
    if (node.classList.contains("modern-header")) return false;
    const alpha = backgroundAlpha(getComputedStyle(node).backgroundColor);
    if (alpha >= 1) return false;
    if (alpha > 0) return true;
  }
  return false;
}

function startWindowControlsSync(api) {
  let dimmed = false;
  let scheduled = false;

  const check = () => {
    scheduled = false;
    const next = titleBarUnderScrim();
    if (next === dimmed) return;
    dimmed = next;
    api.setWindowControlsDimmed(next).catch((error) => {
      console.warn("[WindowControls] Shell rejected the dim state:", error);
    });
  };

  const schedule = () => {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(check);
  };

  new MutationObserver(schedule).observe(document.body, {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: OBSERVED_ATTRIBUTES,
  });
  // An overlay that fades out keeps answering the hit test until `visibility` flips
  // at the end of its transition.
  document.addEventListener("transitionend", schedule, true);
  document.addEventListener("animationend", schedule, true);
  schedule();
}

// A module script runs after the document is parsed, so `document.body` exists here.
const api = window.electronAPI;
if (api?.platform === "darwin" && typeof api.setWindowControlsDimmed === "function") {
  startWindowControlsSync(api);
}
