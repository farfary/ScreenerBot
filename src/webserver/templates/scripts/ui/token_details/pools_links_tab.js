/**
 * Token Details Dialog - Pools & Links tabs
 *
 * Both tabs use continuous, independently scrolling information sheets. Their
 * renderers stay together because they present the same external/reference
 * metadata from the token detail response.
 */
import * as Utils from "../../core/utils.js";
import { renderTabState } from "./state_handling.js";

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

  const totalLiquidity = pools.reduce((sum, pool) => sum + (pool.liquidity_usd || 0), 0);
  const totalVolume24h = pools.reduce((sum, pool) => sum + (pool.volume_h24_usd || 0), 0);
  const canonicalPool = pools.find((pool) => pool.is_canonical);
  const programCounts = pools.reduce((counts, pool) => {
    const program = pool.program || "Unknown";
    counts[program] = (counts[program] || 0) + 1;
    return counts;
  }, {});
  const summaryFacts = [
    ["Total Pools", pools.length],
    ["Liquidity", Utils.formatCurrencyUSD(totalLiquidity)],
    ["24h Volume", Utils.formatCurrencyUSD(totalVolume24h)],
    ["Base Role", pools.filter((pool) => pool.token_role === "base").length],
    ["Quote Role", pools.filter((pool) => pool.token_role === "quote").length],
  ];

  const canonicalSection = canonicalPool
    ? `
      <section class="pools-section">
        <div class="pools-section-title">
          <span><i class="icon-star" aria-hidden="true"></i>Canonical pool</span>
        </div>
        <div class="pools-summary-rows">
          ${renderPoolFact("DEX", escapeHtml(canonicalPool.program || "Unknown"))}
          ${renderPoolFact("Liquidity", formatCurrency(canonicalPool.liquidity_usd))}
          ${renderPoolFact("24h Volume", formatCurrency(canonicalPool.volume_h24_usd))}
        </div>
      </section>
    `
    : "";

  return `
    <div class="pools-container">
      <div class="pools-left-col">
        <section class="pools-section">
          <div class="pools-section-title">
            <span>Pool summary</span>
            ${renderHintTrigger("tokenDetails.pools")}
          </div>
          <div class="pools-summary-grid">
            ${summaryFacts
              .map(
                ([label, value]) => `
                  <div class="pools-summary-fact">
                    <span>${label}</span>
                    <strong>${value}</strong>
                  </div>
                `
              )
              .join("")}
          </div>
        </section>

        <section class="pools-section">
          <div class="pools-section-title"><span>DEX breakdown</span></div>
          <div class="pools-summary-rows">
            ${Object.entries(programCounts)
              .sort((a, b) => b[1] - a[1])
              .map(([program, count]) => renderPoolFact(escapeHtml(program), count))
              .join("")}
          </div>
        </section>

        ${canonicalSection}
      </div>

      <div class="pools-right-col">
        <div class="pools-column-heading">
          <span>All pools</span>
          <strong>${pools.length}</strong>
        </div>
        <div>
          ${pools.map((pool) => buildPoolDetail(pool, { escapeHtml, formatShortAddress })).join("")}
        </div>
      </div>
    </div>
  `;
}

function buildPoolDetail(pool, options = {}) {
  const { escapeHtml, formatShortAddress } = options;
  const reserveAccounts = Array.isArray(pool.reserve_accounts) ? pool.reserve_accounts : [];
  const roleClass = pool.token_role === "base" || pool.token_role === "quote" ? pool.token_role : "";
  const lastUpdated = pool.last_updated_unix
    ? Utils.formatTimestamp(pool.last_updated_unix * 1000)
    : "—";

  return `
    <article class="pool-detail">
      <header class="pool-detail-header">
        <div class="pool-detail-identity">
          <strong>${escapeHtml(pool.program || "Unknown DEX")}</strong>
          ${pool.is_canonical ? '<span class="pool-canonical-label"><i class="icon-star" aria-hidden="true"></i>Canonical</span>' : ""}
        </div>
        <span class="pool-detail-role ${roleClass}">
          ${escapeHtml(pool.token_role || "unknown")}
        </span>
      </header>

      <div class="pool-detail-metrics">
        ${renderPoolMetric("Liquidity", formatCurrency(pool.liquidity_usd))}
        ${renderPoolMetric("24h Volume", formatCurrency(pool.volume_h24_usd))}
        ${renderPoolMetric("Updated", lastUpdated)}
      </div>

      <div class="pool-detail-addresses">
        ${renderAddressRow("Pool", pool.pool_id, { escapeHtml, formatShortAddress })}
        ${renderAddressRow("Base mint", pool.base_mint, { escapeHtml, formatShortAddress })}
        ${renderAddressRow("Quote mint", pool.quote_mint, { escapeHtml, formatShortAddress })}
        ${renderAddressRow("Paired mint", pool.paired_mint, { escapeHtml, formatShortAddress })}
      </div>

      <div class="pool-reserves">
        <div class="pool-reserves-heading">Reserve accounts <span>${reserveAccounts.length}</span></div>
        ${
          reserveAccounts.length
            ? reserveAccounts
                .map((address) => renderAddressRow("", address, { escapeHtml, formatShortAddress }))
                .join("")
            : '<span class="pool-no-data">No reserve accounts</span>'
        }
      </div>
    </article>
  `;
}

