/**
 * Token Details Dialog - Pools & Links tabs
 *
 * Both tabs use continuous, independently scrolling information sheets. Their
 * renderers stay together because they present the same external/reference
 * metadata from the token detail response.
 */
import * as Utils from "../../core/utils.js";
import { formatAddressCompact } from "../../core/format.js";
import { renderTabState } from "./state_handling.js";

const esc = (text) => Utils.escapeHtml(text);
const count = (value) => Utils.formatNumber(value, { decimals: 0 });

// Ids are the `token_role` values of the pools API.
const TOKEN_ROLE_LABELS = Object.freeze({
  base: "tokens-pools-role-base",
  quote: "tokens-pools-role-quote",
  unknown: "tokens-pools-role-unknown",
});

// Ids are the lowercase `platform` values of a token's social links. Names are terms.
const SOCIAL_LABELS = Object.freeze({
  twitter: "tokens-links-social-twitter",
  x: "tokens-links-social-x",
  telegram: "tokens-links-social-telegram",
  discord: "tokens-links-social-discord",
  medium: "tokens-links-social-medium",
  github: "tokens-links-social-github",
  youtube: "tokens-links-social-youtube",
  reddit: "tokens-links-social-reddit",
  facebook: "tokens-links-social-facebook",
  instagram: "tokens-links-social-instagram",
  linkedin: "tokens-links-social-linkedin",
  tiktok: "tokens-links-social-tiktok",
});

// Ids name the explorer and analytics providers of buildExplorerSection. Names are terms.
const EXPLORER_LABELS = Object.freeze({
  solscan: "links-explorer-solscan",
  solana_explorer: "tokens-links-explorer-solana-explorer",
  birdeye: "links-explorer-birdeye",
  dexscreener: "links-explorer-dexscreener",
  geckoterminal: "tokens-links-explorer-geckoterminal",
  dextools: "tokens-links-explorer-dextools",
  gmgn: "links-explorer-gmgn",
  photon: "links-explorer-photon",
  rugcheck: "links-explorer-rugcheck",
  bubblemaps: "links-explorer-bubblemaps",
  coingecko: "tokens-links-explorer-coingecko",
  jupiter_swap: "tokens-links-explorer-jupiter-swap",
});

