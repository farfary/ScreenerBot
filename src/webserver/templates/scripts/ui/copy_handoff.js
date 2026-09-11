// Cross-page handoff into the Copy Trading page: open a task, or start a new task
// for a wallet. The request rides in sessionStorage so it survives the router's
// page swap; the page takes it once on activation (or at once when it is already
// the current page, through the event).
import { loadPage } from "../core/router.js";

const HANDOFF_KEY = "screenerbot.copy.handoff";
export const COPY_HANDOFF_EVENT = "screenerbot:copy-handoff";

function stash(value) {
  try {
    window.sessionStorage.setItem(HANDOFF_KEY, JSON.stringify(value));
  } catch {
    // Storage unavailable: the page opens without the handoff.
  }
  window.dispatchEvent(new CustomEvent(COPY_HANDOFF_EVENT));
  loadPage("copy");
}

/** Open the Copy Trading page on one task. */
export function openCopyTask(taskId) {
  stash({ taskId: Number(taskId) });
}

/** Open the Copy Trading page with the wallet profile of `address`. */
export function openCopyForWallet(address, label = null) {
  stash({ address, label });
}

/** The pending handoff, removed as it is read. */
export function takeCopyHandoff() {
  try {
    const raw = window.sessionStorage.getItem(HANDOFF_KEY);
    window.sessionStorage.removeItem(HANDOFF_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}
