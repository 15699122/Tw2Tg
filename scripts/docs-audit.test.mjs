// Tests for scripts/docs-audit.mjs.
//
// Each case builds a throwaway Git repository and runs the real audit
// against it, so a passing test means the script behaves, not that a mock
// agrees with the implementation.
//
// The three regression cases marked "Regression" cover defects this audit
// shipped with: a line-anchored pattern that missed navigation links, a
// branch capture that stopped at the first slash, and a file-only reference
// rule that reported every skill as unlinked.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

const script = path.resolve('scripts/docs-audit.mjs');

function git(args, cwd) {
  return execFileSync('git', args, { cwd, encoding: 'utf8' });
}

function makeRepo(files) {
  const dir = mkdtempSync(path.join(tmpdir(), 'docs-audit-'));
  git(['init', '-q'], dir);
  git(['config', 'user.email', 'test@example.com'], dir);
  git(['config', 'user.name', 'test'], dir);
  for (const [name, body] of Object.entries(files)) {
    const full = path.join(dir, name);
    mkdirSync(path.dirname(full), { recursive: true });
    writeFileSync(full, body);
  }
  git(['add', '-A'], dir);
  git(['commit', '-qm', 'fixture'], dir);
  return dir;
}

// The audit exits non-zero when it finds problems, so the JSON body has to be
// read without depending on a clean exit.
function audit(dir) {
  // The audit deliberately exits non-zero when it finds problems, so a
  // finding-laden fixture raises here. The JSON body is still the payload.
  try {
    const out = execFileSync('node', [script, '--json'], {
      cwd: dir,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return JSON.parse(out);
  } catch (err) {
    return JSON.parse(err.stdout);
  }
}

function exitCode(dir) {
  try {
    execFileSync('node', [script], { cwd: dir, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
    return 0;
  } catch (err) {
    return err.status;
  }
}

function withRepo(files, fn) {
  const dir = makeRepo(files);
  try {
    fn(audit(dir));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test('a clean repository reports no findings', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n',
      'docs/README.md': '[rules](../AGENTS.md)\n\n[a](a.md)\n',
      'docs/a.md': '# A\n',
    },
    (r) => {
      for (const [name, items] of Object.entries(r.problems)) {
        assert.equal(items.length, 0, `${name} should be empty`);
      }
    },
  );
});

test('a relative link to a missing document is reported with its resolved path', () => {
  withRepo({ 'AGENTS.md': '# Rules\n', 'docs/README.md': '[gone](nope.md)\n' }, (r) => {
    assert.equal(r.problems.deadLinks.length, 1);
    assert.equal(r.problems.deadLinks[0].target, 'nope.md');
    assert.equal(r.problems.deadLinks[0].resolved, 'docs/nope.md');
  });
});

// Regression: the first version anchored the pattern to line start, so
// navigation links written as list items were reported as clean.
test('a branch-pinned navigation link inside a list item is reported', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n',
      'docs/README.md': '[a](architecture/a.md)\n',
      'docs/notes/n1.md':
        '# N\n\n- [index](https://github.com/o/r/blob/old-feature/docs/README.md)\n' +
        '- [arch](https://github.com/o/r/blob/old-feature/docs/architecture/overview.md)\n',
    },
    (r) => {
      assert.equal(r.problems.staleBranch.length, 2);
      assert.ok(r.problems.staleBranch.every((x) => x.branch === 'old-feature'));
    },
  );
});

// Regression: branch names contain slashes; the capture must not stop early.
test('a multi-segment branch name is captured whole', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n',
      'docs/README.md': '[a](architecture/a.md)\n',
      'docs/notes/n1.md':
        '# N\n\n- [d](https://github.com/o/r/blob/feature/u7-desktop-production-integration/docs/README.md)\n',
    },
    (r) => assert.equal(r.problems.staleBranch[0].branch, 'feature/u7-desktop-production-integration'),
  );
});

test('default-branch links are not reported as drift', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n',
      'docs/README.md': '[a](architecture/a.md)\n',
      'docs/notes/n1.md':
        '# N\n\n- [d](https://github.com/o/r/blob/main/docs/README.md)\n' +
        '- [d](https://github.com/o/r/blob/master/docs/README.md)\n',
    },
    (r) => assert.equal(r.problems.staleBranch.length, 0),
  );
});

// Regression: rewriting a validation record's branch would repoint the
// evidence at different code, so these must not be flagged.
test('a branch named in a validation record is preserved, not flagged', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n',
      'docs/README.md': '[q](validation/q.md)\n',
      'docs/validation/q.md':
        '# Q\n\nLinux source was feature/u7-desktop-production-integration / HEAD 79232f24.\n',
    },
    (r) => assert.equal(r.problems.staleBranch.length, 0),
  );
});

// Regression: AGENTS.md referenced `.agents/skills/` as a directory, and a
// file-only rule reported every skill as unlinked.
test('a directory reference counts as a reference to the documents inside it', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n\nProcedures: `.agents/skills/`\n',
      '.agents/skills/demo/SKILL.md': '# Demo\n',
    },
    (r) => assert.equal(r.problems.orphans.length, 0),
  );
});

test('a document nothing references is still reported as unlinked', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n',
      'docs/README.md': '[a](architecture/a.md)\n',
      'docs/architecture/a.md': '# A\n',
      'docs/orphan.md': '# Orphan\n',
    },
    (r) => {
      assert.equal(r.problems.orphans.length, 1);
      assert.equal(r.problems.orphans[0], 'docs/orphan.md');
    },
  );
});

test('a skill without frontmatter is reported', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n\nProcedures: `.agents/skills/`\n',
      '.agents/skills/demo/SKILL.md': '# Demo\n\nBody.\n',
    },
    (r) => assert.equal(r.problems.skillsFrontmatter.length, 1),
  );
});

test('a skill with frontmatter is not reported', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n\nProcedures: `.agents/skills/`\n',
      '.agents/skills/demo/SKILL.md': '---\nname: demo\ndescription: Demo.\n---\n\n# Demo\n',
    },
    (r) => assert.equal(r.problems.skillsFrontmatter.length, 0),
  );
});

test('a release note absent from the index is reported', () => {
  withRepo(
    {
      'AGENTS.md': '# Rules\n',
      'docs/README.md': '[h](release/release-history.md)\n',
      'docs/release/release-history.md': '# H\n\n| v | Notes |\n|---|---|\n| v1 | [notes/v1.md](notes/v1.md) |\n',
      'docs/release/notes/v1.md': '# v1\n',
      'docs/release/notes/v2.md': '# v2\n',
    },
    (r) => {
      assert.equal(r.problems.unreferencedNotes.length, 1);
      assert.equal(r.problems.unreferencedNotes[0], 'docs/release/notes/v2.md');
    },
  );
});

test('findings set a non-zero exit code and a clean tree does not', () => {
  const dirty = makeRepo({ 'AGENTS.md': '# Rules\n', 'docs/README.md': '[gone](nope.md)\n' });
  const clean = makeRepo({ 'AGENTS.md': '# Rules\n' });
  try {
    assert.equal(exitCode(dirty), 1);
    assert.equal(exitCode(clean), 0);
  } finally {
    rmSync(dirty, { recursive: true, force: true });
    rmSync(clean, { recursive: true, force: true });
  }
});
