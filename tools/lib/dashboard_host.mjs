// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Serves the dashboard the way the app does, without the bot.
 *
 * The shell is assembled from the same files the Rust server embeds: the
 * placeholder substitutions follow `base_template` in `src/webserver/templates.rs`,
 * the global and per-page stylesheet lists are read from that file, and every
 * constant is resolved through `src/webserver/embeds.rs`. Scripts, fonts and the
 * i18n catalog are served from their source files. Page fragments come from
 * `templates/pages/` exactly as `/api/pages/{page}` returns them, localized by the
 * same transform the HTML validator uses. Nothing is copied.
 *
 * `/api/*` is answered by the caller-supplied handler; anything this host cannot
 * serve is a 404 so a missing asset fails the test that loads it.
 *
 * Consumers: `tools/tests/dashboard_stability.test.mjs`.
 */

import { createServer } from "node:http";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { resolve, extname } from "node:path";

import { REPO_ROOT, TEMPLATES_ROOT } from "./dashboard_ui.mjs";
import { formatMessage, localeDirection, localizeTemplate } from "../html/l10n_transform.mjs";
import { readServerOnlyDomains, SOURCE_LOCALE } from "../i18n/catalogs.mjs";

/** Port the shell placeholder reports; a session serves on its own port. */
const SHELL_PORT = "8080";

const WEBSERVER = resolve(REPO_ROOT, "src/webserver");
const sources = new Map();
/** Source files never change during a run; every session reads each one once. */
const read = (path) => {
  if (!sources.has(path)) sources.set(path, readFileSync(path, "utf8"));
  return sources.get(path);
};

function memoized(compute) {
  const results = new Map();
  return (...args) => {
    const key = args.join("\0");
    if (!results.has(key)) results.set(key, compute(...args));
    return results.get(key);
  };
}
const TEMPLATES_RS = read(resolve(WEBSERVER, "templates.rs"));

/** `NAME` -> source file, for every `include_str!` constant in embeds.rs. */
const EMBEDS = new Map(
  [
    ...read(resolve(WEBSERVER, "embeds.rs")).matchAll(
      /const\s+([A-Z0-9_]+):\s*&str\s*=\s*include_str!\(\s*"([^"]+)"\s*\)/g
    ),
  ].map((match) => [match[1], resolve(WEBSERVER, match[2])])
);

function embedded(name) {
  const file = EMBEDS.get(name);
  if (!file) throw new Error(`embeds.rs has no include_str! constant ${name}`);
  return read(file);
}

/** Body of the Rust item that starts at `marker`, up to the matching closing brace. */
function itemBody(source, marker) {
  const start = source.indexOf(marker);
  if (start < 0) throw new Error(`${marker} not found in templates.rs`);
  const open = source.indexOf("{", start);
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    else if (source[index] === "}" && --depth === 0) return source.slice(open + 1, index);
  }
  throw new Error(`${marker} is not closed`);
}

const stripComments = (code) => code.replace(/\/\/[^\n]*/g, "");
const identifiers = (list) => [...list.matchAll(/[A-Z][A-Z0-9_]+/g)].map((match) => match[0]);

const LUCIDE_URL_REWRITES = ["eot", "woff2", "woff", "ttf", "svg"];

function lucideCss() {
  return LUCIDE_URL_REWRITES.reduce(
    (css, ext) => css.replaceAll(`url('lucide.${ext}`, `url('/assets/fonts/lucide.${ext}`),
    embedded("LUCIDE_ICON_CSS")
  );
}

const globalStyles = memoized(() => {
  const list = /let combined_styles = \[([\s\S]*?)\n    \];/.exec(TEMPLATES_RS)?.[1];
  if (!list) throw new Error("combined_styles not found in templates.rs");
  return stripComments(list)
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => (entry === "&lucide_css" ? lucideCss() : embedded(entry)))
    .join("\n");
});

/** Page id -> stylesheet constants, from the `page_styles` match in templates.rs. */
function pageStyleManifest() {
  const body = stripComments(itemBody(TEMPLATES_RS, "pub fn page_styles"));
  const manifest = new Map();
  for (const match of body.matchAll(/"(\w+)"\s*=>\s*(\[[^\]]*\]|[A-Z][A-Z0-9_]+)/g)) {
    manifest.set(match[1], identifiers(match[2]));
  }
  return manifest;
}

const PAGE_STYLES = pageStyleManifest();

export const pageStyles = memoized((page) => {
  const names = PAGE_STYLES.get(page);
  return names ? names.map(embedded).join("\n") : null;
});

