import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  assertExtensionIdentityParity,
  decodeManifestKey,
  deriveExtensionIdFromManifestKey,
  extensionIdFromPublicKeyDer,
  readExtensionManifest,
  runExtensionIdentityCommand,
  validateExtensionId,
} from "../scripts/extension-identity.mjs";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const extensionDirectory = join(projectRoot, "extension");
const canonicalExtensionId = "iaajefkoanbkleojofoadeakelihbjne";

function encodeDerLength(length) {
  if (length < 0x80) {
    return Buffer.from([length]);
  }
  const bytes = [];
  let remaining = length;
  while (remaining > 0) {
    bytes.unshift(remaining & 0xff);
    remaining >>= 8;
  }
  return Buffer.from([0x80 | bytes.length, ...bytes]);
}

function buildSyntheticRsaPublicKey(fillByte) {
  const algorithmIdentifier = Buffer.from("300d06092a864886f70d0101010500", "hex");
  const bitStringBody = Buffer.concat([Buffer.from([0x00]), Buffer.alloc(64, fillByte)]);
  const bitString = Buffer.concat([
    Buffer.from([0x03]),
    encodeDerLength(bitStringBody.length),
    bitStringBody,
  ]);
  const body = Buffer.concat([algorithmIdentifier, bitString]);
  return Buffer.concat([Buffer.from([0x30]), encodeDerLength(body.length), body]);
}

test("derives the canonical Extension ID from the committed manifest key", async () => {
  const { manifest } = await readExtensionManifest(extensionDirectory);
  assert.equal(deriveExtensionIdFromManifestKey(manifest.key), canonicalExtensionId);
  assert.equal(validateExtensionId(canonicalExtensionId), canonicalExtensionId);
});

test("rejects Extension IDs outside the Chromium a-p alphabet", () => {
  assert.throws(() => validateExtensionId("zaajefkoanbkleojofoadeakelihbjne"), /32-character/);
  assert.throws(() => validateExtensionId("iaajefkoanbkleojofoadeakelihbjn"), /32-character/);
  assert.throws(() => validateExtensionId("iaajefkoanbkleojofoadeakelihbjnee"), /32-character/);
  assert.throws(() => validateExtensionId("IAAJEFKOANBKLEOJOFOADEAKELIHBJNE"), /32-character/);
  assert.throws(() => validateExtensionId(""), /32-character/);
  assert.throws(() => validateExtensionId(undefined), /32-character/);
});

test("rejects non-canonical or non-DER manifest keys", () => {
  assert.throws(() => decodeManifestKey(undefined), /base64 string/);
  assert.throws(() => decodeManifestKey(""), /must not be empty/);
  assert.throws(() => decodeManifestKey("  QUJD  "), /whitespace|base64/);
  assert.throws(() => decodeManifestKey("not base64 !!"), /canonical base64/);
  assert.throws(() => decodeManifestKey(Buffer.from("short").toString("base64")), /DER-encoded public key/);
});

test("derives stable distinct IDs for distinct public keys", () => {
  const first = deriveExtensionIdFromManifestKey(buildSyntheticRsaPublicKey(0x01).toString("base64"));
  const second = deriveExtensionIdFromManifestKey(buildSyntheticRsaPublicKey(0x02).toString("base64"));
  assert.match(first, /^[a-p]{32}$/);
  assert.match(second, /^[a-p]{32}$/);
  assert.notEqual(first, second);
  assert.equal(first, extensionIdFromPublicKeyDer(buildSyntheticRsaPublicKey(0x01)));
});

test("enforces manifest key and configured Extension ID parity", () => {
  const manifestKey = buildSyntheticRsaPublicKey(0x03).toString("base64");
  const derived = deriveExtensionIdFromManifestKey(manifestKey);
  assert.equal(assertExtensionIdentityParity({ manifestKey, expectedExtensionId: derived }), derived);
  assert.throws(
    () => assertExtensionIdentityParity({ manifestKey, expectedExtensionId: canonicalExtensionId }),
    /does not match the manifest key/,
  );
});

test("verifies identity through the CLI entry point", async () => {
  const verified = await runExtensionIdentityCommand([
    "verify",
    "--extension-dir",
    extensionDirectory,
    "--expected-id",
    canonicalExtensionId,
  ]);
  assert.equal(verified.derivedExtensionId, canonicalExtensionId);
  assert.match(verified.message, /Extension identity OK/);

  const derived = await runExtensionIdentityCommand(["derive", "--extension-dir", extensionDirectory]);
  assert.equal(derived.message.trim(), canonicalExtensionId);

  await assert.rejects(
    runExtensionIdentityCommand(["verify", "--extension-dir", extensionDirectory, "--expected-id", "abcdefghijklmnopabcdefghijklmnop"]),
    /does not match the manifest key/,
  );
  await assert.rejects(runExtensionIdentityCommand(["verify", "--extension-dir", extensionDirectory]), /--expected-id/);
  await assert.rejects(runExtensionIdentityCommand(["unknown", "--extension-dir", extensionDirectory]), /Unknown command/);
});

test("fails when the Extension manifest does not define a public key", async () => {
  const directory = await mkdtemp(join(tmpdir(), "xarchive-identity-"));
  try {
    await writeFile(join(directory, "manifest.json"), `${JSON.stringify({ manifest_version: 3 })}\n`);
    await assert.rejects(runExtensionIdentityCommand(["derive", "--extension-dir", directory]), /must define a public "key"/);
    await assert.rejects(readExtensionManifest(join(directory, "missing")), /ENOENT/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
