import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assertExtensionIdentityParity, readExtensionManifest } from "./extension-identity.mjs";
import { NATIVE_HOST_MANIFEST_FILE, createNativeHostManifest } from "./native-host-package.mjs";

function parseArguments(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) {
      throw new Error(`Unexpected argument: ${token}`);
    }
    const name = token.slice(2);
    const value = argv[index + 1];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`Missing value for --${name}`);
    }
    options[name] = value;
    index += 1;
  }
  return options;
}

function requireOption(options, name) {
  const value = options[name];
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(`Missing required option --${name}`);
  }
  return value;
}

export async function runNativeHostManifestCommand(argv) {
  const options = parseArguments(argv);
  const extensionDirectory = resolve(options["extension-dir"] ?? "extension");
  const expectedExtensionId = requireOption(options, "expected-id");
  const hostExecutable = resolve(requireOption(options, "host-executable"));
  const output = resolve(requireOption(options, "output"));

  const { manifest } = await readExtensionManifest(extensionDirectory);
  const extensionId = assertExtensionIdentityParity({
    manifestKey: manifest.key,
    expectedExtensionId,
  });
  const hostManifest = createNativeHostManifest({
    extensionId,
    path: hostExecutable,
  });
  await mkdir(dirname(output), { recursive: true });
  await writeFile(output, `${JSON.stringify(hostManifest, null, 2)}\n`);
  return {
    hostManifest,
    extensionId,
    output,
    message: `Native Host manifest written: ${output}\n  ${NATIVE_HOST_MANIFEST_FILE} allowed_origins: ${hostManifest.allowed_origins.join(", ")}\n  path: ${hostManifest.path}\n`,
  };
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  runNativeHostManifestCommand(process.argv.slice(2))
    .then(({ message }) => {
      process.stdout.write(message);
    })
    .catch((error) => {
      process.stderr.write(`native-host-manifest: ${error.message}\n`);
      process.exitCode = 1;
    });
}
