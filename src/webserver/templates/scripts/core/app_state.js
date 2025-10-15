// State Manager - Browser Storage
export function save(key, value) {
  try {
    localStorage.setItem(`screenerbot_${key}`, JSON.stringify(value));
  } catch (e) {
    console.warn("Failed to save state:", key, e);
  }
}

export function load(key, defaultValue = null) {
  try {
    const item = localStorage.getItem(`screenerbot_${key}`);
    return item ? JSON.parse(item) : defaultValue;
  } catch (e) {
    console.warn("Failed to load state:", key, e);
    return defaultValue;
  }
}

export function remove(key) {
  try {
    localStorage.removeItem(`screenerbot_${key}`);
  } catch (e) {
    console.warn("Failed to remove state:", key, e);
  }
}

export function clearAll() {
  try {
    Object.keys(localStorage)
      .filter((key) => key.startsWith("screenerbot_"))
      .forEach((key) => localStorage.removeItem(key));
  } catch (e) {
    console.warn("Failed to clear state:", e);
  }
}
