import test from "node:test";
import assert from "node:assert/strict";
import { applyThemePreference, loadThemePreference } from "../src/lib/theme.js";

function rootElement() {
  return { dataset: {}, style: {} };
}

test("light preference is applied explicitly", () => {
  const root = rootElement();
  assert.equal(applyThemePreference(root, "light"), "light");
  assert.equal(root.dataset.theme, "light");
  assert.equal(root.style.colorScheme, "light");
});

test("dark preference is applied explicitly", () => {
  const root = rootElement();
  assert.equal(applyThemePreference(root, "dark"), "dark");
  assert.equal(root.dataset.theme, "dark");
  assert.equal(root.style.colorScheme, "dark");
});

test("system preference removes explicit theme and follows the system", () => {
  const root = rootElement();
  root.dataset.theme = "dark";
  assert.equal(applyThemePreference(root, "system"), "system");
  assert.equal("theme" in root.dataset, false);
  assert.equal(root.style.colorScheme, "light dark");
});

test("preference read failure falls back to system and retains a message", async () => {
  const result = await loadThemePreference(() => Promise.reject(new Error("read failed")));
  assert.equal(result.preference, "system");
  assert.match(result.message, /read failed/);
  const root = rootElement();
  root.dataset.theme = "dark";
  assert.equal(applyThemePreference(root, result.preference), "system");
  assert.equal("theme" in root.dataset, false);
  assert.equal(root.style.colorScheme, "light dark");
});

test("unknown persisted values normalize to system", async () => {
  assert.deepEqual(await loadThemePreference(() => Promise.resolve("sepia")), {
    preference: "system",
    message: "",
  });
});