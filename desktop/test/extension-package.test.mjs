import test from "node:test";
import assert from "node:assert/strict";
import { cp, mkdir, mkdtemp, readFile, rm, rmdir, symlink, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import {
  buildExtensionPackageMetadata,
  collectExtensionPackageFiles,
  runExtensionPackageCommand,
  validateExtensionPackageMetadata,
} from "../scripts/extension-package.mjs";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const extensionDirectory = join(projectRoot, "extension");
const canonicalExtensionId = "iaajefkoanbkleojofoadeakelihbjne";
const releaseTag = "v0.2.0-pre6";

async function withFixture(builder) {
  const directory = await mkdtemp(join(tmpdir(), "xarchive-extension-package-"));
  try {
    return await builder(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

async function writeFixtureExtension(directory, { includeContentCore = true, includeBrowserPairing = true } = {}) {
  await mkdir(join(directory, "src"), { recursive: true });
  await mkdir(join(directory, "tests"), { recursive: true });
  await cp(join(extensionDirectory, "manifest.json"), join(directory, "manifest.json"));
  for (const file of ["popup.html", "popup.js", "popup.css", "options.html", "options.js", "options.css"]) {
    await writeFile(join(directory, file), "// fixture\n");
  }
  await writeFile(join(directory, "src", "background.js"), "export {};\n");
  if (includeBrowserPairing) {
    await writeFile(join(directory, "src", "browser-pairing.js"), "export {};\n");
  }
  await writeFile(join(directory, "src", "websocket-settings.js"), "export {};\n");
  await writeFile(join(directory, "src", "websocket-bridge.js"), "export {};\n");
  await writeFile(join(directory, "src", "content.js"), "export {};\n");
  if (includeContentCore) {
    await writeFile(join(directory, "src", "content-core.js"), "export {};\n");
  }
  await writeFile(join(directory, "tests", "content.test.js"), "// excluded\n");
  await writeFile(join(directory, "package.json"), "{}\n");
  await writeFile(join(directory, "xarchive-extension.pem"), "-----BEGIN PRIVATE KEY-----\n");
}

test("plans a loadable Extension package from the repository Extension", async () => {
  const metadata = await buildExtensionPackageMetadata({
    extensionDirectory,
    extensionId: canonicalExtensionId,
    releaseTag,
  });
  assert.equal(metadata.asset, `XArchive-${releaseTag}-extension.7z`);
  assert.equal(metadata.extension_id, canonicalExtensionId);
  // The packaged Extension version is read from the manifest; keep the
  // expectation derived from the same source so a version bump does not need a
  // second manual edit here.
  const manifestVersion = JSON.parse(
    await readFile(join(extensionDirectory, "manifest.json"), "utf8"),
  ).version;
  assert.equal(metadata.extension_version, manifestVersion);
  assert.deepEqual(metadata.files, [
    "manifest.json",
    "options.css",
    "options.html",
    "options.js",
    "popup.css",
    "popup.html",
    "popup.js",
    "src/background.js",
    "src/browser-pairing.js",
    "src/content-core.js",
    "src/content.js",
    "src/websocket-bridge.js",
    "src/websocket-settings.js",
  ]);
  for (const file of metadata.files) {
    assert.ok(!file.startsWith("tests/"), `tests must not be packaged: ${file}`);
  }
});

test("excludes dev-only files and rejects missing required files", async () => {
  await withFixture(async (directory) => {
    await writeFixtureExtension(directory);
    const files = await collectExtensionPackageFiles(directory);
    assert.deepEqual(files, [
      "manifest.json",
      "options.css",
      "options.html",
      "options.js",
      "popup.css",
      "popup.html",
      "popup.js",
      "src/background.js",
      "src/browser-pairing.js",
      "src/content-core.js",
      "src/content.js",
      "src/websocket-bridge.js",
      "src/websocket-settings.js",
    ]);
    const metadata = await buildExtensionPackageMetadata({
      extensionDirectory: directory,
      extensionId: canonicalExtensionId,
      releaseTag,
    });
    assert.equal(metadata.file_count, 13);
  });

  await withFixture(async (directory) => {
    await writeFixtureExtension(directory, { includeContentCore: false });
    await assert.rejects(
      buildExtensionPackageMetadata({
        extensionDirectory: directory,
        extensionId: canonicalExtensionId,
        releaseTag,
      }),
      /missing required file: src\/content-core\.js/,
    );
  });

  await withFixture(async (directory) => {
    await writeFixtureExtension(directory, { includeBrowserPairing: false });
    await assert.rejects(
      buildExtensionPackageMetadata({
        extensionDirectory: directory,
        extensionId: canonicalExtensionId,
        releaseTag,
      }),
      /missing required file: src\/browser-pairing\.js/,
    );
  });
});

test("verifies an extracted package against the planned inventory", async () => {
  await withFixture(async (directory) => {
    const fixtureExtension = join(directory, "extension");
    await writeFixtureExtension(fixtureExtension);
    const output = join(directory, "extension-package.json");
    await runExtensionPackageCommand([
      "plan",
      "--extension-dir",
      fixtureExtension,
      "--extension-id",
      canonicalExtensionId,
      "--tag",
      releaseTag,
      "--output",
      output,
    ]);
    const planned = JSON.parse(await readFile(output, "utf8"));
    assert.equal(planned.file_count, 13);
    const staged = join(directory, "staged");
    await runExtensionPackageCommand([
      "stage", "--extension-dir", fixtureExtension, "--extension-id", canonicalExtensionId,
      "--tag", releaseTag, "--output", staged,
    ]);

    const verified = await runExtensionPackageCommand([
      "verify",
      "--extension-dir",
      staged,
      "--metadata",
      output,
    ]);
    assert.match(verified.message, /13 files match/);

    const verifyBytes = () => runExtensionPackageCommand([
      "verify", "--extension-dir", staged, "--metadata", output, "--source-dir", fixtureExtension,
    ]);
    await verifyBytes();
    await writeFile(join(staged, "popup.js"), "altered content\n");
    await assert.rejects(verifyBytes(), /content mismatch: popup\.js/);
    const cli = spawnSync(process.execPath, [
      join(projectRoot, "desktop", "scripts", "extension-package.mjs"), "verify",
      "--extension-dir", staged, "--metadata", output, "--source-dir", fixtureExtension,
    ], { encoding: "utf8", timeout: 30000 });
    assert.notEqual(cli.status, 0);
    assert.match(cli.stderr, /content mismatch: popup\.js/);
    await cp(join(fixtureExtension, "popup.js"), join(staged, "popup.js"));

    await writeFile(join(staged, "src", "extra.js"), "export {};\n");
    await assert.rejects(
      runExtensionPackageCommand(["verify", "--extension-dir", staged, "--metadata", output]),
      /inventory mismatch/,
    );
  });
});

test("stages only the validated Extension inventory and refuses an existing destination", async () => {
  await withFixture(async (directory) => {
    const source = join(directory, "extension");
    const output = join(directory, "staged");
    await writeFixtureExtension(source);
    const options = ["--extension-dir", source, "--extension-id", canonicalExtensionId,
      "--tag", releaseTag, "--output", output];
    const result = await runExtensionPackageCommand(["stage", ...options]);
    assert.equal(result.metadata.file_count, 13);
    for (const file of result.metadata.files) {
      assert.deepEqual(await readFile(join(output, file)), await readFile(join(source, file)));
    }
    for (const file of ["tests/content.test.js", "package.json", "xarchive-extension.pem"]) {
      await assert.rejects(readFile(join(output, file)), { code: "ENOENT" });
    }
    await writeFile(join(output, "sentinel"), "preserve");
    await assert.rejects(runExtensionPackageCommand(["stage", ...options]), { code: "EEXIST" });
    assert.equal(await readFile(join(output, "sentinel"), "utf8"), "preserve");
    await assert.rejects(runExtensionPackageCommand(["stage", ...options.slice(0, -1), join(source, "nested")]), /outside the source/);
    await rm(join(source, "popup.js"));
    const missingOutput = join(directory, "missing-output");
    await assert.rejects(runExtensionPackageCommand(["stage", ...options.slice(0, -1), missingOutput]), /missing required file: popup\.js/);
    await assert.rejects(readFile(join(missingOutput, "manifest.json")), { code: "ENOENT" });
  });
});

test("refuses staging through a directory alias into the Extension source", async () => {
  await withFixture(async (directory) => {
    const source = join(directory, "source");
    const alias = join(directory, "alias");
    await writeFixtureExtension(source);
    await symlink(source, alias, process.platform === "win32" ? "junction" : "dir");
    try {
      await assert.rejects(runExtensionPackageCommand([
        "stage", "--extension-dir", source, "--extension-id", canonicalExtensionId,
        "--tag", releaseTag, "--output", join(alias, "nested"),
      ]), /outside the source/);
      await assert.rejects(readFile(join(source, "nested", "manifest.json")), { code: "ENOENT" });
    } finally {
      await unlink(alias);
    }
  });
});

test("Extension staging CLI materializes the release inventory", async () => {
  await withFixture(async (directory) => {
    const source = join(directory, "extension");
    const output = join(directory, "staged-cli");
    await writeFixtureExtension(source);
    const result = spawnSync(process.execPath, [
      join(projectRoot, "desktop", "scripts", "extension-package.mjs"), "stage",
      "--extension-dir", source, "--extension-id", canonicalExtensionId,
      "--tag", releaseTag, "--output", output,
    ], { encoding: "utf8", timeout: 30000 });
    assert.equal(result.status, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /Extension staged: 13 files/);
    assert.deepEqual(await collectExtensionPackageFiles(output), await collectExtensionPackageFiles(source));
    await assert.rejects(readFile(join(output, "tests", "content.test.js")), { code: "ENOENT" });
  });
});

test("verification rejects excluded files in an extracted Extension", async () => {
  await withFixture(async (directory) => {
    const source = join(directory, "extension");
    const staged = join(directory, "staged");
    const metadata = join(directory, "extension-package.json");
    await writeFixtureExtension(source);
    const options = ["--extension-dir", source, "--extension-id", canonicalExtensionId, "--tag", releaseTag];
    await runExtensionPackageCommand(["plan", ...options, "--output", metadata]);
    await runExtensionPackageCommand(["stage", ...options, "--output", staged]);
    const verify = () => runExtensionPackageCommand(["verify", "--extension-dir", staged, "--metadata", metadata]);
    await verify();
    await mkdir(join(staged, "tests"));
    await writeFile(join(staged, "tests", "secret.test.js"), "excluded");
    await assert.rejects(verify(), /must not contain/);
    const cli = spawnSync(process.execPath, [
      join(projectRoot, "desktop", "scripts", "extension-package.mjs"), "verify",
      "--extension-dir", staged, "--metadata", metadata,
    ], { encoding: "utf8", timeout: 30000 });
    assert.notEqual(cli.status, 0);
    assert.match(cli.stderr, /must not contain/);
    await rm(join(staged, "tests", "secret.test.js"));
    await rmdir(join(staged, "tests"));
    await writeFile(join(staged, "local.pem"), "excluded");
    await assert.rejects(verify(), /must not contain/);
  });
});

test("nested development and secret directories are excluded from staging and rejected in artifacts", async () => {
  await withFixture(async (directory) => {
    const source = join(directory, "extension");
    const staged = join(directory, "staged");
    const metadata = join(directory, "extension-package.json");
    await writeFixtureExtension(source);
    for (const name of ["tests", "node_modules", "secrets", "cache"]) {
      await mkdir(join(source, "src", name));
      await writeFile(join(source, "src", name, "fixture.txt"), "excluded");
    }
    const options = ["--extension-dir", source, "--extension-id", canonicalExtensionId, "--tag", releaseTag];
    await runExtensionPackageCommand(["plan", ...options, "--output", metadata]);
    await runExtensionPackageCommand(["stage", ...options, "--output", staged]);
    const files = await collectExtensionPackageFiles(staged);
    assert.equal(files.length, 13);
    for (const name of ["tests", "node_modules", "secrets", "cache"]) {
      await assert.rejects(readFile(join(staged, "src", name, "fixture.txt")), { code: "ENOENT" });
    }
    await mkdir(join(staged, "src", "secrets"));
    await assert.rejects(
      runExtensionPackageCommand(["verify", "--extension-dir", staged, "--metadata", metadata]),
      /must not contain/,
    );
  });
});

test("rejects metadata that does not match its release tag or schema", () => {
  const metadata = {
    schema_version: 1,
    release_tag: releaseTag,
    asset: `XArchive-${releaseTag}-extension.7z`,
    extension_id: canonicalExtensionId,
    extension_version: "0.1.0",
    file_count: 13,
    files: [
      "manifest.json",
      "options.css",
      "options.html",
      "options.js",
      "popup.css",
      "popup.html",
      "popup.js",
      "src/background.js",
      "src/browser-pairing.js",
      "src/content-core.js",
      "src/content.js",
      "src/websocket-bridge.js",
      "src/websocket-settings.js",
    ],
  };
  assert.equal(validateExtensionPackageMetadata(metadata), metadata);
  assert.throws(
    () => validateExtensionPackageMetadata({ ...metadata, asset: "XArchive-v9.9.9-extension.7z" }),
    /does not match its release tag/,
  );
  assert.throws(
    () => validateExtensionPackageMetadata({ ...metadata, schema_version: 2 }),
    /schema_version/,
  );
  assert.throws(
    () => validateExtensionPackageMetadata({ ...metadata, files: [...metadata.files, "secrets/id.txt"] }),
    /must not contain/,
  );
});
