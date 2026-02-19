# AI Backend Critical Fixes - Quick Reference

## 🔴 ISSUE #1: Initialize LLM Manager

**Location:** `src/run.rs` after line 247

**Add this code:**

```rust
// ============================================================================
// Initialize LLM Manager with configured providers
// ============================================================================
logger::info(LogTag::System, "Initializing LLM manager...");

use crate::apis::llm::{init_llm_manager, LlmManager};
use std::sync::Arc;

let mut llm_manager = LlmManager::new();
let mut enabled_providers = Vec::new();

with_config(|cfg| {
    // OpenAI
    if cfg.ai.providers.openai.enabled && !cfg.ai.providers.openai.api_key.is_empty() {
        use crate::apis::llm::openai::OpenAiClient;
        let client = OpenAiClient::new(
            cfg.ai.providers.openai.api_key.clone(),
            cfg.ai.providers.openai.rate_limit_per_minute,
        );
        llm_manager.set_openai(Arc::new(client));
        enabled_providers.push("OpenAI");
    }

    // Anthropic
    if cfg.ai.providers.anthropic.enabled && !cfg.ai.providers.anthropic.api_key.is_empty() {
        use crate::apis::llm::anthropic::AnthropicClient;
        let client = AnthropicClient::new(
            cfg.ai.providers.anthropic.api_key.clone(),
            cfg.ai.providers.anthropic.rate_limit_per_minute,
        );
        llm_manager.set_anthropic(Arc::new(client));
        enabled_providers.push("Anthropic");
    }

    // Groq
    if cfg.ai.providers.groq.enabled && !cfg.ai.providers.groq.api_key.is_empty() {
        use crate::apis::llm::groq::GroqClient;
        let client = GroqClient::new(
            cfg.ai.providers.groq.api_key.clone(),
            cfg.ai.providers.groq.rate_limit_per_minute,
        );
        llm_manager.set_groq(Arc::new(client));
        enabled_providers.push("Groq");
    }

    // DeepSeek
    if cfg.ai.providers.deepseek.enabled && !cfg.ai.providers.deepseek.api_key.is_empty() {
        use crate::apis::llm::deepseek::DeepSeekClient;
        let client = DeepSeekClient::new(
            cfg.ai.providers.deepseek.api_key.clone(),
            cfg.ai.providers.deepseek.rate_limit_per_minute,
        );
        llm_manager.set_deepseek(Arc::new(client));
        enabled_providers.push("DeepSeek");
    }

    // Gemini
    if cfg.ai.providers.gemini.enabled && !cfg.ai.providers.gemini.api_key.is_empty() {
        use crate::apis::llm::gemini::GeminiClient;
        let client = GeminiClient::new(
            cfg.ai.providers.gemini.api_key.clone(),
            cfg.ai.providers.gemini.rate_limit_per_minute,
        );
        llm_manager.set_gemini(Arc::new(client));
        enabled_providers.push("Gemini");
    }

    // Ollama (local, no API key needed)
    if cfg.ai.providers.ollama.enabled {
        use crate::apis::llm::ollama::OllamaClient;
        let client = OllamaClient::new(
            cfg.ai.providers.ollama.base_url.clone(),
            cfg.ai.providers.ollama.rate_limit_per_minute,
        );
        llm_manager.set_ollama(Arc::new(client));
        enabled_providers.push("Ollama");
    }

    // Together AI
    if cfg.ai.providers.together.enabled && !cfg.ai.providers.together.api_key.is_empty() {
        use crate::apis::llm::together::TogetherClient;
        let client = TogetherClient::new(
            cfg.ai.providers.together.api_key.clone(),
            cfg.ai.providers.together.rate_limit_per_minute,
        );
        llm_manager.set_together(Arc::new(client));
        enabled_providers.push("Together AI");
    }

    // OpenRouter
    if cfg.ai.providers.openrouter.enabled && !cfg.ai.providers.openrouter.api_key.is_empty() {
        use crate::apis::llm::openrouter::OpenRouterClient;
        let client = OpenRouterClient::new(
            cfg.ai.providers.openrouter.api_key.clone(),
            cfg.ai.providers.openrouter.rate_limit_per_minute,
        );
        llm_manager.set_openrouter(Arc::new(client));
        enabled_providers.push("OpenRouter");
    }

    // Mistral
    if cfg.ai.providers.mistral.enabled && !cfg.ai.providers.mistral.api_key.is_empty() {
        use crate::apis::llm::mistral::MistralClient;
        let client = MistralClient::new(
            cfg.ai.providers.mistral.api_key.clone(),
            cfg.ai.providers.mistral.rate_limit_per_minute,
        );
        llm_manager.set_mistral(Arc::new(client));
        enabled_providers.push("Mistral");
    }

    // Fireworks
    if cfg.ai.providers.fireworks.enabled && !cfg.ai.providers.fireworks.api_key.is_empty() {
        use crate::apis::llm::fireworks::FireworksClient;
        let client = FireworksClient::new(
            cfg.ai.providers.fireworks.api_key.clone(),
            cfg.ai.providers.fireworks.rate_limit_per_minute,
        );
        llm_manager.set_fireworks(Arc::new(client));
        enabled_providers.push("Fireworks");
    }
});

init_llm_manager(llm_manager)
    .await
    .map_err(|e| format!("Failed to initialize LLM manager: {}", e))?;

if enabled_providers.is_empty() {
    logger::info(LogTag::System, "LLM manager initialized (no providers enabled)");
} else {
    logger::info(
        LogTag::System,
        &format!(
            "LLM manager initialized successfully with {} provider(s): {}",
            enabled_providers.len(),
            enabled_providers.join(", ")
        ),
    );
}
```

