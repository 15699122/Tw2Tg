import test from "node:test";
import assert from "node:assert/strict";
import { displayFileName, extensionSidebarState, aria2StatusText, proxyModeOption, proxyStatusText, proxyCoverageText, proxyBackendLabel, proxyPacStateText, systemProxyRows, validateManualProxy } from "../src/lib/ui-state.js";

test("displayFileName extracts the file name from windows-style paths", () => {
  assert.equal(displayFileName("C:\\tools\\aria2\\aria2c.exe"), "aria2c.exe");
});

test("displayFileName extracts the file name from posix-style paths", () => {
  assert.equal(displayFileName("/home/user/sidecar/gallery-dl/gallery-dl"), "gallery-dl");
});

test("displayFileName keeps a bare file name unchanged", () => {
  assert.equal(displayFileName("gallery-dl.exe"), "gallery-dl.exe");
});

test("displayFileName reports a missing path", () => {
  assert.equal(displayFileName(""), "未检测到路径");
  assert.equal(displayFileName(null), "未检测到路径");
});

test("extensionSidebarState shows checking during initial load", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: false, browserConnection: "unknown", initialLoad: true }),
    { tone: "muted", text: "检测中…" },
  );
});

test("extensionSidebarState does not treat files-ready as browser checking", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: true, browserConnection: "not_loaded", initialLoad: false }),
    { tone: "error", text: "未连接" },
  );
});

test("extensionSidebarState identifies an unregistered Native Host", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: true, browserConnection: "not_loaded", nativeHost: "not_registered", initialLoad: false }),
    { tone: "error", text: "Host 未注册" },
  );
});

test("extensionSidebarState reserves checking for an active refresh", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: true, browserConnection: "not_loaded", initialLoad: false, checking: true }),
    { tone: "muted", text: "检测中…" },
  );
});

test("extensionSidebarState shows connected only for an explicit connected state", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: true, browserConnection: "connected", initialLoad: false }),
    { tone: "online", text: "已连接" },
  );
});

test("extensionSidebarState shows missing files instead of a stuck checking state", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: false, browserConnection: "unknown", initialLoad: false }),
    { tone: "error", text: "文件缺失" },
  );
});

test("extensionSidebarState falls back to not connected when files are ready", () => {
  assert.deepEqual(
    extensionSidebarState({ filesReady: true, browserConnection: "disconnected", initialLoad: false }),
    { tone: "error", text: "未连接" },
  );
});

test("aria2StatusText prefers the detected version", () => {
  assert.equal(
    aria2StatusText({ found: true, version: "1.37.0", path: null, source: null, error: null }),
    "v1.37.0",
  );
});

test("aria2StatusText explains a missing installation", () => {
  assert.equal(aria2StatusText(null), "当前程序未找到 aria2c");
  assert.equal(
    aria2StatusText({ found: false, version: null, path: null, source: null, error: null }),
    "当前程序未找到 aria2c",
  );
});

test("proxyModeOption labels every mode and falls back for an unknown value", () => {
  for (const value of ["system", "direct", "manual"]) {
    assert.equal(proxyModeOption(value).value, value);
  }
  assert.equal(proxyModeOption("pac").value, "system");
  assert.equal(proxyModeOption(undefined).value, "system");
});

test("proxyStatusText reports a manual proxy as in effect with a redacted summary", () => {
  const status = proxyStatusText({
    proxy_mode: "manual",
    proxy_configured: true,
    proxy_active: true,
    proxy_summary: "http://[REDACTED]@proxy.example:8080",
  });
  assert.equal(status.tone, "online");
  assert.match(status.text, /正在使用/);
  assert.match(status.text, /proxy\.example:8080/);
  assert.ok(!status.text.includes("s3cret"));
});

test("proxyStatusText warns when manual mode has no value", () => {
  const status = proxyStatusText({
    proxy_mode: "manual",
    proxy_configured: false,
    proxy_active: false,
    proxy_summary: null,
  });
  assert.equal(status.tone, "warning");
  assert.match(status.text, /需要填写/);
});

test("proxyStatusText never claims an inactive stored proxy is in effect", () => {
  for (const mode of ["system", "direct"]) {
    const status = proxyStatusText({
      proxy_mode: mode,
      proxy_configured: true,
      proxy_active: false,
      proxy_summary: null,
    });
    assert.equal(status.tone, "muted", `${mode} must not report an active proxy`);
    assert.match(status.text, /不会使用它/);
  }
});

