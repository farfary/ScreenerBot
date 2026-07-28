/**
 * Token Details Dialog - Security Tab
 * Extracted from token_details_dialog.js to reduce file size
 */
import * as Utils from "../../core/utils.js";
import { renderTabState } from "./state_handling.js";

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
          <span>Rugcheck analysis in progress...</span>
        </div>
        <section class="security-summary">
          ${buildSectionHeader(
            `<span class="security-section-title"><i class="icon-shield-check"></i> Security Pulse ${renderHintTrigger("tokenDetails.security")}</span>`
          )}
          <div class="security-score-overview">
            ${buildScoreRing(null, "")}
            <div class="security-score-copy">
              <span class="security-grade is-pending">Analyzing</span>
              <span class="security-score-caption">Risk signals are still being collected.</span>
            </div>
          </div>
          <div class="security-summary-section">
            ${buildSubsectionHeader("Token Control", "Authority status")}
            ${buildAuthorityList(token)}
          </div>
        </section>
      </div>
      <div class="security-right-col">
        ${renderTabState({ kind: "loading", message: "Analyzing security…" })}
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
        ${buildTopHoldersSection(token, { escapeHtml })}
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
  const meta = lastUpdated && !token.rugged ? `Updated ${lastUpdated}` : "";

  return `
    <section class="security-summary">
      ${buildSectionHeader(
        `<span class="security-section-title"><i class="icon-shield-check"></i> Security Pulse ${renderHintTrigger("tokenDetails.security")}</span>`,
        meta
      )}
      <div class="security-score-overview">
        ${buildScoreRing(safetyScore, scoreClass)}
        <div class="security-score-copy">
          <span class="security-grade ${gradeClass}">${scoreLabel}</span>
          <span class="security-score-caption">Normalized token risk score out of 100.</span>
          ${
            token.rugged
              ? "<span class='security-rugged'><i class='icon-skull'></i> Rugged</span>"
              : ""
          }
        </div>
      </div>
      ${buildSummaryMetrics(token, { escapeHtml })}
      <div class="security-summary-section">
        ${buildSubsectionHeader("Token Control", "Authority status")}
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
      <span>${title}</span>
      ${meta ? `<span class="security-section-meta">${meta}</span>` : ""}
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
        <span class="security-score-max">Score</span>
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
      label: "Token Type",
      value: safe(String(token.token_type)),
      icon: "icon-box",
    });
  }

  if (token.total_holders !== null && token.total_holders !== undefined) {
    metrics.push({
      label: "Total Holders",
      value: Utils.formatCompactNumber(token.total_holders),
      icon: "icon-users",
    });
  }

  if (token.lp_provider_count !== null && token.lp_provider_count !== undefined) {
    metrics.push({
      label: "LP Providers",
      value: Utils.formatNumber(token.lp_provider_count, { decimals: 0 }),
      icon: "icon-droplet",
    });
  }

  if (token.graph_insiders_detected !== null && token.graph_insiders_detected !== undefined) {
    const hasInsiders = token.graph_insiders_detected > 0;
    metrics.push({
      label: "Graph Insiders",
      value: hasInsiders ? `Detected (${token.graph_insiders_detected})` : "Clean",
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
          <span class="security-metric-label"><i class="${metric.icon}"></i>${metric.label}</span>
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
      ${buildAuthorityRow("Mint", "icon-wrench", token.mint_authority, "Immutable", "Mutable")}
      ${buildAuthorityRow("Freeze", "icon-snowflake", token.freeze_authority, "Revoked", "Active")}
    </div>
  `;
}

