// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// ESLint flat config for the dashboard scripts - browser globals, lint rules and a module-scope override for the advanced chart framing helper.

import js from "@eslint/js";

export default [
  js.configs.recommended,
  {
    files: ["src/webserver/templates/**/*.js"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: {
        console: "readonly",
        window: "readonly",
        document: "readonly",
        fetch: "readonly",
        localStorage: "readonly",
        // Timers
        clearTimeout: "readonly",
        setTimeout: "readonly",
        setInterval: "readonly",
        clearInterval: "readonly",
        requestAnimationFrame: "readonly",
        cancelAnimationFrame: "readonly",
        // Web platform APIs
        URL: "readonly",
        URLSearchParams: "readonly",
        WebSocket: "readonly",
        Event: "readonly",
        CustomEvent: "readonly",
        Headers: "readonly",
        AbortController: "readonly",
        DOMException: "readonly",
        navigator: "readonly",
        CSS: "readonly",
        Blob: "readonly",
        FormData: "readonly",
        performance: "readonly",
        getComputedStyle: "readonly",
        // Event constructors
        PointerEvent: "readonly",
        MouseEvent: "readonly",
        // Media (demo capture: music, narration, window recording)
        Audio: "readonly",
        MediaStream: "readonly",
        MediaRecorder: "readonly",
        // DOM types (used in type checks / instanceof)
        Node: "readonly",
        HTMLElement: "readonly",
        Element: "readonly",
        HTMLTableElement: "readonly",
        HTMLInputElement: "readonly",
        HTMLTextAreaElement: "readonly",
        HTMLSelectElement: "readonly",
        ResizeObserver: "readonly",
        MutationObserver: "readonly",
        // App-specific globals that some legacy files reference
        Router: "readonly",
        I18n: "readonly",
        FluentBundle: "readonly",
      },
    },
    rules: {
      "no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
      "no-console": "off",
      "no-undef": "error",
      "no-redeclare": "error",
      "no-dupe-keys": "error",
      "no-useless-assignment": "off",
      "preserve-caught-error": "off",
      semi: ["error", "always"],
      quotes: ["warn", "double", { avoidEscape: true, allowTemplateLiterals: true }],
    },
  },
  {
    files: ["src/webserver/templates/scripts/ui/advanced_chart/framing.js"],
    languageOptions: {
      globals: { module: "readonly" },
    },
  },
];
