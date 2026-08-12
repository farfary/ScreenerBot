/**
 * Numeric field enhancer.
 *
 * Every `<input type="number">` in the dashboard is a `.number-field`: our own
 * stepper instead of the browser's spin buttons, and any `.input-unit` written
 * next to the input adopted into the field's box over a reserved gutter.
 *
 * It is installed once, document-wide (see `installGlobalNumberFieldEnhancer`),
 * for the same reason the select enhancer is: pages render their markup as HTML
 * strings and repaint constantly, so anything that has to be attached per input
 * is guaranteed to be missing somewhere. Nothing a page writes has to opt in.
 *
 * The design — sizes, colours, the gutter — belongs to
 * `styles/components/form_controls.css`. This module contributes exactly one
 * number to layout: the unit's length in characters.
 */

const STEP_REPEAT_DELAY_MS = 400;
const STEP_REPEAT_INTERVAL_MS = 60;

/**
 * A suffix this short is a symbol — "%", "SOL", "USD", "ms" — and belongs inside
 * the field, right after the value it qualifies. A longer one is a word
 * ("seconds", "positions", "tokens") and stays where the page put it, beside the
 * field: inside a compact field it would leave the value a few pixels to live
 * in. Both are the same `.input-unit`, so the two read as one idea.
 */
const UNIT_INSIDE_MAX_CHARS = 3;

/**
 * Every number input that is not enhanced yet and has not opted out.
 * `data-stepper="off"` is for a field that carries its own inline controls in
 * the gutter the stepper would use — the trade amount has MAX and a percentage
 * slider there, and stepping a buy size by one means nothing.
 */
const PENDING_SELECTOR = 'input[type="number"]:not([data-number-field], [data-stepper="off"])';

/**
 * The next value of a stepped field, clamped to its own constraints.
 *
 * `HTMLInputElement.stepUp()` cannot be used: it throws on `step="any"` and on
 * an empty value, both of which are ordinary states here (a config field with
 * no value set, a free-form SOL amount). Steps are also rounded to the step's
 * own precision, or 0.1 + 0.2 walks a trade size to 0.30000000000000004.
 *
 * @param {string|number} current Current field value ("" when unset)
 * @param {{step?: string|number, min?: string|number, max?: string|number}} constraints
 * @param {1|-1} direction
 * @returns {number} The value to write back
 */
export function stepNumber(current, constraints = {}, direction = 1) {
  const rawStep = Number(constraints.step);
  const stepSize = Number.isFinite(rawStep) && rawStep > 0 ? rawStep : 1;
  const min = Number(constraints.min);
  const max = Number(constraints.max);
  const hasMin = Number.isFinite(min);
  const hasMax = Number.isFinite(max);

  // `Number("")` is 0, so an empty field has to be recognised before it is
  // parsed, or the first press of a field whose minimum is 0.001 lands on 1.
  const raw = typeof current === "string" ? current.trim() : current;
  const value = raw === "" || raw === null || raw === undefined ? Number.NaN : Number(raw);

  // An empty or unparseable field starts at its floor, not at zero.
  if (!Number.isFinite(value)) {
    if (hasMin) return min;
    return hasMax ? Math.min(0, max) : 0;
  }

  const decimals = (String(stepSize).split(".")[1] || "").length;
  let next = Number((value + direction * stepSize).toFixed(decimals));
  if (hasMin) next = Math.max(next, min);
  if (hasMax) next = Math.min(next, max);
  return next;
}

function stepInput(input, direction) {
  if (!input || input.disabled || input.readOnly) return;

  const next = stepNumber(
    input.value,
    { step: input.getAttribute("step"), min: input.min, max: input.max },
    direction
  );
  if (String(next) === input.value) return;

  input.value = String(next);
  // Pages listen for one or the other — config drafts track `input`, saved
  // forms track `change` — so a stepped value has to look exactly like a typed
  // one to both.
  input.dispatchEvent(new Event("input", { bubbles: true }));
  input.dispatchEvent(new Event("change", { bubbles: true }));
}

function buildSuffix(document_, unit) {
  const suffix = document_.createElement("span");
  suffix.className = "number-field-suffix";
  if (unit) suffix.appendChild(unit);

  const spin = document_.createElement("span");
  spin.className = "number-field-spin";
  // The stepper duplicates the field itself: a screen reader gets the input,
  // and the buttons stay out of the tab order so keyboard users keep the arrow
  // keys they already have.
  spin.setAttribute("aria-hidden", "true");
  for (const direction of ["up", "down"]) {
    const button = document_.createElement("button");
    button.type = "button";
    button.className = "number-field-step";
    button.dataset.numberStep = direction;
    button.tabIndex = -1;
    button.innerHTML = `<i class="icon-chevron-${direction}"></i>`;
    spin.appendChild(button);
  }
  suffix.appendChild(spin);
  return suffix;
}

