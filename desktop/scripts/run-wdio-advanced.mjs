import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const desktopDir = fileURLToPath(new URL("..", import.meta.url));
const wdioCli = fileURLToPath(new URL("../../node_modules/@wdio/cli/bin/wdio.js", import.meta.url));
const result = spawnSync(process.execPath, [wdioCli, "run", "wdio.conf.mjs"], {
  cwd: desktopDir,
  env: {
    ...process.env,
    WDIO_ADVANCED: "1",
    WDIO_CAPTURE_LOGS: "1",
  },
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}

process.exit(result.status === null ? 1 : result.status);