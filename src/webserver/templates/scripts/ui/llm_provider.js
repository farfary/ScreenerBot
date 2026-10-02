// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Presentation for the LLM providers, in one place.
 *
 * Ids are the `Provider::as_str` values of src/apis/llm/mod.rs. Provider names
 * are terms, so every locale renders the same name.
 */

/** Message key of each provider name. */
export const LLM_PROVIDER_LABELS = Object.freeze({
  openai: "assistant-provider-openai",
  anthropic: "assistant-provider-anthropic",
  groq: "assistant-provider-groq",
  deepseek: "assistant-provider-deepseek",
  gemini: "assistant-provider-gemini",
  ollama: "assistant-provider-ollama",
  together: "assistant-provider-together",
  openrouter: "assistant-provider-openrouter",
  mistral: "assistant-provider-mistral",
});