---

## 🔴 ISSUE #2: Set AI Engine in AppState

**Location:** `src/webserver/server.rs` line 187

**Replace:**

```rust
// Create application state
let state = Arc::new(AppState::new());
```

**With:**

```rust
// Create application state with AI engine (if enabled)
use crate::config::with_config;

let ai_engine = if with_config(|cfg| cfg.ai.enabled) {
    use crate::ai::AiEngine;
    logger::debug(LogTag::Webserver, "Creating AI engine for AppState");
    Some(Arc::new(AiEngine::new()))
} else {
    logger::debug(LogTag::Webserver, "AI disabled, AppState created without AI engine");
    None
};

let state = Arc::new(AppState::with_ai_engine(ai_engine));
```

---

## 🔴 ISSUE #3: Create Global AI Engine Singleton

**Step 1:** Add to `src/ai/engine.rs` after line 26

```rust
use tokio::sync::OnceCell;

/// Global AI engine singleton
static AI_ENGINE: OnceCell<Arc<AiEngine>> = OnceCell::const_new();

/// Initialize the global AI engine
pub async fn init_ai_engine() -> Result<(), String> {
    let engine = AiEngine::new();
    AI_ENGINE
        .set(Arc::new(engine))
        .map_err(|_| "AI engine already initialized".to_string())
}

/// Get the global AI engine
pub fn get_ai_engine() -> Arc<AiEngine> {
    AI_ENGINE
        .get()
        .expect("AI engine not initialized - call init_ai_engine() first")
        .clone()
}

/// Try to get the global AI engine (non-panicking version)
pub fn try_get_ai_engine() -> Option<Arc<AiEngine>> {
    AI_ENGINE.get().cloned()
}
```

**Step 2:** Call init in `src/run.rs` before line 236 (before strategy system init)

```rust
// Initialize global AI engine if enabled
if with_config(|cfg| cfg.ai.enabled) {
    crate::ai::engine::init_ai_engine()
        .await
        .map_err(|e| format!("Failed to initialize AI engine: {}", e))?;
    logger::info(LogTag::System, "Global AI engine initialized");
}
```

**Step 3:** Update call sites

**In `src/filtering/sources/ai.rs` line 33:**

```rust
// Replace:
let ai_engine = AiEngine::new();

// With:
use crate::ai::engine::get_ai_engine;
let ai_engine = get_ai_engine();
```

**In `src/trader/ai_analysis.rs` line 70:**

```rust
// Replace:
let ai_engine = AiEngine::new();

// With:
use crate::ai::engine::get_ai_engine;
let ai_engine = get_ai_engine();
```

**In `src/trader/ai_analysis.rs` line 120:**

```rust
// Replace:
let ai_engine = AiEngine::new();

// With:
use crate::ai::engine::get_ai_engine;
let ai_engine = get_ai_engine();
```

**In `src/services/implementations/ai_service.rs` line 49:**

```rust
// Replace:
let engine = AiEngine::new();
self.ai_engine = Some(Arc::new(engine));

// With:
use crate::ai::engine::try_get_ai_engine;
if let Some(engine) = try_get_ai_engine() {
    self.ai_engine = Some(engine);
} else {
    return Err("AI engine not initialized".to_string());
}
```

**In `src/webserver/server.rs` the AppState fix above, change to:**

```rust
let ai_engine = if with_config(|cfg| cfg.ai.enabled) {
    use crate::ai::engine::try_get_ai_engine;
    try_get_ai_engine()
} else {
    None
};
```

---