function buildAuthorityRow(label, icon, authority, safeWord, riskWord) {
  const hasAuthority = Boolean(authority);
  const state = hasAuthority ? "is-risk" : "is-safe";
  const stateIcon = hasAuthority ? "icon-triangle-alert" : "icon-circle-check";

  return `
    <div class="security-authority-row ${state}">
      <span class="security-authority-label"><i class="${icon}"></i>${label}</span>
      <span class="security-authority-state"><i class="${stateIcon}"></i>${hasAuthority ? riskWord : safeWord}</span>
      ${
        hasAuthority
          ? `<div class="security-authority-address">${Utils.renderAddressChip(authority, { full: true })}</div>`
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
        '<span class="security-section-title">Holder Health</span>',
        `<span class="security-status-text ${concentration.className}">${concentration.label}</span>`
      )}
      <div class="security-holder-health-body">
        ${buildHolderGauge(top10Pct, concentration.className)}
        <div class="security-holder-metrics">
          <div class="security-holder-metric">
            <span class="security-detail-label">Total Holders</span>
            <span class="security-detail-value">${totalHolders}<small>unique</small></span>
          </div>
          <div class="security-holder-metric">
            <span class="security-detail-label">Creator Share</span>
            <span class="security-detail-value ${
              creatorPct === null || creatorPct === undefined
                ? ""
                : creatorPct > 10
                  ? "is-critical"
                  : "is-good"
            }">${formatPercent(creatorPct)}</span>
          </div>
        </div>
      </div>
    </section>
  `;
}

function getConcentrationState(percent) {
  if (percent === null || percent === undefined) {
    return { className: "is-muted", label: "Unknown" };
  }
  if (percent > 80) return { className: "is-critical", label: "Critical" };
  if (percent > 60) return { className: "is-warning", label: "High" };
  if (percent > 40) return { className: "is-moderate", label: "Moderate" };
  return { className: "is-good", label: "Healthy" };
}

