import { copyFile, mkdir, readFile, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { createHash } from "node:crypto";
import { verifyAsset } from "./verify-windows-external-asset.mjs";

// Each notice is bound to its own recorded hash for this exact pinned asset.
const notices = [
  ["LICENSE", "license_sha256"],
  ...["websockets", "certifi", "urllib3", "cryptodome", "ytdlp", "python",
    "ejs", "astring", "meriyah", "requests", "idna", "truststore", "yaml",
    "charset", "mutagen", "socks", "brotli", "openssl"]
    .map((name) => [`LICENSE.${name}`, `${name}_license_sha256`]),
];

export async function validateGalleryFiles(source, metadata) {
  for (const key of ["asset_sha256", ...notices.map(([, key]) => key)]) {
    if (typeof metadata?.[key] !== "string" || !/^[a-f0-9]{64}$/.test(metadata[key])) {
      throw new Error(`Invalid gallery-dl licence metadata: ${key}`);
    }
  }
  for (const name of ["gallery-dl.exe", "LICENSE"]) {
    const info = await stat(join(source, name));
    if (!info.isFile() || info.size === 0) {
      throw new Error(`gallery-dl input is not a nonempty file: ${name}`);
    }
  }
  const binaryHash = createHash("sha256").update(await readFile(join(source, "gallery-dl.exe"))).digest("hex");
  if (binaryHash === metadata.asset_sha256) {
    for (const [name, key] of notices) await verifyAsset(join(source, name), metadata[key]);
  }
}

// Verify every input before creating or modifying the destination.
export async function copyPinnedGalleryLicense(asset, source, destination, metadata, pin) {
  if (metadata.asset_version !== pin?.version || metadata.asset_sha256 !== pin?.sha256) {
    throw new Error("gallery-dl licence metadata does not match the release pin");
  }
  await verifyAsset(asset, metadata.asset_sha256);
  for (const [name, key] of notices) await verifyAsset(join(source, name), metadata[key]);
  await mkdir(destination, { recursive: true });
  for (const [name] of notices) await copyFile(join(source, name), join(destination, name));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [, , asset, destination, ...extra] = process.argv;
  if (!asset || !destination || extra.length) {
    console.error("Usage: node gallery-license.mjs <gallery-dl.exe> <destination-directory>");
    process.exitCode = 1;
  } else {
    try {
      const metadata = JSON.parse(await readFile(new URL("../third-party/gallery-dl/metadata.json", import.meta.url), "utf8"));
      const pins = JSON.parse(await readFile(new URL("./windows-external-assets.json", import.meta.url), "utf8"));
      const source = fileURLToPath(new URL("../third-party/gallery-dl/", import.meta.url));
      await copyPinnedGalleryLicense(asset, source, destination, metadata, pins.gallery);
    } catch (error) {
      console.error(`Cannot include gallery-dl licence: ${error.message}`);
      process.exitCode = 1;
    }
  }
}
