import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { readFile, access } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

describe("Windows configuration cancellation", () => {
  it("cancels an IPC apply waiting for an active workspace and reopens admission", async function () {
    if (process.platform !== "win32" || !process.env.WDIO_WORKSPACE_HOLDER) this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    const root = path.dirname(process.env.WDIO_APP_BINARY);
    const locks = path.join(root, "config", "locks");
    const config = path.join(root, "config", "config.yaml");
    const readConfig = async () => readFile(config).catch((error) => {
      if (error.code === "ENOENT") return null;
      throw error;
    });
    const candidate = JSON.stringify({ schema_version: 1, kind: "tw2tg.configuration", settings: { log_max_files: 11 } });
    const preview = await browser.executeAsync((document, done) => {
      window.__TAURI__.core.invoke("preview_configuration_exchange", { document }).then(done, (error) => done({ error: String(error) }));
    }, candidate);
    assert.equal(preview.error, undefined);
    const before = await readConfig();
    const holder = spawn(process.env.WDIO_WORKSPACE_HOLDER, ["--exact", "active_workspace_holder", "--nocapture"], {
      env: { ...process.env, TW2TG_ACTIVE_WORKSPACE_HOLDER: locks }, stdio: "ignore", windowsHide: true,
    });
    try {
      await browser.waitUntil(async () => {
        if (holder.exitCode !== null) throw new Error(`workspace holder exited ${holder.exitCode}`);
        return access(path.join(locks, "active-holder.ready")).then(() => true, () => false);
      }, { timeout: 10000, timeoutMsg: "workspace holder did not become ready" });
      const waiting = await browser.executeAsync((document, expectedRevision, done) => {
        const invoke = window.__TAURI__.core.invoke;
        window.__w3Apply = invoke("apply_configuration_exchange", { document, expectedRevision })
          .then((value) => ({ value }), (error) => ({ error: String(error) }));
        Promise.race([window.__w3Apply.then(() => "finished"), new Promise((resolve) => setTimeout(() => resolve("waiting"), 300))]).then(done);
      }, candidate, preview.revision);
      assert.equal(waiting, "waiting");
      const cancelled = await browser.executeAsync((done) => {
        window.__TAURI__.core.invoke("cancel_configuration_exchange")
          .then(() => window.__w3Apply).then(done, (error) => done({ error: String(error) }));
      });
      assert.match(cancelled.error ?? "", /cancelled/);
      assert.doesNotMatch(cancelled.error, /lock failure|timed out/);
      assert.deepEqual(await readConfig(), before);
    } finally {
      if (holder.exitCode === null && holder.signalCode === null) {
        const exited = new Promise((resolve) => holder.once("exit", resolve));
        holder.kill();
        await exited;
      }
    }
    const applied = await browser.executeAsync((document, expectedRevision, done) => {
      window.__TAURI__.core.invoke("apply_configuration_exchange", { document, expectedRevision }).then(done, (error) => done({ error: String(error) }));
    }, candidate, preview.revision);
    assert.equal(applied.error, undefined);
    assert.match((await readConfig()).toString("utf8"), /max_files:\s*11/);
  });
});
