/**
 * Token Details Dialog - Pools & Links Tabs
 * Extracted from token_details_dialog.js to reduce file size
 * These tabs are grouped together as they both display external/reference data
 */
import * as Utils from "../../core/utils.js";
import { renderTabState } from "./state_handling.js";

// =========================================================================
// POOLS TAB
// =========================================================================

/**
 * Render the pools tab content
 * @param {Object} token - Token data object
 * @param {Object} options - Rendering options
 * @returns {string} HTML string for pools tab
 */
export function renderPoolsTab(token, options = {}) {
  const { renderHintTrigger, escapeHtml, formatShortAddress } = options;

  const pools = token.pools || [];

  if (pools.length === 0) {
    return renderTabState({
      icon: "icon-droplet",
      title: "No pools",
      message: "No liquidity pools have been detected for this token.",
    });
  }

  // Calculate summary stats
  const totalLiquidity = pools.reduce((sum, p) => sum + (p.liquidity_usd || 0), 0);
  const totalVolume24h = pools.reduce((sum, p) => sum + (p.volume_h24_usd || 0), 0);
  const canonicalPool = pools.find((p) => p.is_canonical);
  const programCounts = pools.reduce((acc, p) => {
    acc[p.program] = (acc[p.program] || 0) + 1;
    return acc;
  }, {});
  const baseRoleCount = pools.filter((p) => p.token_role === "base").length;
  const quoteRoleCount = pools.filter((p) => p.token_role === "quote").length;

  // Build left column - Summary stats
  const leftCol = `
    <div class="pools-left-col">
      <div class="pools-summary-card">
        <div class="pools-summary-title">
          <span>Pool Summary</span>
          ${renderHintTrigger("tokenDetails.pools")}
        </div>
        <div class="pools-summary-stats">
          <div class="pools-stat">
            <span class="pools-stat-label">Total Pools</span>
            <span class="pools-stat-value">${pools.length}</span>
          </div>
          <div class="pools-stat">
            <span class="pools-stat-label">Total Liquidity</span>
            <span class="pools-stat-value">${Utils.formatCurrencyUSD(totalLiquidity)}</span>
          </div>
          <div class="pools-stat">
            <span class="pools-stat-label">Total 24h Volume</span>
            <span class="pools-stat-value">${Utils.formatCurrencyUSD(totalVolume24h)}</span>
          </div>
          <div class="pools-stat">
            <span class="pools-stat-label">Base Role</span>
            <span class="pools-stat-value">${baseRoleCount}</span>
          </div>
          <div class="pools-stat">
            <span class="pools-stat-label">Quote Role</span>
            <span class="pools-stat-value">${quoteRoleCount}</span>
          </div>
        </div>
      </div>

      <div class="pools-summary-card">
        <div class="pools-summary-title">DEX Breakdown</div>
        <div class="pools-dex-list">
          ${Object.entries(programCounts)
            .sort((a, b) => b[1] - a[1])
            .map(
              ([program, count]) => `
              <div class="pools-dex-row">
                <span class="pools-dex-name">${escapeHtml(program)}</span>
                <span class="pools-dex-count">${count}</span>
              </div>
            `
            )
            .join("")}
        </div>
      </div>

      ${
        canonicalPool
          ? `
      <div class="pools-summary-card canonical-highlight">
        <div class="pools-summary-title">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"></path></svg>
          Canonical Pool
        </div>
        <div class="pools-canonical-info">
          <div class="pools-stat">
            <span class="pools-stat-label">DEX</span>
            <span class="pools-stat-value">${escapeHtml(canonicalPool.program)}</span>
          </div>
          <div class="pools-stat">
            <span class="pools-stat-label">Liquidity</span>
            <span class="pools-stat-value">${canonicalPool.liquidity_usd ? Utils.formatCurrencyUSD(canonicalPool.liquidity_usd) : "—"}</span>
          </div>
          <div class="pools-stat">
            <span class="pools-stat-label">Volume 24h</span>
            <span class="pools-stat-value">${canonicalPool.volume_h24_usd ? Utils.formatCurrencyUSD(canonicalPool.volume_h24_usd) : "—"}</span>
          </div>
        </div>
      </div>
      `
          : ""
      }
    </div>
  `;

  // Build right column - Pool details
  const poolCards = pools
    .map((pool) => buildPoolDetailCard(pool, { escapeHtml, formatShortAddress }))
    .join("");
  const rightCol = `
    <div class="pools-right-col">
      <div class="pools-list-header">
        <span class="pools-list-title">All Pools (${pools.length})</span>
      </div>
      <div class="pools-list">
        ${poolCards}
      </div>
    </div>
  `;

  return `<div class="pools-container">${leftCol}${rightCol}</div>`;
}

