/**
 * The dashboard button contract.
 *
 * A `<button>` starts life wearing the browser's native control chrome:
 * `background-color: ButtonFace`, `border: 2px outset ButtonBorder`,
 * `color: ButtonText` and the OS control font. None of that inherits and none
 * of it is reset anywhere in the dashboard, so a button whose classes do not
 * name those properties does not fall back to something neutral — it falls back
 * to a grey bevelled OS button sitting inside our dark cards. That is what the
 * Settings > About "Open Data Folder" pair looked like for months: the updates
 * tab renamed `.settings-update-btn` to `.updates-btn`, the About markup kept
 * the old name, and the one surviving rule for it set nothing but a background.
 *
 * Nothing in the Rust build reads a selector, so this is the only place that
 * failure can be caught. The checks below all work the same way: read what the
 * markup asks for, read what the concatenated stylesheets actually draw, and
 * report the gap.
 */

import {
  classTokens,
  classesIn,
  findOpenTags,
  hasBareAttribute,
  loadMarkupSources,
  loadStylesheets,
  lineOf,
  rulesIn,
  subjectCompound,
} from "./dashboard_ui.mjs";

/* Properties a `<button>` gets from the UA and therefore must be told. `box`
   is padding or an explicit size: without one the global `* { padding: 0 }`
   leaves the label jammed against the border. The control font is not in this
   list because it is fixed once, globally, in `foundation.css` — see
   `the dashboard never lets a control wear the OS font` in the test. */
const REQUIRED_SKIN = {
  background: /(?:^|;)\s*background(?:-color|-image)?\s*:/,
  border:
    /(?:^|;)\s*border(?:-(?:width|style|color|top|right|bottom|left|inline|block)[\w-]*)?\s*:/,
  color: /(?:^|;)\s*color\s*:/,
  cursor: /(?:^|;)\s*cursor\s*:/,
  box: /(?:^|;)\s*(?:padding[\w-]*|width|height|inline-size|block-size|aspect-ratio)\s*:/,
};

const DISABLED_APPEARANCE =
  /(?:^|;)\s*(?:opacity|cursor|background(?:-color)?|color|border(?:-[a-z-]+)?|filter)\s*:/;

/* Appearance written into the tag instead of a stylesheet. Layout and
   visibility are legitimately dynamic; a colour or a box is not. */
const INLINE_SKIN =
  /(?:^|;)\s*(?:background|border(?!-radius)|color|font|padding|cursor)[\w-]*\s*:/i;

const STATE_PSEUDO =
  /:(?:hover|active|focus|focus-visible|focus-within|disabled|checked|target|visited)\b|\[(?:disabled|aria-pressed|aria-expanded|aria-selected|data-state)/;

/* Classes the rightmost compound requires the element to carry. `:not(.x)` is
   an exclusion, so its classes are not requirements. */
function subjectClasses(selector) {
  const subject = subjectCompound(selector).replace(/:not\([^)]*\)/g, "");
  return classesIn(subject);
}

/* `.x:disabled`, `.x[disabled]`, `.x.is-disabled` - however a skin spells it.
   `:not(:disabled)` is the opposite claim and must not count: reading it as a
   disabled rule is what let `.dt-client-pagination-btn:hover:not(:disabled)`
   stand in for the missing disabled appearance in an earlier draft. */
function disabledClasses(selector) {
  const subject = subjectCompound(selector).replace(/:not\([^)]*\)/g, "");
  if (!/:disabled|\[disabled\]|\.is-disabled\b|\.disabled\b/.test(subject)) return null;
  const classes = classesIn(subject.replace(/\.(?:is-)?disabled\b/g, ""));
  return classes.length ? classes : null;
}

