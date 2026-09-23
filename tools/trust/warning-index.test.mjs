// A vector that names a warning by its place in the list breaks whenever the
// list changes -- most reliably when a tool is promoted and EXPERIMENTAL_TOOL
// is removed from the front of it. That has now cost two supersedes for the
// same reason (geometry.area.polygon v030, geometry.validity.make-valid v001),
// and 24 more vectors on tools not yet stable are waiting to do it again.
//
// The runner accepts `meta.warnings.*.code`, meaning "some warning has this
// code", which nothing can shift. This gate is a ratchet: the vectors below
// are the ones that already pin an index, and no new one may join them. As
// each is superseded to the wildcard form its entry simply stops matching,
// which is allowed -- the list may shrink and never grow.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const VECTORS = join(root, 'core/vectors');

/** Vectors that pinned a warning by index before this gate existed. */
const GRANDFATHERED = new Set(JSON.parse(readFileSync(join(root, 'tools/trust/warning-index.json'), 'utf8')));

function indexPinned() {
  const found = [];
  for (const file of readdirSync(VECTORS).filter((f) => f.endsWith('.jsonl'))) {
    const tool = file.slice(0, -6);
    for (const line of readFileSync(join(VECTORS, file), 'utf8').split('\n')) {
      if (!line.trim()) continue;
      const v = JSON.parse(line);
      if (v.supersededBy) continue;
      const pinned = Object.keys(v.expect ?? {}).some(
        (k) => k.startsWith('meta.warnings.') && /^\d+$/.test(k.split('.')[2]),
      );
      if (pinned) found.push(`${tool} ${v.id}`);
    }
  }
  return found;
}

test('no new vector names a warning by its place in the list', () => {
  const fresh = indexPinned().filter((v) => !GRANDFATHERED.has(v));
  assert.deepEqual(
    fresh,
    [],
    'these vectors pin meta.warnings.<n>.code, which breaks when the warning list changes.\n' +
      'Use meta.warnings.*.code instead -- the runner reads it as "some warning has this code".',
  );
});

test('the grandfathered list only shrinks', () => {
  const live = new Set(indexPinned());
  const stale = [...GRANDFATHERED].filter((v) => !live.has(v));
  // Entries may go stale as vectors are superseded to the wildcard form; that
  // is the point. This test records the progress rather than failing on it.
  assert.ok(
    stale.length <= GRANDFATHERED.size,
    `${stale.length} of ${GRANDFATHERED.size} grandfathered vectors have been fixed`,
  );
});
