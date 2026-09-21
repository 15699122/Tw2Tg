import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const EXTENSION_ID_ALPHABET = "abcdefghijklmnop";
export const EXTENSION_ID_PATTERN = /^[a-p]{32}$/;
export const EXTENSION_MANIFEST_FILE = "manifest.json";

// Chromium public keys are base64-encoded DER SubjectPublicKeyInfo; the RSA
// algorithm identifier (1.2.840.113549.1.1.1) must be present.
const RSA_ALGORITHM_IDENTIFIER = Buffer.from("06092a864886f70d010101", "hex");
const BASE64_PATTERN = /^[A-Za-z0-9+/]+={0,2}$/;

export function validateExtensionId(extensionId) {
  if (typeof extensionId !== "string" || !EXTENSION_ID_PATTERN.test(extensionId)) {
    throw new Error(
      "extensionId must be a 32-character Chromium extension ID using only characters a-p",
    );
  }
  return extensionId;
}

function derContentLength(publicKeyDer) {
  const lengthByte = publicKeyDer[1];
  if (lengthByte === undefined) {
    throw new Error("Extension manifest key must be a DER-encoded public key");
  }
  if (lengthByte < 0x80) {
    return lengthByte + 2;
  }
  const lengthBytes = lengthByte & 0x7f;
  if (lengthBytes === 0 || lengthBytes > 4 || publicKeyDer.length < 2 + lengthBytes) {
    throw new Error("Extension manifest key must be a DER-encoded public key");
  }
  let length = 0;
  for (let index = 0; index < lengthBytes; index += 1) {
    length = (length << 8) | publicKeyDer[2 + index];
  }
  return length + 2 + lengthBytes;
}

function assertDerPublicKey(publicKeyDer) {
  if (publicKeyDer[0] !== 0x30 || derContentLength(publicKeyDer) !== publicKeyDer.length) {
    throw new Error("Extension manifest key must be a DER-encoded public key");
  }
  if (!publicKeyDer.includes(RSA_ALGORITHM_IDENTIFIER)) {
    throw new Error("Extension manifest key must be an RSA DER-encoded public key");
  }
}

export function decodeManifestKey(manifestKey) {
  if (typeof manifestKey !== "string") {
    throw new Error("Extension manifest key must be a base64 string");
  }
  const trimmed = manifestKey.trim();
  if (!trimmed) {
    throw new Error("Extension manifest key must not be empty");
  }
  if (trimmed !== manifestKey) {
    throw new Error("Extension manifest key must not contain surrounding whitespace");
  }
  if (!BASE64_PATTERN.test(manifestKey)) {
    throw new Error("Extension manifest key must be canonical base64");
  }
  const publicKeyDer = Buffer.from(manifestKey, "base64");
  if (publicKeyDer.length === 0 || publicKeyDer.toString("base64") !== manifestKey) {
    throw new Error("Extension manifest key must be canonical base64");
  }
  assertDerPublicKey(publicKeyDer);
  return publicKeyDer;
}

export function extensionIdFromPublicKeyDer(publicKeyDer) {
  if (!Buffer.isBuffer(publicKeyDer) && !(publicKeyDer instanceof Uint8Array)) {
    throw new Error("Extension public key DER bytes are required");
  }
  const buffer = Buffer.from(publicKeyDer);
  if (buffer.length === 0) {
    throw new Error("Extension public key DER bytes are required");
  }
  const digest = createHash("sha256").update(buffer).digest();
  let extensionId = "";
  for (const byte of digest.subarray(0, 16)) {
    extensionId += EXTENSION_ID_ALPHABET[byte >> 4];
    extensionId += EXTENSION_ID_ALPHABET[byte & 0x0f];
  }
  return extensionId;
}

export function deriveExtensionIdFromManifestKey(manifestKey) {
  return extensionIdFromPublicKeyDer(decodeManifestKey(manifestKey));
}

export function assertExtensionIdentityParity({ manifestKey, expectedExtensionId }) {
  const derivedExtensionId = deriveExtensionIdFromManifestKey(manifestKey);
  const configuredExtensionId = validateExtensionId(expectedExtensionId);
  if (derivedExtensionId !== configuredExtensionId) {
    throw new Error(
      `Extension ID does not match the manifest key: derived ${derivedExtensionId}, configured ${configuredExtensionId}`,
    );
  }
  return derivedExtensionId;
}

export async function readExtensionManifest(extensionDirectory) {
  const manifestPath = join(resolve(extensionDirectory), EXTENSION_MANIFEST_FILE);
  const raw = await readFile(manifestPath, "utf8");
  let manifest;
  try {
    manifest = JSON.parse(raw);
  } catch (error) {
    throw new Error(`${manifestPath} is not valid JSON: ${error.message}`);
  }
  return { manifest, manifestPath };
}

function parseIdentityArguments(argv) {
  const [command, ...rest] = argv;
  const options = {};
  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index];
    if (!token.startsWith("--")) {
      throw new Error(`Unexpected argument: ${token}`);
    }
    const name = token.slice(2);
    const value = rest[index + 1];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`Missing value for --${name}`);
    }
    options[name] = value;
    index += 1;
  }
  return { command, options };
}

export async function runExtensionIdentityCommand(argv) {
  const { command, options } = parseIdentityArguments(argv);
  const extensionDirectory = options["extension-dir"] ?? "extension";
  const { manifest, manifestPath } = await readExtensionManifest(extensionDirectory);
  const manifestKey = manifest?.key;
  if (typeof manifestKey !== "string" || !manifestKey.trim()) {
    throw new Error(`${manifestPath} must define a public "key"`);
  }
  const derivedExtensionId = deriveExtensionIdFromManifestKey(manifestKey);
  if (command === "derive") {
    return { derivedExtensionId, message: `${derivedExtensionId}\n` };
  }
  if (command === "verify") {
    const expectedExtensionId = options["expected-id"];
    if (!expectedExtensionId) {
      throw new Error("verify requires --expected-id");
    }
    assertExtensionIdentityParity({ manifestKey, expectedExtensionId });
    return {
      derivedExtensionId,
      message: `Extension identity OK: ${derivedExtensionId}\n`,
    };
  }
  throw new Error(`Unknown command: ${command ?? "(none)"} (expected derive or verify)`);
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  runExtensionIdentityCommand(process.argv.slice(2))
    .then(({ message }) => {
      process.stdout.write(message);
    })
    .catch((error) => {
      process.stderr.write(`extension-identity: ${error.message}\n`);
      process.exitCode = 1;
    });
}
