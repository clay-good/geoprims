import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { nodeHost } from '../../packages/runtime/src/node.mjs';
import { directTools, TOOLSETS } from '../../mcp/toolsets.mjs';
import { artifactsForTool, buildArtifacts } from './artifacts.mjs';

const here = new URL('.', import.meta.url).pathname;
const sample = JSON.parse(readFileSync(join(here, '../../core/crates/gp-base/tests/fixtures/sample-manifest.json'), 'utf8'));

test('the sample manifest produces pinned TypeScript, MCP, and search artifacts', () => {
  const actual = artifactsForTool(sample);
  assert.equal(actual.type + '\n' + actual.outputType + '\n', readFileSync(join(here, 'fixtures/sample-types.txt'), 'utf8'));
  assert.deepEqual(actual.mcp, JSON.parse(readFileSync(join(here, 'fixtures/sample-mcp.json'), 'utf8')));
  assert.deepEqual(actual.search, JSON.parse(readFileSync(join(here, 'fixtures/sample-search.json'), 'utf8')));
});

test('the build has one generated entry per shipped manifest, with no schema drift', () => {
  const catalog = JSON.parse(readFileSync(join(here, '../../dist/catalog/v1.json'), 'utf8'));
  const generated = buildArtifacts(catalog);
  assert.equal(generated.types, readFileSync(join(here, '../../dist/catalog/types.d.ts'), 'utf8'));
  assert.deepEqual(generated.mcp, JSON.parse(readFileSync(join(here, '../../dist/catalog/mcp-tools.json'), 'utf8')));
  assert.deepEqual(generated.search, JSON.parse(readFileSync(join(here, '../../dist/catalog/search.json'), 'utf8')));
  assert.equal(generated.search.length, catalog.tools.length);
  for (const [i, tool] of catalog.tools.entries()) {
    assert.deepEqual(generated.mcp[i].inputSchema, tool.inputs);
    assert.equal(generated.search[i].id, tool.id);
  }
  const byId = new Map(generated.mcp.map((tool) => [tool.id, tool]));
  for (const name of Object.keys(TOOLSETS)) {
    for (const direct of directTools(catalog, [name], {})) {
      const { id, annotations, ...runtime } = direct;
      assert.deepEqual(runtime, Object.fromEntries(Object.entries(byId.get(id)).filter(([key]) => key !== 'id')));
      assert.equal(annotations.title, runtime.title);
    }
  }
});

test('the generated search documents give the same ranking and prefill as the full catalog', async () => {
  const root = join(here, '../..');
  const m = await nodeHost(join(root, 'dist/wasm')).module('search');
  const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const docs = JSON.parse(readFileSync(join(root, 'dist/catalog/search.json'), 'utf8'));
  const queries = ['densty alt', 'crosswind 270 at 15 runway 27', '5000 ft 30C density altitude'];
  const run = async (rows) => {
    await m.callString('gp_search_load', JSON.stringify(rows));
    return Promise.all(queries.map((query) => m.callString('gp_search', JSON.stringify({ query, limit: 8, includeExperimental: true }))));
  };
  assert.deepEqual(await run(docs), await run(catalog.tools));
});
