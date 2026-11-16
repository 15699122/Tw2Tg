import assert from "node:assert/strict";
import { lstat, mkdir, readFile, rmdir, unlink } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

describe("Windows configuration maintenance failure", () => {
  it("reports a lock I/O failure without writing and permits a later apply", async function () {
    if (process.platform !== "win32" || process.env.WDIO_INDUCE_LOCK_FAILURE !== "1") this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    const root = path.dirname(process.env.WDIO_APP_BINARY);
    assert.equal(await readFile(path.join(root, "w3-fixture.marker"), "utf8"), "w3-lock-failure-fixture");
    const config = path.join(root, "config", "config.yaml");
    const lock = path.join(root, "config", "locks", "workspace-admission.lock");
    const readConfig = async () => readFile(config).catch((error) => {
      if (error.code === "ENOENT") return null;
      throw error;
    });
    const candidate = JSON.stringify({ schema_version: 1, kind: "tw2tg.configuration", settings: { log_max_files: 13 } });
    const preview = await browser.executeAsync((document, done) => {
      window.__TAURI__.core.invoke("preview_configuration_exchange", { document }).then(done, (error) => done({ error: String(error) }));
    }, candidate);
    assert.equal(preview.error, undefined);
    const before = await readConfig();
    const exportedBefore = await browser.executeAsync((done) => {
      window.__TAURI__.core.invoke("export_configuration_exchange").then(done, (error) => done({ error: String(error) }));
    });
    assert.equal(typeof exportedBefore, "string");
    const existing = await lstat(lock).catch((error) => {
      if (error.code === "ENOENT") return null;
      throw error;
    });
    if (existing) {
      assert.ok(existing.isFile() && existing.size === 0, "fixture lock is not an empty regular file");
      await unlink(lock);
    }
    await mkdir(lock);
    try {
      const result = await browser.executeAsync((document, expectedRevision, done) => {
        window.__TAURI__.core.invoke("apply_configuration_exchange", { document, expectedRevision })
          .then((value) => done({ value }), (error) => done({ error: String(error) }));
      }, candidate, preview.revision);
      assert.match(result.error ?? "", /workspace maintenance unavailable/);
      assert.doesNotMatch(result.error, /cancelled|timed out/);
      assert.deepEqual(await readConfig(), before);
      const exportedAfter = await browser.executeAsync((done) => {
        window.__TAURI__.core.invoke("export_configuration_exchange").then(done, (error) => done({ error: String(error) }));
      });
      assert.equal(exportedAfter, exportedBefore);
    } finally {
      await rmdir(lock);
    }
    const applied = await browser.executeAsync((document, expectedRevision, done) => {
      window.__TAURI__.core.invoke("apply_configuration_exchange", { document, expectedRevision }).then(done, (error) => done({ error: String(error) }));
    }, candidate, preview.revision);
    assert.equal(applied.error, undefined);
    assert.match((await readConfig()).toString("utf8"), /max_files:\s*13/);
  });
});
