/* global console, process */

import { readFile, readdir } from "node:fs/promises";
import { relative, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const stylesRoot = resolve(root, "src/webserver/templates/styles");
const scriptsRoot = resolve(root, "src/webserver/templates/scripts");
const pagesRoot = resolve(root, "src/webserver/templates/pages");

const canonicalOwners = new Map([
  [".metric-card", "components.css"],
  [".metric-icon", "components.css"],
  [".metric-content", "components.css"],
  [".metric-label", "components.css"],
  [".metric-value", "components.css"],
  [".metric-detail", "components.css"],
  [".empty-state", "components.css"],
  [".empty-state-title", "components.css"],
  [".empty-state-description", "components.css"],
  [".empty-state-action", "components.css"],
  [".modal-overlay", "components.css"],
  [".modal-dialog", "components.css"],
  [".modal-header", "components.css"],
  [".modal-close", "components.css"],
  [".modal-body", "components.css"],
  [".modal-tabs", "components.css"],
  [".modal-tab", "components.css"],
  [".modal-tab-content", "components.css"],
  [".modal-footer", "components.css"],
  [".section-header", "common.css"],
  [".section-subtitle", "common.css"],
  [".form-group", "components/form_controls.css"],
  [".input-group", "components/form_controls.css"],
  [".toggle", "components/form_controls.css"],
  [".toggle-track", "components/form_controls.css"],
  [".toggle-label", "components/form_controls.css"],
  [".toggle-state", "components/form_controls.css"],
  [".checkbox-label", "components/form_controls.css"],
  [".number-field", "components/form_controls.css"],
  [".number-field-suffix", "components/form_controls.css"],
  [".number-field-spin", "components/form_controls.css"],
  [".number-field-step", "components/form_controls.css"],
  [".input-unit", "components/form_controls.css"],
  [".sub-tabs-container", "ui/tab_bar.css"],
  [".sub-tab", "ui/tab_bar.css"],
]);

async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(
    entries.map((entry) => {
      const path = resolve(directory, entry.name);
      return entry.isDirectory() ? walk(path) : [path];
    })
  );
  return files.flat();
}

