import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { access, mkdir } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

describe("Windows record-import cancellation", () => {
  it("cancels a waiting import without committing and accepts it later", async function () {
    if (process.platform !== "win32" || !process.env.WDIO_WORKSPACE_HOLDER) this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    const appRoot = path.dirname(process.env.WDIO_APP_BINARY);
    const locks = path.join(appRoot, "config", "locks");
    const archiveRoot = path.join(appRoot, "synthetic-archive");
    const tweetId = "912345678901234567";
    await mkdir(path.join(archiveRoot, "Tweets", tweetId), { recursive: true });
    const document = JSON.stringify({ schema_version: 1, kind: "tw2tg.archive-records", records: [{
      account_handle: "collector", final_path: `Tweets/${tweetId}`,
      metadata: { schema_version: 1, tweet_id: tweetId, url: `https://x.test/i/status/${tweetId}`,
        tweet_type: "original", author: { username: "alice" }, text: "synthetic",
        archived_at: "2026-10-18T00:00:00Z", media: [] },
    }] });
    const preview = await browser.executeAsync((recordsJson, root, done) => {
      window.__TAURI__.core.invoke("preview_record_import", { recordsJson, archiveRoot: root })
        .then(done, (error) => done({ error: String(error) }));
    }, document, archiveRoot);
    assert.equal(preview.error, undefined);
    assert.equal(preview.entries[0].verdict, "accept");
    const holder = spawn(process.env.WDIO_WORKSPACE_HOLDER, ["--exact", "active_workspace_holder", "--nocapture"], {
      env: { ...process.env, TW2TG_ACTIVE_WORKSPACE_HOLDER: locks }, stdio: "ignore", windowsHide: true,
    });
    try {
      await browser.waitUntil(async () => {
        if (holder.exitCode !== null) throw new Error(`workspace holder exited ${holder.exitCode}`);
        return access(path.join(locks, "active-holder.ready")).then(() => true, () => false);
      }, { timeout: 10000, timeoutMsg: "workspace holder did not become ready" });
      const waiting = await browser.executeAsync((recordsJson, archiveDirectory, expectedRevision, done) => {
        const invoke = window.__TAURI__.core.invoke;
        window.__f3Apply = invoke("accept_record_import", { recordsJson, acceptedJson: '[{"index":0}]',
          tweetId: "", archiveDirectory, expectedRevision }).then((value) => ({ value }), (error) => ({ error: String(error) }));
        Promise.race([window.__f3Apply.then(() => "finished"), new Promise((resolve) => setTimeout(() => resolve("waiting"), 300))]).then(done);
      }, document, archiveRoot, preview.revision);
      assert.equal(waiting, "waiting");
      const cancelled = await browser.executeAsync((done) => {
        window.__TAURI__.core.invoke("cancel_record_import")
          .then(() => window.__f3Apply).then(done, (error) => done({ error: String(error) }));
      });
      assert.match(cancelled.error ?? "", /workspace maintenance unavailable/);
      const after = await browser.executeAsync((recordsJson, root, done) => {
        window.__TAURI__.core.invoke("preview_record_import", { recordsJson, archiveRoot: root })
          .then(done, (error) => done({ error: String(error) }));
      }, document, archiveRoot);
      assert.equal(after.error, undefined);
      assert.equal(after.entries[0].verdict, "accept");
    } finally {
      if (holder.exitCode === null && holder.signalCode === null) {
        const exited = new Promise((resolve) => holder.once("exit", resolve));
        holder.kill();
        await exited;
      }
    }
    const accepted = await browser.executeAsync((recordsJson, archiveDirectory, expectedRevision, done) => {
      window.__TAURI__.core.invoke("accept_record_import", { recordsJson, acceptedJson: '[{"index":0}]',
        tweetId: "", archiveDirectory, expectedRevision }).then(done, (error) => done({ error: String(error) }));
    }, document, archiveRoot, preview.revision);
    assert.equal(accepted.error, undefined);
    assert.equal(accepted.accepted, 1);
  });
});
