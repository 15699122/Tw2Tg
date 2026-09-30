import test from "node:test";
import assert from "node:assert/strict";
import { filterLogEntries, mergeLogLines, parseLogEntries } from "../src/lib/log-lines.js";

test("parses and filters application log levels", () => {
  const entries = parseLogEntries([
    "2026-09-17T12:00:01Z info app: started",
    "2026-09-17T12:00:02Z warning app: degraded",
    "2026-09-17T12:00:03Z error app: failed",
  ]);
  assert.equal(filterLogEntries(entries, "warning").length, 2);
  assert.equal(filterLogEntries(entries, "error").length, 1);
});

test("merges polled log snapshots without duplicate lines and keeps a bound", () => {
  assert.deepEqual(
    mergeLogLines(["a", "b"], ["b", "c", "d"], 3),
    ["b", "c", "d"],
  );
});