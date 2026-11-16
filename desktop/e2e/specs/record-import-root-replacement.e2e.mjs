import assert from "node:assert/strict";
import { mkdir, rename } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

describe("Windows record-import root replacement", () => {
  it("rejects acceptance when the previewed directory is replaced", async function () {
    if (process.platform !== "win32" || process.env.WDIO_F3_ROOT_REPLACEMENT !== "1") this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    const root = path.join(path.dirname(process.env.WDIO_APP_BINARY), "replacement-archive");
    const id = "912345678901234570";
    const directory = path.join(root, "Tweets", id);
    await mkdir(directory, { recursive: true });
    const document = JSON.stringify({ schema_version: 1, kind: "tw2tg.archive-records", records: [{
      account_handle: "collector", final_path: `Tweets/${id}`,
      metadata: { schema_version: 1, tweet_id: id, url: `https://x.test/i/status/${id}`,
        tweet_type: "original", author: { username: "alice" }, text: "synthetic",
        archived_at: "2026-10-18T00:00:00Z", media: [] },
    }] });
    const preview = await browser.executeAsync((recordsJson, archiveRoot, done) => {
      window.__TAURI__.core.invoke("preview_record_import", { recordsJson, archiveRoot })
        .then((value) => done({ value }), (error) => done({ error: String(error) }));
    }, document, root);
    assert.equal(preview.error, undefined);
    assert.equal(preview.value.entries[0].verdict, "accept");
    await rename(root, `${root}-original`);
    await mkdir(directory, { recursive: true });
    const outcome = await browser.executeAsync((recordsJson, archiveDirectory, expectedRevision, done) => {
      window.__TAURI__.core.invoke("accept_record_import", { recordsJson, acceptedJson: '[{"index":0}]',
        tweetId: "", archiveDirectory, expectedRevision })
        .then((value) => done({ value }), (error) => done({ error: String(error) }));
    }, document, root, preview.value.revision);
    assert.match(outcome.error ?? "", /preview is stale/);
    const after = await browser.executeAsync((recordsJson, archiveRoot, done) => {
      window.__TAURI__.core.invoke("preview_record_import", { recordsJson, archiveRoot })
        .then((value) => done({ value }), (error) => done({ error: String(error) }));
    }, document, root);
    assert.equal(after.error, undefined);
    assert.equal(after.value.entries[0].verdict, "accept");
  });
});