test("proxyStatusText explains that direct clears inherited variables", () => {
  const status = proxyStatusText({
    proxy_mode: "direct",
    proxy_configured: false,
    proxy_active: false,
    proxy_summary: null,
  });
  assert.equal(status.tone, "online");
  assert.match(status.text, /直连/);
});

test("proxyCoverageText states the boundary and never promises unsupported PAC support", () => {
  const note = proxyCoverageText({
    system_proxy_note: "System follows the environment.",
    system_proxy_supported: false,
  });
  assert.match(note, /暂不支持 PAC\/WPAD/);
  assert.match(note, /始终直连/);

  const supported = proxyCoverageText({
    system_proxy_note: "System uses the Windows resolver.",
    system_proxy_supported: true,
  });
  assert.match(supported, /支持按 URL 解析 PAC 与 WPAD/);
  assert.ok(!supported.includes("暂不支持"));
});

test("proxyCoverageText returns nothing when the backend sent no note", () => {
  assert.equal(proxyCoverageText(null), "");
  assert.equal(proxyCoverageText({ system_proxy_supported: true }), "");
});

test("validateManualProxy only applies to manual mode", () => {
  assert.equal(validateManualProxy("system", ""), "");
  assert.equal(validateManualProxy("direct", "not a url"), "");
});

test("validateManualProxy rejects values the backend would reject", () => {
  assert.match(validateManualProxy("manual", "   "), /需要填写/);
  assert.match(validateManualProxy("manual", "http://a b:8080"), /不能包含空格/);
  assert.match(validateManualProxy("manual", "proxy.example:8080"), /形如/);
  assert.match(validateManualProxy("manual", "file:///etc/passwd"), /形如/);
});

test("validateManualProxy accepts the supported schemes including credentials", () => {
  for (const value of [
    "http://proxy.example:8080",
    "https://proxy.example:8443",
    "socks5://127.0.0.1:1080",
    "socks5h://127.0.0.1:1080",
    "http://alice:s3cret@proxy.example:8080",
  ]) {
    assert.equal(validateManualProxy("manual", value), "", `${value} must be accepted`);
  }
});

test("the resolver label separates the OS policy from an inherited variable", () => {
  assert.match(proxyBackendLabel("windows-os"), /Windows/);
  assert.match(proxyBackendLabel("environment"), /环境变量/);
  assert.match(proxyBackendLabel("something-new"), /未知/);
});

test("an unsupported PAC state is not reported as disabled", () => {
  assert.notEqual(proxyPacStateText("unsupported"), proxyPacStateText("disabled"));
  assert.equal(proxyPacStateText(undefined), "未报告");
  assert.match(proxyPacStateText("error-download"), /失败/);
  // An unknown future state must be shown, not silently dropped.
  assert.match(proxyPacStateText("brand-new-state"), /brand-new-state/);
});

test("the system summary omits fields the platform cannot report", () => {
  const rows = systemProxyRows({ backend: "windows-os", pac_state: "unconfigured" });
  const labels = rows.map((row) => row.label);
  assert.ok(labels.includes("解析来源"));
  assert.ok(labels.includes("PAC 状态"));
  assert.ok(!labels.includes("PAC 地址"), "an absent PAC URL must not be invented");
  assert.ok(!labels.includes("自动发现"), "WPAD off must not be implied as a row");
});

test("the system summary redacts nothing but trusts the backend and shows every value", () => {
  const rows = systemProxyRows({
    backend: "windows-os",
    auto_detect: true,
    pac_state: "available",
    pac_url: "http://pac.example/proxy.pac",
    static_proxies: ["PROXY proxy.example:8080"],
    bypass: ["localhost"],
    system_bypass: "localhost;*.local",
    environment_proxy: false,
  });
  const byLabel = Object.fromEntries(rows.map((row) => [row.label, row.value]));
  assert.equal(byLabel["PAC 地址"], "http://pac.example/proxy.pac");
  assert.equal(byLabel["系统静态代理"], "PROXY proxy.example:8080");
  assert.equal(byLabel["环境免代理"], "localhost");
  assert.equal(byLabel["系统免代理"], "localhost;*.local");
  assert.equal(byLabel["环境变量代理"], "未设置");
  assert.match(byLabel["自动发现"], /WPAD/);
});

test("the system summary is empty without a backend answer", () => {
  assert.deepEqual(systemProxyRows(null), []);
});
