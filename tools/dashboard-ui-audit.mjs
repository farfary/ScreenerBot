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
    selectors.push(...candidate.split(",").map((selector) => selector.trim()));
  }
  return selectors;
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

for (const file of cssFiles) {
  const css = await readFile(file, "utf8");
  const path = relative(stylesRoot, file);

  for (const selector of selectorsIn(css)) {
    const owner = canonicalOwners.get(selector);
    if (owner && path !== owner) {
      errors.push(`${path}: ${selector} is owned by ${owner}`);
    }
    if (/\.(?:source|category)-switch\b|\.slider\b/.test(selector)) {
      errors.push(`${path}: custom switch selector ${selector}; use .toggle`);
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
}

for (const file of await walk(pagesRoot)) {
  if (!file.endsWith(".html")) continue;
  const source = await readFile(file, "utf8");
  if (/<style\b/i.test(source)) {
    errors.push(`${relative(root, file)}: page templates must not contain <style>`);
  }
  auditSelectContract(file, source);
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

const aiStyleManifest = templatesSource.match(/"ai"\s*=>\s*\[([\s\S]*?)\]\s*\.join/);
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
  if (aiStyleManifest?.[1].includes(style)) {
    errors.push(`${style} must not depend on the AI route-scoped style manifest`);
  }
}

if (errors.length) {
  console.error("Dashboard UI contract violations:\n");
  errors.forEach((error) => console.error(`- ${error}`));
  process.exitCode = 1;
} else {
  console.log(`Dashboard UI audit passed (${cssFiles.length} stylesheets checked).`);
}
