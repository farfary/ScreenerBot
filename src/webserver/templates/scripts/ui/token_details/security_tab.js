// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Token Details Dialog - Security Tab
 * Extracted from token_details_dialog.js to reduce file size
 */
import * as Utils from "../../core/utils.js";
import { formatPercentValue } from "../../core/format.js";
import { RISK_SEVERITY_LABELS } from "../risk_severity.js";
import { rugcheckRiskDescription, rugcheckRiskName } from "../rugcheck_risk.js";
import { renderAddress } from "../token_identity.js";
import { renderStateView } from "../state_view.js";

const esc = (text) => Utils.escapeHtml(text);

// Percentages here are non-negative shares; only a negative value carries a sign.
const PLAIN_PERCENT = Object.freeze({ decimals: 2, plus: "" });

// Ids are the client-side bands from getSafetyScoreLabel.
const SAFETY_SCORE_LABELS = Object.freeze({
  shielded: "tokens-security-grade-shielded",
  safe: "tokens-security-grade-safe",
  caution: "tokens-security-grade-caution",
  vulnerable: "tokens-security-grade-vulnerable",
  unknown: "tokens-security-grade-unknown",
});

// Ids are the client-side bands from getConcentrationState.
const CONCENTRATION_LABELS = Object.freeze({
  unknown: "tokens-security-concentration-unknown",
  critical: "tokens-security-concentration-critical",
  high: "tokens-security-concentration-high",
  moderate: "tokens-security-concentration-moderate",
  healthy: "tokens-security-concentration-healthy",
});

/**
 * Render the security tab content (with loading state)
 * @param {Object} token - Token data object
 * @param {Object} options - Rendering options
 * @returns {string} HTML string for security tab
 */
export function renderSecurityTab(token, options = {}) {
  const { renderHintTrigger, escapeHtml } = options;
  const hasSecurityData = token.safety_score !== undefined && token.safety_score !== null;

  if (!hasSecurityData) {
    return buildSecurityLoadingContent(token, { renderHintTrigger });
  }

  return buildSecurityContent(token, { renderHintTrigger, escapeHtml });
}

function buildSecurityLoadingContent(token, options) {
  const { renderHintTrigger } = options;

  return `
    <div class="security-container">
      <div class="security-left-col">
        <div class="security-loading-notice">
          <div class="loading-spinner-small"></div>
          <span>${esc(I18n.t("tokens-security-analysis-pending"))}</span>
        </div>
        <section class="security-summary">
          ${buildSectionHeader(
            `<span class="security-section-title"><i class="icon-shield-check"></i> ${esc(I18n.t("tokens-security-pulse-title"))} ${renderHintTrigger("tokenDetails.security")}</span>`
          )}
          <div class="security-score-overview">
            ${buildScoreRing(null, "")}
            <div class="security-score-copy">
              <span class="security-grade is-pending">${esc(I18n.t("tokens-security-grade-analyzing"))}</span>
              <span class="security-score-caption">${esc(I18n.t("tokens-security-pending-caption"))}</span>
            </div>
          </div>
          <div class="security-summary-section">
            ${buildSubsectionHeader(I18n.t("tokens-security-control-title"), I18n.t("tokens-security-control-meta"))}
            ${buildAuthorityList(token)}
          </div>
        </section>
      </div>
      <div class="security-right-col">
        ${renderStateView({ kind: "loading", message: I18n.t("tokens-security-analyzing") })}
      </div>
    </div>
  `;
}

function buildSecurityContent(token, options) {
  const { renderHintTrigger, escapeHtml } = options;
  const safetyScore = token.safety_score;
  const scoreClass = getSafetyScoreClass(safetyScore);
  const scoreLabel = getSafetyScoreLabel(safetyScore);

  return `
    <div class="security-container">
      <div class="security-left-col">
        ${buildSecuritySummary(token, safetyScore, scoreClass, scoreLabel, {
          renderHintTrigger,
          escapeHtml,
        })}
      </div>
      <div class="security-right-col">
        ${buildHolderHealthSection(token)}
        ${buildTransferFeeSection(token)}
        ${buildRisksSection(token.security_risks, { escapeHtml })}
        ${buildTopHoldersSection(token)}
      </div>
    </div>
  `;
}

