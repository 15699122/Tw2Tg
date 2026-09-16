import assert from "node:assert/strict";

async function waitForDashboard() {
  await browser.waitUntil(
    async () => (await browser.$("h1").isExisting()) && (await browser.$("h1").isDisplayed()),
    {
      timeout: 20000,
      timeoutMsg: "XArchive dashboard heading did not become visible",
    },
  );
}

describe("XArchive Tauri desktop smoke", () => {
  before(async () => {
    await waitForDashboard();
  });

  it("renders the dashboard shell in the native window", async () => {
    assert.equal(await browser.$("h1").getText(), "工作台");
    assert.equal(await browser.$("main").isDisplayed(), true);
    assert.equal(await browser.$('[aria-label="主导航"]').isExisting(), true);
  });

  it("exposes the stable dashboard regions used by Windows checks", async () => {
    assert.equal(await browser.$('[aria-label="归档概览"]').isExisting(), true);
    assert.equal(await browser.$("button").isExisting(), true);
    assert.equal(await browser.$("body").isDisplayed(), true);
  });
});