function renderPoolFact(label, value) {
  return `
    <div class="pools-summary-row">
      <span>${label}</span>
      <strong>${value}</strong>
    </div>
  `;
}

function renderPoolMetric(label, value) {
  return `
    <div class="pool-detail-metric">
      <span>${label}</span>
      <strong>${value}</strong>
    </div>
  `;
}

function renderAddressRow(label, address, options = {}) {
  const { escapeHtml, formatShortAddress } = options;
  if (!address) return "";
  const safeAddress = escapeHtml(address);
  return `
    <div class="pool-address-row">
      ${label ? `<span>${label}</span>` : ""}
      <div class="pool-address-value">
        <code title="${safeAddress}">${formatShortAddress(address)}</code>
        <button class="copy-btn-mini" type="button" data-copy="${safeAddress}" title="Copy ${label ? label.toLowerCase() : "address"}">
          <i class="icon-copy" aria-hidden="true"></i>
        </button>
      </div>
    </div>
  `;
}

function formatCurrency(value) {
  return value === null || value === undefined ? "—" : Utils.formatCurrencyUSD(value);
}

export function renderLinksTab(token, options = {}) {
  const { escapeHtml } = options;
  const mint = token.mint;
  const websites = Array.isArray(token.websites) ? token.websites : [];
  const socials = Array.isArray(token.socials) ? token.socials : [];
  const logoUrl = Utils.resolveTokenLogoUrl(token);
  const bannerUrl = Utils.resolveTokenBannerUrl(token);

  return `
    <div class="links-container">
      <div class="links-left-col">
        ${buildTokenReferenceSection(token, mint, { escapeHtml })}
        ${buildMediaSection(token, logoUrl, bannerUrl, { escapeHtml })}
        ${buildDescriptionSection(token.description, { escapeHtml })}
      </div>
      <div class="links-right-col">
        ${buildExplorerSection(mint, { escapeHtml })}
        ${buildOfficialSection(websites, { escapeHtml })}
        ${buildSocialSection(socials, { escapeHtml })}
        ${
          websites.length === 0 && socials.length === 0
            ? `
              <div class="links-empty-notice">
                <i class="icon-link-2-off" aria-hidden="true"></i>
                <span>No official website or social links are available for this token.</span>
              </div>
            `
            : ""
        }
      </div>
    </div>
  `;
}

function buildTokenReferenceSection(token, mint, options = {}) {
  const { escapeHtml } = options;
  const safeMint = escapeHtml(mint);
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-info" aria-hidden="true"></i>Token info</div>
      <div>
        <div class="links-info-row">
          <span>Mint address</span>
          <div class="links-info-value">
            <code title="${safeMint}">${formatShortAddress(mint)}</code>
            <button class="copy-btn-mini" type="button" data-copy="${safeMint}" title="Copy mint address">
              <i class="icon-copy" aria-hidden="true"></i>
            </button>
          </div>
        </div>
        ${token.data_source ? renderLinkFact("Data source", escapeHtml(token.data_source)) : ""}
        ${token.verified ? renderLinkFact("Status", "Verified", "verified") : ""}
      </div>
    </section>
  `;
}

function buildMediaSection(token, logoUrl, bannerUrl, options = {}) {
  const { escapeHtml } = options;
  if (!logoUrl && !bannerUrl) return "";
  const symbol = escapeHtml(token.symbol || "Token");
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-image" aria-hidden="true"></i>Media assets</div>
      <div class="links-media-grid">
        ${logoUrl ? renderMediaItem("Logo", logoUrl, symbol, "logo", { escapeHtml }) : ""}
        ${bannerUrl ? renderMediaItem("Banner", bannerUrl, `${symbol} banner`, "banner", { escapeHtml }) : ""}
      </div>
    </section>
  `;
}

