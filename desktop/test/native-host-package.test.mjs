import test from "node:test";
import assert from "node:assert/strict";
import {
  NATIVE_HOST_MANIFEST_FILE,
  NATIVE_HOST_NAME,
  createInstallationManifest,
  createNativeHostManifest,
  validateInstallationManifest,
  validateExtensionId,
  validateExtensionManifest,
} from "../scripts/native-host-package.mjs";

const extensionId = "abcdefghijklmnopabcdefghijklmnop";
const manifest = {
  manifest_version: 3,
  name: "XArchive",
  version: "0.1.0",
  permissions: ["nativeMessaging", "storage"],
  host_permissions: ["https://x.com/*", "https://twitter.com/*"],
  background: { service_worker: "src/background.js", type: "module" },
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