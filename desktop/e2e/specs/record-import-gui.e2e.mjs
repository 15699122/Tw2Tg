import assert from "node:assert/strict";
import { copyFile, mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

describe("Windows record-import interface", () => {
  it("previews evidence and imports only the selected valid record", async function () {
    if (process.platform !== "win32" || process.env.WDIO_F3_GUI !== "1") this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    const root = path.join(path.dirname(process.env.WDIO_APP_BINARY), "gui-archive");
    const validId = "912345678901234568";
    const missingId = "912345678901234569";
    await mkdir(path.join(root, "Tweets", validId), { recursive: true });
    const record = (id) => ({ account_handle: "collector", final_path: `Tweets/${id}`,
      metadata: { schema_version: 1, tweet_id: id, url: `https://x.test/i/status/${id}`,
        tweet_type: "original", author: { username: "alice" }, text: "synthetic",
        archived_at: "2026-10-18T00:00:00Z", media: [] } });
    const document = JSON.stringify({ schema_version: 1, kind: "tw2tg.archive-records", records: [record(validId), record(missingId)] });
    await (await browser.$('button=记录导入')).click();
    await (await browser.$('#record-import-records')).setValue(document);
    await (await browser.$('#record-import-root')).setValue(root);
    await (await browser.$('button=预览导入')).click();
    await browser.waitUntil(async () => (await browser.$$('table.preview-table tbody tr')).length === 2,
      { timeout: 10000, timeoutMsg: "record preview did not render" });
    const valid = await browser.$(`input[aria-label="选择 collector/${validId}"]`);
    const missing = await browser.$(`input[aria-label="选择 collector/${missingId}"]`);
    await browser.waitUntil(async () => await valid.isSelected(), { timeout: 5000 });
    assert.equal(await missing.isEnabled(), false);
    await (await browser.$('button=接受导入')).click();
    await browser.waitUntil(async () => (await browser.$('main').getText()).includes("已提交：1 条接受，0 条相同"),
      { timeout: 10000, timeoutMsg: "record import did not commit the selection" });
    const result = await browser.executeAsync((tweetId, archiveRoot, done) => {
      window.__TAURI__.core.invoke("export_record_exchange", { jobId: "", tweetId, format: "json", kind: "archive", archiveRoot })
        .then((value) => done({ value }), (error) => done({ error: String(error) }));
    }, validId, root);
    assert.equal(result.error, undefined);
    assert.equal(JSON.parse(result.value).records[0].metadata.tweet_id, validId);

    const diskDownload = process.env.WDIO_F3_DISK_GUI === "1";
    const downloadRoot = path.join(path.dirname(process.env.WDIO_APP_BINARY), "downloads");
    if (diskDownload) {
      await mkdir(downloadRoot);
      // Require the driver to confirm the isolated destination before clicking
      // the export button; never fall back to the user's Downloads directory.
      const response = await fetch(`http://127.0.0.1:${process.env.WDIO_DIRECT_DRIVER_PORT}/session/${browser.sessionId}/ms/cdp/execute`, {
        method: "POST", headers: { "content-type": "application/json" },
        body: JSON.stringify({ cmd: "Browser.setDownloadBehavior", params: { behavior: "allow", downloadPath: downloadRoot } }),
      });
      const result = await response.json();
      assert.ok(response.ok && !result.value?.error, `isolated download setup failed: ${JSON.stringify(result)}`);
    } else {
      await browser.execute(() => {
        window.__recordDownload = null;
        URL.createObjectURL = (blob) => { window.__recordBlob = blob; return "blob:captured-record"; };
        HTMLAnchorElement.prototype.click = function () {
          if (this.download) window.__recordDownload = this.download;
        };
      });
    }
    await (await browser.$('#record-export-tweet')).setValue(validId);
    await (await browser.$('#record-export-kind')).selectByAttribute('value', 'archive');
    await (await browser.$('button=导出 JSON')).click();
    if (diskDownload) {
      const output = path.join(downloadRoot, "archive-records.json");
      let body;
      await browser.waitUntil(async () => {
        try { body = await readFile(output, "utf8"); return true; } catch (error) {
          if (error.code === "ENOENT") return false;
          throw error;
        }
      }, { timeout: 10000, timeoutMsg: "record export file was not saved" });
      assert.equal(JSON.parse(body).records[0].metadata.tweet_id, validId);
      await (await browser.$('button=导出 YAML')).click();
      const yamlOutput = path.join(downloadRoot, "archive-records.yaml");
      let yaml;
      await browser.waitUntil(async () => {
        try { yaml = await readFile(yamlOutput, "utf8"); return true; } catch (error) {
          if (error.code === "ENOENT") return false;
          throw error;
        }
      }, { timeout: 10000, timeoutMsg: "YAML record export file was not saved" });
      const yamlPreview = await browser.executeAsync((recordsJson, archiveRoot, done) => {
        window.__TAURI__.core.invoke("preview_record_import", { recordsJson, archiveRoot })
          .then((value) => done({ value }), (error) => done({ error: String(error) }));
      }, yaml, root);
      assert.equal(yamlPreview.error, undefined);
      assert.equal(yamlPreview.value.entries.length, 1);
      assert.equal(yamlPreview.value.entries[0].identity, `collector/${validId}`);
      if (process.env.WDIO_F3_EVIDENCE_DIR) {
        await copyFile(output, path.join(process.env.WDIO_F3_EVIDENCE_DIR, "archive-records.json"));
        await copyFile(yamlOutput, path.join(process.env.WDIO_F3_EVIDENCE_DIR, "archive-records.yaml"));
      }
    } else {
      await browser.waitUntil(async () => await browser.execute(() => window.__recordDownload !== null),
        { timeout: 10000, timeoutMsg: "record export download was not generated" });
      const download = await browser.executeAsync((done) => {
        window.__recordBlob.text().then((body) => done({ name: window.__recordDownload, body }));
      });
      assert.equal(download.name, "archive-records.json");
      assert.equal(JSON.parse(download.body).records[0].metadata.tweet_id, validId);
    }
  });
});