/**
 * Turn one number input into a `.number-field`. Idempotent.
 * @param {HTMLInputElement} input
 * @returns {HTMLElement|null} The shell, or null if the input was not eligible
 */
export function enhanceNumberField(input) {
  if (!input || !input.matches || !input.matches(PENDING_SELECTOR)) return null;

  const document_ = input.ownerDocument;
  if (!document_) return null;
  input.dataset.numberField = "true";

  const sibling = input.nextElementSibling;
  const label = sibling && sibling.classList.contains("input-unit") ? sibling : null;
  const unitLength = label ? label.textContent.trim().length : 0;
  const inside = unitLength > 0 && unitLength <= UNIT_INSIDE_MAX_CHARS;

  const shell = document_.createElement("span");
  shell.className = "number-field";
  input.replaceWith(shell);
  shell.appendChild(input);

  if (inside) shell.style.setProperty("--number-field-unit-len", String(unitLength));
  shell.appendChild(buildSuffix(document_, inside ? label : null));

  // Inline, so no page rule setting the `padding` shorthand can drop the gutter
  // and let the value run under the suffix. The width itself stays in CSS.
  input.style.setProperty("padding-inline-end", "var(--number-field-gutter)");
  return shell;
}

/**
 * Enhance every number input inside a container.
 * @param {ParentNode} [container=document]
 * @returns {number} How many inputs were enhanced
 */
export function enhanceAllNumberFields(container = document) {
  if (!container || !container.querySelectorAll) return 0;
  let count = 0;
  for (const input of container.querySelectorAll(PENDING_SELECTOR)) {
    if (enhanceNumberField(input)) count += 1;
  }
  return count;
}

let _globalEnhancerInstalled = false;

/**
 * Install the one-time, document-wide enhancer plus the shared interactions
 * that belong to numeric fields: press-and-hold stepping, and neutralising the
 * browser's scroll-to-step, which silently rewrites whatever value the pointer
 * happens to rest on while a long settings list is scrolled.
 */
export function installGlobalNumberFieldEnhancer() {
  if (_globalEnhancerInstalled || typeof document === "undefined") return;
  _globalEnhancerInstalled = true;

  const enhanceWithin = (root) => {
    if (!root || root.nodeType !== 1) return;
    if (root.matches && root.matches(PENDING_SELECTOR)) {
      enhanceNumberField(/** @type {HTMLInputElement} */ (root));
    }
    enhanceAllNumberFields(root);
  };

  const initialSweep = () => enhanceWithin(document.body || document.documentElement);
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initialSweep, { once: true });
  } else {
    initialSweep();
  }

  // Scoped to the added subtrees, so it stays cheap while live tables repaint.
  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      if (!mutation.addedNodes || mutation.addedNodes.length === 0) continue;
      mutation.addedNodes.forEach(enhanceWithin);
    }
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });

  let repeatDelay = null;
  let repeatInterval = null;
  const stopRepeat = () => {
    if (repeatDelay !== null) clearTimeout(repeatDelay);
    if (repeatInterval !== null) clearInterval(repeatInterval);
    repeatDelay = null;
    repeatInterval = null;
  };

  document.addEventListener("pointerdown", (event) => {
    const button = event.target.closest && event.target.closest(".number-field-step");
    if (!button) return;
    const input = button.closest(".number-field")?.querySelector('input[type="number"]');
    if (!input) return;

    // Keep the caret in the field being stepped instead of handing focus to a
    // button that is not in the tab order to begin with.
    event.preventDefault();
    const direction = button.dataset.numberStep === "down" ? -1 : 1;
    input.focus({ preventScroll: true });
    stepInput(input, direction);

    stopRepeat();
    repeatDelay = setTimeout(() => {
      repeatInterval = setInterval(() => stepInput(input, direction), STEP_REPEAT_INTERVAL_MS);
    }, STEP_REPEAT_DELAY_MS);
  });
  for (const event of ["pointerup", "pointercancel", "blur"]) {
    document.addEventListener(event, stopRepeat, true);
  }

  // Non-passive, and only while a number field actually holds focus, so the
  // page's normal scrolling is never routed through a listener of ours.
  const blockWheelStep = (event) => event.preventDefault();
  const isNumberInput = (node) => node && node.tagName === "INPUT" && node.type === "number";
  document.addEventListener("focusin", (event) => {
    if (isNumberInput(event.target)) {
      event.target.addEventListener("wheel", blockWheelStep, { passive: false });
    }
  });
  document.addEventListener("focusout", (event) => {
    if (isNumberInput(event.target)) {
      event.target.removeEventListener("wheel", blockWheelStep);
    }
  });
}
