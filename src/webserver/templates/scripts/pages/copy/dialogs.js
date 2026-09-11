// The page's modal overlays: one Escape entry per open dialog, the backdrop and
// every [data-dialog-close] button close it, and focus returns to the opener.
import { pushEscapeHandler } from "../../core/escape_stack.js";

export function createDialogs($, on) {
  const open = new Map();

  function show(id, { onClose } = {}) {
    const overlay = $(`#${id}`);
    if (!overlay) return null;
    if (!open.has(id)) {
      open.set(id, {
        release: pushEscapeHandler(() => hide(id)),
        opener: document.activeElement,
        onClose,
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

  function setup(root) {
    on(root, "click", (event) => {
      const overlay = event.target.closest(".copy-dialog");
      if (!overlay) return;
      if (event.target === overlay || event.target.closest("[data-dialog-close]")) hide(overlay.id);
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
