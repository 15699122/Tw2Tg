import { cp, mkdir, rm, stat, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { componentPlan, createManifest, packageDirectories, validatePackageType } from "./portable-package.mjs";
import { readExtensionManifest } from "./extension-identity.mjs";
import {
  createInstallationManifest,
  createNativeHostManifest,
  NATIVE_HOST_MANIFEST_FILE,
  validateExtensionIdentity,
} from "./native-host-package.mjs";

const desktopDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(desktopDir, "..");
const outputRoot = resolve(projectRoot, process.env.PORTABLE_OUTPUT_DIR || "dist-portable/XArchive");
const packageType = process.env.PORTABLE_PACKAGE_TYPE || "full";
validatePackageType(packageType);
const exeName = process.platform === "win32" ? "xarchive-desktop.exe" : "xarchive-desktop";
const sourceExe = resolve(projectRoot, process.env.PORTABLE_APP_BINARY || `target/release/${exeName}`);
const nativeHostExeName = process.platform === "win32" ? "xarchive-native-host.exe" : "xarchive-native-host";
const sourceNativeHost = resolve(projectRoot, process.env.PORTABLE_NATIVE_HOST_BINARY || `target/release/${nativeHostExeName}`);
const sourceWorkerDirectory = resolve(projectRoot, process.env.PORTABLE_WORKER_DIRECTORY || "sidecar/xarchive-downloader");
const extensionId = process.env.XARCHIVE_EXTENSION_ID || "";

function run(command, args) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, { cwd: projectRoot, stdio: "inherit", shell: false });
    child.on("error", reject);
    child.on("exit", (code) => code === 0 ? resolvePromise() : reject(new Error(`${command} exited with ${code}`)));
  });
}

if (!existsSync(sourceExe)) {
  await run(process.platform === "win32" ? "npm.cmd" : "npm", ["run", "build:tauri", "--workspace", "desktop"]);
}

if (packageType === "full") {
  if (!extensionId) throw new Error("XARCHIVE_EXTENSION_ID is required to build a Full package with Native Host");
  const extensionSourceForIdentity = resolve(projectRoot, "extension");
  if (!existsSync(extensionSourceForIdentity)) {
    throw new Error(`Required Extension directory is missing: ${extensionSourceForIdentity}`);
  }
  const { manifest: extensionManifest } = await readExtensionManifest(extensionSourceForIdentity);
  validateExtensionIdentity({ extensionManifest, extensionId });
  if (!existsSync(sourceNativeHost)) {
    await run(process.platform === "win32" ? "cargo.exe" : "cargo", ["build", "-p", "xarchive-native-host", "--release"]);
  }
  if (!existsSync(sourceNativeHost)) throw new Error(`Required Native Host binary is missing: ${sourceNativeHost}`);
}

await rm(outputRoot, { recursive: true, force: true });
await mkdir(outputRoot, { recursive: true });
for (const directory of packageDirectories(packageType)) {
  await mkdir(join(outputRoot, directory), { recursive: true });
}

await cp(sourceExe, join(outputRoot, exeName));
if (packageType === "full") {
  const extensionSource = resolve(projectRoot, "extension");
  if (!existsSync(extensionSource)) throw new Error(`Required Extension directory is missing: ${extensionSource}`);
  await cp(extensionSource, join(outputRoot, "extension"), { recursive: true });
  const nativeHostTarget = join(outputRoot, "native-host", nativeHostExeName);
  await cp(sourceNativeHost, nativeHostTarget);
  const hostManifest = createNativeHostManifest({ extensionId, path: nativeHostTarget });
  await writeFile(join(outputRoot, "native-host", NATIVE_HOST_MANIFEST_FILE), `${JSON.stringify(hostManifest, null, 2)}\n`);
}

for (const [plannedSource, target, presence] of componentPlan(projectRoot, outputRoot, packageType)) {
  const source = plannedSource === join(projectRoot, "sidecar", "xarchive-downloader")
    ? sourceWorkerDirectory
    : plannedSource;
  // Full-package Native Host was copied from sourceNativeHost above, which may
  // be an isolated validation build. Do not overwrite it with target/release.
  if (packageType === "full" && target === join(outputRoot, "native-host", nativeHostExeName)) {
    continue;
  }
  const present = existsSync(source);
  if (presence === "excluded") {
    continue;
  }
  if (!present) {
    if (presence === "required") throw new Error(`Required portable component is missing: ${source}`);
    continue;
  }
  await cp(source, target, { recursive: true });
}

const nativeHostInstallation = packageType === "full"
  ? createInstallationManifest({
      releaseTag: process.env.PORTABLE_APP_VERSION || "v0.0.0-local",
      extensionId,
      nativeHostPath: join(outputRoot, "native-host", nativeHostExeName),
    })
  : null;
const manifest = createManifest(packageType, exeName, process.env.PORTABLE_APP_VERSION || "unknown", nativeHostInstallation?.native_host || null);
if (nativeHostInstallation) manifest.installation = nativeHostInstallation;
await writeFile(join(outputRoot, "package-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);

const result = await stat(join(outputRoot, exeName));
console.log(`Portable output: ${outputRoot}`);
console.log(`Executable: ${join(outputRoot, exeName)} (${result.size} bytes)`);
console.log(`Package type: ${packageType}`);
console.log("download/ is intentionally not pre-created; first launch performs download-directory setup.");
