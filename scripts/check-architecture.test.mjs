// Tests for scripts/check-architecture.mjs.
//
// Each case builds a throwaway Git repository containing a policy, an
// exceptions file and a reviewed baseline, then runs the real checker against
// it. A passing case therefore means the script behaves, not that a mock agrees
// with the implementation.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

const script = path.resolve('scripts/check-architecture.mjs');

const policy = {
  schemaVersion: 1,
  dependencyCategory: 'frontend',
  categories: [
    {
      id: 'rust',
      description: 'rust sources',
      globs: ['crates/*/src/**/*.rs'],
      limitLines: 10,
    },
    {
      id: 'frontend',
      description: 'frontend sources',
      globs: ['web/**/*.js'],
      limitLines: 10,
    },
  ],
  dependencyRules: [
    {
      id: 'no-back-edge',
      description: 'frontend must not import rust',
      from: ['web/**/*.js'],
      to: ['crates/**/*.rs'],
    },
  ],
};

function git(args, cwd) {
  return execFileSync('git', args, { cwd, encoding: 'utf8' });
}

function makeRepo(files) {
  const dir = mkdtempSync(path.join(tmpdir(), 'arch-check-'));
  git(['init', '-q'], dir);
  git(['config', 'user.email', 'test@example.com'], dir);
  git(['config', 'user.name', 'test'], dir);
  const all = {
    'scripts/architecture-policy.json': JSON.stringify(policy),
    'scripts/architecture-exceptions.json': JSON.stringify({ schemaVersion: 1, files: {} }),
    ...files,
  };
  for (const [name, body] of Object.entries(all)) {
    const full = path.join(dir, name);
    mkdirSync(path.dirname(full), { recursive: true });
    writeFileSync(full, body);
  }
  git(['add', '-A'], dir);
  git(['commit', '-qm', 'fixture'], dir);
  return dir;
}

// Write the reviewed baseline from the current tree, then re-commit it, so the
// remaining checks start from a matching snapshot.
function sealBaseline(dir) {
  execFileSync('node', [script, '--write-baseline'], { cwd: dir, encoding: 'utf8' });
  git(['add', '-A'], dir);
  git(['commit', '-qm', 'baseline'], dir);
}

function run(dir) {
  try {
    const out = execFileSync('node', [script, '--json'], { cwd: dir, encoding: 'utf8' });
    return { code: 0, body: JSON.parse(out) };
  } catch (error) {
    const out = error.stdout ?? '';
    return { code: error.status ?? 1, body: out.trim() ? JSON.parse(out) : { findings: [] } };
  }
}

function kinds(result) {
  return result.body.findings.map((finding) => finding.kind);
}

function repeat(line, times) {
  return Array.from({ length: times }, () => line).join('\n') + '\n';
}

