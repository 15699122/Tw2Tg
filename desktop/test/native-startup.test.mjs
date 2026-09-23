import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { mkdtempSync, rmSync, writeFileSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  isBlankDocument,
  isXArchiveDocument,
  looksLikeApplicationDocument,
  writeArtifact,
  ensureArtifactDir,
  snapshotSessionStart,
} from "../e2e/support/native-startup.mjs";

test("isBlankDocument recognizes blank document URLs", () => {
  assert.equal(isBlankDocument("data:,"), true);
  assert.equal(isBlankDocument("about:blank"), true);
  assert.equal(isBlankDocument(""), true);
  assert.equal(isBlankDocument(null), true);
  assert.equal(isBlankDocument(undefined), true);
  assert.equal(isBlankDocument("  data:,  "), true);
  assert.equal(isBlankDocument("data:, "), true);
});

test("isBlankDocument rejects non-blank URLs", () => {
  assert.equal(isBlankDocument("file:///xarchive"), false);
  assert.equal(isBlankDocument("https://example.com"), false);
  assert.equal(isBlankDocument("tauri://localhost"), false);
  assert.equal(isBlankDocument("  file:///xarchive  "), false);
});

test("isXArchiveDocument detects XArchive markers", () => {
  assert.equal(
    isXArchiveDocument({ rootExists: true, startupState: "", startupFallback: "" }),
    true,
  );
  assert.equal(
    isXArchiveDocument({ rootExists: false, startupState: "react_mount_completed", startupFallback: "" }),
    true,
  );
  assert.equal(
    isXArchiveDocument({ rootExists: false, startupState: "", startupFallback: "启动失败" }),
    true,
  );
  assert.equal(
    isXArchiveDocument({ rootExists: false, startupState: "", startupFallback: "", title: "XArchive" }),
    true,
  );
  assert.equal(
    isXArchiveDocument({ rootExists: false, startupState: "", startupFallback: "", title: "xarchive" }),
    true,
  );
});

test("isXArchiveDocument rejects non-XArchive documents and evidence with error", () => {
  assert.equal(
    isXArchiveDocument({ rootExists: false, startupState: "", startupFallback: "", title: "Other" }),
    false,
  );
  assert.equal(
    isXArchiveDocument({ rootExists: false, startupState: "", startupFallback: "", title: "" }),
    false,
  );
  assert.equal(
    isXArchiveDocument({ rootExists: false, startupState: "", startupFallback: "", title: "unknown", error: "switch failed" }),
    false,
  );
  assert.equal(isXArchiveDocument(null), false);
});

test("looksLikeApplicationDocument combines XArchive markers and non-blank URL", () => {
  assert.equal(
    looksLikeApplicationDocument({
      url: "file:///xarchive",
      rootExists: false,
      startupState: "",
      startupFallback: "",
      title: "",
    }),
    true,
  );
  assert.equal(
    looksLikeApplicationDocument({
      url: "file:///xarchive",
      rootExists: false,
      startupState: "react_mount_started",
      startupFallback: "",
      title: "",
    }),
    true,
  );
  assert.equal(
    looksLikeApplicationDocument({
      url: "data:,",
      rootExists: true,
      startupState: "",
      startupFallback: "",
      title: "",
    }),
    true,
  );
  assert.equal(
    looksLikeApplicationDocument({
      url: "data:,",
      rootExists: false,
      startupState: "",
      startupFallback: "",
      title: "",
    }),
    false,
  );
  assert.equal(
    looksLikeApplicationDocument({
      url: "data:,",
      rootExists: false,
      startupState: "",
      startupFallback: "",
      title: "other",
      error: "switch failed",
    }),
    false,
  );
});