function buildPoolDetailCard(pool, options = {}) {
  const { escapeHtml, formatShortAddress } = options;

  const lastUpdated = pool.last_updated_unix
    ? Utils.formatTimestamp(pool.last_updated_unix * 1000)
    : "—";

  const reserveAccountsHtml =
    pool.reserve_accounts && pool.reserve_accounts.length > 0
      ? pool.reserve_accounts
          .map(
            (addr) => `
          <div class="pool-reserve-item">
            <span class="pool-reserve-addr" title="${escapeHtml(addr)}">${formatShortAddress(addr)}</span>
            <button class="copy-btn-mini" data-copy="${escapeHtml(addr)}" title="Copy address">
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
            </button>
          </div>
        `
          )
          .join("")
      : '<span class="pool-no-data">No reserve accounts</span>';

  return `
    <div class="pool-detail-card ${pool.is_canonical ? "canonical" : ""}">
      <div class="pool-detail-header">
        <div class="pool-detail-left">
          <span class="pool-detail-program">${escapeHtml(pool.program)}</span>
          ${pool.is_canonical ? '<span class="pool-canonical-badge">★ Canonical</span>' : ""}
        </div>
        <div class="pool-detail-role ${pool.token_role}">${escapeHtml(pool.token_role)}</div>
      </div>

      <div class="pool-detail-body">
        <div class="pool-detail-section">
          <div class="pool-detail-row">
            <span class="pool-detail-label">Pool Address</span>
            <div class="pool-detail-value-group">
              <span class="pool-detail-value mono" title="${escapeHtml(pool.pool_id)}">${formatShortAddress(pool.pool_id)}</span>
              <button class="copy-btn-mini" data-copy="${escapeHtml(pool.pool_id)}" title="Copy pool address">
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
              </button>
            </div>
          </div>

          <div class="pool-detail-row">
            <span class="pool-detail-label">Base Mint</span>
            <div class="pool-detail-value-group">
              <span class="pool-detail-value mono" title="${escapeHtml(pool.base_mint)}">${formatShortAddress(pool.base_mint)}</span>
              <button class="copy-btn-mini" data-copy="${escapeHtml(pool.base_mint)}" title="Copy base mint">
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
              </button>
            </div>
          </div>

          <div class="pool-detail-row">
            <span class="pool-detail-label">Quote Mint</span>
            <div class="pool-detail-value-group">
              <span class="pool-detail-value mono" title="${escapeHtml(pool.quote_mint)}">${formatShortAddress(pool.quote_mint)}</span>
              <button class="copy-btn-mini" data-copy="${escapeHtml(pool.quote_mint)}" title="Copy quote mint">
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
              </button>
            </div>
          </div>

          <div class="pool-detail-row">
            <span class="pool-detail-label">Paired Mint</span>
            <div class="pool-detail-value-group">
              <span class="pool-detail-value mono" title="${escapeHtml(pool.paired_mint)}">${formatShortAddress(pool.paired_mint)}</span>
              <button class="copy-btn-mini" data-copy="${escapeHtml(pool.paired_mint)}" title="Copy paired mint">
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
              </button>
            </div>
          </div>
        </div>

        <div class="pool-detail-divider"></div>

        <div class="pool-detail-section">
          <div class="pool-detail-row">
            <span class="pool-detail-label">Liquidity</span>
            <span class="pool-detail-value highlight">${pool.liquidity_usd ? Utils.formatCurrencyUSD(pool.liquidity_usd) : "—"}</span>
          </div>

          <div class="pool-detail-row">
            <span class="pool-detail-label">Volume 24h</span>
            <span class="pool-detail-value highlight">${pool.volume_h24_usd ? Utils.formatCurrencyUSD(pool.volume_h24_usd) : "—"}</span>
          </div>

          <div class="pool-detail-row">
            <span class="pool-detail-label">Last Updated</span>
            <span class="pool-detail-value muted">${lastUpdated}</span>
          </div>
        </div>

        <div class="pool-detail-divider"></div>

        <div class="pool-detail-section">
          <div class="pool-detail-row reserves-row">
            <span class="pool-detail-label">Reserve Accounts (${pool.reserve_accounts?.length || 0})</span>
          </div>
          <div class="pool-reserves-list">
            ${reserveAccountsHtml}
          </div>
        </div>
      </div>
    </div>
  `;
}

