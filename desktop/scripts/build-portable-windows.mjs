import { cp, mkdir, rm, stat } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const desktopDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(desktopDir, "..");
const outputRoot = resolve(projectRoot, process.env.PORTABLE_OUTPUT_DIR || "dist-portable/XArchive");
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
for (const directory of [
  "config",
  "cache/staging",
  "cache/downloads",
  "cache/runtime",
  "logs",
  "sidecar/gallery-dl",
  "sidecar/aria2",
]) {
  await mkdir(join(outputRoot, directory), { recursive: true });
}

await cp(sourceExe, join(outputRoot, exeName));
await cp(resolve(projectRoot, "extension"), join(outputRoot, "extension"), { recursive: true });

for (const [source, target] of [
  [resolve(projectRoot, "sidecar", "gallery-dl"), join(outputRoot, "sidecar", "gallery-dl")],
  [resolve(projectRoot, "sidecar", "aria2"), join(outputRoot, "sidecar", "aria2")],
]) {
  if (existsSync(source)) await cp(source, target, { recursive: true });
}

const result = await stat(join(outputRoot, exeName));
console.log(`Portable output: ${outputRoot}`);
console.log(`Executable: ${join(outputRoot, exeName)} (${result.size} bytes)`);
console.log("download/ is intentionally not pre-created; first launch performs download-directory setup.");