test("writeArtifact creates JSON and text files inside the startup diagnostics dir", () => {
  const tmpDir = mkdtempSync(join(tmpdir(), "xarchive-startup-artifact-"));
  const original = process.env.READINESS_DIAGNOSTICS;
  try {
    process.env.READINESS_DIAGNOSTICS = tmpDir;
    const jsonPath = writeArtifact("evidence.json", { url: "data:,", rootExists: false });
    assert.equal(jsonPath, join(tmpDir, "startup", "evidence.json"));
    assert.deepEqual(JSON.parse(readFileSync(jsonPath, "utf8")), { url: "data:,", rootExists: false });
    const textPath = writeArtifact("trace.log", "hello");
    assert.equal(textPath, join(tmpDir, "startup", "trace.log"));
    assert.equal(readFileSync(textPath, "utf8"), "hello");
  } finally {
    process.env.READINESS_DIAGNOSTICS = original ?? undefined;
    try { rmSync(tmpDir, { recursive: true, force: true }); } catch {}
  }
});

test("ensureArtifactDir creates the startup subdirectory under the diagnostics root", () => {
  const tmpDir = mkdtempSync(join(tmpdir(), "xarchive-startup-dir-"));
  const original = process.env.READINESS_DIAGNOSTICS;
  try {
    process.env.READINESS_DIAGNOSTICS = tmpDir;
    const dir = ensureArtifactDir();
    assert.equal(dir, join(tmpDir, "startup"));
    assert.equal(fs.existsSync(join(tmpDir, "startup")), true);
  } finally {
    process.env.READINESS_DIAGNOSTICS = original ?? undefined;
    try { rmSync(tmpDir, { recursive: true, force: true }); } catch {}
  }
});

test("snapshotSessionStart records the initial session state as a startup artifact", async () => {
  const tmpDir = mkdtempSync(join(tmpdir(), "xarchive-session-start-"));
  const original = process.env.READINESS_DIAGNOSTICS;
  const previousBrowser = globalThis.browser;
  globalThis.browser = {
    getWindowHandles: async () => ["w-1", "w-2"],
    getWindowHandle: async () => "w-1",
    getUrl: async () => "data:,",
    getTitle: async () => "",
    capabilities: { browserName: "tauri" },
  };
  try {
    process.env.READINESS_DIAGNOSTICS = tmpDir;
    const snapshot = await snapshotSessionStart("unit-test");
    assert.equal(snapshot.kind, "session-start");
    assert.equal(snapshot.label, "unit-test");
    assert.deepEqual(snapshot.windowHandles, ["w-1", "w-2"]);
    assert.equal(snapshot.currentHandle, "w-1");
    assert.equal(snapshot.currentUrl, "data:,");
    assert.deepEqual(snapshot.capabilities, { browserName: "tauri" });
    assert.equal(snapshot.error, null);
    const written = JSON.parse(readFileSync(join(tmpDir, "startup", "session-start.json"), "utf8"));
    assert.equal(written.currentUrl, "data:,");
    assert.equal(written.windowHandles.length, 2);
    assert.equal(written.capabilities.browserName, "tauri");
  } finally {
    process.env.READINESS_DIAGNOSTICS = original ?? undefined;
    globalThis.browser = previousBrowser;
    try { rmSync(tmpDir, { recursive: true, force: true }); } catch {}
  }
});

test("snapshotSessionStart still writes the artifact when session commands fail", async () => {
  const tmpDir = mkdtempSync(join(tmpdir(), "xarchive-session-start-error-"));
  const original = process.env.READINESS_DIAGNOSTICS;
  const previousBrowser = globalThis.browser;
  globalThis.browser = {
    getWindowHandles: async () => {
      throw new Error("no valid session");
    },
    getWindowHandle: async () => {
      throw new Error("no valid session");
    },
    getUrl: async () => {
      throw new Error("no valid session");
    },
    getTitle: async () => {
      throw new Error("no valid session");
    },
  };
  try {
    process.env.READINESS_DIAGNOSTICS = tmpDir;
    const snapshot = await snapshotSessionStart("unit-test-error");
    assert.match(snapshot.error, /no valid session/);
    assert.deepEqual(snapshot.windowHandles, []);
    assert.equal(snapshot.currentHandle, null);
    const written = JSON.parse(readFileSync(join(tmpDir, "startup", "session-start.json"), "utf8"));
    assert.match(written.error, /no valid session/);
  } finally {
    process.env.READINESS_DIAGNOSTICS = original ?? undefined;
    globalThis.browser = previousBrowser;
    try { rmSync(tmpDir, { recursive: true, force: true }); } catch {}
  }
});