/** Tabs in navigation order, from `default_tabs()` in the config schema. */
function defaultTabs() {
  const source = read(resolve(REPO_ROOT, "src/config/schemas/gui.rs"));
  const body = itemBody(source, "pub fn default_tabs");
  return [
    ...body.matchAll(
      /id:\s*"(\w+)"\.into\(\),\s*icon:\s*"([\w-]+)"\.into\(\),\s*order:\s*(\d+),\s*enabled:\s*(true|false)/g
    ),
  ]
    .map((match) => ({
      id: match[1],
      icon: match[2],
      order: Number(match[3]),
      enabled: match[4] === "true",
    }))
    .filter((tab) => tab.enabled)
    .sort((a, b) => a.order - b.order);
}

export const TABS = defaultTabs();
export const PAGE_IDS = TABS.map((tab) => tab.id);

function navTabs(active) {
  return TABS.map((tab) => {
    const current = tab.id === active;
    return `<a href="/${tab.id}" data-page="${tab.id}" class="tab${current ? " active" : ""}"${current ? ' aria-current="page"' : ""}><i class="${tab.icon}"></i> <span data-l10n-id="nav-${tab.id}"></span></a>`;
  }).join("\n        ");
}

/** The fragment `/api/pages/{page}` returns: `templates::<page>_content()`. */
export function pageContent(page) {
  const file = resolve(TEMPLATES_ROOT, "pages", `${page}.html`);
  let html = read(file);
  if (page === "trader")
    html = html.replace(
      "{{STRATEGIES_PANEL}}",
      read(resolve(TEMPLATES_ROOT, "pages/strategies.html"))
    );
  return html;
}

function documentTitle(page, locale) {
  const title = formatMessage(`nav-page-title-${page}`, undefined, locale) ?? page;
  return formatMessage("shell-document-title", { page: title }, locale) ?? title;
}

/** The document `GET /{page}` returns for `locale`. */
export const renderShell = memoized((page, locale) => {
  const substitutions = {
    "{{TITLE}}": documentTitle(page, locale),
    "{{LANG}}": locale,
    "{{DIR}}": localeDirection(locale),
    "{{NAV_TABS}}": navTabs(page),
    "{{CONTENT}}": pageContent(page),
    "{{SECURITY_TOKEN}}": "",
    "{{WEBSERVER_PORT}}": SHELL_PORT,
    "{{IS_GUI_MODE}}": "false",
    "{{ASSET_VERSION}}": "test",
    "{{TOKEN_LOGO_SHAPE}}": "circle",
    "{{NEEDS_INITIALIZATION}}": "false",
    "{{SPLASH_SCREEN}}": embedded("SPLASH_PAGE"),
    "{{ONBOARDING_SCREEN}}": embedded("ONBOARDING_PAGE"),
    "{{SETUP_SCREEN}}": embedded("SETUP_PAGE"),
    "{{LOCKSCREEN}}": embedded("LOCKSCREEN_PAGE"),
    "{{ACTIVE_TAB}}": page,
  };
  let html = read(resolve(TEMPLATES_ROOT, "base.html"));
  html = html.replace("/*__GLOBAL_STYLES__*/", () => globalStyles());
  html = html.replace("/*__INITIAL_PAGE_STYLES__*/", () => pageStyles(page) ?? "");
  html = html.replace("/*__THEME_SCRIPTS__*/", () => embedded("THEME_SCRIPTS"));
  html = html.replace("<!--__PROMO_CAPTURE_SCRIPTS__-->", "");
  for (const [placeholder, value] of Object.entries(substitutions))
    html = html.replaceAll(placeholder, () => value);
  return localizeTemplate(html, locale);
});

/** `window.__SCREENERBOT_L10N__` for `locale`: the dashboard catalogs, least specific first. */
export const catalogScript = memoized((locale) => {
  const serverOnly = readServerOnlyDomains();
  const ftl = (code) => {
    const dir = resolve(REPO_ROOT, "locales", code);
    if (!existsSync(dir)) return null;
    return [
      "terms",
      ...readDomains(dir).filter((domain) => domain !== "terms" && !serverOnly.has(domain)),
    ]
      .map((domain) => `${read(resolve(dir, `${domain}.ftl`))}\n`)
      .join("");
  };
  const catalogs = [...new Set([SOURCE_LOCALE, locale])].map((code) => ({
    locale: code,
    ftl: ftl(code),
  }));
  const payload = {
    locale,
    intlLocale: `${locale}-u-nu-latn`,
    dir: localeDirection(locale),
    source: SOURCE_LOCALE,
    catalogs,
  };
  return `window.__SCREENERBOT_L10N__ = ${JSON.stringify(payload)};`;
});

