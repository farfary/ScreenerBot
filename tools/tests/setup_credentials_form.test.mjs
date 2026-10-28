// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the first-run setup wizard and the Replace wallet dialog ask for the wallet
 * and RPC with the same cards, and a created wallet's key stays in memory.
 *
 * - Both mount the shared wallet and RPC cards, so their titles, placeholders, hints,
 *   reveal toggle and provider links read the same (the dialog adds only the note that
 *   an empty RPC field keeps the configured endpoints), the RPC hint says public
 *   endpoints are not supported, and the dialog's title icon is a bare glyph without a
 *   tile. The dialog sizes to its content: with no status to show, the RPC card is
 *   followed by the body's own padding and then the buttons, with no empty band.
 * - The wizard's wallet step preselects no source and Continue stays disabled until
 *   one is chosen; a created wallet needs all three backup acknowledgements.
 * - The generated private key is never written to localStorage, sessionStorage, the
 *   URL, history, a DOM attribute or the console, and is gone once the step is left.
 * - Choosing the ScreenerBot service marks the account Required and hides the
 *   data-access summary, whose "signing in is optional" reading holds only with an
 *   own RPC; choosing an own RPC restores both.
 * - The wizard follows the dashboard theme: in the light theme its header, footer
 *   and secondary buttons are light surfaces with dark text, never the dark chrome
 *   of the always-dark introduction.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");
