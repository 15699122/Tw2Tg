/**
 * Idempotent dependency patch for the Windows WebdriverIO E2E path.
 *
 * Two upstream defects in the pinned E2E dependency chain are still present in
 * the installed tree, and both block a native session on Windows:
 *
 * 1. `@wdio/tauri-service@1.4.0` discovers the Windows Edge driver with
 *    `versionOutput.match(/MSEdgeDriver ([\d.]+)/)`, while the Microsoft
 *    executable prints `Microsoft Edge WebDriver x.y`. The version is reported
 *    as unknown and no WebDriver session can be created (WQ-P1-16/WQ-P1-17
 *    `BLOCKED_AUTOMATION`).
 * 2. `@wdio/native-core@1.2.0` launches the driver `.exe` with
 *    `shell: process.platform === 'win32'`. `cmd.exe` then splits executable
 *    and argument paths containing spaces, so the driver is never started
 *    (2026-09-23 Windows finding, WQ-P0-WHITE).
 *
 * The patch therefore:
 *
 * - touches no product code, test assertion, capability or driver version;
 * - only rewrites the installed dependency under `node_modules`, which is never
 *   Git-tracked;
 * - is idempotent: an already-patched install is detected and left as-is;
 * - is tolerant: an upstream version without the vulnerable pattern is skipped
 *   with a warning instead of failing the install;
 * - verifies every write and exits non-zero when verification fails.
 *
 * It is wired as the repository root `postinstall` hook and as the `pre*` hooks
 * of the desktop E2E scripts, so `npm ci` alone yields a usable service.
 */

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

export const SERVICE_PACKAGE_DIRNAME = "node_modules/@wdio/tauri-service";
export const SERVICE_DIST_FILES = ["dist/esm/index.js", "dist/cjs/index.js"];
export const NATIVE_CORE_PACKAGE_DIRNAME = "node_modules/@wdio/native-core";
export const NATIVE_CORE_DIST_FILES = ["dist/esm/index.js", "dist/cjs/index.js"];

/** The vulnerable driver-banner discovery shipped in @wdio/tauri-service@1.4.0. */
export const VULNERABLE_PATTERN =
  "versionOutput.match(/MSEdgeDriver ([\\d.]+)/)";
/** The patched discovery regex: accepts both real Microsoft banners. */
export const PATCHED_PATTERN =
  "versionOutput.match(/(?:MSEdgeDriver|Microsoft Edge WebDriver) ([\\d.]+)/)";

/** The vulnerable spawn option shipped in @wdio/native-core@1.2.0. */
export const SHELL_TRUE_PATTERN = "shell: process.platform === 'win32',";
/** Direct spawning keeps executable and argument paths intact. */
export const SHELL_FALSE_PATTERN = "shell: false,";

/**
 * Accept both Edge WebDriver banners in one service source file.
 *
 * @param {string} source current file content
 * @returns {{ status: "patched"|"already-patched"|"not-found", content: string }}
 */
export function patchServiceSource(source) {
  if (source.includes(PATCHED_PATTERN)) {
    return { status: "already-patched", content: source };
  }
  if (!source.includes(VULNERABLE_PATTERN)) {
    return { status: "not-found", content: source };
  }
  return {
    status: "patched",
    content: source.replace(VULNERABLE_PATTERN, PATCHED_PATTERN),
  };
}

/**
 * Stop routing the native driver executable through the Windows command shell.
 *
 * @param {string} source current file content
 * @returns {{ status: "patched"|"already-patched"|"not-found", content: string }}
 */
export function patchNativeCoreSource(source) {
  if (source.includes(SHELL_FALSE_PATTERN)) {
    return { status: "already-patched", content: source };
  }
  if (!source.includes(SHELL_TRUE_PATTERN)) {
    return { status: "not-found", content: source };
  }
  return {
    status: "patched",
    content: source.replace(SHELL_TRUE_PATTERN, SHELL_FALSE_PATTERN),
  };
}

/** Repository root: `desktop/scripts/` is two levels below it. */
export function serviceRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
}

/**
 * Apply one patch function to every dist variant of a dependency.
 *
 * @returns {{ touched: number, failures: number }}
 */
function applyPatch({ root, dirname, files, patch, label }) {
  let touched = 0;
  let failures = 0;

  if (!existsSync(path.join(root, dirname, "package.json"))) {
    console.warn(
      `[patch-wdio-tauri-service] ${dirname} is not installed; skipping ${label}.`,
    );
    return { touched, failures };
  }

  for (const relative of files) {
    const filePath = path.join(root, dirname, relative);
    if (!existsSync(filePath)) {
      console.warn(
        `[patch-wdio-tauri-service] ${dirname}/${relative} is missing; ` +
          "skipping (dependency layout may have changed).",
      );
      continue;
    }
    const { status, content } = patch(readFileSync(filePath, "utf8"));
    if (status === "already-patched") {
      console.log(`[patch-wdio-tauri-service] ${label} ${relative} already patched`);
      continue;
    }
    if (status === "not-found") {
      console.warn(
        `[patch-wdio-tauri-service] no ${label} pattern in ${dirname}/${relative}; ` +
          "verify the dependency implementation before relying on this patch.",
      );
      continue;
    }
    writeFileSync(filePath, content, "utf8");
    const verify = patch(readFileSync(filePath, "utf8"));
    if (verify.status !== "already-patched") {
      console.error(
        `[patch-wdio-tauri-service] FAILED to verify the ${label} patch in ` +
          `${dirname}/${relative}.`,
      );
      failures += 1;
      continue;
    }
    touched += 1;
    console.log(`[patch-wdio-tauri-service] patched ${label} ${relative}`);
  }

  return { touched, failures };
}

export function run() {
  const root = serviceRoot();
  const service = applyPatch({
    root,
    dirname: SERVICE_PACKAGE_DIRNAME,
    files: SERVICE_DIST_FILES,
    patch: patchServiceSource,
    label: "driver-banner",
  });
  const nativeCore = applyPatch({
    root,
    dirname: NATIVE_CORE_PACKAGE_DIRNAME,
    files: NATIVE_CORE_DIST_FILES,
    patch: patchNativeCoreSource,
    label: "native-core-spawn",
  });

  const failures = service.failures + nativeCore.failures;
  if (failures > 0) {
    console.error(`[patch-wdio-tauri-service] ${failures} file(s) failed verification.`);
    return 1;
  }
  console.log(
    `[patch-wdio-tauri-service] done (patched now: ${service.touched + nativeCore.touched}); ` +
      "Edge WebDriver banners are accepted and native driver paths avoid Windows shell splitting.",
  );
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  process.exit(run());
}
