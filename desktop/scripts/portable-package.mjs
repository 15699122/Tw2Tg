import { join } from "node:path";

export const PORTABLE_PACKAGE_TYPES = new Set(["full", "core"]);

export function validatePackageType(packageType) {
  if (!PORTABLE_PACKAGE_TYPES.has(packageType)) {
    throw new Error(`PORTABLE_PACKAGE_TYPE must be full or core, got: ${packageType}`);
  }
  return packageType;
}

export function packageDirectories(packageType) {
  validatePackageType(packageType);
  return [
    "config",
    "cache/staging",
    "cache/downloads",
    "cache/runtime",
    "logs",
    ...(packageType === "full" ? ["sidecar/gallery-dl"] : []),
    "sidecar/xarchive-downloader",
    "sidecar/aria2",
    ...(packageType === "full" ? ["native-host"] : []),
  ];
}

export function componentPlan(projectRoot, outputRoot, packageType) {
  validatePackageType(packageType);

  // Component plan uses a three-state presence flag:
  //  - "required":  must exist in the source tree or packaging fails.
  //  - "optional":  copied when present, skipped when absent.
  //  - "excluded":  never copied into the package, regardless of source.
  const isFull = packageType === "full";
  return [
    // worker: always bundled (one-dir layout, matches config.rs default)
    [join(projectRoot, "sidecar", "xarchive-downloader"), join(outputRoot, "sidecar", "xarchive-downloader"), "required"],
    // aria2: copied when a local bundled artifact is available
    [join(projectRoot, "sidecar", "aria2"), join(outputRoot, "sidecar", "aria2"), "optional"],
    // gallery-dl: bundled in Full, explicitly excluded from Core
    [join(projectRoot, "sidecar", "gallery-dl"), join(outputRoot, "sidecar", "gallery-dl"), isFull ? "required" : "excluded"],
    // Native Host is bundled only in Full; Core remains browser-component-free.
    [join(projectRoot, "target", "release", process.platform === "win32" ? "xarchive-native-host.exe" : "xarchive-native-host"), join(outputRoot, "native-host", process.platform === "win32" ? "xarchive-native-host.exe" : "xarchive-native-host"), isFull ? "required" : "excluded"],
  ];
}

export function createManifest(packageType, executable = "xarchive-desktop.exe", version = "unknown", nativeHost = null) {
  validatePackageType(packageType);
  return {
    schema_version: 1,
    package_type: packageType,
    platform: "windows-x64",
    desktop: { executable, version },
    sidecar: {
      worker: "sidecar/xarchive-downloader",
      gallery_dl: packageType === "full" ? "sidecar/gallery-dl/gallery-dl.exe" : null,
      gallery_dl_bundled: packageType === "full",
    },
    extension: {
      directory: "extension",
      bundled: packageType === "full",
      user_importable: false,
    },
    native_host: nativeHost,
  };
}