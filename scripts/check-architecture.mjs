#!/usr/bin/env node
// Architecture checker.
//
// Read-only unless `--write-baseline` is passed. It reports four signals:
//
//   1. file-size categories and limits,
//   2. baseline deltas (a file that grew or shrank since the reviewed snapshot),
//   3. dependency edges and cycles between JavaScript/TypeScript source files,
//   4. renames of baseline files.
//
// The baseline is a *reviewed snapshot*, never regenerated automatically. A real
// change to it must be a deliberate commit that shows the delta. Line counts are
// a review signal, not a decomposition algorithm: a cohesive file may stay large
// only through an explicit, reasoned exception.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const args = process.argv.slice(2);
const jsonMode = args.includes('--json');
const writeBaseline = args.includes('--write-baseline');

function fail(message) {
  if (jsonMode) {
    process.stdout.write(`${JSON.stringify({ error: message }, null, 2)}\n`);
  } else {
    process.stderr.write(`architecture-check: ${message}\n`);
  }
  process.exit(2);
}

function readJson(file) {
  const full = path.join(root, file);
  if (!existsSync(full)) fail(`missing ${file}`);
  try {
    return JSON.parse(readFileSync(full, 'utf8'));
  } catch (error) {
    return fail(`${file} is not valid JSON: ${error.message}`);
  }
}

function trackedFiles() {
  return execFileSync('git', ['ls-files'], { cwd: root, encoding: 'utf8' })
    .split('\n')
    .filter(Boolean);
}

function globToRegExp(glob) {
  const escaped = glob
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*\*\//g, '\u0000')
    .replace(/\*\*/g, '\u0001')
    .replace(/\*/g, '[^/]*')
    .replace(/\u0000/g, '(?:.*/)?')
    .replace(/\u0001/g, '.*');
  return new RegExp(`^${escaped}$`);
}

function lineCount(file) {
  const body = readFileSync(path.join(root, file), 'utf8');
  if (body.length === 0) return 0;
  return body.split('\n').length - (body.endsWith('\n') ? 1 : 0);
}

function contentHash(file) {
  return createHash('sha256')
    .update(readFileSync(path.join(root, file)))
    .digest('hex');
}

const policy = readJson('scripts/architecture-policy.json');
const exceptions = readJson('scripts/architecture-exceptions.json');
const baseline = existsSync(path.join(root, 'scripts/architecture-baseline.json'))
  ? readJson('scripts/architecture-baseline.json')
  : { files: {} };

const compiled = policy.categories.map((category) => ({
  ...category,
  matchers: category.globs.map(globToRegExp),
}));

function categoryFor(file) {
  return compiled.find((category) => category.matchers.some((m) => m.test(file)));
}

const files = trackedFiles();
const findings = [];
const sizes = {};

for (const file of files) {
  const category = categoryFor(file);
  if (!category) continue;
  const lines = lineCount(file);
  sizes[file] = { category: category.id, lines, sha256: contentHash(file) };
  const exception = exceptions.files?.[file];
  if (exception && exception.category !== category.id) {
    findings.push({
      kind: 'exception-category-mismatch',
      file,
      detail: `exception declares ${exception.category} but the path matches ${category.id}`,
    });
  }
  const limit = category.limitLines;
  if (lines > limit && !exception) {
    findings.push({
      kind: 'over-limit-without-exception',
      file,
      detail: `${lines} lines exceeds the ${category.id} limit of ${limit}`,
    });
  }
  if (exception && exception.reason.trim().length < 20) {
    findings.push({
      kind: 'exception-without-reason',
      file,
      detail: 'an exception must explain why the file stays large or exempt',
    });
  }
}

if (writeBaseline) {
  writeFileSync(
    path.join(root, 'scripts/architecture-baseline.json'),
    `${JSON.stringify({ schemaVersion: policy.schemaVersion, files: sizes }, null, 2)}\n`,
  );
  process.stdout.write(`wrote scripts/architecture-baseline.json for ${Object.keys(sizes).length} files\n`);
  process.exit(0);
}

// Baseline deltas and renames.
const baselineFiles = baseline.files ?? {};
for (const [file, current] of Object.entries(sizes)) {
  const previous = baselineFiles[file];
  if (!previous) {
    findings.push({
      kind: 'baseline-missing-entry',
      file,
      detail: 'tracked source file is absent from the reviewed baseline',
    });
    continue;
  }
  if (previous.lines !== current.lines) {
    findings.push({
      kind: 'baseline-delta',
      file,
      detail: `lines changed ${previous.lines} -> ${current.lines} (${current.lines - previous.lines > 0 ? '+' : ''}${current.lines - previous.lines})`,
    });
  }
}
for (const file of Object.keys(baselineFiles)) {
  if (sizes[file]) continue;
  const renamed = Object.entries(sizes).find(
    ([, current]) => current.sha256 === baselineFiles[file].sha256,
  );
  findings.push({
    kind: renamed ? 'baseline-renamed' : 'baseline-missing-file',
    file,
    detail: renamed
      ? `baseline file moved to ${renamed[0]}; update the baseline deliberately`
      : 'baseline file no longer exists as a tracked source file',
  });
}


