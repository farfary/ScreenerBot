/**
 * Licenses Tab Module - Open source software licenses
 * Extracted from settings_dialog.js
 */
import * as Utils from "../../core/utils.js";

/**
 * Build Licenses tab HTML
 */
export function buildLicensesTab() {
  const licenses = [
    {
      category: "Application Framework",
      items: [
        {
          name: "Electron",
          license: "MIT",
          url: "https://www.electronjs.org/",
          desc: "Desktop application framework",
        },
        {
          name: "Tokio",
          license: "MIT",
          url: "https://tokio.rs/",
          desc: "Async runtime for Rust",
        },
        {
          name: "Axum",
          license: "MIT",
          url: "https://github.com/tokio-rs/axum",
          desc: "Web server framework",
        },
        {
          name: "Tower",
          license: "MIT",
          url: "https://github.com/tower-rs/tower",
          desc: "Service abstractions",
        },
        { name: "Hyper", license: "MIT", url: "https://hyper.rs/", desc: "HTTP implementation" },
      ],
    },
    {
      category: "Solana Blockchain",
      items: [
        {
          name: "solana-sdk",
          license: "Apache-2.0",
          url: "https://github.com/anza-xyz/agave",
          desc: "Solana SDK core",
        },
        {
          name: "solana-client",
          license: "Apache-2.0",
          url: "https://github.com/anza-xyz/agave",
          desc: "RPC client",
        },
        {
          name: "solana-program",
          license: "Apache-2.0",
          url: "https://github.com/anza-xyz/agave",
          desc: "Program library",
        },
        {
          name: "spl-token",
          license: "Apache-2.0",
          url: "https://github.com/solana-labs/solana-program-library",
          desc: "SPL Token program",
        },
        {
          name: "spl-token-2022",
          license: "Apache-2.0",
          url: "https://github.com/solana-labs/solana-program-library",
          desc: "Token-2022 extensions",
        },
        {
          name: "spl-associated-token-account",
          license: "Apache-2.0",
          url: "https://github.com/solana-labs/solana-program-library",
          desc: "Associated token accounts",
        },
      ],
    },
    {
      category: "Data & Storage",
      items: [
        {
          name: "SQLite",
          license: "Public Domain",
          url: "https://sqlite.org/",
          desc: "Embedded database engine",
        },
        {
          name: "rusqlite",
          license: "MIT",
          url: "https://github.com/rusqlite/rusqlite",
          desc: "SQLite Rust bindings",
        },
        {
          name: "r2d2",
          license: "MIT / Apache-2.0",
          url: "https://github.com/sfackler/r2d2",
          desc: "Database connection pool",
        },
        {
          name: "Serde",
          license: "MIT / Apache-2.0",
          url: "https://serde.rs/",
          desc: "Serialization framework",
        },
        {
          name: "TOML",
          license: "MIT / Apache-2.0",
          url: "https://github.com/toml-rs/toml",
          desc: "Configuration parsing",
        },
      ],
    },
    {
      category: "Networking",
      items: [
        {
          name: "reqwest",
          license: "MIT / Apache-2.0",
          url: "https://github.com/seanmonstar/reqwest",
          desc: "HTTP client",
        },
        {
          name: "tokio-tungstenite",
          license: "MIT",
          url: "https://github.com/snapview/tokio-tungstenite",
          desc: "WebSocket client",
        },
        {
          name: "RustLS",
          license: "MIT / Apache-2.0",
          url: "https://github.com/rustls/rustls",
          desc: "TLS implementation",
        },
      ],
    },
    {
      category: "Cryptography & Encoding",
      items: [
        {
          name: "BLAKE3",
          license: "CC0 / Apache-2.0",
          url: "https://github.com/BLAKE3-team/BLAKE3",
          desc: "Hash function",
        },
        {
          name: "SHA-2",
          license: "MIT / Apache-2.0",
          url: "https://github.com/RustCrypto/hashes",
          desc: "SHA-256/512 hashing",
        },
        {
          name: "bs58",
          license: "MIT / Apache-2.0",
          url: "https://github.com/Nullus157/bs58-rs",
          desc: "Base58 encoding",
        },
        {
          name: "base64",
          license: "MIT / Apache-2.0",
          url: "https://github.com/marshallpierce/rust-base64",
          desc: "Base64 encoding",
        },
      ],
    },
    {
      category: "UI Assets",
      items: [
        {
          name: "Lucide Icons",
          license: "ISC",
          url: "https://lucide.dev/",
          desc: "Icon font library",
        },
        {
          name: "JetBrains Mono",
          license: "OFL-1.1",
          url: "https://www.jetbrains.com/lp/mono/",
          desc: "Monospace font",
        },
        {
          name: "Orbitron",
          license: "OFL-1.1",
          url: "https://fonts.google.com/specimen/Orbitron",
          desc: "Display font",
        },
      ],
    },
  ];

  const categoriesHtml = licenses
    .map(
      (cat) => `
      <div class="license-category">
        <h4 class="license-category-title">${Utils.escapeHtml(cat.category)}</h4>
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
              <p class="license-item-desc">${Utils.escapeHtml(item.desc)}</p>
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
          <h3>Open Source Licenses</h3>
          <p>ScreenerBot is built with the following open source software</p>
        </div>
      </div>
      <div class="licenses-content">
        ${categoriesHtml}
      </div>
      <div class="licenses-footer">
        <p>
          <i class="icon-info"></i>
          Full license texts are available in the project repository and within each dependency's source code.
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
