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

// Additive Windows E2E channels (WQ-P1-16/WQ-P1-17 unblock path). All three are
// opt-in: unset, every option and the capability shape stay exactly as before.
// `autoDownloadEdgeDriver` keeps its previous Windows default and gains an
// explicit opt-out, because a preinstalled driver pinned by the validation
// recipe must not be shadowed by a downloaded one.
const autoDownloadEdgeDriver =
  process.platform === "win32"
    ? process.env.WDIO_AUTO_DOWNLOAD_EDGE_DRIVER !== "0"
    : false;
const tauriDriverPath = process.env.TAURI_DRIVER_PATH
  ? resolveFromConfigDir(process.env.TAURI_DRIVER_PATH)
  : undefined;
const edgeDriverPath = process.env.EDGEDRIVER_PATH
  ? resolveFromConfigDir(process.env.EDGEDRIVER_PATH)
  : undefined;
const fixedRuntimeFolder = process.env.WEBVIEW2_BROWSER_EXECUTABLE_FOLDER;

// tauri-driver resolves `msedgedriver.exe` by name from PATH on Windows, so a
// directory-pinned driver has to be prepended before the service starts.
if (process.platform === "win32" && edgeDriverPath) {
  if (!fs.existsSync(edgeDriverPath)) {
    throw new Error("EdgeDriver executable not found at " + edgeDriverPath);
  }
  const edgeDriverDir = path.dirname(edgeDriverPath);
  const pathEntries = (process.env.PATH ?? "").split(path.delimiter);
  if (!pathEntries.some((entry) => entry.toLowerCase() === edgeDriverDir.toLowerCase())) {
    process.env.PATH = [edgeDriverDir, ...pathEntries].filter(Boolean).join(path.delimiter);
  }
}

export const config = {
  runner: "local",
  specs: advancedSpecs,
  maxInstances: 1,
  services: [[serviceModule, {
    appBinaryPath,
    driverProvider,
    autoDownloadEdgeDriver,
    autoInstallTauriDriver,
    tauriDriverPort: Number(process.env.TAURI_DRIVER_PORT ?? 4444),
    captureBackendLogs: captureLogs,
    captureFrontendLogs: captureLogs,
    logDir,
    ...(edgeDriverVersion ? { edgeDriverVersion } : {}),
    ...(tauriDriverPath ? { tauriDriverPath } : {}),
    ...(edgeDriverPath ? { nativeDriverPath: edgeDriverPath } : {}),
    ...(fixedRuntimeFolder
      ? { env: { WEBVIEW2_BROWSER_EXECUTABLE_FOLDER: fixedRuntimeFolder } }
      : {}),
  }]],
  capabilities: [{
    browserName: "tauri",
    "tauri:options": {
      application: appBinaryPath,
      // Windows drives the app through the WebView2 runtime; the empty object
      // is the upstream-documented shape for native WebView2 sessions.
      ...(process.platform === "win32" ? { webviewOptions: {} } : {}),
    },
    ...(edgeBinaryProbe
      ? { "ms:edgeOptions": { binary: appBinaryPath, webviewOptions: {} } }
      : {}),
  }],
  logLevel: process.env.WDIO_LOG_LEVEL ?? "info",
  // The service's log capture reads the WDIO config `outputDir`
  // (`_config.outputDir || join(process.cwd(), 'logs')`); the service option
  // `logDir` only applies on the standalone `init()` path, which this runner
  // mode does not use. Pointing `outputDir` at the same directory as
  // `WDIO_LOG_DIR` keeps evidence and service logs together instead of
  // scattering them into `desktop/logs`.
  outputDir: logDir,
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

export {
  appBinaryPath,
  configDir,
  logDir,
  autoDownloadEdgeDriver,
  autoInstallTauriDriver,
  edgeBinaryProbe,
  edgeDriverPath,
  edgeDriverVersion,
  fixedRuntimeFolder,
  tauriDriverPath,
};