function buildHolderGauge(percent, stateClass) {
  if (percent === null || percent === undefined) {
    return `
      <div class="security-holder-gauge is-empty">
        <span class="security-holder-gauge-value">—</span>
        <span class="security-holder-gauge-label">Top 10</span>
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
      <span class="security-holder-gauge-value">${normalizedPercent.toFixed(0)}%</span>
      <span class="security-holder-gauge-label">Top 10</span>
    </div>
  `;
}

function buildTransferFeeSection(token) {
  if (token.transfer_fee_pct === null || token.transfer_fee_pct === undefined) {
    return "";
  }

  const hasFee = Number(token.transfer_fee_pct) > 0;
  const feePercent = formatPercent(token.transfer_fee_pct);
  const status = hasFee
    ? `<span class="security-status-text is-warning"><i class="icon-triangle-alert"></i>${feePercent}</span>`
    : '<span class="security-status-text is-good"><i class="icon-circle-check"></i>No Fee</span>';

  return `
    <section class="security-detail-section">
      ${buildSectionHeader('<span class="security-section-title">Transfer Tax</span>', status)}
      ${
        hasFee
          ? `
        <div class="security-fact-list">
          <div class="security-fact-row">
            <span class="security-detail-label">Fee Percentage</span>
            <span class="security-detail-value">${feePercent}</span>
          </div>
          ${
            token.transfer_fee_max_amount !== null && token.transfer_fee_max_amount !== undefined
              ? `
          <div class="security-fact-row">
            <span class="security-detail-label">Max Fee Amount</span>
            <span class="security-detail-value">${Utils.formatNumber(token.transfer_fee_max_amount)}</span>
          </div>
          `
              : ""
          }
          ${
            token.transfer_fee_authority
              ? `
          <div class="security-fact-row">
            <span class="security-detail-label">Fee Authority</span>
            <span class="security-detail-value">${Utils.renderAddressChip(token.transfer_fee_authority)}</span>
          </div>
          `
              : ""
          }
        </div>
        <div class="security-inline-note is-warning">
          <i class="icon-circle-alert"></i>
          <span>A ${feePercent} fee is charged on every transfer.</span>
        </div>
        `
          : `
        <div class="security-empty-line is-good">
          <i class="icon-shield"></i>
          <span>No transfer fees detected.</span>
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
        ${buildSectionHeader('<span class="security-section-title">Security Risks</span>')}
        <div class="security-empty-line is-good">
          <i class="icon-sparkles"></i>
          <span>No security risks detected.</span>
        </div>
      </section>
    `;
  }

  const severity = {
    danger: { className: "danger", label: "Critical", icon: "icon-octagon-alert", weight: 0 },
    warn: { className: "warn", label: "Warning", icon: "icon-triangle-alert", weight: 1 },
    info: { className: "info", label: "Info", icon: "icon-info", weight: 2 },
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
      counts.danger ? `${counts.danger} critical` : "",
      counts.warn ? `${counts.warn} warning${counts.warn > 1 ? "s" : ""}` : "",
      counts.info ? `${counts.info} info` : "",
    ]
      .filter(Boolean)
      .join(" · ") || `${sorted.length} incidents found`;

  return `
    <section class="security-detail-section">
      ${buildSectionHeader(
        '<span class="security-section-title"><i class="icon-shield-alert"></i> Security Risks</span>',
        breakdown
      )}
      <div class="security-risk-list">
        ${sorted
          .map((risk) => {
            const riskSeverity = severityFor(risk);
            const name = safe(String(risk.name || "Security signal"));
            const description = risk.description ? safe(String(risk.description)) : "";

            return `
          <div class="security-risk-row risk-${riskSeverity.className}">
            <i class="security-risk-icon ${riskSeverity.icon}"></i>
            <div class="security-risk-details">
              <span class="security-risk-name">${name}</span>
              ${description ? `<span class="security-risk-description">${description}</span>` : ""}
            </div>
            <span class="security-risk-level">${riskSeverity.label}</span>
          </div>
        `;
          })
          .join("")}
      </div>
    </section>
  `;
}

function buildTopHoldersSection(token, options = {}) {
  const { escapeHtml } = options;
  const safe = escapeHtml || Utils.escapeHtml;
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
        '<span class="security-section-title">Top Holders</span>',
        `${formatPercent(concentration)} concentration`
      )}
      <div class="security-holder-list">
        ${topHolders
          .slice(0, 10)
          .map((holder, index) => {
            const rank = index + 1;
            const ownerLabel = holder.owner_type ? String(holder.owner_type) : "";
            const ownerIsAddress = ownerLabel.length > 30 && !ownerLabel.includes(" ");
            const ownerDisplay = ownerIsAddress
              ? Utils.formatAddressCompact(ownerLabel, { start: 6, end: 6 })
              : ownerLabel;

            return `
          <div class="security-holder-row ${holder.is_insider ? "is-insider" : ""}">
            <span class="security-holder-rank ${rank <= 3 ? "is-leading" : ""}">${String(rank).padStart(2, "0")}</span>
            <div class="security-holder-identity">
              ${Utils.renderAddressChip(holder.address)}
              <div class="security-holder-tags">
                ${
                  holder.is_insider
                    ? '<span class="security-holder-tag is-insider"><i class="icon-triangle-alert"></i>Insider</span>'
                    : ""
                }
                ${
                  ownerLabel
                    ? `<span class="security-holder-tag ${ownerIsAddress ? "is-address" : ""}" title="${safe(ownerLabel)}">${safe(ownerDisplay)}</span>`
                    : ""
                }
              </div>
            </div>
            <span class="security-holder-share">${formatPercent(holder.percentage)}</span>
          </div>
        `;
          })
          .join("")}
      </div>
    </section>
  `;
}

function formatPercent(value, decimals = 2) {
  if (value === null || value === undefined) return "—";
  const number = Number(value);
  if (!Number.isFinite(number)) return "—";
  return `${Utils.formatNumber(number, { decimals })}%`;
}

function getSafetyScoreClass(score) {
  if (score === null || score === undefined) return "";
  if (score >= 70) return "score-safe";
  if (score >= 40) return "score-caution";
  return "score-vulnerable";
}

function getSafetyScoreLabel(score) {
  if (score === null || score === undefined) return "Unknown";
  if (score >= 90) return "Shielded";
  if (score >= 70) return "Safe";
  if (score >= 40) return "Caution";
  return "Vulnerable";
}
