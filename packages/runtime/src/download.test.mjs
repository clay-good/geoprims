import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { createServer as createMcpServer } from '../../../mcp/server.mjs';
import { fileAssets, nodeHost } from './node.mjs';
import { workerHost } from './worker-host.mjs';

const root = new URL('../../..', import.meta.url).pathname;
const sourceRegistry = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8'));
const entry = sourceRegistry.assets.find((asset) => asset.id === 'egm96-15');
const key = 'egm96-15.pgm';
const grid = readFileSync(join(root, 'assets/data', entry.id, entry.version, key));
const input = '{"lat":16.776,"lon":-3.009}';

function fixture() {
  const dir = mkdtempSync(join(tmpdir(), 'geoprims-download-assets-'));
  const assetDir = join(dir, 'assets');
  const cacheDir = join(dir, 'cache');
  mkdirSync(assetDir);
  writeFileSync(join(assetDir, 'registry.json'), `${JSON.stringify({ assets: [entry] }, null, 2)}\n`);
  return { assetDir, cacheDir };
}

async function serve(body = grid) {
  let requests = 0;
  let closing;
  const server = createServer((request, response) => {
    requests++;
    assert.equal(request.url, `/assets/${entry.id}/${encodeURIComponent(entry.version)}/${key}`);
    response.writeHead(200, { 'content-length': body.length });
    response.end(body);
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const { port } = server.address();
  return {
    origin: `http://127.0.0.1:${port}`,
    requests: () => requests,
    close: () => (closing ??= server.listening ? new Promise((resolve) => server.close(resolve)) : Promise.resolve()),
  };
}

test('a missing Node asset names the dataset, file, size, and opt-in flag', async () => {
  const { assetDir, cacheDir } = fixture();
  const host = nodeHost(join(root, 'dist/wasm'), {
    assets: fileAssets(join(root, 'dist/wasm'), assetDir, { cacheDir }),
  });
  const result = JSON.parse(await host.invoke('geodesy.geoid.geoid-height', input));
  assert.equal(result.error.code, 'ASSET_UNAVAILABLE');
  assert.match(result.error.message, /EGM96 geoid, 15-minute grid/);
  assert.match(result.error.message, /egm96-15\.pgm/);
  assert.match(result.error.message, /2,076,888 bytes/);
  assert.match(result.error.hint, /--allow-asset-download/);
});

test('an allowed Node asset download is verified, cached, reused, and repaired', async (t) => {
  const { assetDir, cacheDir } = fixture();
  const first = await serve();
  t.after(() => first.close());
  const options = { allowAssetDownload: true, assetOrigin: first.origin, assetCacheDir: cacheDir, assetDir };
  let host = workerHost(join(root, 'dist/wasm'), {
    ...options,
  });
  t.after(() => host.close());
  let result = JSON.parse(await host.invoke('geodesy.geoid.geoid-height', input));
  assert.equal(result.ok, true, JSON.stringify(result));
  assert.ok(Math.abs(result.result.geoid_height.value - 28.7079) < 1e-4);
  assert.equal(first.requests(), 1);
  await host.close();
  await first.close();

  host = workerHost(join(root, 'dist/wasm'), {
    ...options,
    assetOrigin: 'http://127.0.0.1:1',
  });
  result = JSON.parse(await host.invoke('geodesy.geoid.geoid-height', input));
  assert.equal(result.ok, true, 'a fresh host must reuse the verified cache while offline');
  await host.close();

  const cached = join(cacheDir, entry.id, encodeURIComponent(entry.version), key);
  mkdirSync(dirname(cached), { recursive: true });
  writeFileSync(cached, Buffer.from('corrupt'));
  const repaired = await serve();
  t.after(() => repaired.close());
  host = workerHost(join(root, 'dist/wasm'), {
    ...options,
    assetOrigin: repaired.origin,
  });
  result = JSON.parse(await host.invoke('geodesy.geoid.geoid-height', input));
  assert.equal(result.ok, true, JSON.stringify(result));
  assert.equal(repaired.requests(), 1, 'a corrupt cached file must be replaced from the asset origin');
  assert.deepEqual(readFileSync(cached), grid);
  await host.close();
  await repaired.close();
});

test('the MCP asset-download option reaches its compute worker', async (t) => {
  const { assetDir, cacheDir } = fixture();
  const remote = await serve();
  t.after(() => remote.close());
  const server = await createMcpServer({
    allowAssetDownload: true,
    assetOrigin: remote.origin,
    assetCacheDir: cacheDir,
    assetDir,
  });
  t.after(() => server.close());
  const response = await server.handle({
    jsonrpc: '2.0',
    id: 1,
    method: 'tools/call',
    params: {
      name: 'geoprims_run',
      arguments: { id: 'geodesy.geoid.geoid-height', args: { lat: 16.776, lon: -3.009 } },
    },
  });
  assert.equal(response.result.structuredContent.ok, true, JSON.stringify(response));
  assert.equal(remote.requests(), 1);
});

test('an asset response larger than the registry size is rejected without caching', async (t) => {
  const { assetDir, cacheDir } = fixture();
  const remote = await serve(Buffer.alloc(grid.length + 1));
  t.after(() => remote.close());
  const provide = fileAssets(join(root, 'dist/wasm'), assetDir, {
    allowDownload: true,
    assetOrigin: remote.origin,
    cacheDir,
  });
  const result = await provide({ id: entry.id, version: entry.version, key });
  assert.equal(result.error.code, 'ASSET_INTEGRITY');
  assert.equal(remote.requests(), 1);
  const cached = join(cacheDir, entry.id, encodeURIComponent(entry.version), key);
  assert.throws(() => readFileSync(cached));
});
