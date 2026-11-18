import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

describe("Windows runtime log filtering", () => {
  it("retains the selected level while the log refreshes", async function () {
    if (process.platform !== "win32" || process.env.WDIO_LOG_FILTER !== "1") this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    await (await browser.$("button=运行日志")).click();
    const level = await browser.$("#log-level");
    const search = await browser.$("#log-search");
    const log = await browser.$('[aria-label="运行日志内容"]');
    const marker = `synthetic_log_${randomUUID()}`;
    await level.selectByAttribute("value", "error");
    await search.setValue(marker);
    const result = await browser.executeAsync((message, done) => {
      window.__TAURI__.core.invoke("log_frontend_event", { event: { event: "validation_log_filter", message } })
        .then(() => done({ ok: true }), (error) => done({ error: String(error) }));
    }, marker);
    assert.equal(result.error, undefined);
    // The page refreshes every second. A unique marker avoids matching prior runs.
    await browser.pause(2500);
    assert.equal(await level.getValue(), "error");
    assert.equal((await log.getText()).includes(marker), false);
    await level.selectByAttribute("value", "info");
    await browser.waitUntil(async () => (await log.getText()).includes(marker), { timeout: 5000 });
    assert.equal(await (await browser.$("button=复制可见日志")).isEnabled(), true);
  });

  it("supports keyboard interaction between the log level and search", async function () {
    if (process.platform !== "win32" || process.env.WDIO_LOG_FILTER !== "1") this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    await (await browser.$("button=\u8fd0\u884c\u65e5\u5fd7")).click();
    const level = await browser.$("#log-level");
    const search = await browser.$("#log-search");
    await browser.execute(() => document.querySelector("#log-level").focus());
    await browser.keys(["End"]);
    assert.equal(await level.getValue(), "silent");
    assert.equal(await level.isFocused(), true);
    await browser.keys(["Tab"]);
    assert.equal(await search.isFocused(), true);
    await browser.keys("keyboard_validation_marker");
    assert.equal(await search.getValue(), "keyboard_validation_marker");
    await browser.keys(["Control", "a"]);
    await browser.keys(["Backspace"]);
    assert.equal(await search.getValue(), "");
  });

});
