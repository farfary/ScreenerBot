// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategy test result - maps the verdicts of a stored strategy tested against one token onto the editor's rules and renders the marks and the overall line.

/**
 * The result is the `rules` outcome tree of `POST /api/strategies/{id}/test`: each node
 * carries its `verdict` (`pass`, `fail`, `unknown` or `off`), a group its `operator`
 * (`AND`, `OR`, `NOT`), a condition its `condition_type`, and an unknown node the
 * localized `reason` its data error gives. The tree mirrors the saved rules, which a test
 * requires to be the ones in the editor, so each outcome belongs to the editor node at the
 * same position. The whole strategy reads as the root's verdict, with the root's reason
 * when it is unknown; each condition and nested group reads as its own verdict, an unknown
 * condition with its reason.
 *
 * Every element a mark adds to a rule carries `data-test-mark`, so the marks can be taken
 * off the rules without redrawing them.
 *
 * Pure: returns markup and maps, touches no DOM.
 */

import { escapeHtml } from "../../core/utils.js";

const VERDICTS = Object.freeze({
  pass: { icon: "icon-circle-check", label: () => I18n.t("strategies-test-verdict-pass") },
  fail: { icon: "icon-circle-x", label: () => I18n.t("strategies-test-verdict-fail") },
  unknown: {
    icon: "icon-circle-question-mark",
    label: () => I18n.t("strategies-test-verdict-unknown"),
  },
  off: { icon: "icon-circle-off", label: () => I18n.t("strategies-chip-off") },
});

function verdictHtml(verdict) {
  const known = Object.hasOwn(VERDICTS, verdict) ? verdict : "unknown";
  const { icon, label } = VERDICTS[known];
  return `<span class="strategy-test-verdict" data-verdict="${known}" data-test-mark><i class="${icon}" aria-hidden="true"></i> ${escapeHtml(label())}</span>`;
}

const isNot = (outcome) => outcome?.operator === "NOT";
const isGroupOutcome = (outcome) => Boolean(outcome) && !outcome.condition_type && !isNot(outcome);

/**
 * The verdict of every editor node the outcome tree reaches, by node id. The editor shows
 * a NOT of the saved tree as `inverted` on the node it wraps, so an inverted node takes
 * the NOT's verdict and its children come from the NOT's only child. A subtree whose shape
 * does not match the editor's is left unmarked rather than marked by guess.
 *
 * @param {object} root - the editor's root group (`rule_tree.js` model)
 * @param {object} rules - the result's `rules` outcome tree
 * @returns {Map<number, {verdict: string, reason: object|null}>}
 */
export function verdictsByNode(root, rules) {
  const marks = new Map();

  function visit(node, outcome) {
    if (!outcome) return;
    let inner = outcome;
    if (node.inverted) {
      if (!isNot(outcome) || outcome.children?.length !== 1) return;
      [inner] = outcome.children;
    }
    if (node.kind === "group" ? !isGroupOutcome(inner) : inner.condition_type !== node.type) return;
    marks.set(node.id, {
      verdict: outcome.verdict,
      reason: outcome.reason ?? inner.reason ?? null,
    });
    if (node.kind !== "group") return;
    const children = inner.children ?? [];
    if (children.length !== node.children.length) return;
    node.children.forEach((child, index) => visit(child, children[index]));
  }

  // A saved tree whose top is a single condition, or a NOT of one, opens as a root group
  // holding that node alone; the root's outcome is then that node's.
  if (rules && !root.inverted && !isGroupOutcome(rules) && root.children.length === 1) {
    marks.set(root.id, { verdict: rules.verdict, reason: rules.reason ?? null });
    visit(root.children[0], rules);
  } else {
    visit(root, rules);
  }
  return marks;
}

/**
 * Markup of one rule's mark: the verdict for the end of its head and, for an unknown
 * condition, the reason line under it.
 *
 * @param {{verdict: string, reason: object|null}} mark
 * @param {object} options
 * @param {boolean} options.condition - whether the rule is a condition row
 * @returns {{verdict: string, reason: string}}
 */
export function renderNodeVerdict({ verdict, reason }, { condition }) {
  // A group's reason is the first one a condition under it reached; it is shown there.
  const showReason = condition && verdict === "unknown" && reason;
  return {
    verdict: verdictHtml(verdict),
    reason: showReason
      ? `<p class="strategy-test-reason" data-test-mark>${escapeHtml(I18n.text(reason))}</p>`
      : "",
  };
}

/**
 * Markup of the overall line: the tested token, the whole strategy's verdict and, when it
 * is unknown, its reason.
 *
 * @param {object} rules - the result's `rules` outcome tree
 * @param {object} options
 * @param {string} options.token - the tested token as the user knows it
 */
export function renderTestSummary(rules, { token }) {
  const reason =
    rules.verdict === "unknown" && rules.reason
      ? `<span class="strategy-test-summary-reason">${escapeHtml(I18n.text(rules.reason))}</span>`
      : "";
  return `<span class="strategy-test-title">${escapeHtml(I18n.t("strategies-test-title", { token }))}</span>
    ${verdictHtml(rules.verdict)}
    ${reason}`;
}

/** Markup of the overall line while the test runs: the shared inline spinner and the token. */
export function renderTestRunning({ token }) {
  return `<span class="strategy-test-running loading-spinner inline">${escapeHtml(I18n.t("strategies-test-running", { token }))}</span>`;
}
