import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const desktopDir = fileURLToPath(new URL("..", import.meta.url));
const tauriCli = fileURLToPath(new URL("../../node_modules/@tauri-apps/cli/tauri.js", import.meta.url));
const result = spawnSync(process.execPath, [
  tauriCli,
  "build",
  "--config",
  "src-tauri/tauri.wdio.conf.json",
  "--features",
  "wdio-e2e",
], {
  cwd: desktopDir,
  env: {
    ...process.env,
    VITE_WDIO_E2E: "1",
  },
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}

process.exit(result.status === null ? 1 : result.status);