export function renderPoolsTab(token, options = {}) {
  const { renderHintTrigger, escapeHtml, formatShortAddress } = options;
  const pools = token.pools || [];

  if (pools.length === 0) {
    return renderTabState({
      icon: "icon-droplet",
      title: I18n.t("tokens-pools-empty-title"),
      message: I18n.t("tokens-pools-empty-message"),
    });
  }

  const totalLiquidity = pools.reduce((sum, pool) => sum + (pool.liquidity_usd || 0), 0);
  const totalVolume24h = pools.reduce((sum, pool) => sum + (pool.volume_h24_usd || 0), 0);
  const canonicalPool = pools.find((pool) => pool.is_canonical);
  const programCounts = pools.reduce((counts, pool) => {
    const program = pool.program || "";
    counts[program] = (counts[program] || 0) + 1;
    return counts;
  }, {});
  const summaryFacts = [
    [I18n.t("tokens-pools-total"), count(pools.length)],
    [I18n.t("tokens-pools-liquidity"), Utils.formatCurrencyUSD(totalLiquidity)],
    [I18n.t("tokens-pools-volume-24h"), Utils.formatCurrencyUSD(totalVolume24h)],
    [I18n.t("tokens-pools-base-role"), count(pools.filter((pool) => pool.token_role === "base").length)],
    [I18n.t("tokens-pools-quote-role"), count(pools.filter((pool) => pool.token_role === "quote").length)],
  ];

  const canonicalSection = canonicalPool
    ? `
      <section class="pools-section">
        <div class="pools-section-title">
          <span><i class="icon-star" aria-hidden="true"></i>${esc(I18n.t("tokens-pools-canonical-title"))}</span>
        </div>
        <div class="pools-summary-rows">
          ${renderPoolFact(I18n.t("tokens-pools-dex"), escapeHtml(canonicalPool.program || I18n.t("tokens-pools-unknown")))}
          ${renderPoolFact(I18n.t("tokens-pools-liquidity"), Utils.formatCurrencyUSD(canonicalPool.liquidity_usd))}
          ${renderPoolFact(I18n.t("tokens-pools-volume-24h"), Utils.formatCurrencyUSD(canonicalPool.volume_h24_usd))}
        </div>
      </section>
    `
    : "";

  return `
    <div class="pools-container">
      <div class="pools-left-col">
        <section class="pools-section">
          <div class="pools-section-title">
            <span>${esc(I18n.t("tokens-pools-summary-title"))}</span>
            ${renderHintTrigger("tokenDetails.pools")}
          </div>
          <div class="pools-summary-grid">
            ${summaryFacts
              .map(
                ([label, value]) => `
                  <div class="pools-summary-fact">
                    <span>${esc(label)}</span>
                    <strong>${value}</strong>
                  </div>
                `
              )
              .join("")}
          </div>
        </section>

        <section class="pools-section">
          <div class="pools-section-title"><span>${esc(I18n.t("tokens-pools-breakdown-title"))}</span></div>
          <div class="pools-summary-rows">
            ${Object.entries(programCounts)
              .sort((a, b) => b[1] - a[1])
              .map(([program, total]) =>
                renderPoolFact(program || I18n.t("tokens-pools-unknown"), count(total))
              )
              .join("")}
          </div>
        </section>

        ${canonicalSection}
      </div>

      <div class="pools-right-col">
        <div class="pools-column-heading">
          <span>${esc(I18n.t("tokens-pools-all-title"))}</span>
          <strong>${count(pools.length)}</strong>
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
  const addressLabels = poolAddressLabels();
  const lastUpdated = pool.last_updated_unix
    ? Utils.formatTimestamp(pool.last_updated_unix * 1000)
    : "—";

  return `
    <article class="pool-detail">
      <header class="pool-detail-header">
        <div class="pool-detail-identity">
          <strong>${escapeHtml(pool.program || I18n.t("tokens-pools-unknown-dex"))}</strong>
          ${pool.is_canonical ? `<span class="pool-canonical-label"><i class="icon-star" aria-hidden="true"></i>${esc(I18n.t("tokens-pools-canonical"))}</span>` : ""}
        </div>
        <span class="pool-detail-role ${roleClass}">
          ${esc(I18n.label(TOKEN_ROLE_LABELS, pool.token_role || "unknown"))}
        </span>
      </header>

      <div class="pool-detail-metrics">
        ${renderPoolMetric(I18n.t("tokens-pools-liquidity"), Utils.formatCurrencyUSD(pool.liquidity_usd))}
        ${renderPoolMetric(I18n.t("tokens-pools-volume-24h"), Utils.formatCurrencyUSD(pool.volume_h24_usd))}
        ${renderPoolMetric(I18n.t("tokens-pools-updated"), lastUpdated)}
      </div>

      <div class="pool-detail-addresses">
        ${renderAddressRow(addressLabels.pool, pool.pool_id, { escapeHtml, formatShortAddress })}
        ${renderAddressRow(addressLabels.base, pool.base_mint, { escapeHtml, formatShortAddress })}
        ${renderAddressRow(addressLabels.quote, pool.quote_mint, { escapeHtml, formatShortAddress })}
        ${renderAddressRow(addressLabels.paired, pool.paired_mint, { escapeHtml, formatShortAddress })}
      </div>

      <div class="pool-reserves">
        <div class="pool-reserves-heading">${esc(I18n.t("tokens-pools-reserves"))} <span>${count(reserveAccounts.length)}</span></div>
        ${
          reserveAccounts.length
            ? reserveAccounts
                .map((address) => renderAddressRow(null, address, { escapeHtml, formatShortAddress }))
                .join("")
            : `<span class="pool-no-data">${esc(I18n.t("tokens-pools-no-reserves"))}</span>`
        }
      </div>
    </article>
  `;
}

function renderPoolFact(label, value) {
  return `
    <div class="pools-summary-row">
      <span>${esc(label)}</span>
      <strong>${value}</strong>
    </div>
  `;
}

function renderPoolMetric(label, value) {
  return `
    <div class="pool-detail-metric">
      <span>${esc(label)}</span>
      <strong>${value}</strong>
    </div>
  `;
}

/** Row label (message value) and copy tooltip (`.title`) of each pool address kind. */
function poolAddressLabels() {
  return {
    pool: {
      label: I18n.t("tokens-pools-address-pool"),
      copyTitle: I18n.attr("tokens-pools-address-pool", "title"),
    },
    base: {
      label: I18n.t("tokens-pools-address-base"),
      copyTitle: I18n.attr("tokens-pools-address-base", "title"),
    },
    quote: {
      label: I18n.t("tokens-pools-address-quote"),
      copyTitle: I18n.attr("tokens-pools-address-quote", "title"),
    },
    paired: {
      label: I18n.t("tokens-pools-address-paired"),
      copyTitle: I18n.attr("tokens-pools-address-paired", "title"),
    },
  };
}

/** `labels` is null for a row without a label (reserve accounts). */
function renderAddressRow(labels, address, options = {}) {
  const { escapeHtml, formatShortAddress } = options;
  if (!address) return "";
  const safeAddress = escapeHtml(address);
  const copyTitle = labels ? labels.copyTitle : I18n.t("tokens-pools-address-copy");
  return `
    <div class="pool-address-row">
      ${labels ? `<span>${esc(labels.label)}</span>` : ""}
      <div class="pool-address-value">
        <code dir="ltr" title="${safeAddress}">${formatShortAddress(address)}</code>
        <button class="copy-btn-mini" type="button" data-copy="${safeAddress}" title="${esc(copyTitle)}">
          <i class="icon-copy" aria-hidden="true"></i>
        </button>
      </div>
    </div>
  `;
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
                <span>${esc(I18n.t("tokens-links-empty"))}</span>
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
      <div class="links-section-title"><i class="icon-info" aria-hidden="true"></i>${esc(I18n.t("tokens-links-info-title"))}</div>
      <div>
        <div class="links-info-row">
          <span>${esc(I18n.t("tokens-links-mint-address"))}</span>
          <div class="links-info-value">
            <code dir="ltr" title="${safeMint}">${formatShortAddress(mint)}</code>
            <button class="copy-btn-mini" type="button" data-copy="${safeMint}" title="${esc(I18n.attr("links-copy-mint", "title"))}">
              <i class="icon-copy" aria-hidden="true"></i>
            </button>
          </div>
        </div>
        ${token.data_source ? renderLinkFact(I18n.t("tokens-links-data-source"), escapeHtml(token.data_source)) : ""}
        ${token.verified ? renderLinkFact(I18n.t("tokens-links-security"), esc(I18n.t("positions-risk-low")), "verified") : ""}
      </div>
    </section>
    ${buildProfileSection(token, safeMint)}
  `;
}