test('a sealed tree with matching sizes reports nothing', () => {
  const dir = makeRepo({ 'crates/a/src/lib.rs': repeat('// line', 5) });
  try {
    sealBaseline(dir);
    const result = run(dir);
    assert.equal(result.code, 0);
    assert.deepEqual(kinds(result), []);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a file above its category limit without an exception is reported', () => {
  const dir = makeRepo({ 'crates/a/src/lib.rs': repeat('// line', 20) });
  try {
    sealBaseline(dir);
    const result = run(dir);
    assert.equal(result.code, 1);
    assert.ok(kinds(result).includes('over-limit-without-exception'));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a reasoned exception clears the limit finding but not the baseline delta', () => {
  const dir = makeRepo({
    'crates/a/src/lib.rs': repeat('// line', 20),
    'scripts/architecture-exceptions.json': JSON.stringify({
      schemaVersion: 1,
      files: {
        'crates/a/src/lib.rs': {
          category: 'rust',
          reason: 'Cohesive recovery module awaiting a reviewed split in a later phase.',
        },
      },
    }),
  });
  try {
    sealBaseline(dir);
    const result = run(dir);
    assert.deepEqual(kinds(result), []);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('an exception with a token reason is rejected', () => {
  const dir = makeRepo({
    'crates/a/src/lib.rs': repeat('// line', 20),
    'scripts/architecture-exceptions.json': JSON.stringify({
      schemaVersion: 1,
      files: { 'crates/a/src/lib.rs': { category: 'rust', reason: 'big' } },
    }),
  });
  try {
    sealBaseline(dir);
    const result = run(dir);
    assert.ok(kinds(result).includes('exception-without-reason'));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('growth after sealing the baseline is reported as a delta', () => {
  const dir = makeRepo({ 'crates/a/src/lib.rs': repeat('// line', 5) });
  try {
    sealBaseline(dir);
    writeFileSync(path.join(dir, 'crates/a/src/lib.rs'), repeat('// line', 8));
    git(['add', '-A'], dir);
    git(['commit', '-qm', 'grow'], dir);
    const result = run(dir);
    assert.equal(result.code, 1);
    const delta = result.body.findings.find((finding) => finding.kind === 'baseline-delta');
    assert.ok(delta, 'expected a baseline-delta finding');
    assert.match(delta.detail, /5 -> 8/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a new source file that is absent from the baseline is reported', () => {
  const dir = makeRepo({ 'crates/a/src/lib.rs': repeat('// line', 5) });
  try {
    sealBaseline(dir);
    mkdirSync(path.join(dir, 'crates/a/src/extra'), { recursive: true });
    writeFileSync(path.join(dir, 'crates/a/src/extra/mod.rs'), repeat('// line', 2));
    git(['add', '-A'], dir);
    git(['commit', '-qm', 'add'], dir);
    const result = run(dir);
    assert.ok(kinds(result).includes('baseline-missing-entry'));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a file moved with identical content is reported as a baseline rename', () => {
  const dir = makeRepo({ 'crates/a/src/lib.rs': repeat('// line', 5) });
  try {
    sealBaseline(dir);
    mkdirSync(path.join(dir, 'crates/a/src/moved'), { recursive: true });
    writeFileSync(
      path.join(dir, 'crates/a/src/moved/mod.rs'),
      readFileSync(path.join(dir, 'crates/a/src/lib.rs')),
    );
    execFileSync('git', ['rm', '-q', 'crates/a/src/lib.rs'], { cwd: dir });
    git(['add', '-A'], dir);
    git(['commit', '-qm', 'move'], dir);
    const result = run(dir);
    assert.ok(kinds(result).includes('baseline-renamed'));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a relative import cycle is reported with the cycle path', () => {
  const dir = makeRepo({
    'web/a.js': "import { b } from './b.js';\nexport const a = b;\n",
    'web/b.js': "import { a } from './a.js';\nexport const b = a;\n",
  });
  try {
    sealBaseline(dir);
    const result = run(dir);
    const cycle = result.body.findings.find((finding) => finding.kind === 'dependency-cycle');
    assert.ok(cycle, 'expected a dependency-cycle finding');
    assert.ok(cycle.detail.includes('->'));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('an unresolvable relative source import is reported', () => {
  const dir = makeRepo({ 'web/a.js': "import { b } from './missing.js';\nexport const a = b;\n" });
  try {
    sealBaseline(dir);
    const result = run(dir);
    assert.ok(kinds(result).includes('unresolved-relative-import'));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a declared dependency boundary violation is reported', () => {
  const dir = makeRepo({
    'scripts/architecture-policy.json': JSON.stringify({
      ...policy,
      dependencyRules: [
        {
          id: 'no-web-back-edge',
          description: 'a leaf module must not import its consumer',
          from: ['web/b.js'],
          to: ['web/a.js'],
        },
      ],
    }),
    'web/a.js': "import { b } from './b.js';\nexport const a = b;\n",
    'web/b.js': "import { a } from './a.js';\nexport const b = a;\n",
  });
  try {
    sealBaseline(dir);
    const result = run(dir);
    const boundary = result.body.findings.filter(
      (finding) => finding.kind === 'dependency-boundary',
    );
    assert.ok(boundary.length > 0, 'expected a dependency-boundary finding');
    assert.match(boundary[0].detail, /no-web-back-edge/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('--write-baseline writes a reviewed snapshot of the tracked sources', () => {
  const dir = makeRepo({ 'crates/a/src/lib.rs': repeat('// line', 4) });
  try {
    execFileSync('node', [script, '--write-baseline'], { cwd: dir, encoding: 'utf8' });
    const baseline = JSON.parse(
      readFileSync(path.join(dir, 'scripts/architecture-baseline.json'), 'utf8'),
    );
    assert.ok(existsSync(path.join(dir, 'scripts/architecture-baseline.json')));
    assert.equal(baseline.files['crates/a/src/lib.rs'].lines, 4);
    assert.equal(baseline.files['crates/a/src/lib.rs'].category, 'rust');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a missing policy file fails closed instead of reporting success', () => {
  const dir = makeRepo({ 'crates/a/src/lib.rs': repeat('// line', 4) });
  try {
    rmSync(path.join(dir, 'scripts/architecture-policy.json'));
    git(['add', '-A'], dir);
    git(['commit', '-qm', 'remove policy'], dir);
    const result = run(dir);
    assert.equal(result.code, 2);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
