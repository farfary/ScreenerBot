// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * A small HTML tokenizer for the localization audit. It reads page templates
 * and the markup inside JS string and template literals with one code path.
 * Only what the audit needs is modelled: elements with attributes, the text
 * that belongs directly to each element, and whether text sits under an
 * element that is already localized or marked `translate="no"`.
 */

import { INTERPOLATION } from "./ast.mjs";

const VOID_ELEMENTS = new Set([
  "area", "base", "br", "col", "embed", "hr", "img", "input",
  "link", "meta", "param", "source", "track", "wbr",
]);
const RAW_TEXT_ELEMENTS = new Set(["script", "style"]);

/** Function mapping a character offset to a 1-based line number. */
export function lineIndex(source) {
  const starts = [0];
  for (let i = 0; i < source.length; i += 1) if (source[i] === "\n") starts.push(i + 1);
  return (offset) => {
    let low = 0;
    let high = starts.length - 1;
    while (low < high) {
      const mid = (low + high + 1) >> 1;
      if (starts[mid] <= offset) low = mid;
      else high = mid - 1;
    }
    return low + 1;
  };
}

/** Text with interpolations, `{{PLACEHOLDER}}` tokens and character references removed. */
export function stripPlaceholders(text) {
  return text
    .replaceAll(INTERPOLATION, " ")
    .replace(/\{\{[^}]*\}\}/g, " ")
    .replace(/&(?:#\d+|#x[0-9a-f]+|[a-z][a-z0-9]*);/gi, " ");
}

export function hasMarkup(text) {
  return /<[a-zA-Z]/.test(text);
}

function isSpace(char) {
  return char === " " || char === "\n" || char === "\t" || char === "\r" || char === "\f";
}

function readTag(source, start) {
  let i = start + 1;
  while (i < source.length && !isSpace(source[i]) && source[i] !== ">" && source[i] !== "/") i += 1;
  const name = source.slice(start + 1, i).toLowerCase();
  const attrs = [];
  let selfClosing = false;

  while (i < source.length) {
    while (i < source.length && isSpace(source[i])) i += 1;
    if (source[i] === ">") {
      i += 1;
      break;
    }
    if (source[i] === "/") {
      if (source[i + 1] === ">") {
        selfClosing = true;
        i += 2;
        break;
      }
      i += 1;
      continue;
    }
    const nameStart = i;
    while (
      i < source.length &&
      !isSpace(source[i]) &&
      source[i] !== "=" &&
      source[i] !== ">" &&
      !(source[i] === "/" && source[i + 1] === ">")
    ) {
      i += 1;
    }
    const attrName = source.slice(nameStart, i).toLowerCase();
    let value = "";
    let valueIndex = i;
    while (i < source.length && isSpace(source[i])) i += 1;
    if (source[i] === "=") {
      i += 1;
      while (i < source.length && isSpace(source[i])) i += 1;
      const quote = source[i] === '"' || source[i] === "'" ? source[i] : null;
      if (quote) i += 1;
      valueIndex = i;
      while (
        i < source.length &&
        (quote ? source[i] !== quote : !isSpace(source[i]) && source[i] !== ">")
      ) {
        i += 1;
      }
      value = source.slice(valueIndex, i);
      if (quote) i += 1;
    }
    if (attrName) attrs.push({ name: attrName, value, index: valueIndex });
  }
  return { name, attrs, selfClosing, end: i };
}

/**
 * Tokenize `source`. Returns `{ elements, texts }`:
 * - element: `{ name, attrs, index, ownText, localized, noTranslate }` where the
 *   last two are true when the element itself or an ancestor carries
 *   `data-l10n-id` / `translate="no"`;
 * - text: `{ text, index, localized, noTranslate }` for text outside script/style.
 */
export function scanMarkup(source) {
  const elements = [];
  const texts = [];
  const stack = [];
  let i = 0;

  const addText = (text, index) => {
    if (!text) return;
    const owner = stack[stack.length - 1];
    if (owner) owner.ownText += text;
    texts.push({
      text,
      index,
      localized: Boolean(owner?.localized),
      noTranslate: Boolean(owner?.noTranslate),
    });
  };

  while (i < source.length) {
    const lt = source.indexOf("<", i);
    if (lt === -1) {
      addText(source.slice(i), i);
      break;
    }
    if (lt > i) addText(source.slice(i, lt), i);
    i = lt;

    if (source.startsWith("<!--", i)) {
      const close = source.indexOf("-->", i + 4);
      i = close === -1 ? source.length : close + 3;
    } else if (source[i + 1] === "!" || source[i + 1] === "?") {
      const close = source.indexOf(">", i);
      i = close === -1 ? source.length : close + 1;
    } else if (source[i + 1] === "/" && /[a-zA-Z]/.test(source[i + 2] ?? "")) {
      const close = source.indexOf(">", i);
      const name = source.slice(i + 2, close === -1 ? source.length : close).trim().toLowerCase();
      for (let s = stack.length - 1; s >= 0; s -= 1) {
        if (stack[s].name === name) {
          stack.length = s;
          break;
        }
      }
      i = close === -1 ? source.length : close + 1;
    } else if (/[a-zA-Z]/.test(source[i + 1] ?? "")) {
      const tag = readTag(source, i);
      const attr = (name) => tag.attrs.find((entry) => entry.name === name);
      const parent = stack[stack.length - 1];
      const hasId = attr("data-l10n-id") !== undefined;
      const hasNoTranslate = attr("translate")?.value.trim().toLowerCase() === "no";
      const element = {
        name: tag.name,
        attrs: tag.attrs,
        index: i,
        ownText: "",
        hasId,
        hasNoTranslate,
        localized: hasId || Boolean(parent?.localized),
        noTranslate: hasNoTranslate || Boolean(parent?.noTranslate),
      };
      elements.push(element);
      i = tag.end;
      if (RAW_TEXT_ELEMENTS.has(tag.name) && !tag.selfClosing) {
        const close = source.toLowerCase().indexOf(`</${tag.name}`, i);
        i = close === -1 ? source.length : close;
      } else if (!tag.selfClosing && !VOID_ELEMENTS.has(tag.name)) {
        stack.push(element);
      }
    } else {
      addText("<", i);
      i += 1;
    }
  }
  return { elements, texts };
}
