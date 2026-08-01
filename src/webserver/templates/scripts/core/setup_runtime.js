// Stateless setup validation, requests, and restart polling shared by the
// full-screen controller and the Preview Mode setup dialog.
(function () {
  "use strict";

  const PUBLIC_SOLANA_RPC = "api.mainnet-beta.solana.com";

  function isPrivateHostname(hostname) {
    const host = hostname.toLowerCase().replace(/^\[|\]$/g, "");
    if (
      host === "localhost" ||
      host.endsWith(".localhost") ||
      host.endsWith(".local") ||
      host.endsWith(".internal") ||
      host === "::1"
    ) {
      return true;
    }

    const octets = host.split(".").map(Number);
    if (octets.length !== 4 || octets.some((octet) => !Number.isInteger(octet))) return false;
    return (
      octets[0] === 0 ||
      octets[0] === 10 ||
      octets[0] === 127 ||
      (octets[0] === 169 && octets[1] === 254) ||
      (octets[0] === 172 && octets[1] >= 16 && octets[1] <= 31) ||
      (octets[0] === 192 && octets[1] === 168)
    );
  }

  function parseRpcUrls(value) {
    return String(value || "")
      .split(/\r?\n/)
      .map((url) => url.trim())
      .filter(Boolean);
  }

  function validateWalletValue(value, required) {
    const key = String(value || "").trim();
    if (!key) {
      return {
        valid: false,
        state: required ? "error" : "",
        message: required ? "Enter a wallet private key." : "",
      };
    }

    if (key.startsWith("[") && key.endsWith("]")) {
      try {
        const bytes = JSON.parse(key);
        const validBytes =
          Array.isArray(bytes) &&
          bytes.length === 64 &&
          bytes.every((byte) => Number.isInteger(byte) && byte >= 0 && byte <= 255);
        if (!validBytes) throw new Error("invalid bytes");
        return {
          valid: true,
          state: "success",
          message: "64-byte JSON key format recognized.",
        };
      } catch {
        return {
          valid: false,
          state: "error",
          message: "Use a JSON array containing exactly 64 byte values (0–255).",
        };
      }
    }

    if (!/^[1-9A-HJ-NP-Za-km-z]{80,90}$/.test(key)) {
      return {
        valid: false,
        state: "error",
        message: "Use a base58 private key or a 64-byte JSON array.",
      };
    }

    return {
      valid: true,
      state: "success",
      message: "Base58 key format recognized.",
    };
  }

  function validateRpcValue(value, required) {
    const urls = parseRpcUrls(value);
    const fail = (message) => ({ valid: false, state: "error", message, urls });

    if (!urls.length) {
      return {
        valid: false,
        state: required ? "error" : "",
        message: required ? "Enter at least one RPC endpoint." : "",
        urls,
      };
    }
    if (urls.length > 10) return fail("Use no more than 10 RPC endpoints.");

    const normalized = new Set();
    for (const value of urls) {
      let parsed;
      try {
        parsed = new URL(value);
      } catch {
        return fail("Every endpoint must be a valid HTTPS URL.");
      }

      if (parsed.protocol !== "https:" || !parsed.hostname) {
        return fail("Every endpoint must be a valid HTTPS URL.");
      }
      if (parsed.username || parsed.password) {
        return fail("RPC URLs cannot include usernames or passwords.");
      }
      if (parsed.hash) return fail("RPC URLs cannot include fragments.");
      if (parsed.hostname.toLowerCase() === PUBLIC_SOLANA_RPC) {
        return fail("The public Solana RPC cannot support continuous polling.");
      }
      if (isPrivateHostname(parsed.hostname)) {
        return fail("RPC endpoints cannot use local or private network hosts.");
      }

      const key = parsed.href.replace(/\/$/, "").toLowerCase();
      if (normalized.has(key)) return fail("Remove duplicate RPC endpoints.");
      normalized.add(key);
    }

    return {
      valid: true,
      state: "success",
      message: `${urls.length} HTTPS endpoint${urls.length === 1 ? "" : "s"} ready to test.`,
      urls,
    };
  }

  function summarizeValidation(validation) {
    const walletValid = Boolean(validation?.wallet_address);
    const wallet = {
      state: walletValid ? "success" : "error",
      label: walletValid ? "Wallet verified" : "Wallet could not be verified",
      details: walletValid
        ? `Address ${validation.wallet_address}`
        : "Check the private key format.",
      address: walletValid ? validation.wallet_address : null,
    };

    const results = Array.isArray(validation?.rpc_test_results) ? validation.rpc_test_results : [];
    const working = results.filter((result) => result.success);
    const failed = results.filter((result) => !result.success);
    let rpc;

    if (!working.length) {
      rpc = {
        state: "error",
        label: "No working mainnet RPC",
        details:
          failed[0]?.error ||
          validation?.errors?.find((message) => /rpc|endpoint|https|mainnet/i.test(message)) ||
          "No endpoint passed the mainnet health checks.",
      };
    } else {
      const fastest = working.reduce((best, current) =>
        current.latency_ms < best.latency_ms ? current : best
      );
      const warning = Boolean(failed.length || validation?.warnings?.length);
      rpc = {
        state: warning ? "warning" : "success",
        label: warning
          ? `${working.length} working; ${failed.length} unavailable`
          : `${working.length} mainnet endpoint${working.length === 1 ? "" : "s"} verified`,
        details: `Fastest: ${fastest.display_url} (${fastest.latency_ms} ms).`,
      };
    }

    return { rpc, wallet };
  }

  async function requestJson(path, options = {}) {
    const response = await fetch(path, options);
    let body = null;
    try {
      body = await response.json();
    } catch {
      body = null;
    }

    if (!response.ok) {
      throw new Error(
        body?.error?.message || body?.message || `Request failed (${response.status})`
      );
    }
    return body?.data ?? body;
  }

  function delay(milliseconds, signal) {
    return new Promise((resolve, reject) => {
      if (signal?.aborted) {
        reject(new DOMException("Aborted", "AbortError"));
        return;
      }

      const onAbort = () => {
        window.clearTimeout(timeout);
        reject(new DOMException("Aborted", "AbortError"));
      };
      const timeout = window.setTimeout(() => {
        signal?.removeEventListener("abort", onAbort);
        resolve();
      }, milliseconds);
      signal?.addEventListener("abort", onAbort, { once: true });
    });
  }

  async function waitForScreenerBotRestart(previousInstanceId, options = {}) {
    const target = options.target || "/home";
    const timeoutMs = options.timeoutMs || 120000;
    const startedAt = Date.now();

    while (Date.now() - startedAt < timeoutMs) {
      if (options.signal?.aborted) throw new DOMException("Aborted", "AbortError");

      try {
        const response = await fetch(`/api/health?restart_check=${Date.now()}`, {
          cache: "no-store",
          signal: options.signal,
        });
        if (response.ok) {
          const health = await response.json();
          if (health.instance_id && health.instance_id !== previousInstanceId) {
            options.onReady?.();
            window.location.replace(target);
            return;
          }
        }
      } catch (error) {
        if (error?.name === "AbortError") throw error;
      }

      options.onWaiting?.(Date.now() - startedAt);
      await delay(500, options.signal);
    }

    throw new Error("Setup is saved, but ScreenerBot has not reconnected yet.");
  }

  window.SetupRuntime = {
    parseRpcUrls,
    requestJson,
    summarizeValidation,
    validateRpcValue,
    validateWalletValue,
    waitForScreenerBotRestart,
  };
  window.waitForScreenerBotRestart = waitForScreenerBotRestart;
})();
