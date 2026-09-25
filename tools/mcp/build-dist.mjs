#!/usr/bin/env node
// Assembles mcp/dist/ from a local build, so the server runs from a clone or
// the npm package with Node alone: Wasm modules, catalog, runtime, and golden
// vectors. Release tags commit this directory (add-local-mcp-server 1.1a).
//
// Usage: node tools/mcp/build-dist.mjs   (after build:wasm and build:catalog)
import { cpSync, existsSync, rmSync } from 'node:fs';
import { join } from 'node:path';

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
cpSync(join(root, 'data/report-limits.json'), join(dist, 'data/report-limits.json'));
cpSync(join(root, 'data/workflows.json'), join(dist, 'data/workflows.json'));
console.log(`wrote ${dist}`);
