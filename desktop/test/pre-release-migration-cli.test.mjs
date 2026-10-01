import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  planMigration,
  readMigrationLedger,
} from "../scripts/pre-release-migration-cli.mjs";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const ledgerPath = join(
  projectRoot,
  "docs",
  "release",
  "migration",
  "pre-release-asset-ledger.json",
);

async function loadLedger() {
  return readMigrationLedger(JSON.parse(await readFile(ledgerPath, "utf8")));
}

test("frozen ledger covers both retired series with unique target tags", async () => {
  const ledger = await loadLedger();
  const sources = ledger.entries.map((entry) => entry.source_tag);
  const targets = ledger.entries.map((entry) => entry.target_tag);
  assert.equal(ledger.entries.length, 10);
  assert.equal(new Set(sources).size, 10);
  assert.equal(new Set(targets).size, 10);
  for (const entry of ledger.entries) {
    assert.match(entry.source_sha, /^[0-9a-f]{40}$/);
    assert.ok(entry.expected_assets.length >= 2);
    for (const asset of entry.expected_assets) {
      assert.match(asset.sha256, /^[0-9a-f]{64}$/);
      assert.ok(asset.size_bytes > 0);
    }
  }
});

test("plans the frozen two-asset era as byte-identical copies", async () => {
  const ledger = await loadLedger();
  const plan = planMigration(ledger, "v0.1.1-pre1");
  assert.equal(plan.source_tag, "v0.1.1-pre.1");
  assert.equal(plan.era, "two-asset");
  assert.equal(plan.regenerate_release_manifest, false);
  assert.deepEqual(
    plan.assets.map((asset) => [asset.kind, asset.action]),
    [
      ["archive", "copy"],
      ["executable", "copy"],
    ],
  );
});

test("plans the four-asset era as copies plus metadata rebuilds", async () => {
  const ledger = await loadLedger();
  const plan = planMigration(ledger, "v0.2.0-pre2");
  assert.equal(plan.source_tag, "v0.2.0-pre.2");
  assert.equal(plan.era, "four-asset");
  assert.equal(plan.regenerate_release_manifest, false);
  const actions = Object.fromEntries(plan.assets.map((asset) => [asset.kind, asset.action]));
  assert.equal(actions.executable, "copy");
  assert.equal(actions.archive, "copy");
  assert.equal(actions.repository_dependencies, "rebuild_metadata");
  assert.equal(actions.full, "rebuild_metadata");
});

test("re-archives the ZIP-era Extension and regenerates the release manifest", async () => {
  const ledger = await loadLedger();
  const plan = planMigration(ledger, "v0.2.0-pre6");
  assert.equal(plan.source_tag, "v0.2.0-pre.7");
  assert.equal(plan.era, "five-asset-extension-zip");
  assert.equal(plan.regenerate_release_manifest, true);
  assert.deepEqual(
    plan.frozen_release_assets
      .map((asset) => asset.name)
      .filter((name) => !name.includes("windows-x64") && !name.includes("extension")),
    ["SHA256SUMS-v0.2.0-pre.7.txt", "XArchive-v0.2.0-pre.7-release-manifest.json"],
  );
  const actions = Object.fromEntries(plan.assets.map((asset) => [asset.kind, asset.action]));
  assert.equal(actions.extension, "rearchive_container");
  assert.equal(actions.full, "rebuild_metadata");
});

test("keeps the 7z-era Extension package as a copy", async () => {
  const ledger = await loadLedger();
  const plan = planMigration(ledger, "v0.2.0-pre7");
  assert.equal(plan.source_tag, "v0.2.0-pre.16");
  assert.equal(plan.era, "five-asset-extension-7z");
  const actions = Object.fromEntries(plan.assets.map((asset) => [asset.kind, asset.action]));
  assert.equal(actions.extension, "copy");
  assert.equal(actions.executable, "copy");
});

test("reuses the isolated verified package set for the polluted source", async () => {
  const ledger = await loadLedger();
  const plan = planMigration(ledger, "v0.2.0-pre5");
  assert.equal(plan.source_tag, "v0.2.0-pre.6");
  assert.equal(plan.mode, "artifact");
  assert.equal(plan.era, "five-asset-extension-7z");
  assert.equal(plan.source_ref.type, "artifact");
  assert.equal(plan.source_ref.run_id, 36655693790);
  assert.equal(plan.source_ref.package_dir, "legacy-packages");
  assert.equal(
    plan.source_ref.expected_executable_sha256,
    "4f8181459c30c682235573649c2c8713ea5bf191f4a65582837ab21607f25fa4",
  );
  assert.equal(plan.assets.length, 5);
  assert.ok(plan.assets.every((asset) => asset.source_name.startsWith("XArchive-v0.2.0-pre.6-")));
  const actions = Object.fromEntries(plan.assets.map((asset) => [asset.kind, asset.action]));
  assert.equal(actions.executable, "copy");
  assert.equal(actions.archive, "copy");
  assert.equal(actions.extension, "copy");
  assert.equal(actions.repository_dependencies, "rebuild_metadata");
  assert.equal(actions.full, "rebuild_metadata");
  // The polluted assets published on the old Release stay frozen evidence only.
  assert.deepEqual(
    plan.frozen_release_assets.map((asset) => asset.name),
    ["XArchive-v0.2.0-pre.6-windows-x64.7z", "XArchive-v0.2.0-pre.6-windows-x64.exe"],
  );
  const executable = plan.source_expectations.find((asset) => asset.name.endsWith(".exe"));
  assert.equal(executable.sha256, plan.source_ref.expected_executable_sha256);
});

test("refuses unpublishable, unknown, and mismatched targets", async () => {
  const ledger = await loadLedger();
  // A free new-format tag that the frozen ledger does not describe.
  assert.throws(() => planMigration(ledger, "v0.2.0-pre9"), /exactly one entry/);
  // Retired numbering can never be a migration target.
  assert.throws(() => planMigration(ledger, "v0.2.0-pre.8"), /read-only/);
  const mistagged = {
    ...ledger,
    entries: [
      {
        ...ledger.entries[0],
        expected_assets: [
          { name: "XArchive-v0.1.1-pre.9-windows-x64.exe", size_bytes: 1, sha256: "a".repeat(64) },
          { name: "XArchive-v0.1.1-pre.1-windows-x64.7z", size_bytes: 1, sha256: "b".repeat(64) },
        ],
      },
    ],
  };
  assert.throws(() => planMigration(mistagged, "v0.1.1-pre1"), /does not belong to/);
});
