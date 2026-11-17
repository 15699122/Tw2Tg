import { copyFile, mkdir, readFile, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { verifyAsset } from "./verify-windows-external-asset.mjs";

export async function validateGalleryFiles(source) {
  for (const name of ["gallery-dl.exe", "LICENSE"]) {
    const info = await stat(join(source, name));
    if (!info.isFile() || info.size === 0) {
      throw new Error(`gallery-dl input is not a nonempty file: ${name}`);
    }
  }
}

// A pinned release asset gets only the licence paired with that exact asset.
// The copied licence is one input to, not a substitute for, dependency review.
export async function copyPinnedGalleryLicense(asset, license, destination, metadata, pin) {
  if (metadata.asset_version !== pin?.version || metadata.asset_sha256 !== pin?.sha256) {
    throw new Error("gallery-dl licence metadata does not match the release pin");
  }
  await verifyAsset(asset, metadata.asset_sha256);
  await verifyAsset(license, metadata.license_sha256);
  await mkdir(destination, { recursive: true });
  await copyFile(license, join(destination, "LICENSE"));
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
      const license = new URL("../third-party/gallery-dl/LICENSE", import.meta.url);
      await copyPinnedGalleryLicense(asset, license, destination, metadata, pins.gallery);
    } catch (error) {
      console.error(`Cannot include gallery-dl licence: ${error.message}`);
      process.exitCode = 1;
    }
  }
}