const SELECTOR_QUERY =
  /(?:querySelector(?:All)?|closest|matches|getElementsByClassName|\$\$?)\(\s*[`'"]([^`'"]*)[`'"]/g;

/** Classes JavaScript looks the element up by. Those are handles, not skins. */
function collectBehaviourHooks(sources) {
  const hooks = new Set();
  for (const { source } of sources) {
    for (const query of source.matchAll(SELECTOR_QUERY)) {
      for (const name of classesIn(query[1])) hooks.add(name);
    }
    for (const name of source.matchAll(/getElementsByClassName\(\s*[`'"]([\w-]+)/g)) {
      hooks.add(name[1]);
    }
  }
  return hooks;
}

/* Names that read as a state rather than as a component: the second class in
   `.updates-btn.primary` is a variant, the second class in `.sub-tab.active`
   is a state the code has to remember to set. */
const STATE_NAME =
  /^(?:is|has)-|^(?:active|selected|open|opened|expanded|collapsed|current|checked|pressed|busy|loading|pending|running|dragging|hidden|visible|error|invalid|success|warning|danger)$/;

/** Every class name some code path can actually put on an element. */
function collectAppliedClasses(sources) {
  const applied = new Set();
  const add = (value) => {
    for (const token of String(value).split(/\s+/)) {
      if (/^[a-z][\w-]*$/i.test(token)) applied.add(token);
    }
  };

  for (const { source } of sources) {
    /* Every element, not only buttons: a state class is often written on the
       row and read by a rule that targets the button inside it. */
    for (const tag of findOpenTags(source)) {
      const { statics, conditional } = classTokens(tag.tag);
      statics.forEach(add);
      conditional.forEach(add);
    }
    for (const call of source.matchAll(
      /classList\s*\.\s*(?:add|remove|toggle|replace|contains)\(([^)]*)\)/g
    )) {
      for (const literal of call[1].matchAll(/["'`]([^"'`]*)["'`]/g)) add(literal[1]);
    }
    for (const assignment of source.matchAll(/className\s*=\s*[`'"]([^`'"]*)/g)) add(assignment[1]);
  }
  return applied;
}

/* Ids and classes some code path switches on and off. A control that is ever
   disabled must be able to LOOK disabled, or the UI reports nothing. */
function collectDisabledTargets(sources) {
  const ids = new Set();
  const classes = new Set();
  for (const { source } of sources) {
    for (const match of source.matchAll(
      /getElementById\(\s*[`'"]([\w-]+)[`'"]\s*\)(?:[?.]|\??\.)*\s*\.?\s*disabled\s*=/g
    )) {
      ids.add(match[1]);
    }
    for (const match of source.matchAll(
      /querySelector(?:All)?\(\s*[`'"]([^`'"]*)[`'"]\s*\)[^;\n]{0,40}\.disabled\s*=/g
    )) {
      for (const name of classesIn(match[1])) classes.add(name);
      for (const id of match[1].matchAll(/#([\w-]+)/g)) ids.add(id[1]);
    }
  }
  return { ids, classes };
}

export async function loadButtonModel() {
  const stylesheets = await loadStylesheets();
  const sources = await loadMarkupSources();

  const declaredClasses = new Set();
  const skinRules = [];
  const disabledRules = [];
  const stateRules = [];

  for (const sheet of stylesheets) {
    for (const rule of sheet.rules) {
      for (const name of classesIn(rule.selector)) declaredClasses.add(name);

      const classes = subjectClasses(rule.selector);
      if (!classes.length) continue;
      const entry = { ...rule, sheet: sheet.path, classes };

      if (STATE_PSEUDO.test(subjectCompound(rule.selector))) {
        stateRules.push(entry);
        const disabled = disabledClasses(rule.selector);
        if (disabled) disabledRules.push({ ...entry, classes: disabled });
      } else {
        skinRules.push(entry);
      }
    }
  }

  const hooks = collectBehaviourHooks(sources);
  const appliedClasses = collectAppliedClasses(sources);
  const disabledTargets = collectDisabledTargets(sources);

  const buttons = [];
  for (const { path, source } of sources) {
    for (const tag of findOpenTags(source, "button")) {
      const classes = classTokens(tag.tag);
      const id = tag.tag.match(/\bid\s*=\s*["'`]([\w-]+)["'`]/)?.[1] ?? null;
      buttons.push({
        file: path,
        line: lineOf(source, tag.index),
        tag: tag.tag,
        id,
        ...classes,
        disabledInMarkup:
          hasBareAttribute(tag.tag, "disabled") ||
          (id !== null && disabledTargets.ids.has(id)) ||
          classes.statics.some((name) => disabledTargets.classes.has(name)),
      });
    }
  }

  return {
    stylesheets,
    sources,
    declaredClasses,
    appliedClasses,
    skinRules,
    disabledRules,
    stateRules,
    hooks,
    buttons,
  };
}

/**
 * Everything that can draw this button in its resting state: every non-state
 * rule whose rightmost compound the button's own classes satisfy. Ancestors
 * are not resolved — a rule that needs one is counted, because we cannot prove
 * from markup that the button is outside it. The check is therefore about
 * whether the property is drawn AT ALL, which is the failure that matters: a
 * class no rule gives a border to has a bevelled OS border everywhere.
 */
function baseDeclarationsFor(model, classes) {
  const carried = new Set(classes);
  return model.skinRules
    .filter((rule) => rule.classes.every((name) => carried.has(name)))
    .map((rule) => `;${rule.body}`)
    .join("");
}

function missingSkin(model, button) {
  const declarations = baseDeclarationsFor(model, [...button.statics, ...button.conditional]);
  return Object.entries(REQUIRED_SKIN)
    .filter(([, pattern]) => !pattern.test(declarations))
    .map(([property]) => property);
}

function resolvesDisabled(model, button) {
  const carried = new Set([...button.statics, ...button.conditional]);
  return model.disabledRules.some(
    (rule) =>
      rule.classes.every((name) => carried.has(name)) && DISABLED_APPEARANCE.test(`;${rule.body}`)
  );
}

/* An expression that produces the WHOLE class attribute is opaque to a reader
   of the markup, so it is exempt from the skin checks - and reported instead,
   because nothing else can see inside it. */
function isOpaque(button) {
  return button.present && button.statics.length === 0 && button.dynamic;
}

export const CHECKS = {
  /** A `<button>` with no class of its own can only wear the UA's chrome. */
  "every button carries a class": (model) =>
    model.buttons
      .filter((button) => !button.present || (!button.statics.length && !button.dynamic))
      .map((button) => ({
        file: button.file,
        line: button.line,
        message: "button has no class; it renders as a native OS control",
      })),

  /** A class no stylesheet declares is a rename or a typo, never a style. */
  "every class on a button is declared or is a behaviour hook": (model) =>
    model.buttons.flatMap((button) =>
      [...button.statics, ...button.conditional]
        .filter((name) => !model.declaredClasses.has(name) && !model.hooks.has(name))
        .map((name) => ({
          file: button.file,
          line: button.line,
          message: `.${name} is in no stylesheet and no querySelector; it styles nothing`,
        }))
    ),

  /**
   * The one that would have caught Settings > About: the button's own classes
   * must draw every property the UA would otherwise supply, without help from
   * an ancestor, because the same class is reused outside that ancestor.
   */
  "every button draws its own base skin": (model) =>
    model.buttons
      .filter((button) => button.present && button.statics.length && !isOpaque(button))
      .map((button) => ({ button, missing: missingSkin(model, button) }))
      .filter(({ missing }) => missing.length)
      .map(({ button, missing }) => ({
        file: button.file,
        line: button.line,
        message: `class="${button.statics.join(" ")}" leaves ${missing.join(", ")} to the browser`,
      })),

  /**
   * A state a stylesheet draws that nothing ever sets. `.sub-tab.active` is
   * only real while some code path writes `active`; rename the class in the
   * JS and the CSS keeps compiling, keeps loading, and silently stops
   * mattering — the button just never looks selected again.
   */
  "every button state a stylesheet draws is a state the code sets": (model) => {
    const onButtons = new Set(
      model.buttons.flatMap((button) => [...button.statics, ...button.conditional])
    );
    const seen = new Set();
    return model.skinRules
      .concat(model.stateRules)
      .flatMap((rule) => {
        if (rule.classes.length < 2) return [];
        if (!rule.classes.some((name) => onButtons.has(name))) return [];
        return rule.classes
          .filter((name) => STATE_NAME.test(name) && !model.appliedClasses.has(name))
          .map((name) => ({ rule, name }));
      })
      .filter(({ rule, name }) => {
        const key = `${rule.sheet}:${rule.line}:${name}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      })
      .map(({ rule, name }) => ({
        file: `src/webserver/templates/styles/${rule.sheet}`,
        line: rule.line,
        message: `${rule.selector} draws the .${name} state, which no markup or classList call ever sets`,
      }));
  },

  /** A control that is ever disabled has to look it. */
  "every button that can be disabled has a disabled appearance": (model) =>
    model.buttons
      .filter((button) => button.disabledInMarkup && button.statics.length && !isOpaque(button))
      .filter((button) => !resolvesDisabled(model, button))
      .map((button) => ({
        file: button.file,
        line: button.line,
        message: `class="${button.statics.join(" ")}" is disabled at runtime but no rule changes how it looks`,
      })),

  /**
   * Appearance is a class. An inline colour or box outranks the stylesheet
   * that owns the component, is invisible to every other check here, and is
   * how one button drifts away from the family it belongs to. Layout and
   * visibility written inline are left alone - those are legitimately dynamic.
   */
  "no button paints itself inline": (model) =>
    model.buttons
      .map((button) => ({
        button,
        style: button.tag.match(/\bstyle\s*=\s*["'`]([^"'`]*)/i)?.[1] ?? "",
      }))
      .filter(({ style }) => INLINE_SKIN.test(`;${style}`))
      .map(({ button, style }) => ({
        file: button.file,
        line: button.line,
        message: `button paints itself inline (${style.trim()}); the skin belongs to its class`,
      })),
};

export async function auditButtonContract() {
  const model = await loadButtonModel();
  const results = new Map();
  for (const [name, check] of Object.entries(CHECKS)) {
    results.set(name, check(model));
  }
  return { model, results };
}

export { rulesIn, subjectCompound };
