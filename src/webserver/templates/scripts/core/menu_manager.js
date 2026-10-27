// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Global Menu Coordinator
 *
 * Single source of truth for "only one transient overlay open at a time" across
 * the whole dashboard. Any dropdown / custom-select / popover registers itself as
 * an open "menu" while visible; the coordinator then guarantees it is dismissed
 * when the user does anything that should close it:
 *
 *   - opens another registered menu        (single-open policy)
 *   - pointer-downs outside every open menu (another control, checkbox, input,
 *     blank space, a dialog backdrop, …)
 *   - moves keyboard focus outside every open menu (Tab into an input, etc.)
 *   - presses Escape
 *   - navigates to another page / opens a dialog (callers invoke closeAllMenus())
 *
 * Why capture-phase global listeners: individual dropdown triggers call
 * `event.stopPropagation()`, which prevents OTHER components' bubble-phase
 * document handlers from ever seeing the click — that is exactly why multiple
 * dropdowns used to stay open at once. Capture-phase listeners on `document`
 * run BEFORE the target handler, so `stopPropagation()` in a trigger cannot hide
 * the interaction from the coordinator.
 *
 * A "menu" is any object shaped like:
 *   { close(reason?: string): void, owns(target: EventTarget): boolean }
 * `owns` must return true for the trigger AND the menu surface (including a
 * portaled dropdown rendered elsewhere in the DOM), so interactions inside the
 * menu itself don't dismiss it.
 */

import { dirSign } from "./dom.js";

const openMenus = new Set();
let listenersAttached = false;

function owns(menu, target) {
  if (!target) return false;
  try {
    return typeof menu.owns === "function" ? !!menu.owns(target) : false;
  } catch {
    return false;
  }
}

function dismiss(menu, reason = "dismiss") {
  // Remove first so a re-entrant close() (which also calls closeMenu) is a no-op
  // and we never recurse or double-fire.
  openMenus.delete(menu);
  try {
    menu.close(reason);
  } catch {
    /* a broken close() must not wedge the coordinator */
  }
}

function closeMenusNotOwning(target, reason) {
  if (openMenus.size === 0) return;
  for (const menu of [...openMenus]) {
    if (!owns(menu, target)) {
      dismiss(menu, reason);
    }
  }
}

function attachGlobalListeners() {
  if (listenersAttached) return;
  listenersAttached = true;

  // Outside pointer — capture phase so a trigger's stopPropagation can't hide it.
  document.addEventListener(
    "pointerdown",
    (e) => closeMenusNotOwning(e.target, "outside-pointer"),
    true
  );

  // Focus moved to something outside every open menu (Tab to an input/checkbox,
  // a programmatic focus, etc.).
  document.addEventListener(
    "focusin",
    (e) => closeMenusNotOwning(e.target, "focus-left"),
    true
  );

  // Escape closes the open menus and nothing else: the overlay a menu sits in waits for
  // the next press (`escape_stack.js` defers to open menus for the same reason).
  document.addEventListener(
    "keydown",
    (e) => {
      if (e.key === "Escape" && openMenus.size > 0) {
        e.preventDefault();
        e.stopImmediatePropagation();
        closeAllMenus("escape");
      }
    },
    true
  );

  // Hard reset on tab hide so menus never linger across a return to the tab.
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) closeAllMenus("document-hidden");
  });
}

/**
 * Register a menu as open. Enforces the single-open policy by closing every other
 * currently-open menu first. Safe to call again for an already-open menu.
 * @param {{close: (reason?: string) => void, owns: (t: EventTarget) => boolean}} menu
 */
export function openMenu(menu) {
  if (!menu || typeof menu.close !== "function") return;
  attachGlobalListeners();
  for (const other of [...openMenus]) {
    if (other !== menu) dismiss(other, "superseded");
  }
  openMenus.add(menu);
}

/**
 * Unregister a menu (call from the component's own close()). Does NOT call
 * close() — the component is already closing.
 * @param {object} menu
 */
export function closeMenu(menu) {
  openMenus.delete(menu);
}

/** Close and unregister every open menu. Used on route change / dialog open. */
export function closeAllMenus(reason = "dismiss") {
  for (const menu of [...openMenus]) {
    dismiss(menu, reason);
  }
}

/**
 * The part of the window a menu may cover: the viewport inset by `margin`, ending above
 * the status bar, which stays on top of every menu. Every menu placement clamps into it.
 * @param {number} margin
 * @returns {{ top: number, bottom: number, left: number, right: number }}
 */
export function menuBounds(margin = 8) {
  const statusBar = document.getElementById("statusBar");
  const floor = statusBar?.getClientRects().length
    ? Math.min(window.innerHeight, statusBar.getBoundingClientRect().top)
    : window.innerHeight;
  return {
    top: margin,
    bottom: floor - margin,
    left: margin,
    right: window.innerWidth - margin,
  };
}

/**
 * Keep a body-portaled menu beside a moving trigger: below it, or above it when that
 * side has more room, `gap` pixels apart and clamped into `menuBounds`. The returned
 * cleanup stops the animation-frame loop. Placement only writes when geometry changes.
 */
export function trackAnchoredMenu({
  trigger,
  menu,
  align = "start",
  viewportMargin = 8,
  gap = 4,
  onDetach,
}) {
  let animationFrame = null;
  let lastSignature = "";

  const update = () => {
    animationFrame = null;
    if (!trigger?.isConnected || !menu?.isConnected) {
      onDetach?.();
      return;
    }

    const triggerRect = trigger.getBoundingClientRect();
    const bounds = menuBounds(viewportMargin);
    menu.style.maxHeight = `${Math.max(0, bounds.bottom - bounds.top)}px`;
    menu.style.overflowY = "auto";
    // Layout size, not the bounding box: the entry slide scales the closed menu.
    const menuRect = { width: menu.offsetWidth, height: menu.offsetHeight };
    const availableBelow = bounds.bottom - triggerRect.bottom - gap;
    const availableAbove = triggerRect.top - bounds.top - gap;
    const openAbove = availableBelow < menuRect.height && availableAbove > availableBelow;
    // `align` is an inline edge: "start" is the trigger's left edge in LTR and its right edge
    // in RTL. The clamp below flips the menu inward when that side has no room.
    const alignRight = (align === "end") === (dirSign() === 1);
    const preferredLeft = alignRight ? triggerRect.right - menuRect.width : triggerRect.left;
    const left = Math.max(bounds.left, Math.min(preferredLeft, bounds.right - menuRect.width));
    const preferredTop = openAbove
      ? triggerRect.top - gap - menuRect.height
      : triggerRect.bottom + gap;
    const top = Math.max(bounds.top, Math.min(preferredTop, bounds.bottom - menuRect.height));
    const signature = `${left}:${top}:${menuRect.width}:${menuRect.height}:${openAbove}`;

    if (signature !== lastSignature) {
      lastSignature = signature;
      menu.style.position = "fixed";
      menu.style.left = `${left}px`;
      menu.style.top = `${top}px`;
      menu.classList.toggle("menu-above", openAbove);
    }

    animationFrame = requestAnimationFrame(update);
  };

  update();
  return () => {
    if (animationFrame !== null) cancelAnimationFrame(animationFrame);
    animationFrame = null;
  };
}

/** Number of currently-open registered menus (mainly for tests/debugging). */
export function openMenuCount() {
  return openMenus.size;
}
