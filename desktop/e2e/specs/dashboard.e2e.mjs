import assert from "node:assert/strict";

async function collectStartupEvidence() {
  const evidence = {
    url: await browser.getUrl().catch(() => "<unavailable>"),
    readyState: await browser.execute(() => document.readyState).catch(() => "<unavailable>"),
    startupState: await browser.execute(() => document.documentElement?.dataset?.xarchiveStartup || "").catch(() => "<unavailable>"),
    rootExists: await browser.execute(() => Boolean(document.getElementById("root"))).catch(() => false),
    rootText: await browser.execute(() => document.getElementById("root")?.innerText?.slice(0, 512) || "").catch(() => "<unavailable>"),
    startupFallback: await browser.execute(() => document.getElementById("startup-fallback")?.innerText || "").catch(() => "<unavailable>"),
  };
  console.log("[xarchive-startup-evidence]", JSON.stringify(evidence));
  return evidence;
}

async function waitForDashboard() {
  try {
    await browser.waitUntil(
      async () => (await browser.$("h1").isExisting()) && (await browser.$("h1").isDisplayed()),
      { timeout: 20000, timeoutMsg: "XArchive dashboard heading did not become visible" },
    );
  } catch (error) {
    const evidence = await collectStartupEvidence();
    await browser.saveScreenshot("test-artifacts/wdio/dashboard-startup-failure.png").catch(() => {});
    console.error("[xarchive-startup-failure]", error.message, JSON.stringify(evidence));
    throw error;
  }
}

describe("XArchive Tauri desktop smoke", () => {
  before(async () => {
    await waitForDashboard();
  });

  it("completes the frontend startup contract before rendering the dashboard", async () => {
    const evidence = await collectStartupEvidence();
    assert.equal(evidence.readyState, "complete");
    assert.equal(evidence.rootExists, true);
    assert.equal(evidence.startupState, "react_mount_completed");
    assert.equal(evidence.startupFallback, "");
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