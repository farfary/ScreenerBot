/**
 * Value formatting outside the formatter owner.
 *
 * `scripts/core/format.js` owns every number, money, date, time and size
 * display and pins the locale through `I18n.intlLocale`. Dashboard scripts
 * therefore may not format values themselves. Each of these is an error, not a
 * baseline count:
 *
 *   - a call to a member named toLocaleString, toLocaleDateString or toLocaleTimeString
 *   - any member of `Intl` (Intl.NumberFormat, Intl.DateTimeFormat, ...)
 *   - the string literal "en-US"
 *
 * `core/format.js` and `core/i18n.js` are exempt. `// l10n-format-ok: <reason>`
 * on the same or the previous line skips a genuinely non-display use, such as a
 * machine-readable input value; the reason is mandatory.
 */

import { memberName, parseJs, staticString, walkAst } from "./ast.mjs";

const LOCALE_METHODS = new Set(["toLocaleString", "toLocaleDateString", "toLocaleTimeString"]);
const HARDCODED_LOCALE = "en-US";
const ALLOWED_FILES = ["scripts/core/format.js", "scripts/core/i18n.js"];
const ESCAPE_MARK = /l10n-format-ok\b:?(.*)$/s;

export const isFormattingOwner = (path) => ALLOWED_FILES.some((allowed) => path.endsWith(`/${allowed}`) || path === allowed);

/** `{ errors, escapes }` for one script. Pure over the source text. */
export function scanFormatting({ source, path }) {
  if (isFormattingOwner(path)) return { errors: [], escapes: 0 };

  const parsed = parseJs(source);
  if (parsed.error) {
    return {
      errors: [{ file: path, line: parsed.error.lineNumber, message: `cannot parse: ${parsed.error.message}` }],
      escapes: 0,
    };
  }

  const errors = [];
  const marked = new Set();
  let escapes = 0;
  for (const comment of parsed.comments) {
    const match = ESCAPE_MARK.exec(comment.value);
    if (!match) continue;
    if (match[1].trim() === "") {
      errors.push({
        file: path,
        line: comment.loc.start.line,
        message: "l10n-format-ok requires a reason: // l10n-format-ok: <reason>",
      });
    } else {
      marked.add(comment.loc.end.line);
      escapes += 1;
    }
  }

  const report = (line, message) => {
    if (marked.has(line) || marked.has(line - 1)) return;
    errors.push({ file: path, line, message });
  };

  walkAst(parsed.ast, (node) => {
    if (node.type === "CallExpression" && LOCALE_METHODS.has(memberName(node.callee))) {
      report(
        node.callee.property.loc.start.line,
        `${memberName(node.callee)}() formats a value outside core/format.js; use the format.js owner`
      );
    } else if (
      node.type === "MemberExpression" &&
      ((node.object.type === "Identifier" && node.object.name === "Intl") || memberName(node) === "Intl")
    ) {
      report(node.loc.start.line, "Intl is used outside core/format.js; use the format.js owner");
    } else if (
      (node.type === "Literal" || node.type === "TemplateLiteral") &&
      staticString(node) === HARDCODED_LOCALE
    ) {
      report(node.loc.start.line, `the "${HARDCODED_LOCALE}" locale literal is used outside core/format.js`);
    }
  });

  return { errors, escapes };
}
