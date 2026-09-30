/**
 * Licenses Tab Module - Open source software licenses
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";

const LICENSE_CATEGORY_LABELS = Object.freeze({
  framework: "settings-licenses-category-framework",
  solana: "settings-licenses-category-solana",
  data: "settings-licenses-category-data",
  networking: "settings-licenses-category-networking",
  cryptography: "settings-licenses-category-cryptography",
  assets: "settings-licenses-category-assets",
});

// Keys are the dependency names; the names, license identifiers and URLs are
// third-party data and render as written.
const LICENSE_DESCRIPTION_LABELS = Object.freeze({
  Electron: "settings-licenses-desc-electron",
  Tokio: "settings-licenses-desc-tokio",
  Axum: "settings-licenses-desc-axum",
  Tower: "settings-licenses-desc-tower",
  Hyper: "settings-licenses-desc-hyper",
  "solana-sdk": "settings-licenses-desc-solana-sdk",
  "solana-client": "settings-licenses-desc-solana-client",
  "solana-program": "settings-licenses-desc-solana-program",
  "spl-token": "settings-licenses-desc-spl-token",
  "spl-token-2022": "settings-licenses-desc-spl-token-2022",
  "spl-associated-token-account": "settings-licenses-desc-spl-associated-token-account",
  SQLite: "settings-licenses-desc-sqlite",
  rusqlite: "settings-licenses-desc-rusqlite",
  r2d2: "settings-licenses-desc-r2d2",
  Serde: "settings-licenses-desc-serde",
  TOML: "settings-licenses-desc-toml",
  reqwest: "settings-licenses-desc-reqwest",
  "tokio-tungstenite": "settings-licenses-desc-tokio-tungstenite",
  RustLS: "settings-licenses-desc-rustls",
  BLAKE3: "settings-licenses-desc-blake3",
  "SHA-2": "settings-licenses-desc-sha-2",
  bs58: "settings-licenses-desc-bs58",
  base64: "settings-licenses-desc-base64",
  "Lucide Icons": "settings-licenses-desc-lucide-icons",
  Inter: "settings-licenses-desc-inter",
  "JetBrains Mono": "settings-licenses-desc-jetbrains-mono",
  Orbitron: "settings-licenses-desc-orbitron",
  Vazirmatn: "settings-licenses-desc-vazirmatn",
  "Noto Sans Devanagari": "settings-licenses-desc-noto-sans-devanagari",
  "Noto Sans SC": "settings-licenses-desc-noto-sans-sc",
  Pretendard: "settings-licenses-desc-pretendard",
  "Pretendard JP": "settings-licenses-desc-pretendard-jp",
});

/**
 * Build Licenses tab HTML
 */
