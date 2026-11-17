import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import YAML from 'yaml';
import { verifyAsset } from '../scripts/verify-windows-external-asset.mjs';
import { ARIA2_COMPANION_FILES, copyAria2Companions } from '../scripts/copy-aria2-companions.mjs';
import { copyPinnedGalleryLicense } from '../scripts/gallery-license.mjs';
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

test('release and worker artifact workflows use the same pinned PyInstaller', async () => {
  const readWorkflow = async name => YAML.parse(await readFile(new URL(`../../.github/workflows/${name}`, import.meta.url), 'utf8'));
  const release = await readWorkflow('windows-release.yml');
  const worker = await readWorkflow('windows-worker-artifact.yml');
  const version = steps => {
    const installs = steps.flatMap(step => [...(step.run ?? '').matchAll(/python\s+-m\s+pip\s+install\s+["']?pyinstaller([^\s"']*)/gi)]);
    assert.equal(installs.length, 1, 'expected exactly one PyInstaller installation');
    assert.match(installs[0][1], /^==\d+\.\d+\.\d+$/, 'PyInstaller must be pinned');
    return installs[0][1];
  };
  assert.equal(version(release.jobs['build-windows'].steps), version(worker.jobs['build-worker'].steps));
});

test('both Windows worker builds restrict inherited PATH', async () => {
  const release = YAML.parse(await readFile(new URL('../../.github/workflows/windows-release.yml', import.meta.url), 'utf8'));
  const worker = YAML.parse(await readFile(new URL('../../.github/workflows/windows-worker-artifact.yml', import.meta.url), 'utf8'));
  for (const [job, name] of [[release.jobs['build-windows'], 'Build Windows worker'], [worker.jobs['build-worker'], 'Build worker']]) {
    const run = job.steps.find(step => step.name === name)?.run ?? '';
    assert.match(run, /Get-Command python -CommandType Application \| Select-Object -First 1/);
    assert.match(run, /\$env:PATH\s*=\s*"\$\(Split-Path \$workerPython\);\$env:SystemRoot\\System32;\$env:SystemRoot"/);
    assert.match(run, /& \$workerPython -m PyInstaller/);
  }
});

test('Windows worker workflows stop on installation or build failures', async () => {
  const release = YAML.parse(await readFile(new URL('../../.github/workflows/windows-release.yml', import.meta.url), 'utf8'));
  const worker = YAML.parse(await readFile(new URL('../../.github/workflows/windows-worker-artifact.yml', import.meta.url), 'utf8'));
  const releaseStep = release.jobs['build-windows'].steps.find(step => step.name === 'Build Windows worker').run;
  const workerSteps = worker.jobs['build-worker'].steps;
  const installs = workerSteps.find(step => step.name === 'Install worker and build tool').run;
  const build = workerSteps.find(step => step.name === 'Build worker').run;
  const checked = (script, command) => {
    const index = script.indexOf(command);
    assert.notEqual(index, -1, `missing ${command}`);
    const next = script.slice(index).split('\n').slice(1).find(line => line.trim() && !line.trim().startsWith('#'));
    assert.match(next ?? '', /if \(\$LASTEXITCODE -ne 0\) \{ throw /);
  };
  for (const script of [releaseStep, installs]) {
    for (const command of ['python -m pip install --upgrade pip', 'python -m pip install -e sidecar', 'python -m pip install "pyinstaller==6.22.3"']) checked(script, command);
  }
  for (const script of [releaseStep, build]) checked(script, 'sidecar\\pyinstaller\\xarchive-downloader.spec');
});

test('aria2 companion files are copied byte-for-byte only after all required inputs exist', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'xarchive-aria-notices-'));
  try {
    const source = join(dir, 'source');
    const destination = join(dir, 'destination');
    const expected = ['AUTHORS', 'COPYING', 'LICENSE.OpenSSL', 'README.mingw', 'README.html', 'NEWS', 'ChangeLog'];
    assert.deepEqual(ARIA2_COMPANION_FILES, expected);
    await mkdir(source);
    for (const name of expected.slice(0, -1)) await writeFile(join(source, name), `upstream:${name}`);
    await assert.rejects(copyAria2Companions(source, destination));
    await assert.rejects(readFile(join(destination, expected[0])));
    const last = expected.at(-1);
    await writeFile(join(source, last), '');
    await assert.rejects(copyAria2Companions(source, destination), /nonempty/);
    await writeFile(join(source, last), `upstream:${last}`);
    await copyAria2Companions(source, destination);
    for (const name of expected) assert.equal(await readFile(join(destination, name), 'utf8'), `upstream:${name}`);
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test('Full release assembly invokes and checks the aria2 companion copier', async () => {
  const workflow = YAML.parse(await readFile(new URL('../../.github/workflows/windows-release.yml', import.meta.url), 'utf8'));
  const script = workflow.jobs['build-windows'].steps.find(step => step.name === 'Create full bundle 7z archive')?.run ?? '';
  const lines = script.split('\n');
  const index = lines.findIndex(line => line.includes('copy-aria2-companions.mjs'));
  assert.notEqual(index, -1);
  assert.match(lines[index], /\$ariaExe\.DirectoryName \$ariaTarget/);
  assert.match(lines[index + 1] ?? '', /if \(\$LASTEXITCODE -ne 0\) \{ throw /);
});


test('gallery licence is bound to the pinned asset and verified before copying', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'xarchive-gallery-notice-'));
  try {
    const binary = join(dir, 'gallery.exe');
    const license = join(dir, 'LICENSE');
    const destination = join(dir, 'output');
    const sha = (value) => createHash('sha256').update(value).digest('hex');
    await writeFile(binary, 'asset');
    await writeFile(license, 'licence');
    const metadata = { asset_version: 'v1', asset_sha256: sha('asset'), license_sha256: sha('licence') };
    const pin = { version: 'v1', sha256: sha('asset') };
    await assert.rejects(copyPinnedGalleryLicense(binary, license, destination, metadata, { ...pin, version: 'v2' }), /does not match/);
    await assert.rejects(readFile(join(destination, 'LICENSE')));
    await writeFile(binary, 'wrong');
    await assert.rejects(copyPinnedGalleryLicense(binary, license, destination, metadata, pin), /mismatch/);
    await writeFile(binary, 'asset');
    await writeFile(license, 'wrong');
    await assert.rejects(copyPinnedGalleryLicense(binary, license, destination, metadata, pin), /mismatch/);
    await writeFile(license, 'licence');
    await copyPinnedGalleryLicense(binary, license, destination, metadata, pin);
    assert.equal(await readFile(join(destination, 'LICENSE'), 'utf8'), 'licence');
  } finally { await rm(dir, { recursive: true, force: true }); }
});

test('Full release assembly invokes and checks the pinned gallery licence copier', async () => {
  const workflow = YAML.parse(await readFile(new URL('../../.github/workflows/windows-release.yml', import.meta.url), 'utf8'));
  const script = workflow.jobs['build-windows'].steps.find(step => step.name === 'Create full bundle 7z archive')?.run ?? '';
  const lines = script.split('\n');
  const index = lines.findIndex(line => line.includes('gallery-license.mjs'));
  assert.notEqual(index, -1);
  assert.match(lines[index], /\$galleryExe \$galleryTarget/);
  assert.match(lines[index + 1] ?? '', /if \(\$LASTEXITCODE -ne 0\) \{ throw /);
});
