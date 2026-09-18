// Browser compute worker (compute-core "Execution off the main thread"). Loads
// the same Wasm modules as the MCP server through packages/runtime.
import { loadModule } from '../../../../packages/runtime/src/module.mjs';

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
        .then((b) => loadModule(b, name, { maxBytes: 50_000_000 })),
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
