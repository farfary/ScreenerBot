/**
 * JavaScript parsing and traversal shared by the localization audit modules.
 * Parsing uses espree; traversal follows eslint-visitor-keys so it stays in
 * step with the ESLint toolchain already used for the dashboard scripts.
 */

import * as espree from "espree";
import { KEYS } from "eslint-visitor-keys";

const OPTIONS = { ecmaVersion: "latest", loc: true, range: true, comment: true };

/** `{ ast, comments }` or `{ error }`. Module goal first, classic script as fallback. */
export function parseJs(source) {
  let failure = null;
  for (const sourceType of ["module", "script"]) {
    try {
      const ast = espree.parse(source, { ...OPTIONS, sourceType });
      return { ast, comments: ast.comments ?? [] };
    } catch (error) {
      failure ??= error;
    }
  }
  return { error: failure };
}

function childKeys(node) {
  return KEYS[node.type] ?? Object.keys(node).filter((key) => !["loc", "range", "parent"].includes(key));
}

/** Depth-first walk; `visit(node, ancestors)` receives the ancestor chain, nearest last. */
export function walkAst(root, visit) {
  const ancestors = [];
  const step = (node) => {
    visit(node, ancestors);
    ancestors.push(node);
    for (const key of childKeys(node)) {
      const value = node[key];
      if (Array.isArray(value)) {
        for (const child of value) if (child && typeof child.type === "string") step(child);
      } else if (value && typeof value.type === "string") {
        step(value);
      }
    }
    ancestors.pop();
  };
  step(root);
}

export function isStringLiteral(node) {
  return node?.type === "Literal" && typeof node.value === "string";
}

/** Static string value of a string literal or expression-free template literal, else null. */
export function staticString(node) {
  if (isStringLiteral(node)) return node.value;
  if (node?.type === "TemplateLiteral" && node.expressions.length === 0) {
    return node.quasis[0].value.cooked ?? node.quasis[0].value.raw;
  }
  return null;
}

/** Name of a non-computed identifier key or a string-literal key. */
export function propertyName(node) {
  if (node.computed) return isStringLiteral(node.key) ? node.key.value : null;
  if (node.key?.type === "Identifier") return node.key.name;
  return isStringLiteral(node.key) ? node.key.value : null;
}

/** Member name of `a.b` / `a["b"]`, or null. */
export function memberName(node) {
  if (node?.type !== "MemberExpression") return null;
  if (!node.computed && node.property.type === "Identifier") return node.property.name;
  return isStringLiteral(node.property) ? node.property.value : null;
}

/** Interpolations are replaced by this mark so literal text keeps its adjacency. */
export const INTERPOLATION = "\u0001";

/** Literal text of a string or template literal with `${...}` replaced by INTERPOLATION. */
export function literalText(node) {
  if (isStringLiteral(node)) return node.value;
  if (node?.type !== "TemplateLiteral") return null;
  return node.quasis
    .map((quasi) => quasi.value.cooked ?? quasi.value.raw)
    .join(INTERPOLATION);
}

/** Comment index by end line: line number to the comment texts that end on it. */
export function commentsByLine(comments) {
  const byLine = new Map();
  for (const comment of comments) {
    const line = comment.loc.end.line;
    if (!byLine.has(line)) byLine.set(line, []);
    byLine.get(line).push(comment.value);
  }
  return byLine;
}
