export function normalizeThemePreference(value) {
  return value === "light" || value === "dark" ? value : "system";
}

export function applyThemePreference(root, preference) {
  const theme = normalizeThemePreference(preference);
  if (theme === "system") {
    delete root.dataset.theme;
    root.style.colorScheme = "light dark";
  } else {
    root.dataset.theme = theme;
    root.style.colorScheme = theme;
  }
  return theme;
}

export async function loadThemePreference(readPreference) {
  try {
    return { preference: normalizeThemePreference(await readPreference()), message: "" };
  } catch (reason) {
    return { preference: "system", message: `主题设置加载失败：${String(reason)}` };
  }
}