/**
 * Physical-direction CSS: declarations that mirror incorrectly in a
 * right-to-left layout. Logical properties (`margin-inline-start`, `inset-inline-end`,
 * `text-align: start`) are the replacement. A horizontal `translate` offset is physical
 * unless it is scaled by `--dir-sign` (foundation.css: 1 in LTR, -1 in RTL), or is `-50%`
 * (centering against `left: 50%`) or zero. `@keyframes` bodies are scanned like any rule.
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

/**
 * Shorthand whose physical left and right sides differ. Box sides only differ with
 * four values; radius corners expand as [a, b, a, b] / [a, b, c, b] from two or three.
 */
function asymmetricShorthand(property, value) {
  const radius = property === "border-radius";
  return value.split("/").some((half) => {
    const parts = splitValues(half);
    if (!radius) return parts.length === 4 && parts[1] !== parts[3];
    const [a, b = a, c = a, d = b] = parts;
    return a !== b || d !== c;
  });
}

const ZERO_LENGTH = /^[+-]?0*\.?0+(px|%|rem|em|vw)?$/;

/** Top-level comma-separated arguments of a CSS function body. */
function splitArguments(body) {
  const parts = [];
  let depth = 0;
  let current = "";
  for (const char of body) {
    if (char === "(") depth += 1;
    else if (char === ")") depth = Math.max(0, depth - 1);
    if (depth === 0 && char === ",") {
      parts.push(current.trim());
      current = "";
    } else {
      current += char;
    }
  }
  parts.push(current.trim());
  return parts;
}

/** True when a translation X component moves along the physical horizontal axis. */
function physicalOffset(x) {
  if (x === undefined || x === "") return false;
  if (x.includes("--dir-sign")) return false;
  const text = x.replace(/\s+/g, "");
  return text !== "-50%" && !ZERO_LENGTH.test(text);
}

/** X components of every translation in a `transform` or `translate` value. */
function translationOffsets(name, value) {
  if (name === "translate") return [splitValues(value)[0]];
  const offsets = [];
  const pattern = /translate(X|3d)?\(/gi;
  let match;
  while ((match = pattern.exec(value))) {
    let depth = 1;
    let end = pattern.lastIndex;
    while (end < value.length && depth > 0) {
      if (value[end] === "(") depth += 1;
      else if (value[end] === ")") depth -= 1;
      end += 1;
    }
    const args = splitArguments(value.slice(pattern.lastIndex, end - 1));
    offsets.push(args[0]);
    pattern.lastIndex = end;
  }
  return offsets;
}

/** Why a declaration is physical, or null. */
export function physicalReason(property, value) {
  const name = property.toLowerCase();
  const text = value.trim().toLowerCase();
  if ((name === "transform" || name === "translate") && translationOffsets(name, text).some(physicalOffset)) {
    return "translateX without --dir-sign";
  }
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
