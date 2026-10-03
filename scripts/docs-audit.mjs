#!/usr/bin/env node
// Documentation consistency audit.
//
// Read-only. Reports structural problems in the tracked Markdown tree:
// unparsable relative links, documents nothing links to, links pinned to
// stale branches, skills missing frontmatter, release notes that the
// release index never mentions, consecutive duplicate top-level titles,
// multiple top-level titles, and suspiciously long non-code lines.
//
// It intentionally does not judge whether prose is true. A failure here is a
// structural signal, not a product finding. Warning-level heading checks
// report possible title problems without failing the audit, because
// historical snapshots may legitimately repeat template headings under
// different parents or revisions.

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
const warnings = { consecutiveDuplicateH1: [], multipleH1: [], longLines: [] };
const text = new Map();
const inbound = new Map(mdFiles.map((f) => [f, 0]));

// True only for lines inside a fenced code block; headings and long lines in
// examples are fixtures, not document structure.
function fenceMap(body) {
  const inside = new Array(body.split('\n').length).fill(false);
  let fenced = false;
  body.split('\n').forEach((line, index) => {
    if (/^(`{3,}|~{3,})/.test(line.trim())) fenced = !fenced;
    inside[index] = fenced;
  });
  return inside;
}

function slugifyHeading(heading) {
  return heading
    .trim()
    .toLowerCase()
    .replace(/[`*_~]/g, '')
    .replace(/[^\p{L}\p{N}\s-]/gu, '')
    .trim()
    .replace(/\s+/g, '-');
}

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
// index and the notes directory disagree. Resolve the index's links rather
// than substring-matching the basename: `notes/v1.md` is a prefix of
// `notes/v10.md`, so a text search reports an indexed note as missing.
const indexFile = 'docs/release/release-history.md';
const indexedNotes = new Set();
if (text.has(indexFile)) {
  const indexBody = text.get(indexFile);
  const indexDir = path.posix.dirname(indexFile);
  for (const m of indexBody.matchAll(/\]\(([^)\s#]+\.md)(?:#[^)\s]*)?\)/g)) {
    const resolved = path.posix.normalize(path.posix.join(indexDir, m[1]));
    if (resolved.startsWith('docs/release/notes/')) indexedNotes.add(resolved);
  }
}
for (const file of mdFiles.filter((f) => category(f) === 'release-note')) {
  if (!indexedNotes.has(file)) problems.unreferencedNotes.push(file);
}

// Warning-level structural checks. These never fail the audit: historical
// snapshots may repeat template headings, and only a human can decide
// whether repeated prose preserves distinct revision-bound evidence.
for (const file of mdFiles) {
  const body = text.get(file);
  const lines = body.split('\n');
  const fenced = fenceMap(body);
  const headings = [];
  lines.forEach((line, index) => {
    if (fenced[index]) return;
    const match = line.match(/^(#{1,6})\s+(.+?)\s*$/);
    if (match) headings.push({ level: match[1].length, text: match[2], line: index + 1 });
    // Only flag extreme non-table prose lines: encoded blobs or pasted
    // output, not ordinary long Markdown paragraphs or tables.
    const trimmed = line.trim();
    if (
      !fenced[index] &&
      line.length > 2000 &&
      !trimmed.startsWith('|') &&
      !trimmed.startsWith('>') &&
      !trimmed.startsWith('-') &&
      !trimmed.startsWith('*') &&
      !/^\d+\./.test(trimmed)
    ) {
      warnings.longLines.push({ file, line: index + 1, length: line.length });
    }
  });
  const h1 = headings.filter((h) => h.level === 1);
  if (h1.length > 1) {
    const distinct = new Set(h1.map((h) => slugifyHeading(h.text)));
    warnings.multipleH1.push({
      file,
      lines: h1.map((h) => h.line),
      // Counter-example: a history file may intentionally preserve several
      // revision-bound snapshots under repeated titles.
      distinctSlugs: distinct.size,
    });
  }
  for (let i = 1; i < h1.length; i += 1) {
    if (slugifyHeading(h1[i].text) === slugifyHeading(h1[i - 1].text) && h1[i].line === h1[i - 1].line + 1) {
      // Positive example: two identical H1 lines back to back with no prose
      // between them are never two distinct snapshots.
      warnings.consecutiveDuplicateH1.push({ file, lines: [h1[i - 1].line, h1[i].line], text: h1[i].text });
    }
  }
}

const counts = {};
for (const file of mdFiles) {
  const c = category(file);
  counts[c] = (counts[c] ?? 0) + 1;
}

if (process.argv.includes('--json')) {
  console.log(JSON.stringify({ total: mdFiles.length, counts, problems, warnings }, null, 2));
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
  console.log('');
  const warningRows = [
    ['consecutive duplicate H1 (warning)', warnings.consecutiveDuplicateH1],
    ['multiple H1 (warning)', warnings.multipleH1],
    ['long prose lines (warning)', warnings.longLines],
  ];
  for (const [label, items] of warningRows) {
    console.log(`WARN  ${label}: ${items.length}`);
    for (const item of items.slice(0, 12)) {
      console.log(`        ${JSON.stringify(item)}`);
    }
    if (items.length > 12) console.log(`        ... ${items.length - 12} more`);
  }
}

if (
  problems.deadLinks.length ||
  problems.skillsFrontmatter.length ||
  problems.unreferencedNotes.length
) {
  process.exitCode = 1;
}
