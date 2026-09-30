import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const desktopRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const distRoot = join(desktopRoot, "dist");

function assetReferences(index) {
  return [...index.matchAll(/(?:src|href)="([^"]+)"/g)]
    .map((match) => match[1])
    .filter((value) => value.endsWith(".js") || value.endsWith(".css"));
}

test("production index references existing local JS and CSS assets", () => {
  const indexPath = join(distRoot, "index.html");
  const index = readFileSync(indexPath, "utf8");
  const references = assetReferences(index);
  assert.ok(references.length >= 2, "production index must reference JS and CSS assets");
  for (const reference of references) {
    assert.match(reference, /^(?:\.\/|\/)?assets\/[A-Za-z0-9._-]+\.(?:js|css)$/);
    const relative = reference.replace(/^\//, "");
    assert.equal(relative.startsWith("assets/"), true);
    assert.ok(readFileSync(join(distRoot, relative), "utf8").length > 0, `asset is empty: ${reference}`);
  }
});

test("production bundle keeps startup fallback and excludes WDIO guest loading", () => {
  const index = readFileSync(join(distRoot, "index.html"), "utf8");
  const bundle = assetReferences(index)
    .filter((reference) => reference.endsWith(".js"))
    .map((reference) => readFileSync(join(distRoot, reference.replace(/^\//, "")), "utf8"))
    .join("\n");
  assert.match(index, /startup-fallback/);
  assert.match(bundle, /react_mount_completed/);
  assert.doesNotMatch(bundle, /VITE_WDIO_E2E/);
});

test("initial IPC diagnostics do not overwrite the final React readiness marker", () => {
  const main = readFileSync(join(desktopRoot, "src", "main.jsx"), "utf8");
  assert.match(main, /setStartupState\("react_mount_started"\)/);
  assert.match(main, /markReactMounted\(\)/);
  assert.match(main, /emitFrontendEvent\("initial_ipc_started"\)/);
  assert.match(main, /emitFrontendEvent\("initial_ipc_settled"\)/);
  assert.doesNotMatch(main, /setStartupState\("initial_ipc_(started|settled)"\)/);
});