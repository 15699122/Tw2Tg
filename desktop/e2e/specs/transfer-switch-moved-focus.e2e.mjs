import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

// Opt-in actual Desktop regression: if the user moves focus away while the save
// is busy, the settle must not steal it back to the switch, and a key pressed on
// the newly focused control must not reach the switch. Requires an isolated
// portable root whose marker proves the fixture, and WDIO_TRANSFER_MOVED_FOCUS_GUI=1.
describe("transfer selection moved focus", () => {
  it("does not reclaim focus or re-apply the choice when focus moved during the save", async function () {
    if (process.platform !== "win32" || process.env.WDIO_TRANSFER_MOVED_FOCUS_GUI !== "1" || !process.env.XARCHIVE_PORTABLE_ROOT) this.skip();
    const root = process.env.XARCHIVE_PORTABLE_ROOT;
    assert.equal(await readFile(path.join(root, "w7-transfer-moved-focus-fixture.marker"), "utf8"), "isolated-transfer-moved-focus");
    await waitForApplicationDocument();
    await waitForStartupContract();
    const status = () => browser.executeAsync((done) => window.__TAURI__.core.invoke("get_app_status").then((value) => done({ value }), (error) => done({ error: String(error) })));
    assert.equal((await status()).value.use_aria2, false);
    await (await browser.$("button=下载配置")).click();
    await (await browser.$("#transfer-backend-settings .settings-panel-toggle")).click();
    const toggle = await browser.$("#transfer-backend-settings input[role=switch]");
    // Start the save and move focus in the same task, so the move happens before
    // the busy settle regardless of how fast the local write completes.
    await browser.execute(() => {
      const input = document.querySelector("#transfer-backend-settings input[role=switch]");
      input.focus();
      input.click();
      document.querySelector("#transfer-backend-settings .settings-panel-toggle").focus();
    });
    await browser.waitUntil(async () => (await status()).value?.use_aria2 === true && await toggle.isEnabled(), { timeout: 15000 });
    await browser.pause(500);
    assert.equal(await browser.execute(() => document.activeElement === document.querySelector("#transfer-backend-settings .settings-panel-toggle")), true, "focus must stay where the user moved it");
    assert.equal(await browser.execute(() => document.activeElement === document.querySelector("#transfer-backend-settings input[role=switch]")), false);
    // Enter belongs to the focused panel toggle (collapse), not to the switch.
    await browser.keys("Enter");
    await browser.pause(500);
    assert.equal((await status()).value.use_aria2, true);
    assert.match(await readFile(path.join(root, "config", "config.yaml"), "utf8"), /use_aria2:\s*true/);
  });
});
