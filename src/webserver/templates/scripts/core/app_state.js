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
    if (item === null) {
      return defaultValue;
    }

    try {
      return JSON.parse(item);
    } catch (parseError) {
      localStorage.removeItem(`screenerbot_${key}`);
      console.warn(
        "Invalid stored state removed:",
        key,
        "value=",
        item,
        parseError
      );
      return defaultValue;
    }
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