/**
 * Initialize event handlers for pools tab (copy buttons)
 */
export function initPoolsTabEvents(container) {
  container.querySelectorAll(".copy-btn-mini[data-copy]").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const text = btn.dataset.copy;
      if (text) {
        Utils.copyToClipboard(text);
      }
    });
  });
}

// =========================================================================
// LINKS TAB
// =========================================================================

/**
 * Render the links tab content
 * @param {Object} token - Token data object
 * @param {Object} options - Rendering options
 * @returns {string} HTML string for links tab
 */
export function renderLinksTab(token, options = {}) {
  const { escapeHtml } = options;

  const mint = token.mint;
  const hasWebsites = token.websites && token.websites.length > 0;
  const hasSocials = token.socials && token.socials.length > 0;
  const hasLogo = !!token.logo_url;
  const hasHeader = !!token.header_image_url;
  const hasDescription = !!token.description;

  // Build left column - Media & Info
  const leftCol = buildLinksLeftColumn(token, hasLogo, hasHeader, hasDescription, { escapeHtml });

  // Build right column - All links organized
  const rightCol = buildLinksRightColumn(token, mint, hasWebsites, hasSocials, { escapeHtml });

  return `<div class="links-container">${leftCol}${rightCol}</div>`;
}

function buildLinksLeftColumn(token, hasLogo, hasHeader, hasDescription, options = {}) {
  const { escapeHtml } = options;
  const mint = token.mint;

  // Token info section
  const tokenInfoSection = `
    <div class="links-info-card">
      <div class="links-info-title">
        <i class="icon-info"></i>
        Token Info
      </div>
      <div class="links-info-content">
        <div class="links-info-row">
          <span class="links-info-label">Mint Address</span>
          <div class="links-info-value-group">
            <span class="links-info-value mono" title="${escapeHtml(mint)}">${formatShortAddress(mint)}</span>
            <button class="copy-btn-mini" data-copy="${escapeHtml(mint)}" title="Copy mint address">
              <i class="icon-copy"></i>
            </button>
          </div>
        </div>
        ${
          token.data_source
            ? `
        <div class="links-info-row">
          <span class="links-info-label">Data Source</span>
          <span class="links-info-value badge">${escapeHtml(token.data_source)}</span>
        </div>
        `
            : ""
        }
        ${
          token.verified
            ? `
        <div class="links-info-row">
          <span class="links-info-label">Status</span>
          <span class="links-info-value badge success"><i class="icon-shield-check"></i> Verified</span>
        </div>
        `
            : ""
        }
      </div>
    </div>
  `;

  // Media section - logo and header
  let mediaSection = "";
  if (hasLogo || hasHeader) {
    const logoHtml = hasLogo
      ? `
      <div class="links-media-item">
        <div class="links-media-label">Logo</div>
        <div class="links-media-preview logo">
          <img src="${escapeHtml(token.logo_url)}" alt="Token Logo" onerror="this.parentElement.innerHTML='<i class=\\'icon-image-off\\'></i>'" />
        </div>
        <a href="${escapeHtml(token.logo_url)}" target="_blank" class="links-media-link">
          <i class="icon-external-link"></i> Open Image
        </a>
      </div>
    `
      : "";

    const headerHtml = hasHeader
      ? `
      <div class="links-media-item">
        <div class="links-media-label">Header Image</div>
        <div class="links-media-preview header">
          <img src="${escapeHtml(token.header_image_url)}" alt="Header Image" onerror="this.parentElement.innerHTML='<i class=\\'icon-image-off\\'></i>'" />
        </div>
        <a href="${escapeHtml(token.header_image_url)}" target="_blank" class="links-media-link">
          <i class="icon-external-link"></i> Open Image
        </a>
      </div>
    `
      : "";

    mediaSection = `
      <div class="links-info-card">
        <div class="links-info-title">
          <i class="icon-image"></i>
          Media Assets
        </div>
        <div class="links-media-grid">
          ${logoHtml}
          ${headerHtml}
        </div>
      </div>
    `;
  }

  // Description section
  let descriptionSection = "";
  if (hasDescription) {
    descriptionSection = `
      <div class="links-info-card">
        <div class="links-info-title">
          <i class="icon-file-text"></i>
          Description
        </div>
        <div class="links-description">
          ${escapeHtml(token.description)}
        </div>
      </div>
    `;
  }

  return `
    <div class="links-left-col">
      ${tokenInfoSection}
      ${mediaSection}
      ${descriptionSection}
    </div>
  `;
}

