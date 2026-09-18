#!/usr/bin/env node
// Builds dist/catalog/v1.json from the manifests the built Wasm modules report,
// so the catalog describes exactly what ships (design D5, tool-catalog "Catalog
// export"). Adds each tool's live vector count and the honest counts.
//
// Usage: node tools/codegen/catalog.mjs   (after tools/wasm/build.mjs)
import { mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { loadModule } from '../../packages/runtime/src/module.mjs';

const root = new URL('../..', import.meta.url).pathname;

export async function buildCatalog() {
  const { modules } = JSON.parse(readFileSync(join(root, 'dist/wasm/modules.json'), 'utf8'));
  const tools = [];
  let coreVersion;
  for (const { module } of modules) {
    const mod = await loadModule(readFileSync(join(root, 'dist/wasm', `${module}.wasm`)), module);
    coreVersion ??= mod.version().split('@')[1];
    for (const m of mod.manifest()) {
      const file = join(root, m.vectors);
      const vectorCount = existsSync(file)
        ? readFileSync(file, 'utf8').split('\n').filter((l) => l && !JSON.parse(l).supersededBy).length
        : 0;
      tools.push({ ...m, module, vectorCount });
    }
  }
  tools.sort((a, b) => (a.id < b.id ? -1 : 1));
  const count = (pred) => ({
    operations: tools.filter((t) => pred(t) && t.composedOf.length === 0).length,
    endpoints: tools.filter(pred).length,
  });
  return {
    catalog: 'v1',
    coreVersion,
    counts: {
      stable: count((t) => t.stability === 'stable'),
      experimental: count((t) => t.stability === 'experimental'),
      all: count(() => true),
    },
    tools,
  };
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const catalog = await buildCatalog();
  mkdirSync(join(root, 'dist/catalog'), { recursive: true });
  writeFileSync(join(root, 'dist/catalog/v1.json'), JSON.stringify(catalog) + '\n');
  console.log(JSON.stringify(catalog.counts));
}
