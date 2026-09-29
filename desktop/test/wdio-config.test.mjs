import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const desktopDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const configUrl = pathToFileURL(path.join(desktopDir, "wdio.conf.mjs")).href;
// A stable absolute stand-in for the built app binary; the config only resolves
// paths here, it never launches anything at import time.
const appBinary = path.resolve(desktopDir, "target", "release", "xarchive-desktop");

// The probe, the driver pin and the additive Windows channels are opt-in; start
// from a clean environment so a variable left over in the caller's shell cannot
// change the expected shape.
const OPT_IN_KEYS = [
  "WDIO_EDGE_BINARY_PROBE",
  "TAURI_DRIVER_EDGE_VERSION",
  "EDGEDRIVER_VERSION",
  "TAURI_DRIVER_PATH",
  "EDGEDRIVER_PATH",
  "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
  "WDIO_AUTO_DOWNLOAD_EDGE_DRIVER",
  "WDIO_DIRECT_DRIVER",
  "WDIO_DIRECT_DRIVER_PORT",
];

function cleanEnvironment(environment) {
  const env = { ...process.env, WDIO_APP_BINARY: appBinary, ...environment };
  for (const key of OPT_IN_KEYS) {
    if (!(key in environment)) delete env[key];
  }
  return env;
}

// `wdio.conf.mjs` reads its environment while the module is being evaluated, so
// every case loads it in a throwaway process instead of mutating this one. The
// part to read is named by key because a function cannot cross the process
// boundary.
function loadConfigPart(part, environment) {
  const source =
    "const config = (await import(process.argv[1])).config;" +
    "const part = process.argv[2];" +
    "const value = part === \"capabilities\" ? config.capabilities :" +
    " part === \"serviceOptions\" ? config.services[0][1] : config[part];" +
    // JSON.stringify(undefined) returns undefined and would break the pipe for
    // parts that are intentionally absent in the current mode (e.g. hostname
    // without WDIO_DIRECT_DRIVER); null keeps them readable and distinct.
    "process.stdout.write(JSON.stringify(value === undefined ? null : value));";
  const result = spawnSync(
    process.execPath,
    ["--input-type=module", "-e", source, configUrl, part],
    {
      cwd: desktopDir,
      encoding: "utf8",
      env: cleanEnvironment(environment),
    },
  );
  assert.equal(result.status, 0, "config failed to load: " + result.stderr);
  return JSON.parse(result.stdout);
}

const loadCapabilities = (environment) => loadConfigPart("capabilities", environment);
const loadServiceOptions = (environment) => loadConfigPart("serviceOptions", environment);

// `tauri:options` carries `webviewOptions: {}` on Windows only, where the app is
// driven through the WebView2 runtime. Keep the expectation platform-aware so
// the same suite is valid on both platforms (the 2026-09-28 WQ-ENG-13 lesson).
const expectedTauriOptions = {
  application: appBinary,
  ...(process.platform === "win32" ? { webviewOptions: {} } : {}),
};

describe("wdio.conf capabilities", () => {
  it("keeps the default capability shape free of ms:edgeOptions", () => {
    const [cap] = loadCapabilities({});
    assert.equal(cap.browserName, "tauri");
    assert.deepEqual(cap["tauri:options"], expectedTauriOptions);
    assert.equal("ms:edgeOptions" in cap, false);
  });

  it("declares the app binary through ms:edgeOptions when the probe is enabled", () => {
    const [cap] = loadCapabilities({ WDIO_EDGE_BINARY_PROBE: "1" });
    assert.deepEqual(cap["ms:edgeOptions"], { binary: appBinary, webviewOptions: {} });
    // Single-variable experiment: the probe must not move the app path, only
    // mirror it, so tauri:options keeps pointing at the very same binary.
    assert.equal(cap["tauri:options"].application, appBinary);
  });

  it("keeps browserName inside the set @wdio/tauri-service accepts", () => {
    // Regression guard for the Windows batch 3 probe, which set
    // browserName: "webview2" and was rejected by the service in onPrepare
    // before any WebDriver session was created.
    for (const environment of [{}, { WDIO_EDGE_BINARY_PROBE: "1" }]) {
      const [cap] = loadCapabilities(environment);
      assert.ok(["tauri", "wry"].includes(cap.browserName),
        "browserName " + cap.browserName + " is rejected by @wdio/tauri-service");
    }
  });

  it("only forwards a driver version pin when it is explicitly configured", () => {
    assert.equal("edgeDriverVersion" in loadServiceOptions({}), false);
    assert.equal(
      loadServiceOptions({ TAURI_DRIVER_EDGE_VERSION: "153.0.4234.46" }).edgeDriverVersion,
      "153.0.4234.46",
    );
  });
});

