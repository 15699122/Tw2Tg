import { spawnSync } from "node:child_process";

const command = process.platform === "win32" ? "wdio.cmd" : "wdio";
const result = spawnSync(command, ["run", "wdio.conf.mjs"], {
  cwd: new URL("..", import.meta.url),
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

process.exit(result.status ?? 1);