import { cp, mkdir, readdir, rm, stat, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import {
  componentPlan,
  createManifest,
  filterPackageFiles,
  packageDirectories,
  portablePackageVersion,
  validatePackageType,
  validatePortableOutputDir,
} from "./portable-package.mjs";

const desktopDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(desktopDir, "..");
// Validate before any build or delete step: the output directory is removed
// recursively, so an operator-supplied path must never escape the packaging
// namespace, the project root, or the user home directory.
const outputRoot = validatePortableOutputDir(process.env.PORTABLE_OUTPUT_DIR || "dist-portable/XArchive", {
  projectRoot,
  homeDir: homedir(),
});
const packageType = process.env.PORTABLE_PACKAGE_TYPE || "full";
validatePackageType(packageType);
const exeName = process.platform === "win32" ? "xarchive-desktop.exe" : "xarchive-desktop";
const sourceExe = resolve(projectRoot, process.env.PORTABLE_APP_BINARY || `target/release/${exeName}`);

function run(command, args) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command, args, {
      cwd: projectRoot,
      stdio: "inherit",
      ...spawnOptionsForCommand(command),
    });
    child.on("error", reject);
    child.on("exit", (code) => code === 0 ? resolvePromise() : reject(new Error(`${command} exited with ${code}`)));
  });
}

/**
 * Compute the `spawn` shell option for `command`.
 *
 * On Windows, `npm` resolves to `npm.cmd`, which is a batch file, not an
 * executable image: `spawn(..., { shell: false })` rejects with EINVAL even
 * when the file exists (Windows batch 6, 2026-09-29). Route through the
 * command interpreter only for `.cmd`/`.bat` launchers; everything else keeps
 * `shell: false` so argument splitting cannot change behavior.
 */
export function spawnOptionsForCommand(command, platform = process.platform) {
  const needsShell = platform === "win32" && /\.(cmd|bat)$/i.test(command);
  return { shell: needsShell };
}

/// Recursively list files under `root`, relative to `root`.
async function listFiles(root) {
  const collected = [];
  const walk = async (directory) => {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const full = join(directory, entry.name);
      if (entry.isDirectory()) {
        await walk(full);
      } else if (entry.isFile() || entry.isSymbolicLink()) {
        collected.push(relative(root, full));
      }
    }
  };
  await walk(root);
  return collected;
}

/// ENG-09: copy a component directory while dropping files that must never ship
/// (local `.env`, SQLite databases, logs, caches, test artifacts). A plain
/// recursive `cp` would ignore `.gitignore` and could carry those into a
/// release artifact.
async function copyFiltered(source, target) {
  const all = await listFiles(source);
  const allowed = new Set(filterPackageFiles(all));
  const dropped = all.filter((entry) => !allowed.has(entry));
  for (const relativePath of dropped) {
    console.log(`Excluded from package: ${join(source, relativePath)}`);
  }
  for (const relativePath of allowed) {
    const destination = join(target, relativePath);
    await mkdir(dirname(destination), { recursive: true });
    await cp(join(source, relativePath), destination);
  }
}

// Verify every required input before deleting the previous package, so a missing
// component cannot leave the operator with no output and a failed build.
export async function main() {
  const plan = componentPlan(projectRoot, outputRoot, packageType);
  for (const [source, , presence] of plan) {
    if (presence === "required" && !existsSync(source)) {
      throw new Error(`Required portable component is missing: ${source}`);
    }
  }
  if (packageType === "full" && !existsSync(resolve(projectRoot, "extension"))) {
    throw new Error(`Required Extension directory is missing: ${resolve(projectRoot, "extension")}`);
  }

  // ENG-08: a stale binary from an earlier build must never be packaged by
  // default. Reuse requires an explicit opt-in and is still recorded in the
  // manifest so the package states where its executable came from.
  const allowBinaryReuse = process.env.PORTABLE_ALLOW_BINARY_REUSE === "1";
  const binaryIsStale = !existsSync(sourceExe);

  if (binaryIsStale || !allowBinaryReuse) {
    await run(
      process.platform === "win32" ? "npm.cmd" : "npm",
      ["run", "build:tauri", "--workspace", "desktop"],
    );
  } else {
    console.log(
      "Reusing the existing application binary because PORTABLE_ALLOW_BINARY_REUSE=1.",
    );
  }
  if (!existsSync(sourceExe)) {
    throw new Error(`Application binary was not produced: ${sourceExe}`);
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
    await copyFiltered(extensionSource, join(outputRoot, "extension"));
  }

  for (const [source, target, presence] of plan) {
    const present = existsSync(source);
    if (presence === "excluded") {
      continue;
    }
    if (!present) {
      if (presence === "required") throw new Error(`Required portable component is missing: ${source}`);
      continue;
    }
    await copyFiltered(source, target);
  }

  // Version source: `PORTABLE_APP_VERSION` for a release rehearsal, otherwise
  // the Tauri config version that also stamps the PE file properties, so the
  // manifest can never drift from the executable it describes.
  const manifest = createManifest(packageType, exeName, portablePackageVersion(projectRoot));
  await writeFile(join(outputRoot, "package-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);

  const result = await stat(join(outputRoot, exeName));
  console.log(`Portable output: ${outputRoot}`);
  console.log(`Executable: ${join(outputRoot, exeName)} (${result.size} bytes)`);
  console.log(`Package type: ${packageType}`);
  console.log("download/ is intentionally not pre-created; first launch performs download-directory setup.");
}

// Run the packaging flow only when executed directly
// (`npm run build:portable:windows` / `node scripts/build-portable-windows.mjs`).
// Tests import `spawnOptionsForCommand` from this module and must not trigger a build.
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}