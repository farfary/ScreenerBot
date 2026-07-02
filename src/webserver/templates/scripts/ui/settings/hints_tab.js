/**
 * Hints Tab Module - Manage contextual hints
 *
 * Lets the user review every contextual hint in the dashboard and restore the
 * ones they've dismissed with "Don't show again" — individually or all at once.
 */
import * as Utils from "../../core/utils.js";
import * as Hints from "../../core/hints.js";
import { ConfirmationDialog } from "../confirmation_dialog.js";

/**
 * First meaningful line of a hint's content, used as a short preview.
 */
function hintPreview(content) {
  if (!content) return "";
  const line = content
    .split("\n")
    .map((l) => l.trim())
    .find((l) => l.length > 0);
  if (!line) return "";
  // Strip simple markdown emphasis/bullets for a clean one-liner
  return line.replace(/^[•\-*\s]+/, "").replace(/\*\*|\*|`/g, "");
}

/**
 * Build Hints tab HTML.
 *
 * The dismissed state is applied later in attachHintsHandlers (after Hints.init
 * has loaded it from the server), so the initial markup renders every hint as
 * active and is reconciled once state is available.
 */
export function buildHintsTab() {
  const groups = Hints.getAllHintGroups();
  const total = groups.reduce((n, g) => n + g.hints.length, 0);

  const groupsHtml = groups
    .map((group) => {
      const rows = group.hints
        .map((hint) => {
          const preview = Utils.escapeHtml(hintPreview(hint.content));
          return `
            <div class="settings-field hint-manage-row" data-hint-id="${hint.id}" data-hint-path="${hint.path}">
              <div class="settings-field-info">
                <label>${Utils.escapeHtml(hint.title)}</label>
                <span class="settings-field-hint">${preview}</span>
              </div>
              <div class="settings-field-control hint-manage-control">
                <span class="hint-manage-status"></span>
                <button class="btn btn-secondary btn-sm hint-manage-toggle" type="button"></button>
              </div>
            </div>`;
        })
        .join("");

      return `
        <div class="settings-section">
          <h3 class="settings-section-title">${Utils.escapeHtml(group.categoryLabel)}</h3>
          <div class="settings-group">
            ${rows}
          </div>
        </div>`;
    })
    .join("");

  return `
    <div class="settings-section">
      <h3 class="settings-section-title">
        <i class="icon-lightbulb"></i>
        Contextual Hints
      </h3>
      <p class="settings-section-description">
        Contextual hints are the help icons that explain dashboard features. Review
        every hint below and restore any you've hidden with "Don't show again" —
        one at a time or all together.
      </p>
      <div class="settings-group">
        <div class="settings-field">
          <div class="settings-field-info">
            <label>Hidden Hints</label>
            <span class="settings-field-hint">
              <span id="hintDismissedCount">0</span> of ${total} hints are currently hidden.
            </span>
          </div>
          <div class="settings-field-control">
            <button id="resetAllHintsBtn" class="btn btn-warning btn-sm" disabled>
              <i class="icon-rotate-ccw"></i>
              Restore All Hints
            </button>
          </div>
        </div>
      </div>
    </div>
    ${groupsHtml}
  `;
}

/**
 * Update a single row's control to reflect its dismissed state.
 */
function renderRowState(row) {
  const hintId = row.dataset.hintId;
  const dismissed = Hints.isDismissed(hintId);
  const status = row.querySelector(".hint-manage-status");
  const toggle = row.querySelector(".hint-manage-toggle");

  row.classList.toggle("hint-manage-row--hidden", dismissed);

  if (dismissed) {
    status.innerHTML = '<span class="badge secondary">Hidden</span>';
    toggle.innerHTML = '<i class="icon-eye"></i> Restore';
    toggle.classList.remove("btn-secondary");
    toggle.classList.add("btn-primary");
  } else {
    status.innerHTML = '<span class="badge success">Visible</span>';
    toggle.innerHTML = '<i class="icon-eye-off"></i> Hide';
    toggle.classList.remove("btn-primary");
    toggle.classList.add("btn-secondary");
  }
}

/**
 * Refresh the "N of M hidden" counter and the Restore-All button state.
 */
function refreshSummary(content) {
  const countEl = content.querySelector("#hintDismissedCount");
  const resetBtn = content.querySelector("#resetAllHintsBtn");
  const dismissedCount = Hints.getDismissedHints().length;
  if (countEl) countEl.textContent = String(dismissedCount);
  if (resetBtn) resetBtn.disabled = dismissedCount === 0;
}

/**
 * Attach handlers for Hints tab.
 */
export async function attachHintsHandlers(dialog, content) {
  // Ensure dismissed state is loaded before reconciling the rendered rows
  await Hints.init();

  const rows = content.querySelectorAll(".hint-manage-row");
  rows.forEach((row) => renderRowState(row));
  refreshSummary(content);

  // Per-hint hide/restore toggle
  content.querySelectorAll(".hint-manage-toggle").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const row = btn.closest(".hint-manage-row");
      if (!row) return;
      const hintId = row.dataset.hintId;

      if (Hints.isDismissed(hintId)) {
        await Hints.undismissHint(hintId);
      } else {
        await Hints.dismissHint(hintId);
      }

      renderRowState(row);
      refreshSummary(content);
      // Re-render any live triggers on the current page to reflect the change
      document.dispatchEvent(
        new CustomEvent("hints:toggle", { detail: { enabled: Hints.isEnabled() } })
      );
    });
  });

  // Restore all dismissed hints
  const resetBtn = content.querySelector("#resetAllHintsBtn");
  if (resetBtn) {
    resetBtn.addEventListener("click", async () => {
      const confirmResult = await ConfirmationDialog.show({
        title: "Restore All Hints",
        message: "Show all contextual hints again, including every one you've hidden?",
        confirmLabel: "Restore All",
        cancelLabel: "Cancel",
        variant: "warning",
      });
      if (!confirmResult.confirmed) return;

      await Hints.resetDismissedHints();
      rows.forEach((row) => renderRowState(row));
      refreshSummary(content);
      document.dispatchEvent(
        new CustomEvent("hints:toggle", { detail: { enabled: Hints.isEnabled() } })
      );
      Utils.showToast("All hints restored", "success");
    });
  }
}
