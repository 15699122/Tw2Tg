import { spawn } from "node:child_process";
import { once } from "node:events";
import assert from "node:assert/strict";
import { describe, it } from "node:test";

import service, { isPidAlive, killTree, parseListeningPids, launcher } from "../scripts/wdio-tauri-service.mjs";

const NETSTAT_SAMPLE = [
  "",
  "Active Connections",
  "",
  "  Proto  Local Address          Foreign Address        State           PID",
  "  TCP    127.0.0.1:4444         0.0.0.0:0              LISTENING       36640",
  "  TCP    127.0.0.1:4445         0.0.0.0:0              LISTENING       36640",
  "  TCP    [::]:4444              [::]:0                 LISTENING       36640",
  "  TCP    127.0.0.1:4444         127.0.0.1:52000        TIME_WAIT       0",
  "  TCP    127.0.0.1:44445        0.0.0.0:0              LISTENING       999",
  "  TCP    127.0.0.1:52000        127.0.0.1:4444         ESTABLISHED     4242",
  "  UDP    0.0.0.0:5353           *:*                                    5555",
  "",
].join("\r\n");

describe("parseListeningPids", () => {
  it("extracts listener PIDs on the requested port only", () => {
    assert.deepEqual(parseListeningPids(NETSTAT_SAMPLE, 4444), [36640]);
    assert.deepEqual(parseListeningPids(NETSTAT_SAMPLE, 4445), [36640]);
  });

  it("ignores non-listen states and look-alike ports", () => {
    assert.deepEqual(parseListeningPids(NETSTAT_SAMPLE, 4444), [36640]);
    assert.deepEqual(parseListeningPids(NETSTAT_SAMPLE, 44445), [999]);
    assert.deepEqual(parseListeningPids(NETSTAT_SAMPLE, 52000), []);
    assert.deepEqual(parseListeningPids(NETSTAT_SAMPLE, 5353), []);
  });

  it("returns an empty list for output without listeners", () => {
    assert.deepEqual(parseListeningPids("", 4444), []);
    assert.deepEqual(parseListeningPids("garbage lines\r\nonly", 4444), []);
  });
});

describe("isPidAlive", () => {
  it("reports the current process as alive", () => {
    assert.equal(isPidAlive(process.pid), true);
  });

  it("reports an exited child process as not alive", async () => {
    const child = spawn(process.execPath, ["-e", "process.exit(0)"]);
    await once(child, "exit");
    assert.equal(isPidAlive(child.pid), false);
  });

  it("rejects non-positive PIDs", () => {
    assert.equal(isPidAlive(0), false);
    assert.equal(isPidAlive(-1), false);
  });
});

describe("service module shape", () => {
  it("exposes the worker service as default and the launcher as named export", () => {
    assert.equal(typeof service, "function");
    assert.equal(typeof launcher, "function");
    assert.equal(typeof service.prototype.beforeCommand, "function");
    assert.equal(typeof service.prototype.afterSession, "function");
    assert.equal(typeof launcher.prototype.onComplete, "function");
    assert.equal(typeof launcher.prototype.collectDriverPids, "function");
    assert.equal(typeof launcher.prototype.reapSurvivorDrivers, "function");
  });
});

describe("killTree", () => {
  it("terminates a spawned child process", async () => {
    const child = spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)"], {
      stdio: "ignore",
    });
    await once(child, "spawn");
    assert.equal(isPidAlive(child.pid), true);
    // On Windows, killTree resolves on taskkill's own 'close' event, which
    // can fire after the victim already emitted 'exit'. Attach the exit
    // listeners BEFORE killTree so the termination evidence cannot be
    // missed — attaching afterwards hangs until the timeout and fails the
    // assertion even though the process was killed (2026-09-16 Windows
    // re-validation: 7 passed, 1 failed with the late attachment).
    const exitEvidence = Promise.race([
      once(child, "exit"),
      once(child, "close"),
    ]).then(
      () => true,
      () => false,
    );
    const result = await killTree(child.pid);
    const exited = await Promise.race([
      exitEvidence,
      new Promise((resolve) => setTimeout(() => resolve(false), 10000)),
    ]);
    assert.equal(result, true);
    assert.equal(exited, true);
    assert.equal(isPidAlive(child.pid), false);
  });
});
