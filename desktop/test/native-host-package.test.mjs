import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  NATIVE_HOST_MANIFEST_FILE,
  NATIVE_HOST_NAME,
  createInstallationManifest,
  createNativeHostManifest,
  validateExtensionId,
  validateExtensionIdentity,
  validateExtensionManifest,
  validateInstallationManifest,
} from "../scripts/native-host-package.mjs";
import { deriveExtensionIdFromManifestKey } from "../scripts/extension-identity.mjs";
import { runNativeHostManifestCommand } from "../scripts/native-host-manifest-cli.mjs";

const canonicalExtensionId = "iaajefkoanbkleojofoadeakelihbjne";
const extensionDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "extension");

const extensionId = "abcdefghijklmnopabcdefghijklmnop";
const manifest = {
  manifest_version: 3,
  name: "XArchive",
  version: "0.1.0",
  permissions: ["nativeMessaging", "storage", "activeTab"],
  host_permissions: ["https://x.com/*", "https://twitter.com/*"],
  background: { service_worker: "src/background.js", type: "module" },
  action: { default_popup: "popup.html" },
  options_ui: { page: "options.html", open_in_tab: true },
  content_scripts: [{ matches: ["https://x.com/*"], js: ["src/content.js"] }],
};

test("validates the fixed-length Chrome Extension ID contract", () => {
  assert.equal(validateExtensionId(extensionId), extensionId);
  assert.throws(() => validateExtensionId("not-an-extension-id"), /32-character/);
  assert.throws(() => validateExtensionId("ABCDEFghijklmnopabcdefghijklmnop"), /32-character/);
});

test("accepts the current minimal MV3 Extension manifest", () => {
  assert.equal(validateExtensionManifest(manifest), manifest);
  assert.throws(() => validateExtensionManifest({ ...manifest, manifest_version: 2 }), /manifest_version/);
  assert.throws(() => validateExtensionManifest({ ...manifest, permissions: ["storage"] }), /permission/);
  assert.throws(() => validateExtensionManifest({ ...manifest, host_permissions: ["<all_urls>"] }), /host permission/);
});

test("generates a Native Messaging host manifest without Registry side effects", () => {
  const host = createNativeHostManifest({ extensionId, path: "C:\\XArchive\\xarchive-native-host.exe" });
  assert.deepEqual(host, {
    name: NATIVE_HOST_NAME,
    description: "XArchive Native Messaging Host",
    path: "C:\\XArchive\\xarchive-native-host.exe",
    type: "stdio",
    allowed_origins: [`chrome-extension://${extensionId}/`],
  });
});

test("creates versioned installation layout manifest", () => {
  const installation = createInstallationManifest({
    releaseTag: "v0.2.0-pre.2",
    extensionId,
    nativeHostPath: "C:\\XArchive\\native-host\\xarchive-native-host.exe",
  });
  assert.equal(installation.native_host.name, NATIVE_HOST_NAME);
  assert.equal(installation.native_host.manifest, `native-host/${NATIVE_HOST_MANIFEST_FILE}`);
  assert.equal(installation.extension.id, extensionId);
  assert.equal(installation.platform, "windows-x64");
  assert.equal(validateInstallationManifest(installation), installation);
  assert.throws(() => createInstallationManifest({ releaseTag: "latest", extensionId, nativeHostPath: "host.exe" }), /release tag/);
});

test("requires the Extension manifest key to match the configured Extension ID", () => {
  const manifestKey = Buffer.from(
    "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyZ2hjrhBhFf5Nqllgq9R5z9ZuLz6+hzWdbiLNQmz4iAS3dLDLISyxvEEUxE2nh2wBho0JZCMEwPQq4Gkb+YE/GRX1+wkw1AsbO3kt0sAr79BEBX1orNM3hrZIuxo8mZO4yYO6g+bxuFmF54sj9HA250Ye50uxXmA2jql7GjaryckVLP9jXEMctxOllq0Wxt/66CWfD2JuqVx8QAXQGmMn5CHkNvCAr1aXmIAdZxXhlxKdqDfrGZPHFQpJLlbiERFv3SbuPwn77/38khZXZbxczT/Vyf37rJb1GuohOXpCgBdmYWIsut/vUugjaeQDBIVdc62aEEDEJqoy9/kMdBRyQIDAQAB",
    "base64",
  );
  const manifestKeyBase64 = manifestKey.toString("base64");
  const derivedExtensionId = deriveExtensionIdFromManifestKey(manifestKeyBase64);
  const manifestWithKey = { ...manifest, key: manifestKeyBase64 };

  assert.equal(
    validateExtensionIdentity({ extensionManifest: manifestWithKey, extensionId: derivedExtensionId }),
    derivedExtensionId,
  );
  assert.throws(
    () => validateExtensionIdentity({ extensionManifest: manifestWithKey, extensionId }),
    /does not match the Extension manifest key/,
  );
  assert.throws(
    () => validateExtensionIdentity({ extensionManifest: manifest, extensionId }),
    /public "key"/,
  );
  assert.throws(
    () => validateExtensionIdentity({ extensionManifest: manifestWithKey, extensionId: "zaajefkoanbkleojofoadeakelihbjne" }),
    /32-character/,
  );
});

test("writes the Native Host manifest from the verified Extension identity", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xarchive-host-manifest-"));
  const output = join(directory, "native-host", NATIVE_HOST_MANIFEST_FILE);
  const hostExecutable = join(directory, "native-host", "xarchive-native-host.exe");
  try {
    const result = await runNativeHostManifestCommand([
      "--extension-dir",
      extensionDirectory,
      "--expected-id",
      canonicalExtensionId,
      "--host-executable",
      hostExecutable,
      "--output",
      output,
    ]);
    const written = JSON.parse(await readFile(output, "utf8"));
    assert.equal(result.extensionId, canonicalExtensionId);
    assert.equal(written.name, NATIVE_HOST_NAME);
    assert.equal(written.type, "stdio");
    assert.equal(written.path, resolve(hostExecutable));
    assert.deepEqual(written.allowed_origins, [`chrome-extension://${canonicalExtensionId}/`]);

    await assert.rejects(
      runNativeHostManifestCommand([
        "--extension-dir",
        extensionDirectory,
        "--expected-id",
        "abcdefghijklmnopabcdefghijklmnop",
        "--host-executable",
        hostExecutable,
        "--output",
        output,
      ]),
      /does not match the manifest key/,
    );
    await assert.rejects(
      runNativeHostManifestCommand(["--extension-dir", extensionDirectory, "--expected-id", canonicalExtensionId]),
      /Missing required option --host-executable/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});