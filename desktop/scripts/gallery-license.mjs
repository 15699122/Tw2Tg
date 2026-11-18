import { copyFile, mkdir, readFile, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";
import { verifyAsset } from "./verify-windows-external-asset.mjs";

export async function validateGalleryFiles(source, metadata) {
  for (const key of ["asset_sha256", "license_sha256", "websockets_license_sha256", "certifi_license_sha256", "urllib3_license_sha256", "cryptodome_license_sha256", "ytdlp_license_sha256"]) {
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
    await verifyAsset(join(source, "LICENSE"), metadata.license_sha256);
    await verifyAsset(join(source, "LICENSE.websockets"), metadata.websockets_license_sha256);
    await verifyAsset(join(source, "LICENSE.certifi"), metadata.certifi_license_sha256);
    await verifyAsset(join(source, "LICENSE.urllib3"), metadata.urllib3_license_sha256);
    await verifyAsset(join(source, "LICENSE.cryptodome"), metadata.cryptodome_license_sha256);
    await verifyAsset(join(source, "LICENSE.ytdlp"), metadata.ytdlp_license_sha256);
  }
}

// A pinned release asset gets only the licence paired with that exact asset.
// The copied licence is one input to, not a substitute for, dependency review.
export async function copyPinnedGalleryLicense(asset, license, websocketsLicense, certifiLicense, urllib3License, cryptodomeLicense, ytDlpLicense, destination, metadata, pin) {
  if (metadata.asset_version !== pin?.version || metadata.asset_sha256 !== pin?.sha256) {
    throw new Error("gallery-dl licence metadata does not match the release pin");
  }
  await verifyAsset(asset, metadata.asset_sha256);
  await verifyAsset(license, metadata.license_sha256);
  await verifyAsset(websocketsLicense, metadata.websockets_license_sha256);
  await verifyAsset(certifiLicense, metadata.certifi_license_sha256);
  await verifyAsset(urllib3License, metadata.urllib3_license_sha256);
  await verifyAsset(cryptodomeLicense, metadata.cryptodome_license_sha256);
  await verifyAsset(ytDlpLicense, metadata.ytdlp_license_sha256);
  await mkdir(destination, { recursive: true });
  await copyFile(license, join(destination, "LICENSE"));
  await copyFile(websocketsLicense, join(destination, "LICENSE.websockets"));
  await copyFile(certifiLicense, join(destination, "LICENSE.certifi"));
  await copyFile(urllib3License, join(destination, "LICENSE.urllib3"));
  await copyFile(cryptodomeLicense, join(destination, "LICENSE.cryptodome"));
  await copyFile(ytDlpLicense, join(destination, "LICENSE.ytdlp"));
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
      const websocketsLicense = new URL("../third-party/gallery-dl/LICENSE.websockets", import.meta.url);
      const certifiLicense = new URL("../third-party/gallery-dl/LICENSE.certifi", import.meta.url);
      const urllib3License = new URL("../third-party/gallery-dl/LICENSE.urllib3", import.meta.url);
      const cryptodomeLicense = new URL("../third-party/gallery-dl/LICENSE.cryptodome", import.meta.url);
      const ytDlpLicense = new URL("../third-party/gallery-dl/LICENSE.ytdlp", import.meta.url);
      await copyPinnedGalleryLicense(asset, license, websocketsLicense, certifiLicense, urllib3License, cryptodomeLicense, ytDlpLicense, destination, metadata, pins.gallery);
    } catch (error) {
      console.error(`Cannot include gallery-dl licence: ${error.message}`);
      process.exitCode = 1;
    }
  }
}
