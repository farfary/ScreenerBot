/**
 * Shared expand/collapse-all control.
 *
 * One button instead of a pair: it carries the state of the tree it drives and
 * flips between "Expand all" and "Collapse all". Callers own the tree, this
 * module owns markup, state display and ARIA.
 *
 * Both glyphs and both labels are rendered into the same grid cell and
 * cross-faded, so the button's width is the width of its widest state and the
 * control never resizes while toggling.
 */

import { create, on } from "../core/dom.js";

const DEFAULTS = {
  expandLabel: "Expand all",
  collapseLabel: "Collapse all",
  expandTitle: "Expand everything",
  collapseTitle: "Collapse everything",
};

/**
 * @param {object} options
 * @param {boolean} [options.expanded] initial state
 * @param {(expanded: boolean) => void} options.onToggle called with the NEW state
 * @param {string} [options.expandLabel] label shown while collapsed
 * @param {string} [options.collapseLabel] label shown while expanded
 * @param {string} [options.expandTitle] tooltip shown while collapsed
 * @param {string} [options.collapseTitle] tooltip shown while expanded
 * @param {string} [options.className] extra class on the button
 * @returns {{ element: HTMLButtonElement, setExpanded: (v: boolean) => void, isExpanded: () => boolean }}
 */
export function createExpandToggle(options = {}) {
  const config = { ...DEFAULTS, ...options };
  let expanded = Boolean(config.expanded);

  const button = create("button", {
    type: "button",
    className: config.className ? `expand-toggle ${config.className}` : "expand-toggle",
  });

  button.innerHTML = `
    <span class="expand-toggle-glyphs" aria-hidden="true">
      <i class="icon-chevrons-up-down" data-state="collapsed"></i>
      <i class="icon-chevrons-down-up" data-state="expanded"></i>
    </span>
    <span class="expand-toggle-labels">
      <span class="expand-toggle-label" data-state="collapsed"></span>
      <span class="expand-toggle-label" data-state="expanded"></span>
    </span>
  `;

  button.querySelector('.expand-toggle-label[data-state="collapsed"]').textContent =
    config.expandLabel;
  button.querySelector('.expand-toggle-label[data-state="expanded"]').textContent =
    config.collapseLabel;

  function apply() {
    button.setAttribute("aria-pressed", expanded ? "true" : "false");
    button.title = expanded ? config.collapseTitle : config.expandTitle;
  }

  on(button, "click", () => {
    expanded = !expanded;
    apply();
    if (typeof config.onToggle === "function") {
      config.onToggle(expanded);
    }
  });

  apply();

  return {
    element: button,
    setExpanded(value) {
      expanded = Boolean(value);
      apply();
    },
    isExpanded() {
      return expanded;
    },
  };
}
