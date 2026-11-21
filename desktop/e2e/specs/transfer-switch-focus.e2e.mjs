import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

describe("transfer selection keyboard focus", () => {
  it("retains focus after Space and restores the original choice with Enter", async function () {
    if (process.platform !== "win32" || process.env.WDIO_TRANSFER_SWITCH_GUI !== "1" || !process.env.XARCHIVE_PORTABLE_ROOT) this.skip();
    const root = process.env.XARCHIVE_PORTABLE_ROOT;
    assert.equal(await readFile(path.join(root, "w7-transfer-switch-fixture.marker"), "utf8"), "isolated-transfer-switch");
    await waitForApplicationDocument();
    await waitForStartupContract();
    const status = () => browser.executeAsync((done) => window.__TAURI__.core.invoke("get_app_status").then((value) => done({ value }), (error) => done({ error: String(error) })));
    assert.equal((await status()).value.use_aria2, false);
    await (await browser.$("button=下载配置")).click();
    await (await browser.$("#transfer-backend-settings .settings-panel-toggle")).click();
    const toggle = await browser.$("#transfer-backend-settings input[role=switch]");
    await browser.execute(() => document.querySelector("#transfer-backend-settings input[role=switch]").focus());
    await browser.keys("Space");
    await browser.waitUntil(async () => (await status()).value?.use_aria2 === true && await toggle.isEnabled(), { timeout: 15000 });
    assert.equal(await toggle.isSelected(), true);
    const config = path.join(root, "config", "config.yaml");
    assert.match(await readFile(config, "utf8"), /use_aria2:\s*true/);
    await browser.waitUntil(() => browser.execute(() => document.activeElement === document.querySelector("#transfer-backend-settings input[role=switch]")), { timeout: 5000 });
    await browser.keys("Enter");
    await browser.waitUntil(async () => (await status()).value?.use_aria2 === false && await toggle.isEnabled(), { timeout: 15000 });
    assert.equal(await toggle.isSelected(), false);
    assert.match(await readFile(config, "utf8"), /use_aria2:\s*false/);
  });
});
