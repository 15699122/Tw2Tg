import { cp, mkdir, rm, stat, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { componentPlan, createManifest, packageDirectories, validatePackageType } from "./portable-package.mjs";

const desktopDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(desktopDir, "..");
const outputRoot = resolve(projectRoot, process.env.PORTABLE_OUTPUT_DIR || "dist-portable/XArchive");
const packageType = process.env.PORTABLE_PACKAGE_TYPE || "full";
validatePackageType(packageType);
const exeName = process.platform === "win32" ? "xarchive-desktop.exe" : "xarchive-desktop";
const sourceExe = resolve(projectRoot, process.env.PORTABLE_APP_BINARY || `target/release/${exeName}`);

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
}

for (const [source, target, presence] of componentPlan(projectRoot, outputRoot, packageType)) {
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

const manifest = createManifest(packageType, exeName, process.env.PORTABLE_APP_VERSION || "unknown");
await writeFile(join(outputRoot, "package-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);

const result = await stat(join(outputRoot, exeName));
console.log(`Portable output: ${outputRoot}`);
console.log(`Executable: ${join(outputRoot, exeName)} (${result.size} bytes)`);
console.log(`Package type: ${packageType}`);
console.log("download/ is intentionally not pre-created; first launch performs download-directory setup.");