const BOOTSTRAP = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/system_bootstrap.json`, "utf8"));
/** A synthetic 64-byte key in base58; it controls no wallet. */
const GENERATED = {
  address: "GenWa11et11111111111111111111111111111111111",
  private_key:
    "99uqd56BHxksGD8TMFFMLMQdt2aHjfQS8wDVYbbNT3kyFrMmSLrerhqhPwqCXwDWuGgKc5jGvRgQ9kASAn8t5LZ",
};
/** A signed-out account whose ScreenerBot data waits for a sign-in. */
const SIGNED_OUT = {
  signed_in: false,
  online: true,
  use_gateway_rpc: false,
  wallet_has_account: false,
  data_access: {
    state: "sign_in_required",
    available: false,
    actionable: true,
    text: { id: "account-data-access-signed-out" },
  },
};
const FIRST_RUN = {
  ...BOOTSTRAP,
  initialization_required: true,
  initialization_complete: false,
  onboarding_complete: false,
  ui_ready: false,
};

/** The visible text and attributes that make up a credential form under `root`. */
function readForm(root) {
  const text = (node) => (node ? node.textContent.replace(/\s+/g, " ").trim() : null);
  const form = document.querySelector(root);
  return {
    titles: [...form.querySelectorAll(".setup-card-title")].map(text),
    placeholders: [...form.querySelectorAll("textarea")].map((input) => input.placeholder),
    notes: [
      ...form.querySelectorAll(
        '.setup-security-note:not([id$="rpc-optional-note"]), .setup-rpc-guidance'
      ),
    ].map(text),
    toggles: form.querySelectorAll(".setup-input-toggle").length,
    links: [...form.querySelectorAll("a[href]")].map((link) => link.href),
  };
}

/** Opens the first-run setup wizard and returns the page. */
async function openWizard(t, { beforeLoad } = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([shell], "populated");
  const onApi = (request) => {
    const path = request.url.pathname;
    if (path === "/api/system/bootstrap") return { body: JSON.stringify(FIRST_RUN) };
    if (path === "/api/initialization/wallet/generate") {
      return { body: JSON.stringify(GENERATED) };
    }
    if (path === "/api/account/status") {
      return { body: JSON.stringify({ success: true, data: SIGNED_OUT }) };
    }
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  if (beforeLoad) await beforeLoad(page);
  await page.goto(`${host.origin}/`);
  await page.locator("#onboardingScreen").waitFor({ state: "visible" });
  await page.evaluate(() => {
    document.getElementById("setupScreenWrapper").style.display = "block";
    window.SetupController.init();
  });
  await page.waitForSelector("#setup-wallet-source .setup-choice-input");
  return page;
}

test("the wizard and the Replace wallet dialog render the same wallet and RPC cards", async (t) => {
  const page = await openWizard(t);
  await page.waitForFunction(
    () => document.querySelector("#setup-wallet-import .setup-card-title")?.textContent
  );
  const onboarding = {
    wallet: await page.evaluate(readForm, "#setup-wallet-import"),
    rpc: await page.evaluate(readForm, "#setup-rpc-field"),
  };

  // The dialog opens from the Wallets page, never over the full-screen wizard; its
  // fields follow the replacement's confirmation.
  await page.evaluate(async () => {
    document.getElementById("setupScreenWrapper").style.display = "none";
    document.getElementById("onboardingScreen").style.display = "none";
    const { SetupDialog } = await import("/scripts/ui/setup_dialog.js");
    void SetupDialog.show();
  });
  await page.click('.setup-dialog [data-action="submit"]');
  await page.check("#setup-dialog-wallet-source-import");
  await page.waitForFunction(
    () => document.querySelector(".setup-dialog-wallet .setup-card-title")?.textContent
  );
  const dialog = {
    wallet: await page.evaluate(readForm, ".setup-dialog-wallet"),
    rpc: await page.evaluate(readForm, ".setup-dialog-rpc"),
  };

  assert.deepEqual(dialog, onboarding);
  assert.equal(onboarding.wallet.titles.length, 1);
  assert.equal(onboarding.rpc.titles.length, 1);
  assert.equal(onboarding.wallet.toggles, 1);
  const notes = [...onboarding.wallet.notes, ...onboarding.rpc.notes];
  assert.ok(
    notes.some((note) => note.includes("public endpoints are not supported")),
    notes.join(" | ")
  );

  const ids = await page.evaluate(() =>
    [...document.querySelectorAll("textarea")].map((input) => input.id)
  );
  assert.equal(new Set(ids).size, ids.length, `credential ids collide: ${ids.join(", ")}`);

  const icon = await page.evaluate(() => {
    const glyph = document.querySelector(".setup-dialog-title i");
    const style = getComputedStyle(glyph);
    const parent = getComputedStyle(glyph.parentElement);
    return {
      inTitle: glyph.parentElement.classList.contains("setup-dialog-title"),
      background: style.backgroundColor,
      border: style.borderStyle,
      parentBackground: parent.backgroundColor,
    };
  });
  assert.equal(icon.inTitle, true, "the glyph sits inline in the title");
  assert.equal(icon.background, "rgba(0, 0, 0, 0)");
  assert.equal(icon.border, "none");
  assert.equal(icon.parentBackground, "rgba(0, 0, 0, 0)");

  // The dialog scales in; the gap is compared with unscaled padding once that ends. An
  // animation cancelled by a style change rejects `finished`, and has ended all the same.
  await page.locator(".setup-dialog").evaluate((node) =>
    Promise.all(
      node.getAnimations({ subtree: true }).map((animation) => animation.finished.catch(() => null))
    )
  );
  const band = await page.evaluate(() => {
    const body = document.querySelector(".setup-dialog-body");
    const fields = document.querySelector(".setup-dialog-fields").getBoundingClientRect();
    const footer = document.querySelector(".setup-dialog-footer").getBoundingClientRect();
    return {
      statusBoxes: document.querySelector("#setup-dialog-status").getClientRects().length,
      gap: Math.round(footer.top - fields.bottom),
      padding: Math.round(parseFloat(getComputedStyle(body).paddingBottom)),
    };
  });
  assert.equal(band.statusBoxes, 0, "an idle status line takes no space");
  assert.equal(band.gap, band.padding, "the buttons follow the RPC card after the body padding");
});

test("the wallet step preselects nothing and a created wallet needs three acknowledgements", async (t) => {
  const page = await openWizard(t);
  const next = page.locator("#setup-next");

  assert.equal(
    await page.evaluate(() => document.querySelectorAll("#setup-wallet-source :checked").length),
    0,
    "no wallet source is preselected"
  );
  assert.equal(await next.isDisabled(), true, "Continue waits for a wallet source");

  await page.check("#setup-wallet-source-create");
  assert.equal(await next.isDisabled(), false);
  await next.click();
  await page.waitForFunction(
    (address) => document.querySelector("#setup-wallet-backup")?.textContent.includes(address),
    GENERATED.address
  );

  const acks = page.locator("#setup-wallet-backup [data-ack]");
  assert.equal(await acks.count(), 3);
  for (let index = 0; index < 3; index += 1) {
    assert.equal(await next.isDisabled(), true, `Continue is disabled with ${index} of 3 ticks`);
    await acks.nth(index).check();
  }
  assert.equal(await next.isDisabled(), false, "all three ticks enable Continue");
});

test("the ScreenerBot service makes the account required and drops the optional reading", async (t) => {
  const page = await openWizard(t);
  const next = page.locator("#setup-next");

  await page.check("#setup-wallet-source-create");
  await next.click();
  const acks = page.locator("#setup-wallet-backup [data-ack]");
  for (let index = 0; index < 3; index += 1) await acks.nth(index).check();
  await next.click();
  await page.locator("#setup-connection-service").waitFor({ state: "visible" });
  await page.locator("#setupAccountPanel .account-data").waitFor({ state: "visible" });

  const read = () =>
    page.evaluate(() => ({
      badge: document.getElementById("setup-account-requirement").textContent.trim(),
      summary: !document.querySelector("#setupAccountPanel .account-data").hidden,
    }));
  assert.deepEqual(await read(), { badge: "Optional", summary: true }, "nothing chosen yet");

  await page.check("#setup-connection-service");
  assert.deepEqual(await read(), { badge: "Required", summary: false });
  assert.equal(await next.isDisabled(), true, "the service waits for a sign-in");

  await page.check("#setup-connection-own");
  assert.deepEqual(await read(), { badge: "Optional", summary: true });
});

test("the wizard follows the light theme", async (t) => {
  const page = await openWizard(t);
  await page.evaluate(() => document.documentElement.setAttribute("data-theme", "light"));
  // Colours read mid-transition are interpolated values; read the settled theme.
  await page.waitForFunction(() =>
    document
      .getAnimations()
      .every(
        (animation) =>
          !(animation instanceof window.CSSTransition) || animation.playState !== "running"
      )
  );

  const lightness = await page.evaluate(() => {
    // Mean sRGB channel of a computed colour, from 0 (black) to 1 (white).
    const mean = (value) => {
      const mixed = value.match(/color\(srgb ([\d.]+) ([\d.]+) ([\d.]+)/);
      if (mixed)
        return (
          mixed
            .slice(1, 4)
            .map(Number)
            .reduce((a, b) => a + b) / 3
        );
      const rgb = value.match(/rgba?\((\d+), (\d+), (\d+)/);
      return (
        rgb
          .slice(1, 4)
          .map(Number)
          .reduce((a, b) => a + b) /
        3 /
        255
      );
    };
    const style = (selector) => getComputedStyle(document.querySelector(selector));
    return {
      header: mean(style(".setup-header").backgroundColor),
      brand: mean(style(".setup-header .onboarding-brand").color),
      footer: mean(style(".setup-footer").backgroundColor),
      back: mean(style("#setup-back").color),
    };
  });
  assert.ok(lightness.header > 0.5, `the header is a light surface (${lightness.header})`);
  assert.ok(lightness.footer > 0.5, `the footer is a light surface (${lightness.footer})`);
  assert.ok(lightness.brand < 0.5, `the brand reads dark on it (${lightness.brand})`);
  assert.ok(lightness.back < 0.5, `Back reads dark on it (${lightness.back})`);
});

test("the generated private key stays in memory, and setup reopens on an unchosen source", async (t) => {
  const page = await openWizard(t, {
    beforeLoad: (page) =>
      page.addInitScript(() => {
        const writes = [];
        window.__keyWrites = writes;
        for (const store of [window.localStorage, window.sessionStorage]) {
          const setItem = store.setItem.bind(store);
          store.setItem = (name, value) => {
            writes.push(`${name}=${value}`);
            setItem(name, value);
          };
        }
        for (const method of ["pushState", "replaceState"]) {
          const original = window.history[method].bind(window.history);
          window.history[method] = (state, title, url) => {
            writes.push(`${JSON.stringify(state)} ${url ?? ""}`);
            original(state, title, url);
          };
        }
      }),
  });
  const logs = [];
  page.on("console", (message) => logs.push(message.text()));

  await page.check("#setup-wallet-source-create");
  await page.locator("#setup-next").click();
  await page.waitForFunction(
    (address) => document.querySelector("#setup-wallet-backup")?.textContent.includes(address),
    GENERATED.address
  );
  // Reveal the key once, so the check covers the revealed state as well.
  await page.click('#setup-wallet-backup [data-action="toggle-key"]');
  await page.waitForFunction(
    (key) => document.querySelector("#setup-wallet-backup").textContent.includes(key),
    GENERATED.private_key
  );

  const leaks = (key) => ({
    writes: window.__keyWrites.filter((entry) => entry.includes(key)),
    url: window.location.href.includes(key),
    history: JSON.stringify(window.history.state ?? null).includes(key),
    attributes: [...document.querySelectorAll("*")].flatMap((node) =>
      [...node.attributes]
        .filter((attribute) => attribute.value.includes(key))
        .map((attribute) => `${node.tagName.toLowerCase()}[${attribute.name}]`)
    ),
    storage: [window.localStorage, window.sessionStorage].some((store) =>
      Object.keys(store).some((name) => String(store.getItem(name)).includes(key))
    ),
  });
  const clean = { writes: [], url: false, history: false, attributes: [], storage: false };
  assert.deepEqual(await page.evaluate(leaks, GENERATED.private_key), clean);

  // Leaving the detail view for the choice and then to onboarding drops the key.
  await page.locator("#setup-back").click();
  await page.locator("#setup-back").click();
  assert.equal(
    await page.evaluate((key) => document.body.textContent.includes(key), GENERATED.private_key),
    false,
    "the key is cleared once the wallet step is left"
  );
  assert.deepEqual(await page.evaluate(leaks, GENERATED.private_key), clean);
  assert.deepEqual(
    logs.filter((line) => line.includes(GENERATED.private_key)),
    [],
    "the key never reaches the console"
  );

  // Setup reopens on an unchosen source, and creating again shows a wallet to back up.
  await page.evaluate(() => window.OnboardingController.complete());
  await page.locator("#setup-wallet-source").waitFor({ state: "visible" });
  assert.equal(
    await page.evaluate(() => document.querySelectorAll("#setup-wallet-source :checked").length),
    0
  );
  assert.equal(await page.locator("#setup-next").isDisabled(), true);
  await page.check("#setup-wallet-source-create");
  await page.locator("#setup-next").click();
  await page.waitForFunction(
    (address) => document.querySelector("#setup-wallet-backup")?.textContent.includes(address),
    GENERATED.address
  );
});
