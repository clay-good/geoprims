// Node host: loads modules from a directory of .wasm files (dist/wasm in a
// clone, mcp/dist/wasm in a release).
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { loadModule } from './module.mjs';

const MODULE_FOR_DOMAIN = { units: 'base' };

export function moduleFor(toolId) {
  const domain = toolId.split('.')[0];
  return MODULE_FOR_DOMAIN[domain] ?? domain;
}

/** A lazy set of modules: each loads on first use. */
export function nodeHost(wasmDir, opts = {}) {
  const loaded = new Map();
  const get = (name) => {
    if (!loaded.has(name)) loaded.set(name, loadModule(readFileSync(join(wasmDir, `${name}.wasm`)), name, opts));
    return loaded.get(name);
  };
  const unsupported = (id) =>
    JSON.stringify({
      ok: false,
      error: { code: 'UNSUPPORTED', message: `There is no tool with id "${id}".`, hint: 'Search the catalog for the right id.' },
    });
  const route = async (id, method, input) => {
    let mod;
    try {
      mod = await get(moduleFor(id));
    } catch {
      loaded.delete(moduleFor(id));
      return unsupported(id);
    }
    return mod[method](id, input);
  };
  return {
    module: get,
    invoke: (id, input) => route(id, 'invoke', input),
    invokeBatch: (id, inputs) => route(id, 'invokeBatch', inputs),
  };
}