function buildProfileSection(token, safeMint) {
  const isPublished = Boolean(token.profile);
  return `
    <section class="links-sheet-section links-profile-section">
      <div class="links-section-title">
        <i class="${isPublished ? "icon-badge-check" : "icon-badge-plus"}" aria-hidden="true"></i>
        ${esc(isPublished ? I18n.t("tokens-links-profile-published-title") : I18n.t("tokens-links-profile-title"))}
      </div>
      <p>
        ${
          esc(
            isPublished
              ? I18n.t("tokens-links-profile-published-note")
              : I18n.t("tokens-links-profile-create-note")
          )
        }
      </p>
      <button
        class="links-profile-action"
        type="button"
        data-profile-mint="${safeMint}"
        title="${esc(isPublished ? I18n.t("tokens-links-profile-update-hint") : I18n.t("tokens-links-profile-create-hint"))}"
      >
        <span>${esc(isPublished ? I18n.t("tokens-links-profile-update") : I18n.t("tokens-links-profile-create"))}</span>
        <i class="icon-external-link" aria-hidden="true"></i>
      </button>
    </section>
  `;
}

function buildMediaSection(token, logoUrl, bannerUrl, options = {}) {
  const { escapeHtml } = options;
  if (!logoUrl && !bannerUrl) return "";
  const symbol = token.symbol || I18n.t("tokens-links-media-fallback-symbol");
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-image" aria-hidden="true"></i>${esc(I18n.t("tokens-links-media-title"))}</div>
      <div class="links-media-grid">
        ${logoUrl ? renderMediaItem(I18n.t("tokens-links-media-logo"), logoUrl, symbol, "logo", { escapeHtml }) : ""}
        ${bannerUrl ? renderMediaItem(
              I18n.t("tokens-links-media-banner"),
              bannerUrl,
              I18n.t("tokens-links-media-banner-alt", { symbol }),
              "banner",
              { escapeHtml }
            ) : ""}
      </div>
    </section>
  `;
}

function renderMediaItem(label, url, alt, type, options = {}) {
  const { escapeHtml } = options;
  const safeUrl = escapeHtml(url);
  const logoClass = type === "logo" ? " token-logo-frame" : "";
  const imageClass = type === "logo" ? ' class="token-logo-artwork"' : "";
  return `
    <div class="links-media-item">
      <div class="links-media-label">${esc(label)}</div>
      <div class="links-media-preview ${type}${logoClass}">
        <img src="${safeUrl}" alt="${esc(alt)}"${imageClass} />
      </div>
      <a href="${safeUrl}" target="_blank" rel="noopener noreferrer" class="links-media-link">
        ${esc(I18n.t("tokens-links-media-open"))} <i class="icon-external-link" aria-hidden="true"></i>
      </a>
    </div>
  `;
}

function buildDescriptionSection(description, options = {}) {
  const { escapeHtml } = options;
  if (!description) return "";
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-file-text" aria-hidden="true"></i>${esc(I18n.t("tokens-links-description-title"))}</div>
      <p class="links-description">${escapeHtml(description)}</p>
    </section>
  `;
}

