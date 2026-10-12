import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { publicInputs } from '../src/lib/private.mjs';

test('private manifest inputs are excluded from persisted state', () => {
  const inputs = { raster: 'opaque-file-handle', nir: 4, options: { precision: 3 } };
  const tool = {
    inputs: {
      properties: {
        raster: { type: 'string', 'x-private': true },
        nir: { type: 'number' },
        options: { type: 'object' },
      },
    },
  };

  assert.deepEqual(publicInputs(tool, inputs), { nir: 4, options: { precision: 3 } });
  assert.deepEqual(inputs, { raster: 'opaque-file-handle', nir: 4, options: { precision: 3 } });
});

test('unknown migrated inputs remain shareable unless marked private', () => {
  const tool = { inputs: { properties: {} } };
  assert.deepEqual(publicInputs(tool, { legacy: 12 }), { legacy: 12 });
});

test('tool and workflow permalinks filter private inputs', async () => {
  for (const component of ['ToolApp.svelte', 'WorkflowApp.svelte']) {
    const source = await readFile(new URL(`../src/components/${component}`, import.meta.url), 'utf8');
    assert.match(source, /encodeLink\(\{ i: publicInputs\(/, component);
  }
});
