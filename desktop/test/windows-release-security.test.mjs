import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, rm, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import YAML from 'yaml';
import { verifyAsset } from '../scripts/verify-windows-external-asset.mjs';
test('trusted asset gate accepts matching bytes and rejects altered, empty, missing and invalid pins', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'xarchive-integrity-'));
  try {
    const file = join(dir, 'fixture');
    const bytes = Buffer.from('controlled fixture');
    const hash = createHash('sha256').update(bytes).digest('hex');
    await writeFile(file, bytes);
    assert.equal(await verifyAsset(file, hash), hash);
    for (const pin of [undefined, '', 'bad', '0'.repeat(64)]) await assert.rejects(verifyAsset(file, pin));
    await writeFile(file, 'altered');
    await assert.rejects(verifyAsset(file, hash), /mismatch/);
    await writeFile(file, '');
    await assert.rejects(verifyAsset(file, hash), /empty/);
    await assert.rejects(verifyAsset(join(dir, 'absent'), hash));
  } finally { await rm(dir, { recursive: true, force: true }); }
});
test('release build and validation cannot publish; publication depends on successful build and never overwrites assets', async () => {
  const workflow = YAML.parse(await readFile(new URL('../../.github/workflows/windows-release.yml', import.meta.url), 'utf8'));
  assert.equal(workflow.permissions.contents, 'read');
  const { 'build-windows': build, 'validate-wdio': validation, 'publish-release': publish } = workflow.jobs;
  assert.equal(publish.permissions.contents, 'write');
  assert.equal(publish.needs, 'build-windows');
  assert.equal(publish.if, "needs.build-windows.result == 'success'");
  for (const job of [build, validation]) {
    assert.equal(job.permissions?.contents ?? workflow.permissions.contents, 'read');
    for (const step of job.steps) {
      assert.equal(step.with?.['persist-credentials'] ?? (step.uses?.startsWith('actions/checkout@') ? true : false), false);
      assert.doesNotMatch(step.run ?? '', /gh release (create|upload)/);
      assert.equal(step.env?.GH_TOKEN, undefined);
    }
  }
  const download = build.steps.find(s => s.name === 'Download official external dependencies').run;
  assert(download.indexOf('verify-windows-external-asset.mjs') < download.indexOf('& $galleryPath'));
  assert(download.lastIndexOf('verify-windows-external-asset.mjs') < download.indexOf('Expand-Archive'));
  assert.doesNotMatch(JSON.stringify(publish), /--clobber|npm ci|cargo build/);
});
