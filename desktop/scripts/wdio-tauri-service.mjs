import { spawn } from "node:child_process";
import TauriWorkerService, { launcher as TauriLauncherService } from "@wdio/tauri-service";

/**
 * Project WDIO worker adapter.
 *
 * The official service's focus recovery and session teardown assume that
 * tauri-plugin-wdio is present in every native artifact. This project
 * intentionally keeps the plugin out of ordinary release builds, so the
 * adapter retains the official launcher while making worker cleanup safe for
 * both ordinary and wdio-e2e artifacts.
 */
export default class Tw2TgTauriWorkerService extends TauriWorkerService {
  async beforeCommand() {
    // This project has one stable main window. The upstream focus probe calls
    // plugin:wdio|get_window_states before DOM commands, which is unavailable
    // in the ordinary artifact by design.
  }

  async afterSession() {
    const browser = this.browser;
    if (!browser || browser.isMultiremote) {
      return;
    }

    if (browser.sessionId) {
      await browser.deleteSession();
    }
  }
}

/**
 * Launcher teardown safety net.
 *
 * `@wdio/native-core`'s DriverProcess.stop() kills only the direct
 * tauri-driver child process; it has no process-tree teardown (the same
 * library uses `taskkill /T /F` for dev servers). The 2026-09-16 Windows
 * re-validation showed both advanced and ordinary runs left tauri-driver,
 * its msedgedriver child and the 4444/4445 listeners alive after a
 * successful, exit-code-0 run. This launcher snapshots driver PIDs and the
 * driver port owners before the upstream teardown, then tree-kills anything
 * that survived it. A manual Stop-Process on the validation machine is
 * environment recovery, not a PASS; this safety net is the automated fix.
 */
const DRIVER_TEARDOWN_CONFIRM_MS = 10000;
const DRIVER_TEARDOWN_EXIT_EVENT_MS = 3000;
const DRIVER_TEARDOWN_POLL_MS = 250;

export function isPidAlive(pid) {
  // PID 0 means "every process in our process group" for kill(2) — it must
  // never be treated as a live driver PID.
  if (!Number.isInteger(pid) || pid <= 0) {
    return false;
  }
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

export function killTree(pid) {
  if (!Number.isInteger(pid) || pid <= 0) {
    return Promise.resolve(false);
  }
  if (process.platform === "win32") {
    return new Promise((resolve) => {
      const child = spawn("taskkill", ["/pid", String(pid), "/T", "/F"], {
        stdio: "ignore",
      });
      child.once("error", () => resolve(false));
      child.once("close", (code) => resolve(code === 0));
    });
  }
  try {
    process.kill(pid, "SIGKILL");
    return Promise.resolve(true);
  } catch {
    return Promise.resolve(false);
  }
}

/**
 * Extract PIDs of TCP sockets in LISTEN state on `port` from
 * `netstat -ano -p tcp` output. Returns [] when the port has no listener.
 */
export function parseListeningPids(netstatOutput, port) {
  const pids = new Set();
  const suffix = `:${port}`;
  for (const line of netstatOutput.split(/\r?\n/)) {
    const columns = line.trim().split(/\s+/);
    if (columns.length < 5) {
      continue;
    }
    const [protocol, localAddress, , state, pidText] = columns;
    if (!/^tcp$/i.test(protocol) || !/^listen/i.test(state ?? "")) {
      continue;
    }
    if (!localAddress || !localAddress.toLowerCase().endsWith(suffix)) {
      continue;
    }
    const pid = Number.parseInt(pidText, 10);
    if (Number.isInteger(pid) && pid > 0) {
      pids.add(pid);
    }
  }
  return [...pids];
}

function listeningPids(port) {
  if (process.platform !== "win32") {
    // POSIX teardown relies on the PID snapshot; the Linux native WDIO run is
    // currently BLOCKED_AUTOMATION, so there is no validated Linux leftover.
    return Promise.resolve([]);
  }
  return new Promise((resolve) => {
    const child = spawn("netstat", ["-ano", "-p", "tcp"], {
      stdio: ["ignore", "pipe", "ignore"],
    });
    let output = "";
    child.stdout.on("data", (chunk) => {
      output += chunk;
    });
    child.once("error", () => resolve([]));
    child.once("close", () => resolve(parseListeningPids(output, port)));
  });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function onceExit(child) {
  return new Promise((resolve) => {
    child.once("exit", () => resolve(true));
  });
}

/**
 * Wait until a tracked driver PID is really gone.
 *
 * A requested termination (taskkill /T /F included) reports success before
 * the OS finishes reaping, so a fixed alive-check window misjudges both
 * ways: it can FAIL a run whose survivor died a moment later, or PASS one
 * whose PID was already reused by the OS. The authoritative signal is the
 * child 'exit' event when we still hold the handle; the final arbitration is
 * whether a tracked driver port is still LISTENing. A stale (reused) PID
 * with no listener is no longer our driver and must not fail the run.
 */
export async function waitForProcessGone(
  pid,
  { child = null, verifyGone = null, timeoutMs = DRIVER_TEARDOWN_CONFIRM_MS } = {},
) {
  if (!isPidAlive(pid)) {
    return true;
  }
  if (child) {
    const exited = await Promise.race([
      onceExit(child),
      sleep(DRIVER_TEARDOWN_EXIT_EVENT_MS).then(() => false),
    ]);
    if (exited) {
      return true;
    }
  }
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!isPidAlive(pid)) {
      return true;
    }
    await sleep(DRIVER_TEARDOWN_POLL_MS);
  }
  if (typeof verifyGone === "function") {
    return (await verifyGone()) === true;
  }
  return !isPidAlive(pid);
}

