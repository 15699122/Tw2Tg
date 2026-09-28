import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";

import {
  NATIVE_CORE_DIST_FILES,
  NATIVE_CORE_PACKAGE_DIRNAME,
  PATCHED_PATTERN,
  SERVICE_DIST_FILES,
  SERVICE_PACKAGE_DIRNAME,
  SHELL_FALSE_PATTERN,
  SHELL_TRUE_PATTERN,
  VULNERABLE_PATTERN,
  patchNativeCoreSource,
  patchServiceSource,
  serviceRoot,
} from "../scripts/patch-wdio-tauri-service.mjs";

const SERVICE_SNIPPET = [
  "        try {",
  '            const { stdout: versionOutput } = await execAsync(`"${driverPath}" --version`, {',
  '                encoding: "utf8",',
  "                timeout: 5000,",
  "            });",
  "            const match = versionOutput.match(/MSEdgeDriver ([\\d.]+)/);",
  "            if (match) {",
].join("\n");

const NATIVE_CORE_SNIPPET = [
  "            const child = spawn(executable, args, {",
  "                stdio: 'pipe',",
  "                shell: process.platform === 'win32',",
  "            });",
].join("\n");

describe("driver banner patch", () => {
  it("rewrites the 1.4.0 banner discovery in both dist variants", () => {
    assert.equal(SERVICE_DIST_FILES.join(","), "dist/esm/index.js,dist/cjs/index.js");
    const { status, content } = patchServiceSource(SERVICE_SNIPPET);
    assert.equal(status, "patched");
    assert.doesNotMatch(content, /match\(\/MSEdgeDriver \(/);
    const literal = content.match(/versionOutput\.match\((\/.+?\/)\)/)[1];
    const regex = new RegExp(literal.slice(1, -1));
    assert.equal(regex.exec("Microsoft Edge WebDriver 153.0.4234.46")[1], "153.0.4234.46");
    assert.equal(regex.exec("MSEdgeDriver 152.0.4191.66")[1], "152.0.4191.66");
  });

  it("is idempotent", () => {
    const first = patchServiceSource(SERVICE_SNIPPET);
    const second = patchServiceSource(first.content);
    assert.equal(second.status, "already-patched");
    assert.equal(second.content, first.content);
  });

  it("leaves a future service version without the vulnerable pattern untouched", () => {
    const futureSource = "const match = versionOutput.match(/Whatever ([\\d.]+)/);";
    const { status, content } = patchServiceSource(futureSource);
    assert.equal(status, "not-found");
    assert.equal(content, futureSource);
  });

  it("replaces the exact vulnerable string and nothing else", () => {
    const source = "x = " + VULNERABLE_PATTERN + "; y = 1;";
    const { status, content } = patchServiceSource(source);
    assert.equal(status, "patched");
    assert.equal(content, "x = " + PATCHED_PATTERN + "; y = 1;");
  });
});

describe("native-core spawn patch", () => {
  it("stops routing the driver executable through the Windows shell", () => {
    const { status, content } = patchNativeCoreSource(NATIVE_CORE_SNIPPET);
    assert.equal(status, "patched");
    assert.match(content, /shell: false,/);
    assert.doesNotMatch(content, /shell: process\.platform === 'win32',/);
    assert.equal(NATIVE_CORE_DIST_FILES.join(","), "dist/esm/index.js,dist/cjs/index.js");
  });

  it("is idempotent and tolerant of an upstream rewrite", () => {
    const first = patchNativeCoreSource(NATIVE_CORE_SNIPPET);
    const second = patchNativeCoreSource(first.content);
    assert.equal(second.status, "already-patched");
    assert.equal(second.content, first.content);

    const rewritten = "spawn(file, args, { shell: false });";
    const untouched = patchNativeCoreSource(rewritten);
    assert.equal(untouched.status, "not-found");
    assert.equal(untouched.content, rewritten);
  });

  it("only replaces the Windows-conditional spawn, not other spawn options", () => {
    const unrelated = "const opts = { windowsHide: true, cwd: process.cwd() };";
    const { status, content } = patchNativeCoreSource(unrelated);
    assert.equal(status, "not-found");
    assert.equal(content, unrelated);
  });
});

describe("installed dependency wiring", () => {
  it("resolves the repository root that owns node_modules", () => {
    const root = serviceRoot();
    const manifest = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8"));
    assert.equal(Array.isArray(manifest.workspaces), true);
    assert.match(manifest.scripts.postinstall, /patch-wdio-tauri-service\.mjs/);
    assert.match(manifest.scripts.postinstall, /desktop\/scripts\//);
  });

  for (const [dirname, files, vulnerable, patched, label] of [
    [SERVICE_PACKAGE_DIRNAME, SERVICE_DIST_FILES, VULNERABLE_PATTERN, PATCHED_PATTERN, "driver-banner"],
    [NATIVE_CORE_PACKAGE_DIRNAME, NATIVE_CORE_DIST_FILES, SHELL_TRUE_PATTERN, SHELL_FALSE_PATTERN, "native-core-spawn"],
  ]) {
    it(`${dirname} still matches the ${label} patch target`, () => {
      for (const relative of files) {
        const source = readFileSync(path.join(serviceRoot(), dirname, relative), "utf8");
        assert.ok(
          source.includes(vulnerable) || source.includes(patched),
          `${dirname}/${relative} contains neither the vulnerable nor the patched ${label} pattern`,
        );
      }
    });
  }
});