function buildExplorerSection(mint, options = {}) {
  const { escapeHtml } = options;
  const explorers = [
    ["solscan", `https://solscan.io/token/${mint}`],
    ["solana_explorer", `https://explorer.solana.com/address/${mint}`],
    ["birdeye", `https://birdeye.so/token/${mint}?chain=solana`],
    ["dexscreener", `https://dexscreener.com/solana/${mint}`],
    ["geckoterminal", `https://www.geckoterminal.com/solana/tokens/${mint}`],
    ["dextools", `https://www.dextools.io/app/en/solana/pair-explorer/${mint}`],
    ["gmgn", `https://gmgn.ai/sol/token/${mint}`],
    ["photon", `https://photon-sol.tinyastro.io/en/lp/${mint}`],
    ["rugcheck", `https://rugcheck.xyz/tokens/${mint}`],
    ["bubblemaps", `https://app.bubblemaps.io/sol/token/${mint}`],
    ["coingecko", `https://www.coingecko.com/en/coins/${mint}`],
    ["jupiter_swap", `https://jup.ag/swap/SOL-${mint}`],
  ];
  return `
    <section class="links-sheet-section">
      <div class="links-section-title"><i class="icon-search" aria-hidden="true"></i>${esc(I18n.t("tokens-links-explorers-title"))}</div>
      <div class="links-explorer-grid">
        ${explorers
          .map(([id, url]) =>
            renderExternalRow(I18n.label(EXPLORER_LABELS, id), url, "", { escapeHtml })
          )
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
      <div class="links-section-title"><i class="icon-globe" aria-hidden="true"></i>${esc(I18n.t("tokens-links-websites-title"))}</div>
      <div class="links-list">
        ${websites
          .map((site) => {
            const label = site.label || extractDomainName(site.url) || I18n.t("positions-link-website");
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
      <div class="links-section-title"><i class="icon-share-2" aria-hidden="true"></i>${esc(I18n.t("tokens-links-socials-title"))}</div>
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
      <span>${esc(label)}</span>
      <strong class="${modifier}">${value}</strong>
    </div>
  `;
}

function formatShortAddress(address) {
  return formatAddressCompact(address, { start: 6, end: 4, ellipsis: "..." });
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
  const id = platform?.toLowerCase();
  if (id && Object.hasOwn(SOCIAL_LABELS, id)) return I18n.label(SOCIAL_LABELS, id);
  return platform || I18n.t("tokens-links-social-fallback");
}
