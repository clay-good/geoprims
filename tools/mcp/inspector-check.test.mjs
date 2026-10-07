// The Inspector surface check's comparison (agent/mcp-server "Surface drift"),
// without the network: key order is not drift, and a changed input schema
// shows as the lines that changed. CI runs the check itself through the
// pinned MCP Inspector CLI.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { canonical, lineDiff } from './inspector-check.mjs';

const root = new URL('../..', import.meta.url).pathname;
const surface = JSON.parse(readFileSync(join(root, 'mcp/surface.json'), 'utf8'));

test('reordered keys are the same surface', () => {
  const reordered = surface.tools.map((t) => Object.fromEntries(Object.entries(t).reverse()));
  assert.equal(lineDiff(surface.tools, reordered), '');
  assert.deepEqual(Object.keys(canonical({ b: 1, a: { d: 1, c: 2 } })), ['a', 'b']);
});

test('a changed input schema shows as just the lines that changed', () => {
  const drifted = structuredClone(surface.tools);
  drifted.find((t) => t.name === 'geoprims_run').inputSchema.properties.units.enum.push('nautical');
  const diff = lineDiff(surface.tools, drifted);
  const changes = diff.split('\n').filter((l) => /^[+-] /.test(l));
  assert.deepEqual(changes.map((l) => l.replace(/\s+/g, ' ')), ['- "survey-us"', '+ "survey-us",', '+ "nautical"']);
  assert.match(diff, /^@@ line \d+/);
});
