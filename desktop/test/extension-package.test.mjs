import test from "node:test";
import assert from "node:assert/strict";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  buildExtensionPackageMetadata,
  collectExtensionPackageFiles,
  runExtensionPackageCommand,
  validateExtensionPackageMetadata,
} from "../scripts/extension-package.mjs";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const extensionDirectory = join(projectRoot, "extension");
const canonicalExtensionId = "iaajefkoanbkleojofoadeakelihbjne";
const releaseTag = "v0.2.0-pre.7";

async function withFixture(builder) {
  const directory = await mkdtemp(join(tmpdir(), "xarchive-extension-package-"));
  try {
    return await builder(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

async function writeFixtureExtension(directory, { includeContentCore = true } = {}) {
  await mkdir(join(directory, "src"), { recursive: true });
  await mkdir(join(directory, "tests"), { recursive: true });
  await cp(join(extensionDirectory, "manifest.json"), join(directory, "manifest.json"));
  for (const file of ["popup.html", "popup.js", "popup.css", "options.html", "options.js", "options.css"]) {
    await writeFile(join(directory, file), "// fixture\n");
  }
  await writeFile(join(directory, "src", "background.js"), "export {};\n");
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
  assert.equal(metadata.asset, `XArchive-${releaseTag}-extension.zip`);
  assert.equal(metadata.extension_id, canonicalExtensionId);
  assert.equal(metadata.extension_version, "0.1.0");
  assert.deepEqual(metadata.files, [
    "manifest.json",
    "options.css",
    "options.html",
    "options.js",
    "popup.css",
    "popup.html",
    "popup.js",
    "src/background.js",
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
    assert.equal(metadata.file_count, 12);
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
    assert.equal(planned.file_count, 12);

    const verified = await runExtensionPackageCommand([
      "verify",
      "--extension-dir",
      fixtureExtension,
      "--metadata",
      output,
    ]);
    assert.match(verified.message, /12 files match/);

    await writeFile(join(fixtureExtension, "src", "extra.js"), "export {};\n");
    await assert.rejects(
      runExtensionPackageCommand(["verify", "--extension-dir", fixtureExtension, "--metadata", output]),
      /inventory mismatch/,
    );
  });
});

test("rejects metadata that does not match its release tag or schema", () => {
  const metadata = {
    schema_version: 1,
    release_tag: releaseTag,
    asset: `XArchive-${releaseTag}-extension.zip`,
    extension_id: canonicalExtensionId,
    extension_version: "0.1.0",
    file_count: 12,
    files: [
      "manifest.json",
      "options.css",
      "options.html",
      "options.js",
      "popup.css",
      "popup.html",
      "popup.js",
      "src/background.js",
      "src/content-core.js",
      "src/content.js",
      "src/websocket-bridge.js",
      "src/websocket-settings.js",
    ],
  };
  assert.equal(validateExtensionPackageMetadata(metadata), metadata);
  assert.throws(
    () => validateExtensionPackageMetadata({ ...metadata, asset: "XArchive-v9.9.9-extension.zip" }),
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
