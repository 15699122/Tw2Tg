import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const configDir = path.dirname(fileURLToPath(import.meta.url));
const platformBinaryName = process.platform === "win32"
  ? "xarchive-desktop.exe"
  : "xarchive-desktop";
const defaultBinaryPath = "target/release/" + platformBinaryName;

function resolveFromConfigDir(value) {
  return path.isAbsolute(value) ? value : path.resolve(configDir, value);
}

const appBinaryPath = resolveFromConfigDir(
  process.env.WDIO_APP_BINARY ?? "../" + defaultBinaryPath,
);
const logDir = resolveFromConfigDir(
  process.env.WDIO_LOG_DIR ?? "test-artifacts/wdio",
);
const serviceModule = fileURLToPath(new URL("./scripts/wdio-tauri-service.mjs", import.meta.url));
const driverProvider = process.env.TAURI_DRIVER_PROVIDER ?? "external";
const captureLogs = process.env.WDIO_CAPTURE_LOGS === "1";
const autoInstallTauriDriver = process.env.WDIO_AUTO_INSTALL_TAURI_DRIVER !== "0";
const advancedSpecs = process.env.WDIO_ADVANCED === "1"
  ? ["./e2e/specs/**/*.e2e.mjs"]
  : ["./e2e/specs/dashboard.e2e.mjs"];
// The service otherwise derives the driver from the Edge browser version in
// the Windows registry, while a Tauri app is driven through the WebView2
// runtime. Pinning the driver lets a validation run target the runtime the app
// actually loads (WQ-ENG-09b); unset keeps the upstream detection.
const edgeDriverVersion =
  process.env.TAURI_DRIVER_EDGE_VERSION ?? process.env.EDGEDRIVER_VERSION;
// WQ-ENG-09b probe. Upstream tauri-driver moves `tauri:options.application` out
// of the W3C alwaysMatch into the legacy desiredCapabilities field, which
// webdriver 9.x never sends, so msedgedriver falls back to msedge.exe and the
// session opens a blank Edge window instead of this app. Setting
// WDIO_EDGE_BINARY_PROBE=1 additionally declares the same binary through the W3C
// ms:edgeOptions key, so a run can observe whether tauri-driver forwards the
// caller's alwaysMatch to msedgedriver at all. `browserName` stays "tauri"
// because @wdio/tauri-service rejects any other value in onPrepare (Windows
// batch 3), and the key is omitted when the probe is off so the default
// capability shape is byte-for-byte what it was before.
const edgeBinaryProbe = process.env.WDIO_EDGE_BINARY_PROBE === "1";

export const config = {
  runner: "local",
  specs: advancedSpecs,
  maxInstances: 1,
  services: [[serviceModule, {
    appBinaryPath,
    driverProvider,
    autoDownloadEdgeDriver: process.platform === "win32",
    autoInstallTauriDriver,
    tauriDriverPort: Number(process.env.TAURI_DRIVER_PORT ?? 4444),
    captureBackendLogs: captureLogs,
    captureFrontendLogs: captureLogs,
    logDir,
    ...(edgeDriverVersion ? { edgeDriverVersion } : {}),
  }]],
  capabilities: [{
    browserName: "tauri",
    "tauri:options": {
      application: appBinaryPath,
    },
    ...(edgeBinaryProbe
      ? { "ms:edgeOptions": { binary: appBinaryPath, webviewOptions: {} } }
      : {}),
  }],
  logLevel: process.env.WDIO_LOG_LEVEL ?? "info",
  framework: "mocha",
  reporters: ["spec"],
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 2,
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
  onPrepare: async () => {
    if (!fs.existsSync(appBinaryPath)) {
      throw new Error(
        "Tauri application binary not found at " + appBinaryPath + ". " +
        "Run npm run build:tauri first or set WDIO_APP_BINARY.",
      );
    }
    fs.mkdirSync(logDir, { recursive: true });
  },
};

export { appBinaryPath, configDir, logDir };