function buildSecuritySummary(token, safetyScore, scoreClass, scoreLabel, options) {
  const { renderHintTrigger, escapeHtml } = options;
  const lastUpdated = token.security_last_updated
    ? Utils.formatTimestamp(token.security_last_updated)
    : null;
  const gradeClassMap = {
    "score-safe": "is-good",
    "score-caution": "is-warning",
    "score-vulnerable": "is-critical",
  };
  const gradeClass = gradeClassMap[scoreClass] || "is-warning";
  const meta =
    lastUpdated && !token.rugged
      ? esc(I18n.t("tokens-security-updated", { time: lastUpdated }))
      : "";

  return `
    <section class="security-summary">
      ${buildSectionHeader(
        `<span class="security-section-title"><i class="icon-shield-check"></i> ${esc(I18n.t("tokens-security-pulse-title"))} ${renderHintTrigger("tokenDetails.security")}</span>`,
        meta
      )}
      <div class="security-score-overview">
        ${buildScoreRing(safetyScore, scoreClass)}
        <div class="security-score-copy">
          <span class="security-grade ${gradeClass}">${esc(scoreLabel)}</span>
          <span class="security-score-caption">${esc(I18n.t("tokens-security-score-caption"))}</span>
          ${
            token.rugged
              ? `<span class='security-rugged'><i class='icon-skull'></i> ${esc(I18n.t("tokens-security-rugged"))}</span>`
              : ""
          }
        </div>
      </div>
      ${buildSummaryMetrics(token, { escapeHtml })}
      <div class="security-summary-section">
        ${buildSubsectionHeader(I18n.t("tokens-security-control-title"), I18n.t("tokens-security-control-meta"))}
        ${buildAuthorityList(token)}
      </div>
    </section>
  `;
}

function buildSectionHeader(titleHtml, meta = "") {
  return `
    <header class="security-section-header">
      ${titleHtml}
      ${meta ? `<span class="security-section-meta">${meta}</span>` : ""}
    </header>
  `;
}

function buildSubsectionHeader(title, meta = "") {
  return `
    <div class="security-subsection-header">
      <span>${esc(title)}</span>
      ${meta ? `<span class="security-section-meta">${esc(meta)}</span>` : ""}
    </div>
  `;
}

function buildScoreRing(score, scoreClass) {
  const isPending = score === null || score === undefined;
  const normalizedScore = isPending ? 0 : Math.min(100, Math.max(0, Number(score)));
  const circumference = 2 * Math.PI * 46;
  const offset = circumference - (normalizedScore / 100) * circumference;

  return `
    <div class="security-score-ring ${isPending ? "is-pending" : ""}">
      <svg class="security-score-progress" width="108" height="108" viewBox="0 0 120 120" aria-hidden="true">
        <circle class="security-score-track" cx="60" cy="60" r="46"></circle>
        ${
          isPending
            ? ""
            : `<circle class="security-score-value-ring ${scoreClass}" cx="60" cy="60" r="46"
                style="stroke-dasharray:${circumference};stroke-dashoffset:${offset}"></circle>`
        }
      </svg>
      <div class="security-score-readout">
        <span class="security-score-value">${isPending ? "—" : normalizedScore}</span>
        <span class="security-score-max">${esc(I18n.t("tokens-security-score-label"))}</span>
      </div>
    </div>
  `;
}

function buildSummaryMetrics(token, options = {}) {
  const { escapeHtml } = options;
  const safe = escapeHtml || Utils.escapeHtml;
  const metrics = [];

  if (token.token_type) {
    metrics.push({
      label: I18n.t("tokens-security-metric-token-type"),
      value: safe(String(token.token_type)),
      icon: "icon-box",
    });
  }

  if (token.total_holders !== null && token.total_holders !== undefined) {
    metrics.push({
      label: I18n.t("tokens-security-metric-total-holders"),
      value: Utils.formatCompactNumber(token.total_holders),
      icon: "icon-users",
    });
  }

  if (token.lp_provider_count !== null && token.lp_provider_count !== undefined) {
    metrics.push({
      label: I18n.t("tokens-security-metric-lp-providers"),
      value: Utils.formatNumber(token.lp_provider_count, { decimals: 0 }),
      icon: "icon-droplet",
    });
  }

  if (token.graph_insiders_detected !== null && token.graph_insiders_detected !== undefined) {
    const hasInsiders = token.graph_insiders_detected > 0;
    metrics.push({
      label: I18n.t("tokens-security-metric-graph-insiders"),
      value: esc(
        hasInsiders
          ? I18n.t("tokens-security-insiders-detected", { count: token.graph_insiders_detected })
          : I18n.t("tokens-security-insiders-clean")
      ),
      icon: hasInsiders ? "icon-triangle-alert" : "icon-search",
      state: hasInsiders ? "is-warning" : "is-good",
    });
  }

  if (metrics.length === 0) return "";

  return `
    <div class="security-metric-grid">
      ${metrics
        .map(
          (metric) => `
        <div class="security-metric ${metric.state || ""}">
          <span class="security-metric-label"><i class="${metric.icon}"></i>${esc(metric.label)}</span>
          <span class="security-metric-value">${metric.value}</span>
        </div>
      `
        )
        .join("")}
    </div>
  `;
}

