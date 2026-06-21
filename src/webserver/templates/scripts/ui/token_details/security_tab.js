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
        <div class="security-header security-loading" style="--i:0">
          <div class="security-header-title">
            <span class="section-title"><i class="icon-shield-check"></i> Security Pulse</span>
            ${renderHintTrigger("tokenDetails.security")}
          </div>
          <div class="security-score-container">
            <div class="security-score-circle">
              <svg class="score-progress" width="120" height="120" viewBox="0 0 120 120">
                <circle class="score-bg" cx="60" cy="60" r="46"></circle>
              </svg>
              <div class="score-content">
                <span class="score-value" style="opacity: 0.3;">—</span>
              </div>
            </div>
            <div class="safety-badge" style="background: var(--bg-secondary); color: var(--text-muted);">
              Analyzing...
            </div>
          </div>
        </div>
        <div class="security-bento-grid">
           <div class="security-bento-card security-loading" style="--i:1"></div>
           <div class="security-bento-card security-loading" style="--i:2"></div>
        </div>
        ${buildAuthoritiesCard(token, options)}
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
        ${buildSecurityHeader(token, safetyScore, scoreClass, scoreLabel, { renderHintTrigger })}
        ${buildSecurityOverview(token)}
        ${buildAuthoritiesCard(token, { escapeHtml, formatShortAddress })}
      </div>
      <div class="security-right-col">
        ${buildHoldersCard(token)}
        ${buildTransferFeeCard(token, { escapeHtml, formatShortAddress })}
        ${buildRisksSection(token.security_risks, { escapeHtml })}
        ${buildTopHoldersSection(token, { escapeHtml, formatShortAddress })}
      </div>
    </div>
  `;
}

function buildSecurityHeader(token, safetyScore, scoreClass, scoreLabel, options) {
  const { renderHintTrigger } = options;

  const lastUpdated = token.security_last_updated
    ? Utils.formatTimestamp(token.security_last_updated)
    : null;

  // Safety score is 0-100 (higher = safer)
  const score = safetyScore ?? 0;
  const scorePercent = Math.min(100, Math.max(0, score));
  const circumference = 2 * Math.PI * 46; // radius = 46 for 120px ring
  const offset = circumference - (scorePercent / 100) * circumference;

  // Map score label to safety badge
  const badgeConfigs = {
    "score-good": { label: "Shielded", class: "good" },
    "score-ok": { label: "Safe", class: "good" },
    "score-warn": { label: "Caution", class: "warning" },
    "score-danger": { label: "Vulnerable", class: "critical" },
  };
  const badge = badgeConfigs[scoreClass] || { label: scoreLabel, class: "warning" };

  return `
    <div class="security-header" style="--i:0">
      <div class="security-header-title">
        <span class="section-title"><i class="icon-shield-check"></i> Security Pulse</span>
        ${renderHintTrigger("tokenDetails.security")}
      </div>
      <div class="security-score-container">
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
        <div class="safety-badge ${badge.class}">
          ${badge.label}
        </div>
      </div>
      ${
        token.rugged
          ? `
      <div class="rugged-warning" style="margin-top: 15px; border-radius: 10px;">
        <i class="icon-skull" style="font-size: 20px;"></i>
        <span style="font-size: 1rem; letter-spacing: 0.1em; font-weight: 800;">RUGGED</span>
      </div>
      `
          : ""
      }
      ${
        lastUpdated && !token.rugged
          ? `<div style="text-align: center; font-size: 0.65rem; color: var(--text-muted); margin-top: 10px;">Updated ${lastUpdated}</div>`
          : ""
      }
    </div>
  `;
}

function buildSecurityOverview(token) {
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
      icon: isDangerous ? '<i class="icon-alert-triangle"></i>' : '<i class="icon-search"></i>',
      class: isDangerous ? "warning" : "good",
    });
  }

  if (items.length === 0) return "";

  return `
    <div class="security-bento-grid" style="margin-top: 8px;">
      ${items
        .map(
          (item, idx) => `
        <div class="security-bento-card" style="--i:${idx + 1}">
          <div class="bento-icon">${item.icon}</div>
          <div class="bento-label">${item.label}</div>
          <div class="bento-value">${item.value}</div>
        </div>
      `
        )
        .join("")}
    </div>
  `;
}

function buildAuthoritiesCard(token, options = {}) {
  const { escapeHtml, formatShortAddress } = options;

  return `
    <div class="security-card" style="--i:2">
      <div class="card-header">
        <span>Token Control</span>
        <span class="card-subtitle">Authority status</span>
      </div>
      <div class="card-body">
        <div class="authority-grid">
          <div class="authority-item ${token.mint_authority ? "danger" : "safe"}">
            <div class="authority-header">
              <span class="authority-label">Mint <i class="icon-wrench"></i></span>
              ${renderAuthorityBadge(token.mint_authority)}
            </div>
            ${
              token.mint_authority
                ? `
            <div class="authority-address">
              <span class="address-value" title="${token.mint_authority}">${formatShortAddress ? formatShortAddress(token.mint_authority) : token.mint_authority}</span>
              <button class="btn-copy-mini" onclick="Utils.copyToClipboard('${token.mint_authority}')"><i class="icon-copy"></i></button>
            </div>
            <div class="authority-status-text">At Risk</div>
            `
                : '<div class="authority-status-text">Immutable</div>'
            }
          </div>
          
          <div class="authority-item ${token.freeze_authority ? "danger" : "safe"}">
            <div class="authority-header">
              <span class="authority-label">Freeze <i class="icon-snowflake"></i></span>
              ${renderAuthorityBadge(token.freeze_authority)}
            </div>
            ${
              token.freeze_authority
                ? `
            <div class="authority-address">
              <span class="address-value" title="${token.freeze_authority}">${formatShortAddress ? formatShortAddress(token.freeze_authority) : token.freeze_authority}</span>
              <button class="btn-copy-mini" onclick="Utils.copyToClipboard('${token.freeze_authority}')"><i class="icon-copy"></i></button>
            </div>
            <div class="authority-status-text">At Risk</div>
            `
                : '<div class="authority-status-text">Revoked</div>'
            }
          </div>
        </div>
      </div>
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

