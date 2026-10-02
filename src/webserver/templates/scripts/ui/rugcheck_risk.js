// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for Rugcheck risks. The provider sends each risk as English
 * `name` and `description` strings with no stable id, so its risk name is the
 * key. A name the map does not list shows the provider's own text.
 */

/** Provider risk name -> message key; the message's `.description` is the explanation. */
export const RUGCHECK_RISK_LABELS = Object.freeze({
  "Single holder ownership": "tokens-rugcheck-risk-single-holder-ownership",
  "Low Liquidity": "tokens-rugcheck-risk-low-liquidity",
  "Low amount of LP Providers": "tokens-rugcheck-risk-few-lp-providers",
  "High holder concentration": "tokens-rugcheck-risk-high-holder-concentration",
  "Top 10 holders high ownership": "tokens-rugcheck-risk-top-10-holders-high-ownership",
  "High ownership": "tokens-rugcheck-risk-high-ownership",
  "Creator history of rugged tokens": "tokens-rugcheck-risk-creator-rug-history",
  "Large Amount of LP Unlocked": "tokens-rugcheck-risk-large-lp-unlocked",
  "Mutable metadata": "tokens-rugcheck-risk-mutable-metadata",
  "Low Amount of holders": "tokens-rugcheck-risk-few-holders",
  "Copycat token": "tokens-rugcheck-risk-copycat-token",
  "Fee config enabled": "tokens-rugcheck-risk-fee-config-enabled",
  "High holder correlation": "tokens-rugcheck-risk-high-holder-correlation",
  "Freeze Authority still enabled": "tokens-rugcheck-risk-freeze-authority-enabled",
  "Mint Authority still enabled": "tokens-rugcheck-risk-mint-authority-enabled",
  "Missing file metadata": "tokens-rugcheck-risk-missing-file-metadata",
  "High market cap per holder": "tokens-rugcheck-risk-high-market-cap-per-holder",
  "Symbol Mismatch": "tokens-rugcheck-risk-symbol-mismatch",
  "Name Mismatch": "tokens-rugcheck-risk-name-mismatch",
  "Permanent Control Enabled": "tokens-rugcheck-risk-permanent-control-enabled",
  "Missing metadata": "tokens-rugcheck-risk-missing-metadata",
  "LP Unlock in": "tokens-rugcheck-risk-lp-unlock-soon",
  "LP Vault unlocked": "tokens-rugcheck-risk-lp-vault-unlocked",
  "Mint Authority locked": "tokens-rugcheck-risk-mint-authority-locked",
  "High transfer fee": "tokens-rugcheck-risk-high-transfer-fee",
});

const reported = new Set();

function isListed(name) {
  if (Object.hasOwn(RUGCHECK_RISK_LABELS, name)) return true;
  if (name && !reported.has(name)) {
    reported.add(name);
    console.debug("Rugcheck risk without a label, shown as sent:", name);
  }
  return false;
}

/** Localized risk name, or the provider's name when the risk is not listed. */
export function rugcheckRiskName(name) {
  const value = String(name ?? "");
  return isListed(value) ? I18n.label(RUGCHECK_RISK_LABELS, value) : value;
}

/** Localized explanation of a risk, or the provider's description when the risk is not listed. */
export function rugcheckRiskDescription(risk) {
  const value = String(risk?.name ?? "");
  const localized = isListed(value) ? I18n.labelAttr(RUGCHECK_RISK_LABELS, value, "description") : null;
  return localized || String(risk?.description ?? "");
}
