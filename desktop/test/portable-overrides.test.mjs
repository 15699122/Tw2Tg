import test from "node:test";
import assert from "node:assert/strict";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, unlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { deriveExtensionIdFromManifestKey } from "../scripts/extension-identity.mjs";
import { collectExtensionPackageFiles } from "../scripts/extension-package.mjs";

test("Full packaging validates and copies the selected worker and Native Host", async (t) => {
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
  cpSync(join(desktop, "third-party"), join(root, "desktop", "third-party"), { recursive: true });
  cpSync(join(desktop, "src-tauri", "tauri.conf.json"), join(root, "desktop", "src-tauri", "tauri.conf.json"));
  cpSync(join(project, "extension"), join(root, "extension"), { recursive: true });
  for (const name of ["LICENSE", "THIRD_PARTY_NOTICES.md"]) cpSync(join(project, name), join(root, name));
  mkdirSync(join(root, "sidecar", "gallery-dl"), { recursive: true });
  writeFileSync(join(root, "sidecar", "gallery-dl", "gallery-dl.exe"), "fixture gallery");
  writeFileSync(join(root, "sidecar", "gallery-dl", "LICENSE"), "fixture gallery licence");
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
  const run = (overrides = {}) => spawnSync(process.execPath, [join(root, "desktop", "scripts", "build-portable-windows.mjs")], {
    cwd: root, encoding: "utf8", timeout: 30000,
    env: { ...process.env, PORTABLE_OUTPUT_DIR: output, PORTABLE_PACKAGE_TYPE: "full",
      PORTABLE_ALLOW_BINARY_REUSE: "1", PORTABLE_APP_BINARY: app,
      PORTABLE_NATIVE_HOST_BINARY: host, PORTABLE_WORKER_DIRECTORY: worker,
      XARCHIVE_EXTENSION_ID: deriveExtensionIdFromManifestKey(manifest.key), ...overrides },
  });
  const result = run();
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.equal(existsSync(join(output, "extension", "tests")), false);
  assert.equal(existsSync(join(output, "extension", "package.json")), false);
  for (const file of await collectExtensionPackageFiles(join(root, "extension"))) {
    assert.deepEqual(readFileSync(join(output, "extension", file)), readFileSync(join(root, "extension", file)));
  }
  assert.equal(existsSync(join(root, "sidecar", "xarchive-downloader")), false);
  assert.equal(existsSync(join(root, "target", "release")), false);
  assert.equal(readFileSync(join(output, "sidecar", "xarchive-downloader", "worker-marker"), "utf8"), "selected worker");
  const hostName = process.platform === "win32" ? "xarchive-native-host.exe" : "xarchive-native-host";
  assert.equal(readFileSync(join(output, "native-host", hostName), "utf8"), "selected host");
  for (const name of ["LICENSE", "THIRD_PARTY_NOTICES.md"]) {
    assert.deepEqual(readFileSync(join(output, name)), readFileSync(join(root, name)));
  }
  const sentinel = join(output, "preserve-on-preflight-failure");
  writeFileSync(sentinel, "preserve");
  const extensionPopup = join(root, "extension", "popup.js");
  unlinkSync(extensionPopup);
  const missingExtensionFile = run();
  assert.notEqual(missingExtensionFile.status, 0);
  assert.match(missingExtensionFile.stderr, /missing required file: popup\.js/);
  assert.equal(readFileSync(sentinel, "utf8"), "preserve");
  cpSync(join(project, "extension", "popup.js"), extensionPopup);
  mkdirSync(join(worker, "_internal"));
  writeFileSync(join(worker, "_internal", "python312.dll"), "fixture runtime");
  const missingPythonLicence = run();
  assert.notEqual(missingPythonLicence.status, 0);
  assert.equal(readFileSync(sentinel, "utf8"), "preserve");
  writeFileSync(join(worker, "LICENSE.Python.txt"), "fixture Python licence");
  writeFileSync(join(worker, "_internal", "libcrypto-3-x64.dll"), "fixture crypto runtime");
  const missingOpenSslLicence = run();
  assert.notEqual(missingOpenSslLicence.status, 0);
  assert.equal(readFileSync(sentinel, "utf8"), "preserve");
  writeFileSync(join(worker, "LICENSE.OpenSSL.txt"), "fixture OpenSSL licence");
  unlinkSync(join(root, "THIRD_PARTY_NOTICES.md"));
  const missing = run();
  assert.notEqual(missing.status, 0);
  assert.match(missing.stderr, /Required portable notice is missing/);
  assert.equal(readFileSync(sentinel, "utf8"), "preserve");
  cpSync(join(project, "THIRD_PARTY_NOTICES.md"), join(root, "THIRD_PARTY_NOTICES.md"));
  const galleryLicence = join(root, "sidecar", "gallery-dl", "LICENSE");
  unlinkSync(galleryLicence);
  const missingGalleryLicence = run();
  assert.notEqual(missingGalleryLicence.status, 0);
  assert.equal(readFileSync(sentinel, "utf8"), "preserve");
  writeFileSync(galleryLicence, "fixture gallery licence");
  const pinPath = join(root, "desktop", "scripts", "windows-external-assets.json");
  const originalPins = readFileSync(pinPath);
  const changedPins = JSON.parse(originalPins.toString("utf8"));
  changedPins.gallery.version = "different-release";
  writeFileSync(pinPath, JSON.stringify(changedPins));
  const staleBinding = run();
  assert.notEqual(staleBinding.status, 0);
  assert.match(staleBinding.stderr, /licence metadata does not match/);
  assert.equal(readFileSync(sentinel, "utf8"), "preserve");
  writeFileSync(pinPath, originalPins);
  const aria = join(root, "sidecar", "aria2");
  mkdirSync(aria, { recursive: true });
  writeFileSync(join(aria, "aria2c.exe"), "selected aria2");
  const incompleteAria = run();
  assert.notEqual(incompleteAria.status, 0);
  assert.equal(readFileSync(sentinel, "utf8"), "preserve");
  const companions = ["AUTHORS", "COPYING", "LICENSE.OpenSSL", "README.mingw", "README.html", "NEWS", "ChangeLog"];
  for (const name of companions) writeFileSync(join(aria, name), `notice:${name}`);
  const bundledAria = run();
  assert.equal(bundledAria.status, 0, bundledAria.stderr || bundledAria.stdout);
  for (const name of companions) assert.equal(readFileSync(join(output, "sidecar", "aria2", name), "utf8"), `notice:${name}`);
  assert.equal(readFileSync(join(output, "sidecar", "gallery-dl", "LICENSE"), "utf8"), "fixture gallery licence");
  assert.equal(readFileSync(join(output, "sidecar", "xarchive-downloader", "LICENSE.Python.txt"), "utf8"), "fixture Python licence");
  assert.equal(readFileSync(join(output, "sidecar", "xarchive-downloader", "LICENSE.OpenSSL.txt"), "utf8"), "fixture OpenSSL licence");
  const core = join(root, "dist-portable", "core");
  const coreResult = run({ PORTABLE_PACKAGE_TYPE: "core", PORTABLE_OUTPUT_DIR: core });
  assert.equal(coreResult.status, 0, coreResult.stderr || coreResult.stdout);
  for (const name of ["LICENSE", "THIRD_PARTY_NOTICES.md"]) {
    assert.deepEqual(readFileSync(join(core, name)), readFileSync(join(root, name)));
  }
  assert.equal(existsSync(join(core, "native-host")), false);
});