function buildLinksRightColumn(token, mint, hasWebsites, hasSocials, options = {}) {
  const { escapeHtml } = options;

  // Explorers section - comprehensive list
  const explorersSection = `
    <div class="links-section-card">
      <div class="links-section-title">
        <i class="icon-search"></i>
        Explorers & Analytics
      </div>
      <div class="links-grid-compact">
        ${buildExplorerLink("https://solscan.io/token/" + mint, "Solscan", { escapeHtml })}
        ${buildExplorerLink("https://explorer.solana.com/address/" + mint, "Solana Explorer", { escapeHtml })}
        ${buildExplorerLink("https://birdeye.so/token/" + mint + "?chain=solana", "Birdeye", { escapeHtml })}
        ${buildExplorerLink("https://dexscreener.com/solana/" + mint, "DEX Screener", { escapeHtml })}
        ${buildExplorerLink("https://www.geckoterminal.com/solana/tokens/" + mint, "GeckoTerminal", { escapeHtml })}
        ${buildExplorerLink("https://www.dextools.io/app/en/solana/pair-explorer/" + mint, "DexTools", { escapeHtml })}
        ${buildExplorerLink("https://gmgn.ai/sol/token/" + mint, "GMGN", { escapeHtml })}
        ${buildExplorerLink("https://photon-sol.tinyastro.io/en/lp/" + mint, "Photon", { escapeHtml })}
        ${buildExplorerLink("https://rugcheck.xyz/tokens/" + mint, "RugCheck", { escapeHtml })}
        ${buildExplorerLink("https://app.bubblemaps.io/sol/token/" + mint, "Bubblemaps", { escapeHtml })}
        ${buildExplorerLink("https://www.coingecko.com/en/coins/" + mint, "CoinGecko", { escapeHtml })}
        ${buildExplorerLink("https://jup.ag/swap/SOL-" + mint, "Jupiter Swap", { escapeHtml })}
      </div>
    </div>
  `;

  // Official websites section
  let websitesSection = "";
  if (hasWebsites) {
    const websiteLinks = token.websites
      .map((site) => {
        const label = site.label || extractDomainName(site.url) || "Website";
        return buildOfficialLink(site.url, label, { escapeHtml });
      })
      .join("");

    websitesSection = `
      <div class="links-section-card">
        <div class="links-section-title">
          <i class="icon-globe"></i>
          Official Websites
        </div>
        <div class="links-list">
          ${websiteLinks}
        </div>
      </div>
    `;
  }

  // Social links section
  let socialsSection = "";
  if (hasSocials) {
    const socialLinks = token.socials
      .map((social) => {
        const { label } = getSocialMeta(social.platform);
        return buildSocialLink(social.url, label, { escapeHtml });
      })
      .join("");

    socialsSection = `
      <div class="links-section-card">
        <div class="links-section-title">
          <i class="icon-share-2"></i>
          Social Media
        </div>
        <div class="links-list">
          ${socialLinks}
        </div>
      </div>
    `;
  }

  // No links message
  let noLinksMessage = "";
  if (!hasWebsites && !hasSocials) {
    noLinksMessage = `
      <div class="links-empty-notice">
        <i class="icon-link-2-off"></i>
        <span>No official website or social links available for this token.</span>
      </div>
    `;
  }

  return `
    <div class="links-right-col">
      ${explorersSection}
      ${websitesSection}
      ${socialsSection}
      ${noLinksMessage}
    </div>
  `;
}

