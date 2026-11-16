import assert from "node:assert/strict";
import { waitForApplicationDocument, waitForStartupContract } from "../support/native-startup.mjs";

describe("Windows system theme", () => {
  it("follows the system preference while preserving explicit overrides", async function () {
    if (process.platform !== "win32" || process.env.WDIO_DIRECT_DRIVER !== "1") this.skip();
    await waitForApplicationDocument();
    await waitForStartupContract();
    const endpoint = `http://127.0.0.1:${Number(process.env.WDIO_DIRECT_DRIVER_PORT ?? 4445)}/session/${browser.sessionId}/ms/cdp/execute`;
    const original = await browser.execute(() => document.documentElement.dataset.theme ?? "");
    async function check(system, preference, expected) {
      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ cmd: "Emulation.setEmulatedMedia", params: {
          features: [{ name: "prefers-color-scheme", value: system }],
        } }),
      });
      assert.equal(response.ok, true, await response.text());
      const result = await browser.execute((value) => {
        const root = document.documentElement;
        if (value) root.dataset.theme = value;
        else delete root.dataset.theme;
        return {
          systemDark: matchMedia("(prefers-color-scheme: dark)").matches,
          surface: getComputedStyle(root).getPropertyValue("--surface").trim(),
        };
      }, preference);
      assert.equal(result.systemDark, system === "dark");
      assert.equal(result.surface, expected);
    }
    try {
      await check("dark", "", "#17191c");
      await check("light", "", "#fff");
      await check("dark", "light", "#fff");
      await check("light", "dark", "#17191c");
    } finally {
      await fetch(endpoint, { method: "POST", headers: { "content-type": "application/json" },
        body: JSON.stringify({ cmd: "Emulation.setEmulatedMedia", params: { features: [] } }) });
      await browser.execute((value) => {
        if (value) document.documentElement.dataset.theme = value;
        else delete document.documentElement.dataset.theme;
      }, original);
    }
  });
});
