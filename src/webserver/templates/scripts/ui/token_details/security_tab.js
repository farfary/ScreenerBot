/**
 * Token Details Dialog - Security Tab
 * Extracted from token_details_dialog.js to reduce file size
 */
import * as Utils from "../../core/utils.js";

/**
 * Render the security tab content (with loading state)
 * @param {Object} token - Token data object
 * @param {Object} options - Rendering options
 * @returns {string} HTML string for security tab
 */
export function renderSecurityTab(token, options = {}) {
  const { renderHintTrigger, escapeHtml, formatShortAddress } = options;

  // Check if we have security data
  const hasSecurityData = token.safety_score !== undefined && token.safety_score !== null;

  if (!hasSecurityData) {
    return buildSecurityLoadingContent(token, { renderHintTrigger, escapeHtml, formatShortAddress });
  }

  return buildSecurityContent(token, { renderHintTrigger, escapeHtml, formatShortAddress });
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
        <div class="security-card security-summary" style="--i:0">
          <div class="card-header">
            <span class="section-title"><i class="icon-shield-check"></i> Security Pulse ${renderHintTrigger("tokenDetails.security")}</span>
          </div>
          <div class="card-body security-summary__body">
            <div class="security-summary__score">
              <div class="security-score-circle">
                <svg class="score-progress" width="120" height="120" viewBox="0 0 120 120">
                  <circle class="score-bg" cx="60" cy="60" r="46"></circle>
                </svg>
                <div class="score-content">
                  <span class="score-value" style="opacity: 0.3;">—</span>
                  <span class="score-max">SCORE</span>
                </div>
              </div>
              <div class="safety-badge" style="background: var(--bg-secondary); color: var(--text-muted);">
                Analyzing...
              </div>
            </div>
            <div class="security-summary__section">
              <div class="sec-section-head">
                <span>Token Control</span>
                <span class="sec-section-sub">Authority status</span>
              </div>
              ${buildAuthorityGrid(token)}
            </div>
          </div>
        </div>
      </div>
      <div class="security-right-col">
        <div class="security-card security-loading" style="height: 400px; --i:3">
           <div class="loading-spinner" style="margin-top: 150px;"></div>
        </div>
      </div>
    </div>
  `;
}

function buildSecurityContent(token, options) {
  const { renderHintTrigger, escapeHtml, formatShortAddress } = options;

  // Use safety_score (0-100, higher = safer) for display
  const safetyScore = token.safety_score;
  const scoreClass = getSafetyScoreClass(safetyScore);
  const scoreLabel = getSafetyScoreLabel(safetyScore);

  return `
    <div class="security-container">
      <div class="security-left-col">
        ${buildSecuritySummaryCard(token, safetyScore, scoreClass, scoreLabel, { renderHintTrigger })}
      </div>
      <div class="security-right-col">
        ${buildHoldersCard(token)}
        ${buildTransferFeeCard(token)}
        ${buildRisksSection(token.security_risks, { escapeHtml })}
        ${buildTopHoldersSection(token, { escapeHtml, formatShortAddress })}
      </div>
    </div>
  `;
}

/**
 * Single unified left-column card: score hero + key stats + token control.
 * Replaces the three separate cards (header / bento grid / authorities) with
 * one cohesive panel divided into sections.
 */
function buildSecuritySummaryCard(token, safetyScore, scoreClass, scoreLabel, options) {
  const { renderHintTrigger } = options;

  const lastUpdated = token.security_last_updated
    ? Utils.formatTimestamp(token.security_last_updated)
    : null;

  // Safety score is 0-100 (higher = safer)
  const score = safetyScore ?? 0;
  const scorePercent = Math.min(100, Math.max(0, score));
  const circumference = 2 * Math.PI * 46; // radius = 46 for 120px ring
  const offset = circumference - (scorePercent / 100) * circumference;

  // Badge colour follows the score class (good/warn/critical). Keys must match
  // getSafetyScoreClass() output — previously they didn't, so a "Safe" token
  // was rendered with the amber fallback instead of green.
  const badgeClassMap = {
    "score-safe": "good",
    "score-caution": "warning",
    "score-vulnerable": "critical",
  };
  const badge = { label: scoreLabel, class: badgeClassMap[scoreClass] || "warning" };

  const statsHtml = buildSummaryStats(token);

  return `
    <div class="security-card security-summary" style="--i:0">
      <div class="card-header">
        <span class="section-title"><i class="icon-shield-check"></i> Security Pulse ${renderHintTrigger("tokenDetails.security")}</span>
        ${lastUpdated && !token.rugged ? `<span class="card-subtitle">Updated ${lastUpdated}</span>` : ""}
      </div>
      <div class="card-body security-summary__body">
        <div class="security-summary__score">
          <div class="security-score-circle">
            <div class="score-glow ${scoreClass}"></div>
            <svg class="score-progress" width="120" height="120" viewBox="0 0 120 120">
              <circle class="score-bg" cx="60" cy="60" r="46"></circle>
              <circle class="score-ring ${scoreClass}" cx="60" cy="60" r="46"
                style="stroke-dasharray: ${circumference}; stroke-dashoffset: ${offset};"></circle>
            </svg>
            <div class="score-content">
              <span class="score-value">${score}</span>
              <span class="score-max">SCORE</span>
            </div>
          </div>
          <div class="safety-badge ${badge.class}">${badge.label}</div>
          ${
            token.rugged
              ? `<div class="rugged-warning" style="margin-top: 4px; border-radius: 10px;">
                  <i class="icon-skull" style="font-size: 20px;"></i>
                  <span style="font-size: 1rem; letter-spacing: 0.1em; font-weight: 800;">RUGGED</span>
                </div>`
              : ""
          }
        </div>
        ${statsHtml}
        <div class="security-summary__section">
          <div class="sec-section-head">
            <span>Token Control</span>
            <span class="sec-section-sub">Authority status</span>
          </div>
          ${buildAuthorityGrid(token)}
        </div>
      </div>
    </div>
  `;
}

/** Compact inline stat strip (holders / LP providers / graph insiders / type). */
function buildSummaryStats(token) {
  const items = [];

  if (token.token_type) {
    items.push({
      label: "Token Type",
      value: token.token_type,
      icon: '<i class="icon-box"></i>',
    });
  }

  if (token.total_holders !== null && token.total_holders !== undefined) {
    items.push({
      label: "Total Holders",
      value: Utils.formatCompactNumber(token.total_holders),
      icon: '<i class="icon-users"></i>',
    });
  }

  if (token.lp_provider_count !== null && token.lp_provider_count !== undefined) {
    items.push({
      label: "LP Providers",
      value: Utils.formatNumber(token.lp_provider_count, { decimals: 0 }),
      icon: '<i class="icon-droplet"></i>',
    });
  }

  if (token.graph_insiders_detected !== null && token.graph_insiders_detected !== undefined) {
    const isDangerous = token.graph_insiders_detected > 0;
    items.push({
      label: "Graph Insiders",
      value: `${isDangerous ? "Detected" : "Clean"} ${token.graph_insiders_detected > 0 ? `(${token.graph_insiders_detected})` : ""}`,
      icon: isDangerous ? '<i class="icon-triangle-alert"></i>' : '<i class="icon-search"></i>',
      class: isDangerous ? "warning" : "good",
    });
  }

  if (items.length === 0) return "";

  return `
    <div class="security-summary__stats">
      ${items
        .map(
          (item) => `
        <div class="sec-stat ${item.class || ""}">
          <span class="sec-stat__label">${item.icon} ${item.label}</span>
          <span class="sec-stat__value">${item.value}</span>
        </div>
      `
        )
        .join("")}
    </div>
  `;
}

/** Token-control authority grid (mint + freeze), shared by summary + loading. */
function buildAuthorityGrid(token) {
  return `
        <div class="authority-grid">
          ${buildAuthorityTile("Mint", "icon-wrench", token.mint_authority, "Immutable", "Mutable")}
          ${buildAuthorityTile("Freeze", "icon-snowflake", token.freeze_authority, "Revoked", "Active")}
        </div>
  `;
}

/**
 * One authority tile. Minimal: an icon chip + label, a soft status pill, and
 * the address chip only when the authority is still live (a risk).
 */
function buildAuthorityTile(label, icon, authority, safeWord, riskWord) {
  const isRisk = !!authority;
  const state = isRisk ? "risk" : "ok";
  const pillIcon = isRisk ? "icon-triangle-alert" : "icon-circle-check";
  return `
    <div class="authority-item ${state}">
      <div class="authority-top">
        <span class="authority-icon"><i class="${icon}"></i></span>
        <span class="authority-label">${label}</span>
      </div>
      <span class="auth-pill ${state}"><i class="${pillIcon}"></i>${isRisk ? riskWord : safeWord}</span>
      ${
        isRisk
          ? `<div class="authority-address">${Utils.renderAddressChip(authority, { full: true })}</div>`
          : ""
      }
    </div>
  `;
}

function buildHoldersCard(token) {
  const top10Pct =
    token.top_10_holders_pct !== undefined
      ? token.top_10_holders_pct
      : token.top_10_concentration;
  const creatorPct = token.creator_balance_pct;

  // Determine concentration risk level
  let concentrationClass = "good";
  let concentrationLabel = "Healthy";
  if (top10Pct !== null && top10Pct !== undefined) {
    if (top10Pct > 80) {
      concentrationClass = "danger";
      concentrationLabel = "Critical";
    } else if (top10Pct > 60) {
      concentrationClass = "warning";
      concentrationLabel = "High";
    } else if (top10Pct > 40) {
      concentrationClass = "moderate";
      concentrationLabel = "Moderate";
    }
  }

  const gaugeHtml = buildHolderGauge(top10Pct, concentrationClass);

  return `
    <div class="security-card" style="--i:2">
      <div class="card-header">
        <span>Holder Health</span>
        ${concentrationLabel ? `<span class="concentration-badge ${concentrationClass}" style="margin-left:auto">${concentrationLabel}</span>` : ""}
      </div>
      <div class="card-body" style="padding: 16px;">
         <div style="display: flex; align-items: center; justify-content: space-around; gap: 20px;">
           <div style="text-align: center;">
              ${gaugeHtml}
              <div style="margin-top: 8px; font-size: 10px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 1px; font-weight: 600;">Top 10 Supply</div>
           </div>
           
           <div style="display: flex; flex-direction: column; gap: 14px; flex: 1;">
              <div class="holder-stat-item">
                 <div style="font-size: 10px; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 2px;">Total Holders</div>
                 <div style="font-weight: 700; font-size: 16px; color: var(--text-primary); display: flex; align-items: baseline; gap: 4px;">
                    ${token.total_holders ? Utils.formatNumber(token.total_holders, { decimals: 0 }) : "—"}
                    <span style="font-size: 10px; font-weight: 400; color: var(--text-muted);">unique</span>
                 </div>
              </div>
              
              ${
                creatorPct !== null && creatorPct !== undefined
                  ? `
              <div class="holder-stat-item">
                 <div style="font-size: 10px; color: var(--text-secondary); text-transform: uppercase; margin-bottom: 2px;">Creator Share</div>
                 <div style="font-weight: 700; font-size: 16px; color: ${creatorPct > 10 ? "var(--error-color)" : "var(--success-color)"};">
                    ${creatorPct.toFixed(2)}%
                 </div>
              </div>
              `
                  : ""
              }
           </div>
         </div>
      </div>
    </div>
  `;
}

function buildHolderGauge(percent, colorClass) {
  if (percent === null || percent === undefined)
    return `
      <div class="gauge-placeholder" style="width:90px;height:90px;display:flex;align-items:center;justify-content:center;color:var(--text-secondary);background:var(--bg-secondary);border-radius:50%;border:1px dashed var(--border-color);">
          <span style="font-size: 20px; opacity: 0.5;">?</span>
      </div>`;

  const p = Math.min(100, Math.max(0, percent));
  const r = 38;
  const c = 2 * Math.PI * r;
  const off = c - (p / 100) * c;

  // Color mapping
  let strokeColor = "var(--success-color)";
  if (colorClass === "danger") strokeColor = "var(--error-color)";
  if (colorClass === "warning") strokeColor = "#d29922";
  if (colorClass === "moderate") strokeColor = "#db6d28";

  return `
      <div class="gauge-wrapper" style="position:relative; width:90px; height:90px;">
           <svg width="90" height="90" viewBox="0 0 90 90" style="transform: rotate(-90deg);">
            <circle cx="45" cy="45" r="${r}" fill="none" stroke="var(--bg-secondary)" stroke-width="8"></circle>
            <circle cx="45" cy="45" r="${r}" fill="none" stroke="${strokeColor}" stroke-width="8"
              style="stroke-dasharray: ${c}; stroke-dashoffset: ${off}; transition: stroke-dashoffset 1.5s cubic-bezier(0.4, 0, 0.2, 1); stroke-linecap: round; filter: drop-shadow(0 0 3px ${strokeColor}44);"></circle>
          </svg>
          <div style="position:absolute; top:50%; left:50%; transform:translate(-50%, -50%); text-align:center;">
             <div style="font-size:16px; font-weight:800; color:var(--text-primary); font-family:var(--font-mono);">${p.toFixed(0)}<span style="font-size: 10px; margin-left: 1px;">%</span></div>
          </div>
      </div>
   `;
}

function buildTransferFeeCard(token) {
  // Only show if token has transfer fee data
  if (token.transfer_fee_pct === null && token.transfer_fee_pct === undefined) {
    return "";
  }

  const hasFee = token.transfer_fee_pct > 0;

  return `
    <div class="security-card ${hasFee ? "has-fee" : ""}" style="--i:2.5">
      <div class="card-header">
        <span>Transfer Tax</span>
        ${hasFee ? `<span class="fee-badge"><i class="icon-triangle-alert"></i> ${token.transfer_fee_pct}%</span>` : '<span class="no-fee-badge"><i class="icon-check"></i> No Fee</span>'}
      </div>
      <div class="card-body" style="padding: 0;">
        ${
          hasFee
            ? `
        <div class="fee-details">
          <div class="fee-row">
            <span class="fee-label">Fee Percentage</span>
            <span class="fee-value">${token.transfer_fee_pct}%</span>
          </div>
          ${
            token.transfer_fee_max_amount
              ? `
          <div class="fee-row">
            <span class="fee-label">Max Fee Amount</span>
            <span class="fee-value">${Utils.formatNumber(token.transfer_fee_max_amount)}</span>
          </div>
          `
              : ""
          }
          ${
            token.transfer_fee_authority
              ? `
          <div class="fee-row">
            <span class="fee-label">Fee Authority</span>
            <span class="fee-value">${Utils.renderAddressChip(token.transfer_fee_authority)}</span>
          </div>
          `
              : ""
          }
        </div>
        <div style="padding: 10px 16px; border-top: 1px solid var(--border-color); font-size: 0.65rem; color: var(--warning-color); background: var(--warning-alpha-10); display: flex; align-items: center; gap: 8px;">
          <i class="icon-circle-alert" style="font-size: 14px;"></i>
          <span>A ${token.transfer_fee_pct}% fee is charged on every transfer</span>
        </div>
        `
            : `
        <div style="padding: 16px; color: var(--success-color); font-size: 0.75rem; font-weight: 600;">
           <i class="icon-shield"></i> No transfer fees detected.
        </div>
        `
        }
      </div>
    </div>
  `;
}

function buildTopHoldersSection(token, options = {}) {
  const { escapeHtml, formatShortAddress } = options;

  const topHolders = token.top_holders;
  if (!topHolders || topHolders.length === 0) {
    return "";
  }

  // Calculate concentration (use backend value or fallback sum)
  let concentration = token.top_10_concentration;
  if (concentration === undefined || concentration === null) {
    const limit = Math.min(topHolders.length, 10);
    concentration = topHolders.slice(0, limit).reduce((sum, h) => sum + (h.percentage || 0), 0);
  }

  // Top 3 Podium
  const top3 = topHolders.slice(0, 3);
  const podiumHtml = `
    <div class="holders-podium">
      ${[1, 0, 2]
        .map((idx) => {
          const h = top3[idx];
          if (!h) return '<div class="podium-spot empty"></div>';
          const rank = idx + 1;
          const name =
            h.owner_type && h.owner_type.length < 15
              ? h.owner_type
              : formatShortAddress
                ? formatShortAddress(h.address)
                : h.address;
          const solscanUrl = Utils.solscanAccountUrl(h.address);
          return `
          <a class="podium-spot rank-${rank}" href="${solscanUrl}" target="_blank" rel="noopener noreferrer" title="${escapeHtml ? escapeHtml(h.address) : h.address} — open in Solscan">
            <div class="podium-avatar">${rank === 1 ? '<i class="icon-crown"></i>' : rank}</div>
            <div class="podium-pedestal">
              <span class="podium-value">${h.percentage.toFixed(1)}%</span>
            </div>
            <div class="podium-name">${escapeHtml ? escapeHtml(name) : name}</div>
          </a>
        `;
        })
        .join("")}
    </div>
  `;

  const holderRows = topHolders
    .slice(3, 10)
    .map((holder, idx) => {
      const insiderClass = holder.is_insider ? "insider" : "";
      const ownerLabel = holder.owner_type || "";
      const badges = [];
      if (holder.is_insider) badges.push('<span class="insider-badge">Insider</span>');

      if (ownerLabel) {
        const isAddress = ownerLabel.length > 30 && !ownerLabel.includes(" ");
        const displayLabel = isAddress
          ? formatShortAddress
            ? formatShortAddress(ownerLabel)
            : ownerLabel
          : ownerLabel;
        const cssClass = isAddress ? "owner-badge address-badge" : "owner-badge";
        badges.push(
          `<span class="${cssClass}" title="${escapeHtml ? escapeHtml(ownerLabel) : ownerLabel}">${escapeHtml ? escapeHtml(displayLabel) : displayLabel}</span>`
        );
      }

      return `
        <div class="holder-row ${insiderClass}" style="--i: ${idx + 4}">
          <div class="holder-rank">#${idx + 4}</div>
          <div class="holder-address-container">
            ${Utils.renderAddressChip(holder.address, { full: true })}
            <div class="holder-badges">${badges.join("")}</div>
          </div>
          <div class="holder-share">${holder.percentage.toFixed(2)}%</div>
        </div>
      `;
    })
    .join("");

  return `
    <div class="security-card main-holders-card" style="--i:3.5">
      <div class="card-header">
        <span>Top 10 Holders Concentration</span>
        <span class="concentration-value">${concentration.toFixed(2)}%</span>
      </div>
      <div class="card-body" style="padding: 26px 16px 12px;">
        ${podiumHtml}
        <div class="holders-list-small">
          ${holderRows}
        </div>
      </div>
    </div>
  `;
}

function buildRisksSection(risks, options = {}) {
  const { escapeHtml } = options;

  if (!risks || risks.length === 0) {
    return `
      <div class="security-card" style="--i:3">
        <div class="card-header">
          <span>Security Risks</span>
        </div>
        <div class="card-body">
          <div class="no-data-message" style="color: var(--success-color); font-weight: 700; display: flex; align-items: center; gap: 8px;">
             <i class="icon-sparkles"></i> No security risks detected.
          </div>
        </div>
      </div>
    `;
  }

  // Severity metadata: deterministic icon (verified against the Lucide font),
  // colour class, weight for ordering, and a human label for the badge.
  const SEVERITY = {
    danger: { cls: "danger", label: "Critical", icon: "icon-octagon-alert", weight: 0 },
    warn: { cls: "warn", label: "Warning", icon: "icon-triangle-alert", weight: 1 },
    info: { cls: "info", label: "Info", icon: "icon-info", weight: 2 },
  };
  const sevOf = (risk) => {
    const level = risk.level?.toLowerCase();
    if (level === "danger") return SEVERITY.danger;
    if (level === "warn" || level === "warning") return SEVERITY.warn;
    return SEVERITY.info;
  };

  // Most severe first so the worst risks are read first.
  const sorted = [...risks].sort((a, b) => sevOf(a).weight - sevOf(b).weight);

  // Header breakdown ("2 critical · 1 warning") instead of a flat count.
  const counts = sorted.reduce((acc, r) => {
    const cls = sevOf(r).cls;
    acc[cls] = (acc[cls] || 0) + 1;
    return acc;
  }, {});
  const breakdown =
    [
      counts.danger ? `${counts.danger} critical` : "",
      counts.warn ? `${counts.warn} warning${counts.warn > 1 ? "s" : ""}` : "",
      counts.info ? `${counts.info} info` : "",
    ]
      .filter(Boolean)
      .join(" · ") || `${sorted.length} incidents found`;

  const riskItems = sorted
    .map((risk, idx) => {
      const sev = sevOf(risk);
      const name = escapeHtml ? escapeHtml(risk.name) : risk.name;
      const description = escapeHtml ? escapeHtml(risk.description) : risk.description;

      return `
      <div class="risk-row risk-${sev.cls}" style="--i:${idx}">
        <div class="risk-icon"><i class="${sev.icon}"></i></div>
        <div class="risk-details">
          <div class="risk-name">${name}</div>
          ${description ? `<div class="risk-description">${description}</div>` : ""}
        </div>
        <span class="risk-sev-badge sev-${sev.cls}">${sev.label}</span>
      </div>
    `;
    })
    .join("");

  return `
    <div class="security-card" style="--i:3">
      <div class="card-header">
        <span><i class="icon-shield-alert"></i> Security Risks</span>
        <span class="card-subtitle">${breakdown}</span>
      </div>
      <div class="card-body" style="padding: 0;">
        <div class="risks-list">
          ${riskItems}
        </div>
      </div>
    </div>
  `;
}

// Helper functions

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