## 🔴 ISSUE #4: Integrate AI into Filtering Pipeline

**Action Required:** Find the filtering pipeline file (likely `src/filtering/mod.rs` or `src/filtering/pipeline.rs`)

**Add AI evaluation step AFTER all other checks:**

```rust
// After DexScreener, GeckoTerminal, Rugcheck checks
if let Err(reason) = crate::filtering::sources::ai::evaluate(&token).await {
    return Err(reason);
}
```

**Note:** I need to see the filtering pipeline to provide exact location.

---

## 🔴 ISSUE #5: Integrate AI into Trader

**Action Required:** Find trader buy/sell decision logic

**For Entry (Buy):**

```rust
// In trader buy decision flow
use crate::trader::ai_analysis::{should_analyze_entry, analyze_entry};

if should_analyze_entry() {
    if let Some(analysis) = analyze_entry(&token).await {
        logger::info(
            LogTag::Trader,
            &format!(
                "AI entry analysis for {}: {} ({}% confidence) - {}",
                token.symbol,
                if analysis.should_enter { "BUY" } else { "SKIP" },
                analysis.confidence,
                analysis.reasoning
            ),
        );

        if !analysis.should_enter {
            logger::info(
                LogTag::Trader,
                &format!("Skipping {} - AI recommends against entry", token.symbol),
            );
            return; // Skip this token
        }
    }
}
```

**For Exit (Sell):**

```rust
// In trader sell decision flow
use crate::trader::ai_analysis::{should_analyze_exit, analyze_exit, ExitAction};

if should_analyze_exit() {
    if let Some(analysis) = analyze_exit(&position, &token).await {
        logger::info(
            LogTag::Trader,
            &format!(
                "AI exit analysis for {}: {:?} ({}% confidence) - {}",
                position.symbol,
                analysis.action,
                analysis.confidence,
                analysis.reasoning
            ),
        );

        match analysis.action {
            ExitAction::Exit if analysis.confidence >= 70 => {
                // Execute sell
            }
            ExitAction::PartialExit if analysis.confidence >= 60 => {
                // Execute partial sell
            }
            _ => {
                // Hold
            }
        }
    }
}
```

**Note:** I need to see the trader implementation to provide exact location.

---

## 🔴 ISSUE #6: Fix Config Cache TTL Propagation

**Location:** `src/ai/cache.rs`

**Find the `get` method and update it:**

```rust
pub fn get(&self, mint: &str, priority: Priority) -> Option<AiDecision> {
    let cache = self.cache.read().unwrap();

    if let Some(entry) = cache.get(mint) {
        // Read TTL from config dynamically (allows runtime config updates)
        use crate::config::with_config;
        let ttl = with_config(|cfg| cfg.ai.cache_ttl_seconds);

        if entry.timestamp.elapsed().as_secs() < ttl {
            return Some(entry.decision.clone());
        }
    }
    None
}
```

**Remove `ttl_seconds` field from `AiCache` struct** (no longer needed)

---

## ✅ Verification Checklist

After applying all fixes:

1. Run `cargo check --lib` - should compile ✅
2. Run `cargo clippy` - should have no critical warnings
3. Start the bot with AI enabled
4. Check logs for "LLM manager initialized successfully"
5. Check logs for "Global AI engine initialized"
6. Test `/api/ai/status` endpoint - should show providers
7. Test `/api/ai/test/evaluate` endpoint - should work
8. Test filtering with AI enabled - should see AI decisions in logs
9. Test trading with AI enabled - should see AI analysis in logs

---

## 📊 Impact Summary

**Before Fixes:**

- ❌ All AI features crash with "LLM manager not initialized"
- ❌ API endpoints return 500 errors
- ❌ Cache not shared across integrations
- ❌ AI filtering never runs
- ❌ AI trading analysis never runs
- ❌ Config changes not propagated

**After Fixes:**

- ✅ All AI features functional
- ✅ API endpoints working
- ✅ Shared cache across all integrations
- ✅ AI filtering integrated
- ✅ AI trading integrated
- ✅ Dynamic config updates

---

## 🚀 Deployment Order

1. Apply Issue #1 (LLM manager init) - REQUIRED
2. Apply Issue #3 (Global AI engine) - REQUIRED
3. Apply Issue #2 (AppState) - REQUIRED
4. Apply Issue #6 (Config cache TTL) - RECOMMENDED
5. Apply Issue #4 (Filtering integration) - AS NEEDED
6. Apply Issue #5 (Trader integration) - AS NEEDED

**Minimum for basic functionality:** Issues #1, #2, #3
**For production:** All issues #1-#6

---

**End of Critical Fixes Guide**
