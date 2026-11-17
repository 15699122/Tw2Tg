import { copyFile, mkdir, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

// These files accompany aria2c.exe in the pinned upstream Windows archive.
// Their inclusion does not by itself establish complete distribution terms.
export const ARIA2_COMPANION_FILES = Object.freeze([
  "AUTHORS", "COPYING", "LICENSE.OpenSSL", "README.mingw",
  "README.html", "NEWS", "ChangeLog",
]);

export async function validateAria2Companions(source) {
  for (const name of ARIA2_COMPANION_FILES) {
    const info = await stat(join(source, name));
    if (!info.isFile() || info.size === 0) {
      throw new Error(`aria2 companion is not a nonempty file: ${name}`);
    }
  }
}

export async function copyAria2Companions(source, destination) {
  await validateAria2Companions(source);
  await mkdir(destination, { recursive: true });
  for (const name of ARIA2_COMPANION_FILES) {
    await copyFile(join(source, name), join(destination, name));
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [, , source, destination, ...extra] = process.argv;
  if (!source || !destination || extra.length) {
    console.error("Usage: node copy-aria2-companions.mjs <source-directory> <destination-directory>");
    process.exitCode = 1;
  } else {
    copyAria2Companions(source, destination).catch((error) => {
      console.error(`Cannot include aria2 companion files: ${error.message}`);
      process.exitCode = 1;
    });
  }
}
