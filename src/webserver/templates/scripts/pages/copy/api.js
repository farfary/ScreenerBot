// Copy Trading API client. Every failure carries the server's own explanation
// (`error.detail`) so the page can say why a request was refused instead of a
// generic "could not be saved".

const BASE = "/api/copy-trading";

/** Attach the server's error detail to a request failure. */
export async function withDetail(error) {
  if (error?.detail) return error;
  let detail = null;
  try {
    const body = await error?.response?.json();
    detail = body?.error?.details || body?.error?.message || null;
  } catch {
    // The body was not JSON or is already read; the status line is all there is.
  }
  const wrapped = error instanceof Error ? error : new Error(String(error));
  wrapped.detail = detail || wrapped.message || "Request failed";
  return wrapped;
}

export function createApi(requestManager) {
  async function call(path, { method = "GET", body, signal } = {}) {
    const write = method !== "GET";
    try {
      return await requestManager.fetch(`${BASE}${path}`, {
        method,
        ...(body !== undefined
          ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
          : {}),
        priority: write ? "high" : "normal",
        skipDedup: write,
        signal,
      });
    } catch (error) {
      throw await withDetail(error);
    }
  }

  const query = (params) => {
    const search = new URLSearchParams();
    Object.entries(params || {}).forEach(([key, value]) => {
      if (value !== null && value !== undefined && value !== "") search.set(key, String(value));
    });
    const text = search.toString();
    return text ? `?${text}` : "";
  };

  return {
    overview: () => call("/overview"),
    defaults: () => call("/defaults"),
    workspace: (id) => call(`/tasks/${id}/workspace`),
    activity: (id, params) => call(`/tasks/${id}/activity${query(params)}`),
    insights: (id, range) => call(`/tasks/${id}/insights${query(range)}`),
    compare: (range) => call(`/insights${query(range)}`),
    profile: (address) => call(`/wallets/${encodeURIComponent(address)}`),
    create: (task) => call("/tasks", { method: "POST", body: task }),
    update: (id, patch) => call(`/tasks/${id}`, { method: "PATCH", body: patch }),
    remove: (id) => call(`/tasks/${id}`, { method: "DELETE" }),
    setMode: (id, mode, confirmation) =>
      call(`/tasks/${id}/mode`, {
        method: "POST",
        body: confirmation ? { mode, confirmation } : { mode },
      }),
    reset: (id) => call(`/tasks/${id}/reset`, { method: "POST" }),
    closeHolding: (id, mint) =>
      call(`/tasks/${id}/holdings/${encodeURIComponent(mint)}/close`, { method: "POST" }),
    async config() {
      try {
        const response = await requestManager.fetch("/api/config/copy_trading");
        return response.copy_trading || response.data?.copy_trading || response.data || response;
      } catch (error) {
        throw await withDetail(error);
      }
    },
    async patchConfig(patch) {
      try {
        return await requestManager.fetch("/api/config/copy_trading", {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(patch),
          priority: "high",
          skipDedup: true,
        });
      } catch (error) {
        throw await withDetail(error);
      }
    },
  };
}