function selectorsIn(css) {
  const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const selectors = [];
  const ruleStart = /([^{}]+)\{/g;
  let match;
  while ((match = ruleStart.exec(withoutComments))) {
    const candidate = match[1].slice(match[1].lastIndexOf(";") + 1).trim();
    if (!candidate || candidate.startsWith("@")) continue;
    selectors.push(...splitSelectorList(candidate));
  }
  return selectors;
}

function splitSelectorList(value) {
  const selectors = [];
  let start = 0;
  let depth = 0;

  for (let index = 0; index < value.length; index += 1) {
    const char = value[index];
    if (char === "(" || char === "[") depth += 1;
    else if (char === ")" || char === "]") depth = Math.max(0, depth - 1);
    else if (char === "," && depth === 0) {
      selectors.push(value.slice(start, index).trim());
      start = index + 1;
    }
  }

  selectors.push(value.slice(start).trim());
  return selectors.filter(Boolean);
}

/* Selector + declaration body per rule, so a check can look at what a rule
   actually draws and not only at what it targets. */
function rulesIn(css) {
  const withoutComments = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const rules = [];
  const pattern = /([^{}]+)\{([^{}]*)\}/g;
  let match;
  while ((match = pattern.exec(withoutComments))) {
    const selector = match[1].slice(match[1].lastIndexOf(";") + 1).trim();
    if (!selector || selector.startsWith("@")) continue;
    splitSelectorList(selector).forEach((part) => rules.push([part, match[2]]));
  }
  return rules;
}

const controlSkinDeclaration =
  /^\s*(width|height|min-width|max-width|min-height|max-height|padding(?:-[a-z-]+)?|background(?:-color)?|border(?:-[a-z-]+)?|border-radius|box-shadow|outline|appearance|accent-color)\s*:/m;

function inputMayBeChoiceControl(selector) {
  let depth = 0;
  let compoundStart = 0;
  for (let index = 0; index < selector.length; index += 1) {
    const char = selector[index];
    if (char === "(" || char === "[") depth += 1;
    else if (char === ")" || char === "]") depth = Math.max(0, depth - 1);
    else if (depth === 0 && /[\s>+~]/.test(char)) compoundStart = index + 1;
  }

  const target = selector.slice(compoundStart).trim();
  const inputTarget = target.match(/^input(?=$|[:.#]|\[)(.*)$/i);
  if (!inputTarget) return false;

  const tail = inputTarget[1];
  const exactType = tail.match(/^\s*\[\s*type\s*=\s*["']?([\w-]+)["']?\s*\]/i);
  if (exactType) return /^(?:checkbox|radio)$/i.test(exactType[1]);

  const exclusions = tail.match(/^\s*:not\(([^)]*)\)/i)?.[1] || "";
  const excludesCheckbox = /\[\s*type\s*=\s*["']?checkbox["']?\s*\]/i.test(exclusions);
  const excludesRadio = /\[\s*type\s*=\s*["']?radio["']?\s*\]/i.test(exclusions);
  return !excludesCheckbox || !excludesRadio;
}

const errors = [];
const cssFiles = (await walk(stylesRoot)).filter((file) => file.endsWith(".css"));

function auditSelectContract(file, source) {
  for (const match of source.matchAll(/<select\b[^>]*>/gi)) {
    if (/\bdata-custom-select\b/i.test(match[0])) continue;
    const line = source.slice(0, match.index).split("\n").length;
    errors.push(
      `${relative(root, file)}:${line}: native select is forbidden; add data-custom-select`
    );
  }
}

function auditNativeChoiceContract(file, source) {
  for (const match of source.matchAll(
    /<[^>]+\brole\s*=\s*["'](?:switch|checkbox|radio)["'][^>]*>/gi
  )) {
    const line = source.slice(0, match.index).split("\n").length;
    errors.push(
      `${relative(root, file)}:${line}: simulated choice control is forbidden; use a native checkbox/radio and the shared control family`
    );
  }
}

for (const file of cssFiles) {
  const css = await readFile(file, "utf8");
  const path = relative(stylesRoot, file);

  for (const selector of selectorsIn(css)) {
    const owner = canonicalOwners.get(selector);
    if (owner && path !== owner) {
      errors.push(`${path}: ${selector} is owned by ${owner}`);
    }
    if (/\.(?:source|category)-switch\b|\.slider\b|-switch__/.test(selector)) {
      errors.push(`${path}: custom switch selector ${selector}; use .toggle`);
    }
    if (/(?:^|[\s>+~,])\.[\w-]+-radio\b/.test(selector)) {
      errors.push(`${path}: custom radio selector ${selector}; use input[type="radio"]`);
    }
  }

  /* The control family (switch, checkbox, radio) has one skin. A page may place
     a control - margins, order, alignment - but never re-draw its box, which is
     how four different switches and five checkbox sizes grew in the first place. */
  if (path !== "components/form_controls.css") {
    for (const [selector, body] of rulesIn(css)) {
      if (!inputMayBeChoiceControl(selector)) continue;
      const skin = body.match(controlSkinDeclaration);
      if (skin) {
        errors.push(
          `${path}: ${selector} sets ${skin[1]}; the control skin is owned by components/form_controls.css`
        );
      }
    }
  }

  if (
    path !== "ui/tab_bar.css" &&
    /\.sub-tabs-container[^{]*\{[^}]*display\s*:[^;}]*!important/is.test(css)
  ) {
    errors.push(`${path}: shared subtab visibility must be controlled by TabBar`);
  }

  if (path !== "components/form_controls.css" && /-webkit-(?:inner|outer)-spin-button/.test(css)) {
    errors.push(`${path}: the number stepper is owned by components/form_controls.css`);
  }
}

for (const file of await walk(scriptsRoot)) {
  if (!file.endsWith(".js")) continue;
  const source = await readFile(file, "utf8");
  if (/createElement\(["']style["']\)|<style\b/i.test(source)) {
    errors.push(`${relative(root, file)}: runtime or embedded CSS is forbidden; use styles/`);
  }
  if (!file.endsWith("custom_select.js")) auditSelectContract(file, source);
  auditNativeChoiceContract(file, source);
}

for (const file of await walk(pagesRoot)) {
  if (!file.endsWith(".html")) continue;
  const source = await readFile(file, "utf8");
  if (/<style\b/i.test(source)) {
    errors.push(`${relative(root, file)}: page templates must not contain <style>`);
  }
  auditSelectContract(file, source);
  auditNativeChoiceContract(file, source);
}

const templatesSource = await readFile(resolve(root, "src/webserver/templates.rs"), "utf8");
const baseSource = await readFile(resolve(root, "src/webserver/templates/base.html"), "utf8");
const routerSource = await readFile(
  resolve(root, "src/webserver/templates/scripts/core/router.js"),
  "utf8"
);
const chatWidgetSource = await readFile(
  resolve(root, "src/webserver/templates/scripts/core/chat_widget.js"),
  "utf8"
);
const chatWidgetLayoutSource = await readFile(
  resolve(root, "src/webserver/templates/styles/components/chat_widget/layout.css"),
  "utf8"
);

if (templatesSource.includes("__PAGE_STYLES__") || baseSource.includes("__PAGE_STYLES__")) {
  errors.push("Page CSS must not be serialized into a global JavaScript registry");
}
if (!templatesSource.includes("pub fn page_styles")) {
  errors.push("templates.rs must expose the centralized page_styles manifest");
}
if (!routerSource.includes("/styles/pages/")) {
  errors.push("router.js must load route-scoped page styles from /styles/pages/");
}

const assistantStyleManifest = templatesSource.match(
  /"assistant"\s*=>\s*\[([\s\S]*?)\]\s*\.join/
);
const globalStyleManifest = templatesSource.match(/let combined_styles = \[([\s\S]*?)\];/);
const chatWidgetStyles = [
  "CHAT_WIDGET_LAYOUT_STYLES",
  "CHAT_WIDGET_MESSAGES_STYLES",
  "CHAT_WIDGET_INPUT_STYLES",
];

if (!chatWidgetSource.includes('class="chat-widget chat-container')) {
  errors.push("ChatWidget must expose the .chat-widget CSS scope root");
}
if (/(?:^|\n)\s*\.chat-container\s*\{/m.test(chatWidgetLayoutSource)) {
  errors.push("ChatWidget scope-root layout must target :scope, not descendant .chat-container");
}
if (/(^|[\s,{])\.cw-host-(?:page|dialog)(?:\.sessions-open)?\s+\./m.test(chatWidgetLayoutSource)) {
  errors.push("ChatWidget root host modifiers inside @scope must use :scope.cw-host-*");
}
for (const style of chatWidgetStyles) {
  if (!globalStyleManifest?.[1].includes(style)) {
    errors.push(`${style} must be loaded by the global shared-style manifest`);
  }
  if (assistantStyleManifest?.[1].includes(style)) {
    errors.push(
      `${style} must not depend on the Assistant route-scoped style manifest`
    );
  }
}

if (errors.length) {
  console.error("Dashboard UI contract violations:\n");
  errors.forEach((error) => console.error(`- ${error}`));
  process.exitCode = 1;
} else {
  console.log(`Dashboard UI audit passed (${cssFiles.length} stylesheets checked).`);
}