export function buildLicensesTab() {
  const licenses = [
    {
      category: "framework",
      items: [
        {
          name: "Electron",
          license: "MIT",
          url: "https://www.electronjs.org/",
        },
        {
          name: "Tokio",
          license: "MIT",
          url: "https://tokio.rs/",
        },
        {
          name: "Axum",
          license: "MIT",
          url: "https://github.com/tokio-rs/axum",
        },
        {
          name: "Tower",
          license: "MIT",
          url: "https://github.com/tower-rs/tower",
        },
        { name: "Hyper", license: "MIT", url: "https://hyper.rs/" },
      ],
    },
    {
      category: "solana",
      items: [
        {
          name: "solana-sdk",
          license: "Apache-2.0",
          url: "https://github.com/anza-xyz/agave",
        },
        {
          name: "solana-client",
          license: "Apache-2.0",
          url: "https://github.com/anza-xyz/agave",
        },
        {
          name: "solana-program",
          license: "Apache-2.0",
          url: "https://github.com/anza-xyz/agave",
        },
        {
          name: "spl-token",
          license: "Apache-2.0",
          url: "https://github.com/solana-labs/solana-program-library",
        },
        {
          name: "spl-token-2022",
          license: "Apache-2.0",
          url: "https://github.com/solana-labs/solana-program-library",
        },
        {
          name: "spl-associated-token-account",
          license: "Apache-2.0",
          url: "https://github.com/solana-labs/solana-program-library",
        },
      ],
    },
    {
      category: "data",
      items: [
        {
          name: "SQLite",
          license: "Public Domain",
          url: "https://sqlite.org/",
        },
        {
          name: "rusqlite",
          license: "MIT",
          url: "https://github.com/rusqlite/rusqlite",
        },
        {
          name: "r2d2",
          license: "MIT / Apache-2.0",
          url: "https://github.com/sfackler/r2d2",
        },
        {
          name: "Serde",
          license: "MIT / Apache-2.0",
          url: "https://serde.rs/",
        },
        {
          name: "TOML",
          license: "MIT / Apache-2.0",
          url: "https://github.com/toml-rs/toml",
        },
      ],
    },
    {
      category: "networking",
      items: [
        {
          name: "reqwest",
          license: "MIT / Apache-2.0",
          url: "https://github.com/seanmonstar/reqwest",
        },
        {
          name: "tokio-tungstenite",
          license: "MIT",
          url: "https://github.com/snapview/tokio-tungstenite",
        },
        {
          name: "RustLS",
          license: "MIT / Apache-2.0",
          url: "https://github.com/rustls/rustls",
        },
      ],
    },
    {
      category: "cryptography",
      items: [
        {
          name: "BLAKE3",
          license: "CC0 / Apache-2.0",
          url: "https://github.com/BLAKE3-team/BLAKE3",
        },
        {
          name: "SHA-2",
          license: "MIT / Apache-2.0",
          url: "https://github.com/RustCrypto/hashes",
        },
        {
          name: "bs58",
          license: "MIT / Apache-2.0",
          url: "https://github.com/Nullus157/bs58-rs",
        },
        {
          name: "base64",
          license: "MIT / Apache-2.0",
          url: "https://github.com/marshallpierce/rust-base64",
        },
      ],
    },
    {
      category: "assets",
      items: [
        {
          name: "Lucide Icons",
          license: "ISC",
          url: "https://lucide.dev/",
        },
        {
          name: "Inter",
          license: "OFL-1.1",
          url: "https://rsms.me/inter/",
        },
        {
          name: "JetBrains Mono",
          license: "OFL-1.1",
          url: "https://www.jetbrains.com/lp/mono/",
        },
        {
          name: "Orbitron",
          license: "OFL-1.1",
          url: "https://fonts.google.com/specimen/Orbitron",
        },
        {
          name: "Vazirmatn",
          license: "OFL-1.1",
          url: "https://github.com/rastikerdar/vazirmatn",
        },
        {
          name: "Noto Sans Devanagari",
          license: "OFL-1.1",
          url: "https://fonts.google.com/noto/specimen/Noto+Sans+Devanagari",
        },
        {
          name: "Noto Sans SC",
          license: "OFL-1.1",
          url: "https://fonts.google.com/noto/specimen/Noto+Sans+SC",
        },
        {
          name: "Pretendard",
          license: "OFL-1.1",
          url: "https://github.com/orioncactus/pretendard",
        },
        {
          name: "Pretendard JP",
          license: "OFL-1.1",
          url: "https://github.com/orioncactus/pretendard",
        },
      ],
    },
  ];

  const categoriesHtml = licenses
    .map(
      (cat) => `
      <div class="license-category">
        <h4 class="license-category-title">${Utils.escapeHtml(I18n.label(LICENSE_CATEGORY_LABELS, cat.category))}</h4>
        <div class="license-items">
          ${cat.items
            .map(
              (item) => `
            <div class="license-item">
              <div class="license-item-header">
                <button class="license-item-name" data-external-url="${Utils.escapeHtml(item.url)}">
                  ${Utils.escapeHtml(item.name)}
                  <i class="icon-external-link"></i>
                </button>
                <span class="license-item-badge">${Utils.escapeHtml(item.license)}</span>
              </div>
              <p class="license-item-desc">${Utils.escapeHtml(I18n.label(LICENSE_DESCRIPTION_LABELS, item.name))}</p>
            </div>
          `
            )
            .join("")}
        </div>
      </div>
    `
    )
    .join("");

  return `
    <div class="settings-licenses">
      <div class="licenses-header">
        <i class="icon-scale"></i>
        <div>
          <h3 data-l10n-id="settings-licenses-title"></h3>
          <p data-l10n-id="settings-licenses-subtitle"></p>
        </div>
      </div>
      <div class="licenses-content">
        ${categoriesHtml}
      </div>
      <div class="licenses-footer">
        <p>
          <i class="icon-info"></i>
          <span data-l10n-id="settings-licenses-footer"></span>
        </p>
      </div>
    </div>
  `;
}

/**
 * Attach handlers for Licenses tab
 */
export function attachLicensesHandlers(content) {
  // External links (license project URLs)
  const externalLinks = content.querySelectorAll("[data-external-url]");
  externalLinks.forEach((btn) => {
    btn.addEventListener("click", () => {
      const url = btn.dataset.externalUrl;
      if (url) {
        Utils.openExternal(url);
      }
    });
  });
}
