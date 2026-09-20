import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { artifactDigests } from './artifact-digests.mjs';

test('artifact manifest is sorted and detects a changed shipped byte', (t) => {
  const dir = mkdtempSync(join(tmpdir(), 'geoprims-digests-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  mkdirSync(join(dir, 'site'));
  writeFileSync(join(dir, 'site/z.js'), 'first');
  writeFileSync(join(dir, 'site/a.js'), 'same');
  const before = artifactDigests(dir, ['site']);
  assert.deepEqual(before.trim().split('\n').map((row) => row.split('  ')[1]), ['site/a.js', 'site/z.js']);
  writeFileSync(join(dir, 'site/z.js'), 'second');
  assert.notEqual(artifactDigests(dir, ['site']), before);
});
