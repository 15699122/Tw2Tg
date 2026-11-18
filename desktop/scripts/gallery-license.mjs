import { copyFile, mkdir, readFile, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";
import { verifyAsset } from "./verify-windows-external-asset.mjs";

export async function validateGalleryFiles(source, metadata) {
  for (const key of ["asset_sha256", "license_sha256", "websockets_license_sha256", "certifi_license_sha256", "urllib3_license_sha256", "cryptodome_license_sha256", "ytdlp_license_sha256", "python_license_sha256", "ejs_license_sha256", "astring_license_sha256", "meriyah_license_sha256", "requests_license_sha256", "idna_license_sha256", "truststore_license_sha256", "yaml_license_sha256", "charset_license_sha256", "mutagen_license_sha256", "socks_license_sha256"]) {
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
    await verifyAsset(join(source, "LICENSE.python"), metadata.python_license_sha256);
    await verifyAsset(join(source, "LICENSE.ejs"), metadata.ejs_license_sha256);
    await verifyAsset(join(source, "LICENSE.astring"), metadata.astring_license_sha256);
    await verifyAsset(join(source, "LICENSE.meriyah"), metadata.meriyah_license_sha256);
    await verifyAsset(join(source, "LICENSE.requests"), metadata.requests_license_sha256);
    await verifyAsset(join(source, "LICENSE.idna"), metadata.idna_license_sha256);
    await verifyAsset(join(source, "LICENSE.truststore"), metadata.truststore_license_sha256);
    await verifyAsset(join(source, "LICENSE.yaml"), metadata.yaml_license_sha256);
    await verifyAsset(join(source, "LICENSE.charset"), metadata.charset_license_sha256);
    await verifyAsset(join(source, "LICENSE.mutagen"), metadata.mutagen_license_sha256);
    await verifyAsset(join(source, "LICENSE.socks"), metadata.socks_license_sha256);
  }
}

// A pinned release asset gets only the licence paired with that exact asset.
// The copied licence is one input to, not a substitute for, dependency review.
export async function copyPinnedGalleryLicense(asset, license, websocketsLicense, certifiLicense, urllib3License, cryptodomeLicense, ytDlpLicense, pythonLicense, ejsLicense, astringLicense, meriyahLicense, requestsLicense, idnaLicense, truststoreLicense, yamlLicense, charsetLicense, mutagenLicense, socksLicense, destination, metadata, pin) {
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
  await verifyAsset(pythonLicense, metadata.python_license_sha256);
  await verifyAsset(ejsLicense, metadata.ejs_license_sha256);
  await verifyAsset(astringLicense, metadata.astring_license_sha256);
  await verifyAsset(meriyahLicense, metadata.meriyah_license_sha256);
  await verifyAsset(requestsLicense, metadata.requests_license_sha256);
  await verifyAsset(idnaLicense, metadata.idna_license_sha256);
  await verifyAsset(truststoreLicense, metadata.truststore_license_sha256);
  await verifyAsset(yamlLicense, metadata.yaml_license_sha256);
  await verifyAsset(charsetLicense, metadata.charset_license_sha256);
  await verifyAsset(mutagenLicense, metadata.mutagen_license_sha256);
  await verifyAsset(socksLicense, metadata.socks_license_sha256);
  await mkdir(destination, { recursive: true });
  await copyFile(license, join(destination, "LICENSE"));
  await copyFile(websocketsLicense, join(destination, "LICENSE.websockets"));
  await copyFile(certifiLicense, join(destination, "LICENSE.certifi"));
  await copyFile(urllib3License, join(destination, "LICENSE.urllib3"));
  await copyFile(cryptodomeLicense, join(destination, "LICENSE.cryptodome"));
  await copyFile(ytDlpLicense, join(destination, "LICENSE.ytdlp"));
  await copyFile(pythonLicense, join(destination, "LICENSE.python"));
  await copyFile(ejsLicense, join(destination, "LICENSE.ejs"));
  await copyFile(astringLicense, join(destination, "LICENSE.astring"));
  await copyFile(meriyahLicense, join(destination, "LICENSE.meriyah"));
  await copyFile(requestsLicense, join(destination, "LICENSE.requests"));
  await copyFile(idnaLicense, join(destination, "LICENSE.idna"));
  await copyFile(truststoreLicense, join(destination, "LICENSE.truststore"));
  await copyFile(yamlLicense, join(destination, "LICENSE.yaml"));
  await copyFile(charsetLicense, join(destination, "LICENSE.charset"));
  await copyFile(mutagenLicense, join(destination, "LICENSE.mutagen"));
  await copyFile(socksLicense, join(destination, "LICENSE.socks"));
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
      const pythonLicense = new URL("../third-party/gallery-dl/LICENSE.python", import.meta.url);
      const ejsLicense = new URL("../third-party/gallery-dl/LICENSE.ejs", import.meta.url);
      const astringLicense = new URL("../third-party/gallery-dl/LICENSE.astring", import.meta.url);
      const meriyahLicense = new URL("../third-party/gallery-dl/LICENSE.meriyah", import.meta.url);
      const requestsLicense = new URL("../third-party/gallery-dl/LICENSE.requests", import.meta.url);
      const idnaLicense = new URL("../third-party/gallery-dl/LICENSE.idna", import.meta.url);
      const truststoreLicense = new URL("../third-party/gallery-dl/LICENSE.truststore", import.meta.url);
      const yamlLicense = new URL("../third-party/gallery-dl/LICENSE.yaml", import.meta.url);
      const charsetLicense = new URL("../third-party/gallery-dl/LICENSE.charset", import.meta.url);
      const mutagenLicense = new URL("../third-party/gallery-dl/LICENSE.mutagen", import.meta.url);
      const socksLicense = new URL("../third-party/gallery-dl/LICENSE.socks", import.meta.url);
      await copyPinnedGalleryLicense(asset, license, websocketsLicense, certifiLicense, urllib3License, cryptodomeLicense, ytDlpLicense, pythonLicense, ejsLicense, astringLicense, meriyahLicense, requestsLicense, idnaLicense, truststoreLicense, yamlLicense, charsetLicense, mutagenLicense, socksLicense, destination, metadata, pins.gallery);
    } catch (error) {
      console.error(`Cannot include gallery-dl licence: ${error.message}`);
      process.exitCode = 1;
    }
  }
}