function readDomains(dir) {
  return readdirSync(dir)
    .sort()
    .filter((name) => name.endsWith(".ftl"))
    .map((name) => name.slice(0, -4));
}

const localizedFragment = memoized((id, locale) => localizeTemplate(pageContent(id), locale));
const binaries = new Map();
const readBinary = (file) => {
  if (!binaries.has(file)) binaries.set(file, readFileSync(file));
  return binaries.get(file);
};

const CONTENT_TYPES = {
  ".js": "application/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".woff2": "font/woff2",
  ".woff": "font/woff",
  ".ttf": "font/ttf",
  ".eot": "application/vnd.ms-fontobject",
};

/** Static asset URL path -> source file, or null when the server would not serve it. */
function assetFile(pathname) {
  const scripts = /^\/scripts\/(core|pages|ui)\/(.+)$/.exec(pathname);
  if (scripts) return resolve(TEMPLATES_ROOT, "scripts", scripts[1], scripts[2]);
  if (pathname === "/assets/fluent-bundle.js" || pathname === "/assets/lightweight-charts.js")
    return resolve(WEBSERVER, pathname.slice(1));
  const fonts = /^\/assets\/fonts\/([^/]+)$/.exec(pathname);
  if (fonts) {
    const bundled = resolve(WEBSERVER, "assets/fonts", fonts[1]);
    return existsSync(bundled) ? bundled : resolve(WEBSERVER, "assets/lucide-font", fonts[1]);
  }
  const sub = /^\/assets\/(providers|solana)\/([^/]+)$/.exec(pathname);
  if (sub) return resolve(WEBSERVER, "assets", sub[1], sub[2]);
  const top = /^\/assets\/([^/]+)$/.exec(pathname);
  if (top) return resolve(WEBSERVER, "assets", top[1]);
  return null;
}

/**
 * Start a local HTTP server that answers like the dashboard's server and point
 * `context` at it; every other origin is blocked and recorded in `unserved`.
 *
 * `onApi({ method, url })` returns `{ status?, body, contentType? }` for an
 * `/api` request. A plain HTTP server keeps asset serving off the DevTools
 * protocol, which is what bounds the suite's runtime.
 */
export async function serveDashboard(context, { locale, onApi }) {
  const unserved = [];
  const server = createServer(async (request, response) => {
    const url = new URL(request.url, "http://localhost");
    const send = (status, contentType, body) => {
      response.writeHead(status, { "content-type": contentType, "cache-control": "no-store" });
      response.end(body);
    };
    const { pathname } = url;
    const page = pathname.replace(/^\//, "");
    if (
      request.headers["sec-fetch-dest"] === "document" &&
      (pathname === "/" || PAGE_IDS.includes(page))
    ) {
      return send(200, "text/html; charset=utf-8", renderShell(page || "home", locale));
    }
    if (pathname.startsWith("/api/pages/")) {
      const id = pathname.slice("/api/pages/".length);
      if (!PAGE_IDS.includes(id)) return send(404, "text/plain", "");
      return send(200, "text/html; charset=utf-8", localizedFragment(id, locale));
    }
    if (pathname.startsWith("/api/")) {
      const answer = await onApi({ method: request.method, url });
      return send(answer.status ?? 200, answer.contentType ?? "application/json", answer.body);
    }
    const catalog = /^\/i18n\/([^/]+)\/catalog\.js$/.exec(pathname);
    if (catalog) return send(200, CONTENT_TYPES[".js"], catalogScript(catalog[1]));
    const styles = /^\/styles\/pages\/([^/]+?)(?:\.css)?$/.exec(pathname);
    if (styles) {
      const css = pageStyles(styles[1]);
      return css === null ? send(404, "text/plain", "") : send(200, CONTENT_TYPES[".css"], css);
    }
    const file = assetFile(pathname);
    if (file && existsSync(file)) {
      return send(
        200,
        CONTENT_TYPES[extname(file)] ?? "application/octet-stream",
        readBinary(file)
      );
    }
    return send(404, "text/plain", "");
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  await context.route(
    (url) => url.origin !== origin,
    (route) => {
      unserved.push(route.request().url());
      return route.abort("blockedbyclient");
    }
  );
  return {
    origin,
    unserved,
    close: () =>
      new Promise((resolve) => {
        server.close(resolve);
        server.closeAllConnections();
      }),
  };
}
