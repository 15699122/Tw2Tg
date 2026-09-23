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
const autoInstallTauriDriver = process.env.WDIO_AUTO_INSTALL_TAURI_DRIVER === "1";
const autoDownloadEdgeDriver = process.env.WDIO_AUTO_DOWNLOAD_EDGE_DRIVER === "1";
const edgeDriverVersion = process.env.EDGEDRIVER_VERSION;
const advancedSpecs = process.env.WDIO_ADVANCED === "1"
  ? ["./e2e/specs/**/*.e2e.mjs"]
  : ["./e2e/specs/dashboard.e2e.mjs"];

export const config = {
  runner: "local",
  specs: advancedSpecs,
  maxInstances: 1,
  services: [[serviceModule, {
    appBinaryPath,
    driverProvider,
    autoDownloadEdgeDriver,
    autoInstallTauriDriver,
    ...(edgeDriverVersion ? { edgeDriverVersion } : {}),
    tauriDriverPort: Number(process.env.TAURI_DRIVER_PORT ?? 4444),
    captureBackendLogs: captureLogs,
    captureFrontendLogs: captureLogs,
    logDir,
  }]],
  capabilities: [{
    browserName: "tauri",
    "tauri:options": {
      application: appBinaryPath,
    },
  }],
  logLevel: process.env.WDIO_LOG_LEVEL ?? "info",
  // WQ-P0-WHITE-04B: @wdio/tauri-service 的日志捕获读取 WDIO config 的
  // outputDir（依赖源码 dist/esm/index.js: `_config.outputDir ||
  // join(process.cwd(), 'logs')`）；service 选项 logDir 只在 standalone
  // init() 路径生效，本项目的 runner 模式不走该路径。必须显式指向与
  // WDIO_LOG_DIR 相同的目录，否则日志会落到 desktop/logs。
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
  edgeDriverVersion,
};