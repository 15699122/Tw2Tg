import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

// CROSS_PLATFORM_CHANGE_REQUIRED: the existing revision binds local bytes only.
// Opt in on a disposable portable root to reproduce and later verify the fix.
describe("configuration exchange preview binding", () => {
  it("rejects an edited document with the earlier preview revision", async function () {
    if (process.platform !== "win32" || process.env.WDIO_F2_BINDING_GUI !== "1" || !process.env.XARCHIVE_PORTABLE_ROOT) this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    assert.equal(await readFile(path.join(process.env.XARCHIVE_PORTABLE_ROOT, "w3-binding-fixture.marker"), "utf8"), "isolated-w3-binding");
    const document = (count) => JSON.stringify({ schema_version: 1, kind: "tw2tg.configuration", settings: { log_max_files: count } });
    const config = path.join(process.env.XARCHIVE_PORTABLE_ROOT, "config", "config.yaml");
    const readConfig = () => readFile(config).catch((error) => { if (error.code === "ENOENT") return null; throw error; });
    const exported = () => browser.executeAsync((done) => {
      window.__TAURI__.core.invoke("export_configuration_exchange").then((value) => done({ value }), (error) => done({ error: String(error) }));
    });
    const before = await readConfig();
    const runtimeBefore = await exported();
    assert.equal(runtimeBefore.error, undefined);
    const preview = await browser.executeAsync((input, done) => {
      window.__TAURI__.core.invoke("preview_configuration_exchange", { document: input }).then((value) => done({ value }), (error) => done({ error: String(error) }));
    }, document(11));
    assert.equal(preview.error, undefined);
    const result = await browser.executeAsync((input, expectedRevision, done) => {
      window.__TAURI__.core.invoke("apply_configuration_exchange", { document: input, expectedRevision }).then((value) => done({ value }), (error) => done({ error: String(error) }));
    }, document(12), preview.value.revision);
    assert.match(result.error ?? "", /stale|preview/i, "edited document must not use the prior preview");
    assert.deepEqual(await readConfig(), before);
    assert.deepEqual(await exported(), runtimeBefore);
  });
});
