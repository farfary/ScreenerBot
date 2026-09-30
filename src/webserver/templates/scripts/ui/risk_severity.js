/**
 * Presentation for the severity of a token security risk, in one place.
 *
 * Ids are the normalized RugCheck risk levels: `danger`, `warn` and `info`.
 */

/** Message key of each severity label. */
export const RISK_SEVERITY_LABELS = Object.freeze({
  danger: "common-severity-critical",
  warn: "common-severity-warning",
  info: "common-severity-info",
});
