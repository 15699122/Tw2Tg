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

// The probe and the driver pin are opt-in; start from a clean environment so a
// variable left over in the caller's shell cannot change the expected shape.
function cleanEnvironment(environment) {
  const env = { ...process.env, WDIO_APP_BINARY: appBinary, ...environment };
  for (const key of ["WDIO_EDGE_BINARY_PROBE", "TAURI_DRIVER_EDGE_VERSION", "EDGEDRIVER_VERSION"]) {
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
    "const value = part === \"capabilities\" ? config.capabilities : config.services[0][1];" +
    "process.stdout.write(JSON.stringify(value));";
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

describe("wdio.conf capabilities", () => {
  it("keeps the default capability shape free of ms:edgeOptions", () => {
    const [cap] = loadCapabilities({});
    assert.equal(cap.browserName, "tauri");
    assert.deepEqual(cap["tauri:options"], { application: appBinary });
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
