// A test file that no suite runs is a check nobody is doing. This holds the
// repository's test scripts to every test file in the tree, so a new directory
// of tests cannot sit silently unrun (worker/ did, for a while).
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const root = new URL('../..', import.meta.url).pathname.replace(/\/$/, '');
const SKIP = new Set(['node_modules', 'dist', 'target', '.git', '.claude', '.astro', 'public']);

function testFiles(dir, out = []) {
  for (const f of readdirSync(dir)) {
    if (SKIP.has(f)) continue;
    const p = join(dir, f);
    if (statSync(p).isDirectory()) testFiles(p, out);
    else if (f.endsWith('.test.mjs')) out.push(relative(root, p));
  }
  return out;
}

/** The quoted globs a `node --test` script passes, as anchored regexes. */
function globs(script) {
  return [...script.matchAll(/'([^']+)'/g)].map(([, g]) =>
    new RegExp('^' + g.replace(/[.]/g, '\\.').replace(/\*\*\//g, '(?:.+/)?').replace(/\*/g, '[^/]*') + '$'),
  );
}

test('every test file in the repository is run by a test script', () => {
  const rootScript = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')).scripts['test:js'];
  const webScripts = JSON.parse(readFileSync(join(root, 'apps/web/package.json'), 'utf8')).scripts;
  const webPatterns = [...globs(webScripts.test), ...globs(webScripts['test:browser'])];
  const patterns = [...globs(rootScript), ...webPatterns.map((r) => new RegExp(r.source.replace('^', '^apps/web/')))];
  const files = testFiles(root);
  assert.ok(files.length > 10, 'found the test files');
  for (const f of files) {
    assert.ok(patterns.some((p) => p.test(f)), `${f} is not run by a declared test script`);
  }
});

test('continuous integration runs those same scripts, not its own copy of the globs', () => {
  const ci = readFileSync(join(root, '.github/workflows/ci.yml'), 'utf8');
  assert.match(ci, /run: npm run test:js$/m);
  assert.match(ci, /npm test --prefix apps\/web$/m);
  assert.match(ci, /run: npm run test:browser --prefix apps\/web$/m);
  assert.doesNotMatch(ci, /node --test/, 'CI spells out globs that can drift from the scripts');
});
