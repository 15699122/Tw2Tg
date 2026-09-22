// Copyright (c) 2026 Tw2Tg contributors.
// SPDX-License-Identifier: MIT

import test from "node:test";
import assert from "node:assert/strict";

import {
  PATCHED_PATTERN,
  SERVICE_DIST_FILES,
  VULNERABLE_PATTERN,
  patchServiceSource,
} from "../scripts/patch-wdio-tauri-service.mjs";

const ESN_SNIPPET = [
  "        try {",
  "            const { stdout: versionOutput } = await execAsync(`\"${driverPath}\" --version`, {",
  "                encoding: \"utf8\",",
  "                timeout: 5000,",
  "            });",
  "            const match = versionOutput.match(/MSEdgeDriver ([\\d.]+)/);",
  "            if (match) {",
].join("\n");

test("patch rewrites the 1.4.0 banner discovery in both dist variants", () => {
  assert.equal(SERVICE_DIST_FILES.join(","), "dist/esm/index.js,dist/cjs/index.js");
  const { status, content } = patchServiceSource(ESN_SNIPPET);
  assert.equal(status, "patched");
  assert.match(content, /\(\?:MSEdgeDriver\|Microsoft Edge WebDriver\) \(\[\\d\.\]\+\)/);
  assert.doesNotMatch(content, /match\(\/MSEdgeDriver \(/);
  // The legacy banner must remain accepted by the patched regex.
  const regexBody = content.match(/versionOutput\.match\((\/.+\/)\)/)[1];
  assert.match("MSEdgeDriver 152.0.4191.66", new RegExp(regexBody.slice(1, -1)));
  assert.match("Microsoft Edge WebDriver 152.0.4191.66", new RegExp(regexBody.slice(1, -1)));
});

test("patch is idempotent", () => {
  const first = patchServiceSource(ESN_SNIPPET);
  assert.equal(first.status, "patched");
  const second = patchServiceSource(first.content);
  assert.equal(second.status, "already-patched");
  assert.equal(second.content, first.content);
});

test("a future service version without the vulnerable pattern is left untouched", () => {
  const futureSource = "const match = versionOutput.match(/Whatever ([\\d.]+)/);";
  const { status, content } = patchServiceSource(futureSource);
  assert.equal(status, "not-found");
  assert.equal(content, futureSource);
});

test("the patch replaces the exact vulnerable string and nothing else", () => {
  const source = "x = " + VULNERABLE_PATTERN + "; y = 1;";
  const { status, content } = patchServiceSource(source);
  assert.equal(status, "patched");
  assert.equal(content, "x = " + PATCHED_PATTERN + "; y = 1;");
});

test("patched content still parses as a regex accepting both banners", () => {
  const { content } = patchServiceSource(
    "const match = versionOutput.match(/MSEdgeDriver ([\\d.]+)/);",
  );
  const literal = content.match(/versionOutput\.match\((\/.*\/)\)/)[1];
  const flags = literal.slice(literal.lastIndexOf("/") + 1);
  const body = literal.slice(1, literal.lastIndexOf("/") - flags.length);
  const regex = new RegExp(body, flags);
  assert.equal(regex.exec("Microsoft Edge WebDriver 152.0.4191.66")[1], "152.0.4191.66");
  assert.equal(regex.exec("MSEdgeDriver 153.0.4234.48")[1], "153.0.4234.48");
});
