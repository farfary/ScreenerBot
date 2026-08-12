/**
 * Escape Stack - single owner of the Escape key for stacked overlays.
 *
 * Overlays used to each bind their own `document` keydown listener, which is
 * correct only while exactly one is open: with two stacked (token details opened
 * from the featured dialog) a single Escape reached BOTH listeners and closed
 * both at once. Event propagation cannot arbitrate that -- both are bound on
 * `document`, so they fire in registration order, not stacking order.
 *
 * Instead there is ONE document listener here, and Escape is delivered only to
 * the most recently registered (topmost) overlay.
 *
 * Usage:
 *   this._releaseEscape = pushEscapeHandler(() => this.close());
 *   ...
 *   this._releaseEscape();   // on close; safe to call twice
 */

const stack = [];

function onKeydown(event) {
  if (event.key !== "Escape") return;

  const top = stack[stack.length - 1];
  if (!top) return;

  // Stop lower overlays' own listeners (any not yet migrated to this stack) from
  // also reacting to the same key press.
  event.preventDefault();
  event.stopImmediatePropagation();
  top.handler(event);
}

export function hasOpenOverlay() {
  if (stack.length > 0) return true;

  return Array.from(
    document.querySelectorAll(
      'dialog[open], [aria-modal="true"], [role="dialog"], .modal-overlay, .dialog-overlay'
    )
  ).some((element) => {
    if (element.hidden || element.getAttribute("aria-hidden") === "true") return false;
    const style = window.getComputedStyle(element);
    return style.display !== "none" && style.visibility !== "hidden" && style.opacity !== "0";
  });
}

export function closeStackedOverlays() {
  [...stack].reverse().forEach((entry) => {
    try {
      entry.handler();
    } catch (error) {
      console.error("[EscapeStack] Overlay close failed:", error);
    }
  });
}

/**
 * Register an overlay's Escape handler as the topmost one.
 * @param {Function} handler - invoked when Escape is pressed and this overlay is on top
 * @returns {Function} release function; call it when the overlay closes
 */
export function pushEscapeHandler(handler) {
  const entry = { handler };
  stack.push(entry);

  if (stack.length === 1) {
    // Capture phase so we run before any per-overlay listener still bound on document.
    document.addEventListener("keydown", onKeydown, true);
  }

  let released = false;
  return () => {
    if (released) return;
    released = true;

    const index = stack.indexOf(entry);
    if (index !== -1) stack.splice(index, 1);

    if (stack.length === 0) {
      document.removeEventListener("keydown", onKeydown, true);
    }
  };
}
