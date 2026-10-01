#!/usr/bin/env node
// Documentation consistency audit.
//
// Read-only. Reports structural problems in the tracked Markdown tree:
// unparsable relative links, documents nothing links to, links pinned to
// stale branches, skills missing frontmatter, and release notes that the
// release index never mentions.
//
// It intentionally does not judge whether prose is true. A failure here is a
// structural signal, not a product finding.

import { execFileSync } from 'node:child_process';
import { readFileSync, existsSync } from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function gitLines(args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8' })
    .split('\n')
    .filter(Boolean);
}

const tracked = gitLines(['ls-files']);
const mdFiles = tracked.filter((f) => f.endsWith('.md')).sort();

// Category defines the single authority a document is allowed to speak for.
function category(file) {
  if (file === 'AGENTS.md' || file.startsWith('.agents/')) return 'instruction';
  if (file.startsWith('docs/release/notes/')) return 'release-note';
  if (file.startsWith('docs/release/migration/')) return 'release-migration';
  if (file.startsWith('docs/release/')) return 'release-governance';
  if (file.startsWith('docs/development/')) return 'development';
  if (file.startsWith('docs/architecture/')) return 'architecture';
  if (file.startsWith('docs/validation/')) return 'validation';
  if (file.startsWith('docs/status/')) return 'handoff-status';
  if (file.startsWith('docs/review/')) return 'audit';
  if (file.startsWith('docs/references/')) return 'external-sources';
  if (file.startsWith('aidlc-docs/')) return 'design-snapshot';
  if (file === 'docs/README.md' || file === 'docs/AGENTS.md') return 'docs-index';
  return 'root';
}

// Entry points are reachable by convention, so they are not orphans.
const entryPoints = new Set([
  'README.md',
  'AGENTS.md',
  'SECURITY.md',
  'THIRD_PARTY_NOTICES.md',
  'docs/README.md',
  'docs/AGENTS.md',
]);

const problems = { deadLinks: [], orphans: [], staleBranch: [], skillsFrontmatter: [], unreferencedNotes: [] };
const text = new Map();
const inbound = new Map(mdFiles.map((f) => [f, 0]));

for (const file of mdFiles) {
  const body = readFileSync(path.join(root, file), 'utf8');
  text.set(file, body);
}

for (const file of mdFiles) {
  const body = text.get(file);
  const dir = path.posix.dirname(file);

  // Relative markdown links, both `](x.md)` and `](<dir>/x.md)`.
  for (const m of body.matchAll(/\]\(([^)\s#]+\.md)(?:#[^)\s]*)?\)/g)) {
    const target = m[1];
    if (/^https?:/i.test(target)) continue;
    const resolved = target.startsWith('/')
      ? target.slice(1)
      : path.posix.normalize(path.posix.join(dir, target));
    if (!existsSync(path.join(root, resolved))) {
      problems.deadLinks.push({ from: file, target, resolved });
    } else if (inbound.has(resolved)) {
      inbound.set(resolved, inbound.get(resolved) + 1);
    }
  }

  // Markdown links carry most authority, but bare backtick paths are how this
  // repository has historically pointed at documents, so count those too.
  for (const m of body.matchAll(/`((?:docs|\.agents|aidlc-docs)\/[\w./-]+\.md)`/g)) {
    const resolved = m[1];
    if (existsSync(path.join(root, resolved)) && inbound.has(resolved)) {
      inbound.set(resolved, inbound.get(resolved) + 1);
    }
  }

  // Directory-level references. `AGENTS.md:171` points at `.agents/skills/`
  // and `docs/README.md:68` points at `../.agents/skills/`, which names no
  // single file, so a file-only rule would wrongly call every skill an
  // orphan. Count a reference to a directory as a reference to its docs.
  for (const m of body.matchAll(/`((?:\.\.\/)?(?:\.agents|docs|aidlc-docs)(?:\/[\w.-]+)*\/?)`/g)) {
    const dir = m[1].replace(/^\.\.\//, '').replace(/\/$/, '');
    if (!existsSync(path.join(root, dir))) continue;
    for (const [candidate, count] of inbound) {
      if (count > 0) continue;
      if (candidate === dir || candidate.startsWith(`${dir}/`)) {
        inbound.set(candidate, 1);
      }
    }
  }

  // Branch-pinned GitHub links. A link that points at an ordinary document is
  // navigation and must follow the default branch. A validation, handoff or
  // audit record that names the branch it was executed against is evidence:
  // the branch and SHA together define which code the result applies to, so
  // rewriting it would silently repoint the record at different code.
  for (const m of body.matchAll(/https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/blob\/(.+?)\/docs?\//g)) {
    const branch = m[1];
    if (/^(main|master)$/.test(branch)) continue;
    const line = body.slice(0, m.index).split('\n').pop();
    // These categories record which revision was tested or handed over.
    const isEvidence =
      /(HEAD|source|revision|基于|针对|验证|批次|batch|input|commit|当前分支)/i.test(line);
    if (isEvidence) continue;
    problems.staleBranch.push({ from: file, branch, line: line.trim().slice(0, 100) });
  }
}
for (const file of mdFiles) {
  if (inbound.get(file) === 0 && !entryPoints.has(file)) {
    problems.orphans.push(file);
  }
}

for (const file of tracked.filter((f) => f.endsWith('SKILL.md'))) {
  const body = readFileSync(path.join(root, file), 'utf8');
  if (!body.startsWith('---')) {
    problems.skillsFrontmatter.push(file);
  }
}

// Every release note must be reachable from the release index, otherwise the
// index and the notes directory disagree.
const index = text.get('docs/release/release-history.md') ?? '';
for (const file of mdFiles.filter((f) => category(f) === 'release-note')) {
  const name = path.posix.basename(file);
  if (!index.includes(name)) problems.unreferencedNotes.push(file);
}

const counts = {};
for (const file of mdFiles) {
  const c = category(file);
  counts[c] = (counts[c] ?? 0) + 1;
}

if (process.argv.includes('--json')) {
  console.log(JSON.stringify({ total: mdFiles.length, counts, problems }, null, 2));
} else {
  console.log(`tracked markdown: ${mdFiles.length}`);
  for (const [c, n] of Object.entries(counts).sort((a, b) => b[1] - a[1])) {
    console.log(`  ${String(n).padStart(3)}  ${c}`);
  }
  const rows = [
    ['dead relative links', problems.deadLinks],
    ['unlinked documents', problems.orphans],
    ['stale branch links', problems.staleBranch],
    ['skills without frontmatter', problems.skillsFrontmatter],
    ['release notes missing from index', problems.unreferencedNotes],
  ];
  console.log('');
  let failed = 0;
  for (const [label, items] of rows) {
    console.log(`${items.length === 0 ? 'OK  ' : 'FAIL'}  ${label}: ${items.length}`);
    for (const item of items.slice(0, 12)) {
      console.log(`        ${typeof item === 'string' ? item : JSON.stringify(item)}`);
    }
    if (items.length > 12) console.log(`        ... ${items.length - 12} more`);
    failed += items.length;
  }
  console.log('');
  console.log(failed === 0 ? 'docs audit: PASS' : `docs audit: ${failed} finding(s)`);
}

// Non-zero exit lets CI or a pre-commit hook fail on structural drift.
if (
  problems.deadLinks.length ||
  problems.skillsFrontmatter.length ||
  problems.unreferencedNotes.length
) {
  process.exitCode = 1;
}
