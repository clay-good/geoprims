import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { packageRelease } from './package.mjs';

test('release assets have deterministic checksums and useful notes', () => {
  const first = packageRelease(mkdtempSync(join(tmpdir(), 'geoprims-release-')));
  const second = packageRelease(mkdtempSync(join(tmpdir(), 'geoprims-release-')));
  assert.equal(first.sums, second.sums);
  assert.match(first.sums, /geoprims-[\d.]+\.mcpb/);
  assert.match(first.notes, /gh attestation verify/);
  assert.match(first.notes, /operations and \d+ endpoints/);
  assert.equal(first.assets.length, 5);
});
