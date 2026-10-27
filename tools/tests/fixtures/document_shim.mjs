// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Minimal `window` and `document` globals for Node tests of dashboard modules:
// core/utils.js registers document listeners when imported, and the escape helpers
// set an element's `textContent` and read back its `innerHTML`. Importing this file
// installs them; import it before the dashboard module under test.
globalThis.window = globalThis.window || { addEventListener() {} };
globalThis.document = {
  addEventListener() {},
  readyState: "complete",
  createElement() {
    let text = "";
    return {
      set textContent(value) {
        text = String(value);
      },
      get innerHTML() {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
      },
    };
  },
};