function renderMediaItem(label, url, alt, type, options = {}) {
  const { escapeHtml } = options;
  const safeUrl = escapeHtml(url);
  return `
    <div class="links-media-item">
      <div class="links-media-label">${label}</div>
      <div class="links-media-preview ${type}">
        <img src="${safeUrl}" alt="${alt}" />
      </div>
      <a href="${safeUrl}" target="_blank" rel="noopener noreferrer" class="links-media-link">
        Open image <i class="icon-external-link" aria-hidden="true"></i>
      </a>
    </div>
  `;
}

function buildDescriptionSection(description, options = {}) {
  const { escapeHtml } = options;
  if (!description) return "";
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-file-text" aria-hidden="true"></i>Description</div>
      <p class="links-description">${escapeHtml(description)}</p>
    </section>
  `;
}

function buildExplorerSection(mint, options = {}) {
  const { escapeHtml } = options;
  const explorers = [
    ["Solscan", `https://solscan.io/token/${mint}`],
    ["Solana Explorer", `https://explorer.solana.com/address/${mint}`],
    ["Birdeye", `https://birdeye.so/token/${mint}?chain=solana`],
    ["DEX Screener", `https://dexscreener.com/solana/${mint}`],
    ["GeckoTerminal", `https://www.geckoterminal.com/solana/tokens/${mint}`],
    ["DexTools", `https://www.dextools.io/app/en/solana/pair-explorer/${mint}`],
    ["GMGN", `https://gmgn.ai/sol/token/${mint}`],
    ["Photon", `https://photon-sol.tinyastro.io/en/lp/${mint}`],
    ["RugCheck", `https://rugcheck.xyz/tokens/${mint}`],
    ["Bubblemaps", `https://app.bubblemaps.io/sol/token/${mint}`],
    ["CoinGecko", `https://www.coingecko.com/en/coins/${mint}`],
    ["Jupiter Swap", `https://jup.ag/swap/SOL-${mint}`],
  ];
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-search" aria-hidden="true"></i>Explorers &amp; analytics</div>
      <div class="links-explorer-grid">
        ${explorers
          .map(([label, url]) => renderExternalRow(label, url, "", { escapeHtml }))
          .join("")}
      </div>
    </section>
  `;
}

function buildOfficialSection(websites, options = {}) {
  const { escapeHtml } = options;
  if (websites.length === 0) return "";
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-globe" aria-hidden="true"></i>Official websites</div>
      <div class="links-list">
        ${websites
          .map((site) => {
            const label = site.label || extractDomainName(site.url) || "Website";
            return renderExternalRow(label, site.url, formatUrl(site.url), { escapeHtml });
          })
          .join("")}
      </div>
    </section>
  `;
}

function buildSocialSection(socials, options = {}) {
  const { escapeHtml } = options;
  if (socials.length === 0) return "";
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-share-2" aria-hidden="true"></i>Social media</div>
      <div class="links-list">
        ${socials
          .map((social) => {
            const label = socialLabel(social.platform);
            return renderExternalRow(label, social.url, extractSocialUsername(social.url), {
              escapeHtml,
            });
          })
          .join("")}
      </div>
    </section>
  `;
}

function renderExternalRow(label, url, detail, options = {}) {
  const { escapeHtml } = options;
  return `
    <a href="${escapeHtml(url)}" target="_blank" rel="noopener noreferrer" class="links-external-row">
      <span class="links-external-copy">
        <strong>${escapeHtml(label)}</strong>
        ${detail ? `<small>${escapeHtml(detail)}</small>` : ""}
      </span>
      <i class="icon-external-link" aria-hidden="true"></i>
    </a>
  `;
}

function renderLinkFact(label, value, modifier = "") {
  return `
    <div class="links-info-row">
      <span>${label}</span>
      <strong class="${modifier}">${value}</strong>
    </div>
  `;
}

function formatShortAddress(address) {
  if (!address || address.length < 12) return address;
  return `${address.slice(0, 6)}...${address.slice(-4)}`;
}

function extractDomainName(url) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
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
    const path = new URL(url).pathname.replace(/^\/+|\/+$/g, "");
    return path && !path.includes("/") ? `@${path}` : "";
  } catch {
    return "";
  }
}

function socialLabel(platform) {
  const labels = {
    twitter: "Twitter / X",
    x: "X (Twitter)",
    telegram: "Telegram",
    discord: "Discord",
    medium: "Medium",
    github: "GitHub",
    youtube: "YouTube",
    reddit: "Reddit",
    facebook: "Facebook",
    instagram: "Instagram",
    linkedin: "LinkedIn",
    tiktok: "TikTok",
  };
  return labels[platform?.toLowerCase()] || platform || "Social";
}
