import assert from "node:assert/strict";

describe("XArchive Tauri WebdriverIO plugin", () => {
  before(async () => {
    await browser.waitUntil(
      async () => (await browser.$("h1").isExisting()) && (await browser.$("h1").isDisplayed()),
      { timeout: 20000, timeoutMsg: "XArchive dashboard did not become visible" },
    );
  });

  it("exposes the Tauri plugin API and executes frontend code", async () => {
    const pluginAvailable = await browser.tauri.execute(
      () => Boolean(window.wdioTauri && typeof window.wdioTauri.execute === "function"),
    );
    assert.equal(pluginAvailable, true);
    assert.equal(await browser.tauri.execute(() => document.querySelector("h1")?.textContent), "工作台");
  });

  it("mocks a Tauri command and restores the interception", async () => {
    const mock = await browser.tauri.mock("get_app_status");
    await mock.mockReturnValue({ app_name: "WDIO", database: "mocked" });

    const mocked = await browser.tauri.execute(({ core }) => core.invoke("get_app_status"));
    assert.deepEqual(mocked, { app_name: "WDIO", database: "mocked" });

    await browser.tauri.restoreAllMocks();
  });

  after(async () => {
    if (globalThis.browser?.tauri?.restoreAllMocks) {
      await browser.tauri.restoreAllMocks();
    }
  });
});