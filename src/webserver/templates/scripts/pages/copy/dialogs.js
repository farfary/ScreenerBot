// The page's modal overlays: one Escape entry per open dialog, the backdrop and
// every [data-dialog-close] button close it, and focus returns to the opener. A
// dialog holding unsaved work passes `beforeClose`, which can keep it open.
import { pushEscapeHandler } from "../../core/escape_stack.js";

export function createDialogs($, on) {
  const open = new Map();

  function show(id, { onClose, beforeClose } = {}) {
    const overlay = $(`#${id}`);
    if (!overlay) return null;
    if (!open.has(id)) {
      open.set(id, {
        release: pushEscapeHandler(() => {
          void requestClose(id);
        }),
        opener: document.activeElement,
        onClose,
        beforeClose,
        closing: false,
      });
    }
    overlay.classList.remove("hidden");
    return overlay;
  }

  function hide(id) {
    const overlay = $(`#${id}`);
    overlay?.classList.add("hidden");
    const entry = open.get(id);
    if (!entry) return;
    open.delete(id);
    entry.release();
    entry.onClose?.();
    if (entry.opener?.isConnected) entry.opener.focus?.();
  }

  /** Close on the user's behalf; the dialog's `beforeClose` may keep it open. */
  async function requestClose(id) {
    const entry = open.get(id);
    if (!entry || entry.closing) return;
    entry.closing = true;
    let close = true;
    try {
      if (entry.beforeClose) close = await entry.beforeClose();
    } finally {
      entry.closing = false;
    }
    if (close) hide(id);
  }

  function setup(root) {
    on(root, "click", (event) => {
      const overlay = event.target.closest(".copy-dialog");
      if (!overlay) return;
      if (event.target === overlay || event.target.closest("[data-dialog-close]")) {
        void requestClose(overlay.id);
      }
    });
  }

  return {
    show,
    hide,
    setup,
    isOpen: (id) => open.has(id),
    closeAll: () => [...open.keys()].forEach(hide),
  };
}
