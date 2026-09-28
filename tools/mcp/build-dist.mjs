#!/usr/bin/env node
// Assembles mcp/dist/ from a local build, so the server runs from a clone or
// the npm package with Node alone: Wasm modules, catalog, runtime, and golden
// vectors. Release tags commit this directory (add-local-mcp-server 1.1a).
//
// Usage: node tools/mcp/build-dist.mjs   (after build:wasm and build:catalog)
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const dist = join(root, 'mcp/dist');
for (const need of ['dist/wasm/base.wasm', 'dist/catalog/v1.json']) {
  if (!existsSync(join(root, need))) throw new Error(`${need} is missing; run npm run build first`);
}
rmSync(dist, { recursive: true, force: true });
cpSync(join(root, 'dist/wasm'), join(dist, 'wasm'), { recursive: true });
cpSync(join(root, 'dist/catalog'), join(dist, 'catalog'), { recursive: true });
cpSync(join(root, 'packages/runtime/src'), join(dist, 'runtime'), { recursive: true, filter: (p) => !p.endsWith('.test.mjs') });
cpSync(join(root, 'core/vectors'), join(dist, 'vectors'), { recursive: true });
cpSync(join(root, 'assets/registry.json'), join(dist, 'assets/registry.json'));
cpSync(join(root, 'assets/data'), join(dist, 'assets/data'), { recursive: true });
// Bundled files outside assets/data (the Natural Earth base map) are copied
// into the package's uniform asset path. Files under core/ are already inside
// their Wasm module and must not be duplicated here.
const registry = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8'));
for (const asset of registry.assets.filter((a) => a.loadPolicy === 'bundled' && a.bundledIn && !a.bundledIn.startsWith('core/'))) {
  const files = Object.keys(asset.files);
  if (files.length !== 1) throw new Error(`${asset.id}: an external bundledIn entry must name exactly one file`);
  const target = join(dist, 'assets/data', asset.id, asset.version, files[0]);
  mkdirSync(dirname(target), { recursive: true });
  cpSync(join(root, asset.bundledIn), target);
}
cpSync(join(root, 'data/report-limits.json'), join(dist, 'data/report-limits.json'));
cpSync(join(root, 'data/workflows.json'), join(dist, 'data/workflows.json'));
console.log(`wrote ${dist}`);