function buildAuthorityList(token) {
  return `
    <div class="security-authority-list">
      ${buildAuthorityRow(
        I18n.t("tokens-security-authority-mint"),
        "icon-wrench",
        token.mint_authority,
        I18n.t("tokens-security-authority-immutable"),
        I18n.t("tokens-security-authority-mutable")
      )}
      ${buildAuthorityRow(
        I18n.t("tokens-security-authority-freeze"),
        "icon-snowflake",
        token.freeze_authority,
        I18n.t("tokens-security-authority-revoked"),
        I18n.t("tokens-security-authority-active")
      )}
    </div>
  `;
}

function buildAuthorityRow(label, icon, authority, safeWord, riskWord) {
  const hasAuthority = Boolean(authority);
  const state = hasAuthority ? "is-risk" : "is-safe";
  const stateIcon = hasAuthority ? "icon-triangle-alert" : "icon-circle-check";

  return `
    <div class="security-authority-row ${state}">
      <span class="security-authority-label"><i class="${icon}"></i>${esc(label)}</span>
      <span class="security-authority-state"><i class="${stateIcon}"></i>${esc(hasAuthority ? riskWord : safeWord)}</span>
      ${
        hasAuthority
          ? `<div class="security-authority-address">${renderAddress(authority, { explorer: "account" })}</div>`
          : ""
      }
    </div>
  `;
}

function buildHolderHealthSection(token) {
  const top10Pct =
    token.top_10_holders_pct !== undefined && token.top_10_holders_pct !== null
      ? token.top_10_holders_pct
      : token.top_10_concentration;
  const creatorPct = token.creator_balance_pct;
  const concentration = getConcentrationState(top10Pct);
  const totalHolders =
    token.total_holders !== null && token.total_holders !== undefined
      ? Utils.formatNumber(token.total_holders, { decimals: 0 })
      : "—";

  return `
    <section class="security-detail-section">
      ${buildSectionHeader(
        `<span class="security-section-title">${esc(I18n.t("tokens-security-holder-health-title"))}</span>`,
        `<span class="security-status-text ${concentration.className}">${esc(concentration.label)}</span>`
      )}
      <div class="security-holder-health-body">
        ${buildHolderGauge(top10Pct, concentration.className)}
        <div class="security-holder-metrics">
          <div class="security-holder-metric">
            <span class="security-detail-label">${esc(I18n.t("tokens-security-metric-total-holders"))}</span>
            <span class="security-detail-value">${totalHolders}<small>${esc(I18n.t("tokens-security-holders-unique"))}</small></span>
          </div>
          <div class="security-holder-metric">
            <span class="security-detail-label">${esc(I18n.t("tokens-security-creator-share"))}</span>
            <span class="security-detail-value ${
              creatorPct === null || creatorPct === undefined
                ? ""
                : creatorPct > 10
                  ? "is-critical"
                  : "is-good"
            }">${formatPercentValue(creatorPct, PLAIN_PERCENT)}</span>
          </div>
        </div>
      </div>
    </section>
  `;
}

function getConcentrationState(percent) {
  if (percent === null || percent === undefined) {
    return { className: "is-muted", label: I18n.label(CONCENTRATION_LABELS, "unknown") };
  }
  if (percent > 80) {
    return { className: "is-critical", label: I18n.label(CONCENTRATION_LABELS, "critical") };
  }
  if (percent > 60) {
    return { className: "is-warning", label: I18n.label(CONCENTRATION_LABELS, "high") };
  }
  if (percent > 40) {
    return { className: "is-moderate", label: I18n.label(CONCENTRATION_LABELS, "moderate") };
  }
  return { className: "is-good", label: I18n.label(CONCENTRATION_LABELS, "healthy") };
}

