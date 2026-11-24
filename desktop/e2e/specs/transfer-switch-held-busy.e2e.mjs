import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

// Opt-in actual Desktop regression: a held key must not queue a second write
// while the transfer switch is busy (disabled), and the single accepted change
// must survive the settle. Requires an isolated portable root whose marker proves
// the fixture, and WDIO_TRANSFER_HELD_BUSY_GUI=1.
describe("transfer selection held busy", () => {
  it("ignores repeated activation while saving and keeps exactly one applied choice", async function () {
    if (process.platform !== "win32" || process.env.WDIO_TRANSFER_HELD_BUSY_GUI !== "1" || !process.env.XARCHIVE_PORTABLE_ROOT) this.skip();
    const root = process.env.XARCHIVE_PORTABLE_ROOT;
    assert.equal(await readFile(path.join(root, "w7-transfer-held-busy-fixture.marker"), "utf8"), "isolated-transfer-held-busy");
    await waitForApplicationDocument();
    await waitForStartupContract();
    const status = () => browser.executeAsync((done) => window.__TAURI__.core.invoke("get_app_status").then((value) => done({ value }), (error) => done({ error: String(error) })));
    assert.equal((await status()).value.use_aria2, false);
    await (await browser.$("button=下载配置")).click();
    await (await browser.$("#transfer-backend-settings .settings-panel-toggle")).click();
    const toggle = await browser.$("#transfer-backend-settings input[role=switch]");
    // Start the save and immediately hold Enter: one non-repeat keydown plus
    // autorepeats must all be refused while the control is disabled.
    await browser.execute(() => {
      const input = document.querySelector("#transfer-backend-settings input[role=switch]");
      input.focus();
      input.click();
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", repeat: false, bubbles: true, cancelable: true }));
      for (let index = 0; index < 4; index += 1) {
        input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", repeat: true, bubbles: true, cancelable: true }));
      }
    });
    assert.equal(await toggle.isEnabled(), false, "the switch must be disabled while the save is in flight");
    await browser.waitUntil(async () => (await status()).value?.use_aria2 === true && await toggle.isEnabled(), { timeout: 15000 });
    // Settle window: a queued second write would flip the value back here.
    await browser.pause(1000);
    assert.equal((await status()).value.use_aria2, true);
    assert.equal(await toggle.isSelected(), true);
    assert.match(await readFile(path.join(root, "config", "config.yaml"), "utf8"), /use_aria2:\s*true/);
    // The document body held focus while disabled, so the switch regains it.
    assert.equal(await browser.execute(() => document.activeElement === document.querySelector("#transfer-backend-settings input[role=switch]")), true);
  });
});
