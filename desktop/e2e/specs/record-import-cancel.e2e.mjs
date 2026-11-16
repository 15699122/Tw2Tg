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
    await (await browser.$('button=记录导入')).click();
    await (await browser.$('#record-import-records')).setValue(document);
    await (await browser.$('#record-import-root')).setValue(archiveRoot);
    await (await browser.$('button=预览导入')).click();
    const selection = await browser.$(`input[aria-label="选择 collector/${tweetId}"]`);
    await browser.waitUntil(async () => await selection.isSelected(), { timeout: 10000 });
    const holder = spawn(process.env.WDIO_WORKSPACE_HOLDER, ["--exact", "active_workspace_holder", "--nocapture"], {
      env: { ...process.env, TW2TG_ACTIVE_WORKSPACE_HOLDER: locks }, stdio: "ignore", windowsHide: true,
    });
    try {
      await browser.waitUntil(async () => {
        if (holder.exitCode !== null) throw new Error(`workspace holder exited ${holder.exitCode}`);
        return access(path.join(locks, "active-holder.ready")).then(() => true, () => false);
      }, { timeout: 10000, timeoutMsg: "workspace holder did not become ready" });
      await (await browser.$('button=接受导入')).click();
      const cancelButton = await browser.$('button=取消等待');
      await cancelButton.waitForDisplayed({ timeout: 10000 });
      await cancelButton.click();
      await browser.waitUntil(async () => (await browser.$('main').getText()).includes("已取消等待；记录未导入。"),
        { timeout: 10000, timeoutMsg: "cancelled import was not reported in the interface" });
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
