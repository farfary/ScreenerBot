// Minimal `I18n` global for tests that import dashboard modules depending on
// core/format.js outside a browser. Formatting behaviour is covered by
// format_golden.test.mjs with the real runtime and catalogs.
globalThis.I18n = {
  intlLocale: "en-US",
  registerArgFormatter() {},
  t: (id) => id,
};
