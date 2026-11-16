import test from "node:test";
import assert from "node:assert/strict";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { deriveExtensionIdFromManifestKey } from "../scripts/extension-identity.mjs";

test("Full packaging validates and copies the selected worker and Native Host", (t) => {
  const desktop = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const project = resolve(desktop, "..");
  const root = mkdtempSync(join(tmpdir(), "tw2tg-portable-overrides-"));
  t.after(() => {
    const resolved = realpathSync(root);
    assert.equal(dirname(resolved), realpathSync(tmpdir()));
    assert.ok(resolved.split(/[\\/]/).pop().startsWith("tw2tg-portable-overrides-"));
    rmSync(resolved, { recursive: true });
  });
  mkdirSync(join(root, "desktop", "src-tauri"), { recursive: true });
  cpSync(join(desktop, "scripts"), join(root, "desktop", "scripts"), { recursive: true });
  cpSync(join(desktop, "src-tauri", "tauri.conf.json"), join(root, "desktop", "src-tauri", "tauri.conf.json"));
  cpSync(join(project, "extension"), join(root, "extension"), { recursive: true });
  mkdirSync(join(root, "sidecar", "gallery-dl"), { recursive: true });
  writeFileSync(join(root, "sidecar", "gallery-dl", "gallery-dl.exe"), "fixture gallery");
  const inputs = join(root, "inputs");
  const worker = join(inputs, "worker");
  mkdirSync(worker, { recursive: true });
  writeFileSync(join(worker, "worker-marker"), "selected worker");
  const host = join(inputs, "selected-host.exe");
  const app = join(inputs, "selected-app.exe");
  writeFileSync(host, "selected host");
  writeFileSync(app, "selected app");
  const manifest = JSON.parse(readFileSync(join(root, "extension", "manifest.json"), "utf8"));
  const output = join(root, "dist-portable", "output");
  const result = spawnSync(process.execPath, [join(root, "desktop", "scripts", "build-portable-windows.mjs")], {
    cwd: root, encoding: "utf8", timeout: 30000,
    env: { ...process.env, PORTABLE_OUTPUT_DIR: output, PORTABLE_PACKAGE_TYPE: "full",
      PORTABLE_ALLOW_BINARY_REUSE: "1", PORTABLE_APP_BINARY: app,
      PORTABLE_NATIVE_HOST_BINARY: host, PORTABLE_WORKER_DIRECTORY: worker,
      XARCHIVE_EXTENSION_ID: deriveExtensionIdFromManifestKey(manifest.key) },
  });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.equal(existsSync(join(root, "sidecar", "xarchive-downloader")), false);
  assert.equal(existsSync(join(root, "target", "release")), false);
  assert.equal(readFileSync(join(output, "sidecar", "xarchive-downloader", "worker-marker"), "utf8"), "selected worker");
  const hostName = process.platform === "win32" ? "xarchive-native-host.exe" : "xarchive-native-host";
  assert.equal(readFileSync(join(output, "native-host", hostName), "utf8"), "selected host");
});
