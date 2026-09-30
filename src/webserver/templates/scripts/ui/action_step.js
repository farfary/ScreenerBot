/**
 * Presentation for the step codes of a trade action.
 *
 * The API sends `ActionStepCode` (src/actions/step_code.rs) as a snake_case
 * code in `steps[].name`, `state.current_step` and `step_name`. Rows stored
 * before steps were codes are mapped to the same codes by the Rust
 * deserializer, so only codes reach this module.
 */

/** Message key of each step's label; keep in step with `ActionStepCode`. */
export const ACTION_STEP_LABELS = Object.freeze({
  evaluate: "actions-step-evaluate",
  validate: "actions-step-validate",
  quote: "actions-step-quote",
  swap: "actions-step-swap",
  verify: "actions-step-verify",
  unknown: "actions-step-unknown",
});

/** Compact wording for the positions state caption. */
export const ACTION_STEP_SHORT_LABELS = Object.freeze({
  evaluate: "actions-step-evaluate-short",
  validate: "actions-step-validate-short",
  quote: "actions-step-quote-short",
  swap: "actions-step-swap-short",
  verify: "actions-step-verify-short",
  unknown: "actions-step-unknown-short",
});

export function stepLabel(code) {
  return I18n.label(ACTION_STEP_LABELS, code);
}

export function stepShortLabel(code) {
  return I18n.label(ACTION_STEP_SHORT_LABELS, code);
}