function buildHolderGauge(percent, stateClass) {
  if (percent === null || percent === undefined) {
    return `
      <div class="security-holder-gauge is-empty">
        <span class="security-holder-gauge-value">—</span>
        <span class="security-holder-gauge-label">${esc(I18n.t("tokens-security-gauge-top-10"))}</span>
      </div>
    `;
  }

  const normalizedPercent = Math.min(100, Math.max(0, Number(percent)));
  const circumference = 2 * Math.PI * 34;
  const offset = circumference - (normalizedPercent / 100) * circumference;

  return `
    <div class="security-holder-gauge ${stateClass}">
      <svg width="82" height="82" viewBox="0 0 82 82" aria-hidden="true">
        <circle class="security-holder-gauge-track" cx="41" cy="41" r="34"></circle>
        <circle class="security-holder-gauge-ring" cx="41" cy="41" r="34"
          style="stroke-dasharray:${circumference};stroke-dashoffset:${offset}"></circle>
      </svg>
      <span class="security-holder-gauge-value">${formatPercentValue(normalizedPercent, { decimals: 0, plus: "" })}</span>
      <span class="security-holder-gauge-label">${esc(I18n.t("tokens-security-gauge-top-10"))}</span>
    </div>
  `;
}

function buildTransferFeeSection(token) {
  if (token.transfer_fee_pct === null || token.transfer_fee_pct === undefined) {
    return "";
  }

  const hasFee = Number(token.transfer_fee_pct) > 0;
  const feePercent = formatPercentValue(token.transfer_fee_pct, PLAIN_PERCENT);
  const status = hasFee
    ? `<span class="security-status-text is-warning"><i class="icon-triangle-alert"></i>${esc(feePercent)}</span>`
    : `<span class="security-status-text is-good"><i class="icon-circle-check"></i>${esc(I18n.t("tokens-security-transfer-no-fee"))}</span>`;

  return `
    <section class="security-detail-section">
      ${buildSectionHeader(
        `<span class="security-section-title">${esc(I18n.t("tokens-security-transfer-title"))}</span>`,
        status
      )}
      ${
        hasFee
          ? `
        <div class="security-fact-list">
          <div class="security-fact-row">
            <span class="security-detail-label">${esc(I18n.t("tokens-security-transfer-fee-percentage"))}</span>
            <span class="security-detail-value">${esc(feePercent)}</span>
          </div>
          ${
            token.transfer_fee_max_amount !== null && token.transfer_fee_max_amount !== undefined
              ? `
          <div class="security-fact-row">
            <span class="security-detail-label">${esc(I18n.t("tokens-security-transfer-max-fee"))}</span>
            <span class="security-detail-value">${Utils.formatNumber(token.transfer_fee_max_amount)}</span>
          </div>
          `
              : ""
          }
          ${
            token.transfer_fee_authority
              ? `
          <div class="security-fact-row">
            <span class="security-detail-label">${esc(I18n.t("tokens-security-transfer-authority"))}</span>
            <span class="security-detail-value">${renderAddress(token.transfer_fee_authority, { explorer: "account" })}</span>
          </div>
          `
              : ""
          }
        </div>
        <div class="security-inline-note is-warning">
          <i class="icon-circle-alert"></i>
          <span>${esc(I18n.t("tokens-security-transfer-note", { percent: feePercent }))}</span>
        </div>
        `
          : `
        <div class="security-clear-line is-good">
          <i class="icon-shield"></i>
          <span>${esc(I18n.t("tokens-security-transfer-none"))}</span>
        </div>
        `
      }
    </section>
  `;
}

