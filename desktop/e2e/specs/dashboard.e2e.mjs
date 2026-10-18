import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { createHash } from "node:crypto";

import {
  captureReadinessFailure,
  collectCurrentDocumentEvidence,
  snapshotSessionStart,
  waitForApplicationDocument,
  waitForStartupContract,
} from "../support/native-startup.mjs";

async function waitForDashboard() {
  await browser.waitUntil(
    async () => (await browser.$("h1").isExisting()) && (await browser.$("h1").isDisplayed()),
    {
      timeout: 20000,
      timeoutMsg: "XArchive dashboard heading did not become visible",
    },
  );
}

async function waitForDashboardWithEvidence() {
  try {
    // The readiness gate fails on a blank target instead of asserting on it, so
    // a session stuck on `data:,` reports the window-handle timeline rather than
    // a missing <h1> (WQ-ENG-09b). The frontend startup contract separates
    // "React never mounted" from "WebView2 served a blank document".
    await waitForApplicationDocument();
    await waitForStartupContract();
    await waitForDashboard();
  } catch (error) {
    await captureReadinessFailure("dashboard.readiness", error, {
      extraEvidence: { phase: "application-document-startup-contract-or-dashboard-heading" },
    });
    throw error;
  }
}

describe("XArchive Tauri desktop smoke", () => {
  before(async () => {
    // Record the target state at session start (usually still `data:,`) so a
    // later failure can be compared against it.
    await snapshotSessionStart("dashboard-before-hook");
    await waitForDashboardWithEvidence();
  });

  it("completes the frontend startup contract before rendering the dashboard", async () => {
    const evidence = await collectCurrentDocumentEvidence();
    assert.equal(evidence.readyState, "complete");
    assert.equal(evidence.rootExists, true);
    assert.equal(evidence.startupState, "react_mount_completed");
    assert.equal(evidence.startupFallback, "");
  });

  it("renders the dashboard shell in the native window", async () => {
    assert.equal(await browser.$("h1").getText(), "工作台");
    assert.equal(await browser.$("main").isDisplayed(), true);
    assert.equal(await browser.$('[aria-label="主导航"]').isExisting(), true);
  });

  it("exposes the stable dashboard regions used by Windows checks", async () => {
    assert.equal(await browser.$('[aria-label="归档概览"]').isExisting(), true);
    assert.equal(await browser.$("button").isExisting(), true);
    assert.equal(await browser.$("body").isDisplayed(), true);
  });

  it("uses the native record preview command with versioned documents and root validation", async () => {
    const result = await browser.executeAsync((done) => {
      const invoke = window.__TAURI__.core.invoke;
      const document = JSON.stringify({ schema_version: 1, kind: "tw2tg.download-records", records: [] });
      Promise.all([
        invoke("preview_record_import", { recordsJson: document, archiveRoot: "" }).then(
          () => "unexpected success", (reason) => String(reason)),
        invoke("preview_record_import", { recordsJson: "[]", archiveRoot: "" }).then(
          () => "unexpected success", (reason) => String(reason)),
      ]).then(done, (reason) => done([String(reason)]));
    });
    assert.match(result[0], /archive root must not be empty/);
    assert.match(result[1], /invalid record document/);
  });

  it("round-trips P1 archive records through native preview, accept and YAML export", async function () {
    if (!["linux", "win32"].includes(process.platform) || process.env.WDIO_F3_ARCHIVE !== "1") this.skip();
    const root = await mkdtemp(join(tmpdir(), "tw2tg-f3-native-"));
    try {
      const tweetId = `9${Date.now()}`;
      await mkdir(join(root, "Tweets", tweetId), { recursive: true });
      await writeFile(join(root, "Tweets", tweetId, "a.bin"), "original");
      const record = {
        account_handle: "collector", final_path: `Tweets/${tweetId}`,
        metadata: {
          schema_version: 1, tweet_id: tweetId, url: `https://x.test/i/status/${tweetId}`,
          tweet_type: "original", author: { user_id: `native-${tweetId}`, username: "alice" },
          text: "native archive fixture", archived_at: "2026-10-18T00:00:00Z",
          media: [{ index: 1, media_type: "photo", file: "a.bin", size_bytes: 8,
            sha256: createHash("sha256").update("original").digest("hex") }],
        },
      };
      const raw = JSON.stringify({ schema_version: 1, kind: "tw2tg.archive-records", records: [record] });
      const result = await browser.executeAsync((root, raw, tweetId, done) => {
        (async () => {
          const invoke = window.__TAURI__.core.invoke;
          const preview = await invoke("preview_record_import", { recordsJson: raw, archiveRoot: root });
          const accepted = await invoke("accept_record_import", { recordsJson: raw,
            acceptedJson: '[{"index":0}]', tweetId: "", archiveDirectory: root, expectedRevision: preview.revision });
          const yaml = await invoke("export_record_exchange", { jobId: "", tweetId, format: "yaml", kind: "archive", archiveRoot: root });
          const second = await invoke("preview_record_import", { recordsJson: yaml, archiveRoot: root });
          const repeated = await invoke("accept_record_import", { recordsJson: yaml,
            acceptedJson: '[{"index":0}]', tweetId: "", archiveDirectory: root, expectedRevision: second.revision });
          let staleRejected = false;
          try {
            await invoke("accept_record_import", { recordsJson: yaml,
              acceptedJson: '[{"index":0}]', tweetId: "", archiveDirectory: root, expectedRevision: "stale" });
          } catch (error) { staleRejected = String(error).includes("stale"); }
          return { preview, accepted, yaml, repeated, staleRejected };
        })().then(done, (error) => done({ error: String(error) }));
      }, root, raw, tweetId);
      assert.equal(result.error, undefined);
      assert.equal(result.preview.entries[0].verdict, "accept");
      assert.equal(result.accepted.accepted, 1);
      assert.match(result.yaml, /account_handle: collector/);
      assert.ok(result.yaml.includes(`final_path: Tweets/${tweetId}`));
      assert.equal(result.repeated.identical, 1);
      assert.equal(result.staleRejected, true);
      await writeFile(join(root, "Tweets", tweetId, "a.bin"), "tampered");
      const staleMedia = await browser.executeAsync((root, raw, done) => {
        window.__TAURI__.core.invoke("preview_record_import", { recordsJson: raw, archiveRoot: root })
          .then(done, (error) => done({ error: String(error) }));
      }, root, raw);
      assert.equal(staleMedia.entries[0].verdict, "preserve");
    } finally {
      assert.ok(resolve(root).startsWith(resolve(tmpdir()) + sep), "cleanup must stay in the disposable temp namespace");
      await rm(root, { recursive: true, force: true });
    }
  });
});