describe("wdio.conf additive Windows channels", () => {
  const driverStandIn = path.join(desktopDir, "package.json");

  it("passes no driver paths or env overrides by default", () => {
    const options = loadServiceOptions({});
    assert.equal("tauriDriverPath" in options, false);
    assert.equal("nativeDriverPath" in options, false);
    assert.equal("env" in options, false);
    // The Windows default is preserved; the opt-out is what changes it.
    assert.equal(options.autoDownloadEdgeDriver, process.platform === "win32");
  });

  it("resolves an explicitly configured tauri-driver and EdgeDriver path", () => {
    const options = loadServiceOptions({
      TAURI_DRIVER_PATH: driverStandIn,
      EDGEDRIVER_PATH: driverStandIn,
    });
    assert.equal(options.tauriDriverPath, driverStandIn);
    assert.equal(options.nativeDriverPath, driverStandIn);
  });

  it("resolves driver paths relative to the config directory", () => {
    const options = loadServiceOptions({ TAURI_DRIVER_PATH: "package.json" });
    assert.equal(options.tauriDriverPath, driverStandIn);
  });

  it("forwards a fixed WebView2 runtime folder through the service env", () => {
    const options = loadServiceOptions({
      WEBVIEW2_BROWSER_EXECUTABLE_FOLDER: "C:\\WebView2Runtime",
    });
    assert.deepEqual(options.env, {
      WEBVIEW2_BROWSER_EXECUTABLE_FOLDER: "C:\\WebView2Runtime",
    });
  });

  it("honours the auto-download opt-out", () => {
    const options = loadServiceOptions({ WDIO_AUTO_DOWNLOAD_EDGE_DRIVER: "0" });
    assert.equal(options.autoDownloadEdgeDriver, false);
  });

  it("keeps the service log directory identical to the WDIO config outputDir", () => {
    // The service's log capture reads `outputDir`; a mismatch scatters evidence
    // into `desktop/logs` instead of WDIO_LOG_DIR (WQ-P0-WHITE-04B).
    assert.equal(loadServiceOptions({}).logDir, path.join(desktopDir, "test-artifacts", "wdio"));
    const configured = path.join(desktopDir, "custom-logs");
    assert.equal(loadServiceOptions({ WDIO_LOG_DIR: "custom-logs" }).logDir, configured);
    assert.equal(loadConfigPart("outputDir", { WDIO_LOG_DIR: "custom-logs" }), configured);
    assert.equal(
      loadConfigPart("outputDir", {}),
      path.join(desktopDir, "test-artifacts", "wdio"),
    );
  });
});

describe("wdio.conf direct msedgedriver mode (WQ-ENG-09b-ORD reviewed recipe)", () => {
  it("drops @wdio/tauri-service and addresses the recipe-started driver", () => {
    const env = { WDIO_DIRECT_DRIVER: "1" };
    assert.deepEqual(loadConfigPart("services", env), []);
    assert.equal(loadConfigPart("hostname", env), "127.0.0.1");
    assert.equal(loadConfigPart("port", env), 4445);
    assert.equal(
      loadConfigPart("port", { ...env, WDIO_DIRECT_DRIVER_PORT: "45460" }),
      45460,
    );
  });

  it("emits the round-3 proven W3C capability shape", () => {
    const [cap] = loadCapabilities({ WDIO_DIRECT_DRIVER: "1" });
    assert.equal(cap.browserName, "webview2");
    assert.deepEqual(cap["ms:edgeOptions"], { binary: appBinary });
    assert.deepEqual(cap.webviewOptions, {});
    // tauri:options is tauri-driver currency; msedgedriver must not see it, and
    // the batch-3 lesson forbids browserName webview2 only while the
    // tauri-service contract is in play — direct mode has no service.
    assert.equal("tauri:options" in cap, false);
  });

  it("keeps the default path free of direct-mode settings", () => {
    assert.equal(loadConfigPart("hostname", {}), null);
    assert.equal(loadConfigPart("port", {}), null);
    const [cap] = loadCapabilities({});
    assert.equal(cap.browserName, "tauri");
    assert.equal(JSON.stringify(cap).includes("webview2"), false);
  });
});