function buildRisksSection(risks, options = {}) {
  const { escapeHtml } = options;
  const safe = escapeHtml || Utils.escapeHtml;

  if (!risks || risks.length === 0) {
    return `
      <section class="security-detail-section">
        ${buildSectionHeader(`<span class="security-section-title">${esc(I18n.t("tokens-security-risks-title"))}</span>`)}
        <div class="security-clear-line is-good">
          <i class="icon-sparkles"></i>
          <span>${esc(I18n.t("tokens-security-risks-none"))}</span>
        </div>
      </section>
    `;
  }

  const severity = {
    danger: { className: "danger", icon: "icon-octagon-alert", weight: 0 },
    warn: { className: "warn", icon: "icon-triangle-alert", weight: 1 },
    info: { className: "info", icon: "icon-info", weight: 2 },
  };
  const severityFor = (risk) => {
    const level = risk.level?.toLowerCase();
    if (level === "danger") return severity.danger;
    if (level === "warn" || level === "warning") return severity.warn;
    return severity.info;
  };
  const sorted = [...risks].sort(
    (first, second) => severityFor(first).weight - severityFor(second).weight
  );
  const counts = sorted.reduce((totals, risk) => {
    const className = severityFor(risk).className;
    totals[className] = (totals[className] || 0) + 1;
    return totals;
  }, {});
  const breakdown =
    [
      counts.danger ? I18n.t("tokens-security-risks-critical", { count: counts.danger }) : "",
      counts.warn ? I18n.t("tokens-security-risks-warnings", { count: counts.warn }) : "",
      counts.info ? I18n.t("tokens-security-risks-info", { count: counts.info }) : "",
    ]
      .filter(Boolean)
      .join(" · ") || I18n.t("tokens-security-risks-incidents", { count: sorted.length });

  return `
    <section class="security-detail-section">
      ${buildSectionHeader(
        `<span class="security-section-title"><i class="icon-shield-alert"></i> ${esc(I18n.t("tokens-security-risks-title"))}</span>`,
        esc(breakdown)
      )}
      <div class="security-risk-list">
        ${sorted
          .map((risk) => {
            const riskSeverity = severityFor(risk);
            const name = safe(
              risk.name ? rugcheckRiskName(risk.name) : I18n.t("tokens-security-risk-fallback-name")
            );
            const description = safe(rugcheckRiskDescription(risk));

            return `
          <div class="security-risk-row risk-${riskSeverity.className}">
            <i class="security-risk-icon ${riskSeverity.icon}"></i>
            <div class="security-risk-details">
              <span class="security-risk-name">${name}</span>
              ${description ? `<span class="security-risk-description">${description}</span>` : ""}
            </div>
            <span class="security-risk-level">${esc(I18n.label(RISK_SEVERITY_LABELS, riskSeverity.className))}</span>
          </div>
        `;
          })
          .join("")}
      </div>
    </section>
  `;
}

function buildTopHoldersSection(token) {
  const topHolders = token.top_holders;

  if (!topHolders || topHolders.length === 0) {
    return "";
  }

  let concentration = token.top_10_concentration;
  if (concentration === undefined || concentration === null) {
    concentration = topHolders
      .slice(0, 10)
      .reduce((sum, holder) => sum + (Number(holder.percentage) || 0), 0);
  }

  return `
    <section class="security-detail-section">
      ${buildSectionHeader(
        `<span class="security-section-title">${esc(I18n.t("tokens-security-top-holders-title"))}</span>`,
        esc(
          I18n.t("tokens-security-top-holders-concentration", {
            percent: formatPercentValue(concentration, PLAIN_PERCENT),
          })
        )
      )}
      <div class="security-holder-list">
        ${topHolders
          .slice(0, 10)
          .map((holder, index) => {
            const rank = index + 1;
            const walletAddress = holder.owner_type ? String(holder.owner_type) : "";
            const share = Number(holder.percentage);
            const shareWidth = Number.isFinite(share) ? Math.min(100, Math.max(0, share)) : 0;

            return `
          <div class="security-holder-row ${holder.is_insider ? "is-insider" : ""}">
            <span class="security-holder-rank ${rank <= 3 ? "is-leading" : ""}">${String(rank).padStart(2, "0")}</span>
            <div class="security-holder-identity">
              ${renderAddress(walletAddress, { explorer: "account" })}
              ${
                holder.is_insider
                  ? `<span class="security-holder-tag is-insider"><i class="icon-triangle-alert"></i>${esc(I18n.t("tokens-security-insider"))}</span>`
                  : ""
              }
            </div>
            <div class="security-holder-share">
              <span class="security-holder-share-track" aria-hidden="true">
                <span class="security-holder-share-fill" style="width: ${shareWidth}%"></span>
              </span>
              <span class="security-holder-share-value">${formatPercentValue(holder.percentage, PLAIN_PERCENT)}</span>
            </div>
          </div>
        `;
          })
          .join("")}
      </div>
    </section>
  `;
}

function getSafetyScoreClass(score) {
  if (score === null || score === undefined) return "";
  if (score >= 70) return "score-safe";
  if (score >= 40) return "score-caution";
  return "score-vulnerable";
}

function getSafetyScoreLabel(score) {
  if (score === null || score === undefined) return I18n.label(SAFETY_SCORE_LABELS, "unknown");
  if (score >= 90) return I18n.label(SAFETY_SCORE_LABELS, "shielded");
  if (score >= 70) return I18n.label(SAFETY_SCORE_LABELS, "safe");
  if (score >= 40) return I18n.label(SAFETY_SCORE_LABELS, "caution");
  return I18n.label(SAFETY_SCORE_LABELS, "vulnerable");
}
