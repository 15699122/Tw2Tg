import {
  deriveExtensionIdFromManifestKey,
  validateExtensionId,
} from "./extension-identity.mjs";

export { validateExtensionId };

export const NATIVE_HOST_NAME = "com.tw2tg.xarchive";
export const NATIVE_HOST_MANIFEST_FILE = `${NATIVE_HOST_NAME}.json`;
export const INSTALLATION_MANIFEST_SCHEMA_VERSION = 1;

const REQUIRED_PERMISSIONS = new Set(["nativeMessaging", "storage"]);
const REQUIRED_HOST_PERMISSIONS = new Set(["https://x.com/*", "https://twitter.com/*"]);

export function validateExtensionManifest(manifest) {
  if (!manifest || typeof manifest !== "object") {
    throw new Error("Extension manifest must be an object");
  }
  if (manifest.manifest_version !== 3) {
    throw new Error("Extension manifest_version must be 3");
  }
  if (typeof manifest.name !== "string" || !manifest.name.trim()) {
    throw new Error("Extension manifest must define a name");
  }
  if (typeof manifest.version !== "string" || !/^\d+\.\d+\.\d+$/.test(manifest.version)) {
    throw new Error("Extension version must use MAJOR.MINOR.PATCH");
  }
  if (manifest.background?.service_worker !== "src/background.js") {
    throw new Error("Extension background service worker must be src/background.js");
  }
  for (const permission of REQUIRED_PERMISSIONS) {
    if (!manifest.permissions?.includes(permission)) {
      throw new Error(`Extension manifest is missing required permission: ${permission}`);
    }
  }
  for (const permission of REQUIRED_HOST_PERMISSIONS) {
    if (!manifest.host_permissions?.includes(permission)) {
      throw new Error(`Extension manifest is missing required host permission: ${permission}`);
    }
  }
  if (!manifest.content_scripts?.some((entry) => entry.js?.includes("src/content.js"))) {
    throw new Error("Extension manifest must register src/content.js");
  }
  return manifest;
}

export function validateExtensionIdentity({ extensionManifest, extensionId }) {
  validateExtensionManifest(extensionManifest);
  const configuredExtensionId = validateExtensionId(extensionId);
  const manifestKey = extensionManifest.key;
  if (typeof manifestKey !== "string" || !manifestKey.trim()) {
    throw new Error('Extension manifest must define a public "key" for identity verification');
  }
  const derivedExtensionId = deriveExtensionIdFromManifestKey(manifestKey);
  if (derivedExtensionId !== configuredExtensionId) {
    throw new Error(
      `Extension ID does not match the Extension manifest key: derived ${derivedExtensionId}, configured ${configuredExtensionId}`,
    );
  }
  return derivedExtensionId;
}

export function validateInstallationManifest(manifest) {
  if (!manifest || manifest.schema_version !== INSTALLATION_MANIFEST_SCHEMA_VERSION) {
    throw new Error("installation manifest schema_version is invalid");
  }
  if (manifest.package_type !== "native-host-extension" || manifest.platform !== "windows-x64") {
    throw new Error("installation manifest package type or platform is invalid");
  }
  validateExtensionId(manifest.extension?.id);
  if (manifest.native_host?.name !== NATIVE_HOST_NAME) {
    throw new Error("installation manifest Native Host name is invalid");
  }
  if (!Array.isArray(manifest.files) || manifest.files.length === 0) {
    throw new Error("installation manifest must list package files");
  }
  return manifest;
}

export function createNativeHostManifest({ extensionId, path, description = "XArchive Native Messaging Host" }) {
  validateExtensionId(extensionId);
  if (typeof path !== "string" || !path.trim()) {
    throw new Error("Native Host executable path is required");
  }
  return {
    name: NATIVE_HOST_NAME,
    description,
    path,
    type: "stdio",
    allowed_origins: [`chrome-extension://${extensionId}/`],
  };
}

export function createInstallationManifest({ releaseTag, extensionId, nativeHostPath }) {
  if (typeof releaseTag !== "string" || !/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(releaseTag)) {
    throw new Error("releaseTag must be a versioned release tag");
  }
  validateExtensionId(extensionId);
  const nativeHost = createNativeHostManifest({ extensionId, path: nativeHostPath });
  return validateInstallationManifest({
    schema_version: INSTALLATION_MANIFEST_SCHEMA_VERSION,
    package_type: "native-host-extension",
    platform: "windows-x64",
    release_tag: releaseTag,
    extension: {
      directory: "extension",
      manifest: "extension/manifest.json",
      id: extensionId,
    },
    native_host: {
      name: NATIVE_HOST_NAME,
      executable: "native-host/xarchive-native-host.exe",
      manifest: `native-host/${NATIVE_HOST_MANIFEST_FILE}`,
      registration: "windows-registry-or-user-native-messaging-host",
      allowed_origins: nativeHost.allowed_origins,
    },
    files: [
      "extension/manifest.json",
      "extension/src/background.js",
      "extension/src/content-core.js",
      "extension/src/content.js",
      "native-host/xarchive-native-host.exe",
      `native-host/${NATIVE_HOST_MANIFEST_FILE}`,
    ],
  });
}