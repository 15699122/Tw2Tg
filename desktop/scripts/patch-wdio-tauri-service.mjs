// Copyright (c) 2026 Tw2Tg contributors.
// SPDX-License-Identifier: MIT

/**
 * Idempotent compatibility patches for the installed WDIO Tauri dependencies.
 *
 * Windows fact (2026-09-21, see docs/development/windows-validation.md):
 * `@wdio/tauri-service@1.4.0` discovers the Windows Edge driver with
 * `versionOutput.match(/MSEdgeDriver ([\d.]+)/)`, but the actual Microsoft
 * executable reports `Microsoft Edge WebDriver x.y`. The service therefore
 * reports `Driver: unknown` and no WebDriver session can be created
 * (WQ-P1-16/WQ-P1-17 BLOCKED_AUTOMATION). With the banner accepted, Windows
 * proved ordinary 1/1 and advanced 2/2 specs pass on the same artifacts.
 *
 * Windows path fact (2026-09-23): `@wdio/native-core` starts driver `.exe`
 * files with `shell: true`. This splits executable and argument paths that
 * contain spaces; the project's E: checkout has spaces in its path. The
 * driver is already an executable binary, so direct spawning with `shell:
 * false` preserves arguments and handles spaces safely.
 *
 * These patches are applied to installed dependencies so a clean `npm ci`
 * produces a reproducibly usable service. They:
 *
 * - touch no business code, test assertion, capability or driver version;
 * - is idempotent: already-patched installs are detected and left as-is;
 * - is tolerant: a future upstream version without the vulnerable pattern
 *   is skipped with a warning instead of failing installation;
 * - keeps both the legacy `MSEdgeDriver` and the current
 *   `Microsoft Edge WebDriver` banners accepted.
 *
 * It is wired as the repository root `postinstall` hook and as `pre*` hooks
 * of the desktop E2E scripts.
 */

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

export const SERVICE_PACKAGE_DIRNAME = "node_modules/@wdio/tauri-service";
export const SERVICE_DIST_FILES = ["dist/esm/index.js", "dist/cjs/index.js"];
export const NATIVE_CORE_PACKAGE_DIRNAME = "node_modules/@wdio/native-core";
export const NATIVE_CORE_DIST_FILES = ["dist/esm/index.js", "dist/cjs/index.js"];

/** The vulnerable discovery regex shipped in @wdio/tauri-service@1.4.0. */
export const VULNERABLE_PATTERN =
  "versionOutput.match(/MSEdgeDriver ([\\d.]+)/)";
/** The patched discovery regex: accepts both real Windows banners. */
export const PATCHED_PATTERN =
  'versionOutput.match(/(?:MSEdgeDriver|Microsoft Edge WebDriver) ([\\d.]+)/)';

const SHELL_TRUE_PATTERN = "shell: process.platform === 'win32',";
const SHELL_FALSE_PATTERN = "shell: false,";

/**
 * Apply the banner fix to one service source file's content.
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
  return { status: "patched", content: source.replace(VULNERABLE_PATTERN, PATCHED_PATTERN) };
}

/**
 * Disable the Windows command shell for direct native-driver executable
 * spawning, preventing paths with spaces from being split by cmd.exe.
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
  return { status: "patched", content: source.replace(SHELL_TRUE_PATTERN, SHELL_FALSE_PATTERN) };
}

function serviceRoot() {
  // desktop/scripts/ -> repository root is two levels up.
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
}

function run() {
  const root = serviceRoot();
  const packageJsonPath = path.join(root, SERVICE_PACKAGE_DIRNAME, "package.json");
  if (!existsSync(packageJsonPath)) {
    console.warn(
      `[patch-wdio-tauri-service] ${SERVICE_PACKAGE_DIRNAME} is not installed; nothing to patch.`,
    );
    return 0;
  }

  let failures = 0;
  let touched = 0;
  for (const relative of SERVICE_DIST_FILES) {
    const filePath = path.join(root, SERVICE_PACKAGE_DIRNAME, relative);
    if (!existsSync(filePath)) {
      console.warn(
        `[patch-wdio-tauri-service] ${relative} is missing; skipping (service layout may have changed).`,
      );
      continue;
    }
    const original = readFileSync(filePath, "utf8");
    const { status, content } = patchServiceSource(original);
    switch (status) {
      case "patched": {
        writeFileSync(filePath, content, "utf8");
        const verify = patchServiceSource(readFileSync(filePath, "utf8"));
        if (verify.status !== "already-patched") {
          console.error(
            `[patch-wdio-tauri-service] FAILED to verify patched ${relative}.`,
          );
          failures += 1;
          break;
        }
        touched += 1;
        console.log(`[patch-wdio-tauri-service] patched ${relative}`);
        break;
      }
      case "already-patched":
        console.log(`[patch-wdio-tauri-service] ${relative} already patched`);
        break;
      case "not-found":
        console.warn(
          `[patch-wdio-tauri-service] no vulnerable banner pattern in ${relative}; ` +
            "if the service was upgraded, verify the banner discovery before relying on this patch.",
        );
        break;
    }
  }

  const nativeCorePackageJson = path.join(root, NATIVE_CORE_PACKAGE_DIRNAME, "package.json");
  if (existsSync(nativeCorePackageJson)) {
    for (const relative of NATIVE_CORE_DIST_FILES) {
      const filePath = path.join(root, NATIVE_CORE_PACKAGE_DIRNAME, relative);
      if (!existsSync(filePath)) {
        console.warn(
          `[patch-wdio-tauri-service] ${NATIVE_CORE_PACKAGE_DIRNAME}/${relative} is missing; skipping.`,
        );
        continue;
      }
      const original = readFileSync(filePath, "utf8");
      const { status, content } = patchNativeCoreSource(original);
      switch (status) {
        case "patched": {
          writeFileSync(filePath, content, "utf8");
          const verify = patchNativeCoreSource(readFileSync(filePath, "utf8"));
          if (verify.status !== "already-patched") {
            console.error(
              `[patch-wdio-tauri-service] FAILED to verify native-core patch ${relative}.`,
            );
            failures += 1;
            break;
          }
          touched += 1;
          console.log(`[patch-wdio-tauri-service] patched native-core ${relative}`);
          break;
        }
        case "already-patched":
          console.log(`[patch-wdio-tauri-service] native-core ${relative} already patched`);
          break;
        case "not-found":
          console.warn(
            `[patch-wdio-tauri-service] native-core spawn pattern not found in ${relative}; ` +
              "verify the dependency implementation before relying on this patch.",
          );
          break;
      }
    }
  }

  if (failures > 0) {
    console.error(
      `[patch-wdio-tauri-service] ${failures} file(s) failed verification.`,
    );
    return 1;
  }
  console.log(
    `[patch-wdio-tauri-service] done (patched now: ${touched}); ` +
      "EdgeDriver banners are accepted and native driver paths avoid Windows shell splitting.",
  );
  return 0;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  process.exit(run());
}