// Dependency edges and cycles between JavaScript/TypeScript source files.
// Only relative imports are examined: package imports are a dependency policy
// question, not a module-cycle question for this checker.
const jsCategory = policy.categories.find((category) => category.id === policy.dependencyCategory);
const jsMatchers = jsCategory ? jsCategory.globs.map(globToRegExp) : [];
const jsFiles = files.filter((file) => jsMatchers.some((m) => m.test(file)));
const jsSet = new Set(jsFiles);

function isSourceCandidate(spec) {
  return /\.(m?js|cjs|ts|tsx)$/.test(spec);
}

function resolveSpecifier(from, spec) {
  const base = path.posix.normalize(path.posix.join(path.posix.dirname(from), spec));
  if (jsSet.has(base)) return base;
  for (const suffix of ['.js', '.mjs', '.ts', '.tsx', '.cjs', '/index.js', '/index.mjs']) {
    if (jsSet.has(base + suffix)) return base + suffix;
  }
  return null;
}

const graph = new Map();
const unresolved = [];
for (const file of jsFiles) {
  const body = readFileSync(path.join(root, file), 'utf8');
  const edges = new Set();
  for (const match of body.matchAll(/(?:from\s+|import\s*\(\s*|require\s*\(\s*)['"]([^'"]+)['"]/g)) {
    const spec = match[1];
    if (!spec.startsWith('.')) continue;
    if (!isSourceCandidate(spec) && !spec.endsWith('/')) continue;
    const resolved = resolveSpecifier(file, spec);
    if (resolved) edges.add(resolved);
    else if (isSourceCandidate(spec)) unresolved.push({ file, spec });
  }
  graph.set(file, [...edges]);
}

for (const { file, spec } of unresolved) {
  findings.push({
    kind: 'unresolved-relative-import',
    file,
    detail: `relative import ${spec} does not resolve to a tracked source file`,
  });
}

for (const rule of policy.dependencyRules ?? []) {
  const fromMatchers = rule.from.map(globToRegExp);
  const toMatchers = rule.to.map(globToRegExp);
  for (const [file, edges] of graph) {
    if (!fromMatchers.some((m) => m.test(file))) continue;
    for (const edge of edges) {
      if (!toMatchers.some((m) => m.test(edge))) continue;
      findings.push({
        kind: 'dependency-boundary',
        file,
        detail: `${rule.id}: ${file} must not import ${edge}`,
      });
    }
  }
}

// Cycle detection over the file graph (iterative DFS with a colour map).
const WHITE = 0;
const GREY = 1;
const BLACK = 2;
const colour = new Map(jsFiles.map((file) => [file, WHITE]));

for (const start of jsFiles) {
  if (colour.get(start) !== WHITE) continue;
  const stack = [[start, 0]];
  const trail = [];
  while (stack.length) {
    const frame = stack[stack.length - 1];
    const [file, index] = frame;
    if (index === 0) {
      colour.set(file, GREY);
      trail.push(file);
    }
    const edges = graph.get(file) ?? [];
    if (index >= edges.length) {
      colour.set(file, BLACK);
      stack.pop();
      trail.pop();
      continue;
    }
    frame[1] = index + 1;
    const next = edges[index];
    const state = colour.get(next);
    if (state === GREY) {
      const startIndex = trail.indexOf(next);
      findings.push({
        kind: 'dependency-cycle',
        file: next,
        detail: [...trail.slice(startIndex), next].join(' -> '),
      });
    } else if (state === WHITE) {
      stack.push([next, 0]);
    }
  }
}

const summary = {
  files: Object.keys(sizes).length,
  jsFiles: jsFiles.length,
  edges: [...graph.values()].reduce((total, edges) => total + edges.length, 0),
  findings: findings.length,
};

if (jsonMode) {
  process.stdout.write(`${JSON.stringify({ summary, findings }, null, 2)}\n`);
} else {
  for (const finding of findings) {
    process.stdout.write(`FAIL ${finding.kind} ${finding.file}: ${finding.detail}\n`);
  }
  process.stdout.write(
    `architecture-check: ${summary.files} files, ${summary.jsFiles} js/ts, ` +
      `${summary.edges} relative edges, ${summary.findings} findings\n`,
  );
}

process.exit(findings.length === 0 ? 0 : 1);
