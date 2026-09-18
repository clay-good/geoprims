// Browser compute worker (compute-core "Execution off the main thread"). Loads
// the same Wasm modules as the MCP server through packages/runtime.
import { assetProvider } from '../../../../packages/runtime/src/assets.mjs';
import { loadModule } from '../../../../packages/runtime/src/module.mjs';

// Data assets come from the same origin, whole files only, checked against the
// registry's SHA-256 before use (data-assets "Integrity verification").
let registry = null;
const assets = async (want) => {
  registry ??= fetch('/assets/registry.json').then((r) => (r.ok ? r.json() : { assets: [] }));
  const get = assetProvider(await registry, async (id, version, key) => {
    const r = await fetch(`/assets/${id}/${version}/${key}`);
    return r.ok ? r.arrayBuffer() : null;
  });
  return get(want);
};

const modules = new Map();
const moduleFor = (id) => (id.startsWith('units.') ? 'base' : id.split('.')[0]);
const get = (name) => {
  if (!modules.has(name)) {
    modules.set(
      name,
      fetch(`/wasm/${name}.wasm`)
        .then((r) => {
          if (!r.ok) throw new Error(`${name}.wasm: HTTP ${r.status}`);
          return r.arrayBuffer();
        })
        .then((b) => loadModule(b, name, { maxBytes: 50_000_000, assets })),
    );
  }
  return modules.get(name);
};

self.onmessage = async ({ data: { seq, method, args } }) => {
  try {
    let out;
    if (method === 'invoke') out = await (await get(moduleFor(args[0]))).invoke(args[0], args[1]);
    else out = await (await get(args[0])).callString(args[1], args[2]);
    self.postMessage({ seq, out });
  } catch (e) {
    modules.clear();
    self.postMessage({ seq, out: JSON.stringify({ ok: false, error: { code: 'INTERNAL', message: `The calculator could not load: ${e.message}` } }) });
  }
};