export class Tw2TgTauriLauncherService extends TauriLauncherService {
  async onComplete(exitCode, config, capabilities) {
    // Snapshot before super.onComplete(): the upstream stopAll() clears the
    // driver pool, and a killed-but-reaped PID or dynamically allocated port
    // would otherwise be invisible.
    const trackedPorts = this.driverPorts();
    const trackedPids = new Set([
      ...this.collectDriverPids(),
      ...(await this.collectPortOwnerPids(trackedPorts)),
    ]);
    await super.onComplete(exitCode, config, capabilities);
    await this.reapSurvivorDrivers(trackedPids, trackedPorts);
  }

  driverPorts() {
    // PortManager may skip the configured base when another process already
    // owns it. Snapshot the actual pair(s) allocated by @wdio/tauri-service
    // before upstream onComplete clears the pool; otherwise the Windows
    // cleanup fallback only watches the preferred 4444/4445 pair and can
    // leave a driver listening on an ephemeral port.
    try {
      const identifiers = this.driverPool?.getStatus?.().identifiers ?? [];
      const allocated = [];
      for (const identifier of identifiers) {
        const driver = this.driverPool?.getDriver?.(identifier);
        for (const port of [driver?.port, driver?.nativePort]) {
          if (Number.isInteger(port) && port > 0) allocated.push(port);
        }
      }
      if (allocated.length > 0) return [...new Set(allocated)];
    } catch {
      // Fall back to the configured pair when the upstream pool is unavailable.
    }
    const basePort = Number(this.options?.tauriDriverPort) || 4444;
    return [basePort, basePort + 1];
  }

  collectDriverPids() {
    try {
      const pids = this.driverPool?.getRunningPids?.() ?? [];
      return pids.filter((pid) => Number.isInteger(pid) && pid > 0);
    } catch {
      return [];
    }
  }

  async collectPortOwnerPids(ports = this.driverPorts()) {
    const pids = [];
    for (const port of ports) {
      for (const pid of await listeningPids(port)) {
        pids.push(pid);
      }
    }
    return pids;
  }

  portListenerCheck(trackedPorts) {
    return async () => {
      if (process.platform !== "win32") {
        return true;
      }
      for (const port of trackedPorts) {
        const owners = await listeningPids(port);
        if (owners.length > 0) {
          return false;
        }
      }
      return true;
    };
  }

  async reapSurvivorDrivers(trackedPids, trackedPorts = []) {
    const survivors = [...trackedPids].filter((pid) => isPidAlive(pid));
    if (survivors.length === 0) {
      return;
    }
    console.warn(
      `[wdio-tauri-service] ${survivors.length} driver process(es) survived ` +
        `upstream teardown; tree-killing: ${survivors.join(", ")}`,
    );
    const verifyGone = this.portListenerCheck(trackedPorts);
    await Promise.all(survivors.map((pid) => killTree(pid)));
    const gone = await Promise.all(
      survivors.map((pid) => waitForProcessGone(pid, { verifyGone })),
    );
    const failed = survivors.filter((_, index) => !gone[index]);
    if (failed.length > 0) {
      throw new Error(
        `[wdio-tauri-service] failed to confirm driver process(es) gone after teardown: ` +
          `${failed.join(", ")}`,
      );
    }
  }
}

export { Tw2TgTauriLauncherService as launcher };
