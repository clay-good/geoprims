// Node host: loads modules from a directory of .wasm files (dist/wasm in a
// clone, mcp/dist/wasm in a release).
import { closeSync, existsSync, openSync, readFileSync, readSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { assetProvider } from './assets.mjs';
import { loadModule } from './module.mjs';

/**
 * The filesystem asset provider: `<dir>/registry.json` and files under
 * `<dir>/data/<id>/<version>/`. By default the directory beside the Wasm
 * (mcp/dist/assets in a release) or the repository's assets/ in a clone.
 */
export function fileAssets(wasmDir, dir) {
  const found = dir ?? [join(wasmDir, '..', 'assets'), join(wasmDir, '..', '..', 'assets')].find((d) => existsSync(join(d, 'registry.json')));
  if (!found) return undefined;
  const registry = JSON.parse(readFileSync(join(found, 'registry.json'), 'utf8'));
  return assetProvider(registry, async (id, version, key) => {
    const p = join(found, 'data', id, version, key);
    return existsSync(p) ? readFileSync(p) : null;
  });
}

const MODULE_FOR_DOMAIN = { units: 'base' };

export function moduleFor(toolId) {
  const domain = toolId.split('.')[0];
  return MODULE_FOR_DOMAIN[domain] ?? domain;
}

function pathRangeSource(path) {
  const size = statSync(path).size;
  return {
    size,
    slice(start, end) {
      const bytes = new Uint8Array(end - start);
      const descriptor = openSync(path, 'r');
      try {
        let read = 0;
        while (read < bytes.length) {
          const count = readSync(descriptor, bytes, read, bytes.length - read, start + read);
          if (count === 0) break;
          read += count;
        }
        return bytes.subarray(0, read);
      } finally {
        closeSync(descriptor);
      }
    },
  };
}

/** A lazy set of modules: each loads on first use. */
export function nodeHost(wasmDir, opts = {}) {
  const loaded = new Map();
  opts = { ...opts, assets: opts.assets === undefined ? fileAssets(wasmDir, opts.assetDir) : opts.assets };
  const get = (name) => {
    if (!loaded.has(name)) loaded.set(name, loadModule(readFileSync(join(wasmDir, `${name}.wasm`)), name, opts));
    return loaded.get(name);
  };
  const unsupported = (id) =>
    JSON.stringify({
      ok: false,
      error: { code: 'UNSUPPORTED', message: `There is no tool with id "${id}".`, hint: 'Search the catalog for the right id.' },
    });
  const route = async (id, method, ...args) => {
    let mod;
    try {
      mod = await get(moduleFor(id));
    } catch {
      loaded.delete(moduleFor(id));
      return unsupported(id);
    }
    return mod[method](...args);
  };
  const ok = '{"ok":true}';
  return {
    module: get,
    invoke: (id, input) => route(id, 'invoke', id, input),
    invokeBatch: (id, inputs) => route(id, 'invokeBatch', id, inputs),
    putFile: async (id, handle, bytes) => {
      (await get(moduleFor(id))).putFile(handle, bytes);
      return ok;
    },
    openFile: async (id, handle, source) => {
      (await get(moduleFor(id))).openFile(handle, source);
      return ok;
    },
    openFilePath: async (id, handle, path) => {
      (await get(moduleFor(id))).openFile(handle, pathRangeSource(path));
      return ok;
    },
    inspectFile: (id, handle) => route(id, 'inspectFile', handle),
    readFileWindow: (id, handle, window, maxSamples) => route(id, 'readFileWindow', handle, window, maxSamples),
    processFileWindow: (id, leftHandle, leftBand, rightHandle, rightBand, window, maxSamples, resampling, parameters) => (
      route(id, 'processFileWindow', id, leftHandle, leftBand, rightHandle, rightBand, window, maxSamples, resampling, parameters)
    ),
    processFileSources: (id, sources, window, maxSamples, parameters) => (
      route(id, 'processFileSources', id, sources, window, maxSamples, parameters)
    ),
    exportFilePlan: (id, handle, tileWidth, tileHeight) => (
      route(id, 'exportFilePlan', handle, tileWidth, tileHeight)
    ),
    exportFileTile: (id, request) => route(id, 'exportFileTile', request),
    removeFile: async (id, handle) => {
      (await get(moduleFor(id))).removeFile(handle);
      return ok;
    },
    clearFiles: async (id) => {
      (await get(moduleFor(id))).clearFiles();
      return ok;
    },
  };
}