function buildExplorerLink(url, name, options = {}) {
  const { escapeHtml } = options;
  return `
    <a href="${escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-explorer-item">
      <span>${escapeHtml(name)}</span>
      <i class="icon-external-link link-external-icon"></i>
    </a>
  `;
}

function buildOfficialLink(url, label, options = {}) {
  const { escapeHtml } = options;
  return `
    <a href="${escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-official-item">
      <div class="links-official-content">
        <span class="links-official-label">${escapeHtml(label)}</span>
        <span class="links-official-url">${escapeHtml(formatUrl(url))}</span>
      </div>
      <i class="icon-external-link link-external-icon"></i>
    </a>
  `;
}

function buildSocialLink(url, label, options = {}) {
  const { escapeHtml } = options;
  const username = extractSocialUsername(url);
  return `
    <a href="${escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-social-item">
      <div class="links-social-content">
        <span class="links-social-platform">${escapeHtml(label)}</span>
        ${username ? `<span class="links-social-handle">${escapeHtml(username)}</span>` : ""}
      </div>
      <i class="icon-external-link link-external-icon"></i>
    </a>
  `;
}

/**
 * Initialize event handlers for links tab (copy buttons)
 */
export function initLinksTabEvents(container) {
  container.querySelectorAll(".copy-btn-mini[data-copy]").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const text = btn.dataset.copy;
      if (text) {
        Utils.copyToClipboard(text);
      }
    });
  });
}

// Helper functions

function formatShortAddress(address) {
  if (!address || address.length < 12) return address;
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

function getSocialMeta(platform) {
  const platformLower = platform?.toLowerCase() || "";
  const socialMap = {
    twitter: { icon: "icon-twitter", label: "Twitter / X" },
    x: { icon: "icon-twitter", label: "X (Twitter)" },
    telegram: { icon: "icon-send", label: "Telegram" },
    discord: { icon: "icon-message-circle", label: "Discord" },
    medium: { icon: "icon-book-open", label: "Medium" },
    github: { icon: "icon-github", label: "GitHub" },
    youtube: { icon: "icon-youtube", label: "YouTube" },
    reddit: { icon: "icon-message-square", label: "Reddit" },
    facebook: { icon: "icon-facebook", label: "Facebook" },
    instagram: { icon: "icon-instagram", label: "Instagram" },
    linkedin: { icon: "icon-linkedin", label: "LinkedIn" },
    tiktok: { icon: "icon-music", label: "TikTok" },
  };
  return socialMap[platformLower] || { icon: "icon-link", label: platform || "Link" };
}

function extractDomainName(url) {
  try {
    const domain = new URL(url).hostname;
    return domain.replace(/^www\./, "");
  } catch {
    return null;
  }
}

function formatUrl(url) {
  try {
    const parsed = new URL(url);
    return parsed.hostname + (parsed.pathname !== "/" ? parsed.pathname : "");
  } catch {
    return url;
  }
}

function extractSocialUsername(url) {
  try {
    const parsed = new URL(url);
    const path = parsed.pathname.replace(/^\/+|\/+$/g, "");
    if (path && !path.includes("/")) {
      return "@" + path;
    }
    return null;
  } catch {
    return null;
  }
}
