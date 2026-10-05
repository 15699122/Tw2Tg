import { readFile, stat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
export async function verifyAsset(path, expected) {
  if (typeof expected !== 'string' || !/^[a-f0-9]{64}$/i.test(expected)) throw new Error('Missing or invalid trusted SHA-256');
  const info = await stat(path);
  if (!info.isFile() || info.size === 0) throw new Error('Missing or empty external asset');
  const actual = createHash('sha256').update(await readFile(path)).digest('hex');
  if (actual !== expected.toLowerCase()) throw new Error('External asset SHA-256 mismatch');
  return actual;
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [key, path] = process.argv.slice(2);
  const pins = JSON.parse(await readFile(new URL('./windows-external-assets.json', import.meta.url), 'utf8'));
  await verifyAsset(path, pins[key]?.sha256);
  console.log('Trusted external asset SHA-256 PASS');
}