function buildTransferFeeCard(token, options = {}) {
  const { escapeHtml, formatShortAddress } = options;

  // Only show if token has transfer fee data
  if (token.transfer_fee_pct === null && token.transfer_fee_pct === undefined) {
    return "";
  }

  const hasFee = token.transfer_fee_pct > 0;

  return `
    <div class="security-card ${hasFee ? "has-fee" : ""}" style="--i:2.5">
      <div class="card-header">
        <span>Transfer Tax</span>
        ${hasFee ? `<span class="fee-badge"><i class="icon-alert-triangle"></i> ${token.transfer_fee_pct}%</span>` : '<span class="no-fee-badge"><i class="icon-check"></i> No Fee</span>'}
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
            <span class="fee-value" title="${token.transfer_fee_authority}">${formatShortAddress ? formatShortAddress(token.transfer_fee_authority) : token.transfer_fee_authority}</span>
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
          return `
          <div class="podium-spot rank-${rank}" title="${escapeHtml ? escapeHtml(h.address) : h.address}">
            <div class="podium-avatar">${rank === 1 ? '<i class="icon-crown"></i>' : rank}</div>
            <div class="podium-pedestal">
              <span class="podium-value">${h.percentage.toFixed(1)}%</span>
            </div>
            <div class="podium-name">${escapeHtml ? escapeHtml(name) : name}</div>
          </div>
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
            <span class="holder-address mono">${formatShortAddress ? formatShortAddress(holder.address) : holder.address}</span>
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
      <div class="card-body" style="padding: 10px 16px;">
        ${podiumHtml}
        <div class="holders-list-small">
          ${holderRows}
        </div>
      </div>
    </div>
  `;
}

function renderAuthorityBadge(value) {
  if (value === null || value === undefined || value === "") {
    return '<span class="status-badge status-safe">Revoked</span>';
  }
  return '<span class="status-badge status-danger">Present</span>';
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

  const riskItems = risks
    .map((risk, idx) => {
      const level = risk.level?.toLowerCase() || "info";
      const riskClass = level === "danger" ? "risk-danger" : level === "warn" ? "risk-warn" : "";
      const icon =
        level === "danger" ? '<i class="icon-ban"></i>' : '<i class="icon-alert-triangle"></i>';

      const name = escapeHtml ? escapeHtml(risk.name) : risk.name;
      const description = escapeHtml ? escapeHtml(risk.description) : risk.description;

      return `
      <div class="risk-row ${riskClass}" style="--i:${idx}">
        <div class="risk-icon">${icon}</div>
        <div class="risk-details">
          <div class="risk-name">${name}</div>
          <div class="risk-description">${description}</div>
        </div>
      </div>
    `;
    })
    .join("");

  return `
    <div class="security-card" style="--i:3">
      <div class="card-header">
        <span>Security Risks</span>
        <span class="card-subtitle">${risks.length} incidents found</span>
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
