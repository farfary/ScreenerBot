/**
 * Physical-direction CSS: declarations that mirror incorrectly in a
 * right-to-left layout. Logical properties (`margin-inline-start`, `inset-inline-end`,
 * `text-align: start`) are the replacement. `translateX` is out of scope.
 *
 * `/* rtl-ok: <reason> *\/` on the same line or the line above skips a declaration.
 */

import postcss from "postcss";

const PHYSICAL_PROPERTIES = new Set([
  "margin-left", "margin-right", "padding-left", "padding-right", "left", "right",
  "border-top-left-radius", "border-top-right-radius",
  "border-bottom-left-radius", "border-bottom-right-radius",
]);
const SIDE_KEYWORD_PROPERTIES = new Set(["text-align", "float", "clear"]);
const FOUR_VALUE_SHORTHANDS = new Set(["margin", "padding", "inset", "border-width", "border-radius"]);
const RTL_OK = /rtl-ok\b:?(.*)$/s;

/** Whitespace-separated values at top level, so `calc(1px + 2px)` stays whole. */
function splitValues(value) {
  const parts = [];
  let depth = 0;
  let current = "";
  for (const char of value.trim()) {
    if (char === "(") depth += 1;
    else if (char === ")") depth = Math.max(0, depth - 1);
    if (depth === 0 && /\s/.test(char)) {
      if (current) parts.push(current);
      current = "";
    } else {
      current += char;
    }
  }
  if (current) parts.push(current);
  return parts;
}

/** Four-value shorthand whose physical left and right sides differ. */
function asymmetricShorthand(property, value) {
  const radius = property === "border-radius";
  return value.split("/").some((half) => {
    const parts = splitValues(half);
    if (parts.length !== 4) return false;
    return radius ? parts[0] !== parts[1] || parts[2] !== parts[3] : parts[1] !== parts[3];
  });
}

/** Why a declaration is physical, or null. */
export function physicalReason(property, value) {
  const name = property.toLowerCase();
  const text = value.trim().toLowerCase();
  if (PHYSICAL_PROPERTIES.has(name) || /^border-(left|right)(-|$)/.test(name)) return name;
  if (SIDE_KEYWORD_PROPERTIES.has(name) && (text === "left" || text === "right")) return `${name}: ${text}`;
  if (FOUR_VALUE_SHORTHANDS.has(name) && asymmetricShorthand(name, value)) return `${name} with different left and right`;
  return null;
}

export function scanCss({ source, path }) {
  const items = [];
  const errors = [];
  let ignores = 0;
  let root;
  try {
    root = postcss.parse(source, { from: path });
  } catch (error) {
    return { items, ignores, errors: [{ file: path, line: error.line, message: `cannot parse: ${error.reason ?? error.message}` }] };
  }

  const sameLine = new Set();
  const above = new Set();
  root.walkComments((comment) => {
    const match = RTL_OK.exec(comment.text);
    if (!match) return;
    if (match[1].trim() === "") {
      errors.push({ file: path, line: comment.source.start.line, message: "rtl-ok requires a reason: /* rtl-ok: <reason> */" });
    } else {
      sameLine.add(comment.source.end.line);
      /* A comment trailing another declaration must not cover the next one. */
      if (!comment.prev() || comment.raws.before.includes("\n")) above.add(comment.source.end.line);
      ignores += 1;
    }
  });

  root.walkDecls((decl) => {
    const reason = physicalReason(decl.prop, decl.value);
    if (!reason) return;
    const line = decl.source.start.line;
    if (sameLine.has(line) || above.has(line - 1)) return;
    items.push({ line, text: `${decl.prop}: ${decl.value}`, kind: reason });
  });
  return { items, ignores, errors };
}
