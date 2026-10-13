// Node host: loads modules from a directory of .wasm files (dist/wasm in a
// clone, mcp/dist/wasm in a release).
import { randomUUID } from 'node:crypto';
import {
  closeSync, existsSync, mkdirSync, openSync, readFileSync, readSync, renameSync, rmSync, statSync, writeFileSync,
} from 'node:fs';
import { homedir, platform } from 'node:os';
import { dirname, join } from 'node:path';
import { assetProvider } from './assets.mjs';
import { loadModule } from './module.mjs';

const ASSET_ORIGIN = 'https://assets.geoprims.com';
const segment = (value) => {
  const encoded = encodeURIComponent(value);
  return encoded === '.' ? '%2E' : encoded === '..' ? '%2E%2E' : encoded;
};
const count = (value) => String(value).replace(/\B(?=(\d{3})+(?!\d))/g, ',');

function defaultCacheDir() {
  if (platform() === 'darwin') return join(homedir(), 'Library', 'Caches', 'geoprims', 'assets');
  if (platform() === 'win32') return join(process.env.LOCALAPPDATA || join(homedir(), 'AppData', 'Local'), 'geoprims', 'cache', 'assets');
  return join(process.env.XDG_CACHE_HOME || join(homedir(), '.cache'), 'geoprims', 'assets');
}

async function boundedResponse(response, expected) {
  if (!response.ok || !response.body) return null;
  const declared = Number(response.headers.get('content-length'));
  if (Number.isFinite(declared) && declared > expected) {
    await response.body.cancel();
    return new Uint8Array();
  }
  const chunks = [];
  let length = 0;
  const reader = response.body.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    length += value.length;
    if (length > expected) {
      await reader.cancel();
      return new Uint8Array();
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.length;
  }
  return bytes;
}

/**
 * The filesystem asset provider: `<dir>/registry.json` and files under
 * `<dir>/data/<id>/<version>/`. By default the directory beside the Wasm
 * (mcp/dist/assets in a release) or the repository's assets/ in a clone.
 */
export function fileAssets(wasmDir, dir, options = {}) {
  const found = dir ?? [join(wasmDir, '..', 'assets'), join(wasmDir, '..', '..', 'assets')].find((d) => existsSync(join(d, 'registry.json')));
  if (!found) return undefined;
  const registry = JSON.parse(readFileSync(join(found, 'registry.json'), 'utf8'));
  const entries = new Map(registry.assets.map((entry) => [`${entry.id}@${entry.version}`, entry]));
  const cacheDir = options.cacheDir ?? defaultCacheDir();
  const cachePath = ({ id, version, key }) => join(cacheDir, segment(id), segment(version), segment(key));
  const packagedPath = ({ id, version, key }) => join(found, 'data', id, version, key);
  const downloaded = new Set();
  const identity = ({ id, version, key }) => `${id}@${version}/${key}`;
  const read = async (id, version, key, file) => {
    const asset = { id, version, key };
    const packaged = packagedPath(asset);
    if (existsSync(packaged)) return readFileSync(packaged);
    const cached = cachePath(asset);
    if (existsSync(cached)) return readFileSync(cached);
    if (!options.allowDownload) return null;
    try {
      const path = `/assets/${segment(id)}/${segment(version)}/${segment(key)}`;
      const response = await fetch(new URL(path, options.assetOrigin ?? ASSET_ORIGIN), {
        headers: { accept: 'application/octet-stream' },
        redirect: 'error',
      });
      const bytes = await boundedResponse(response, file.bytes);
      if (bytes) downloaded.add(identity(asset));
      return bytes;
    } catch {
      return null;
    }
  };
  const persist = async (asset, bytes) => {
    if (!downloaded.delete(identity(asset))) return;
    const path = cachePath(asset);
    mkdirSync(dirname(path), { recursive: true });
    const temporary = `${path}.${process.pid}.${randomUUID()}.tmp`;
    writeFileSync(temporary, bytes);
    try {
      renameSync(temporary, path);
    } catch (error) {
      rmSync(temporary, { force: true });
      if (!existsSync(path)) throw error;
    }
  };
  const provide = assetProvider(registry, read, persist);
  return async (want) => {
    let out = await provide(want);
    const failed = out.error?.asset ?? want;
    const cached = cachePath(failed);
    if (out.error?.code === 'ASSET_INTEGRITY') downloaded.delete(identity(failed));
    if (out.error?.code === 'ASSET_INTEGRITY' && existsSync(cached)) {
      rmSync(cached, { force: true });
      if (options.allowDownload) out = await provide(want);
    }
    if (out.error?.code === 'ASSET_UNAVAILABLE') {
      const entry = entries.get(`${failed.id}@${failed.version}`);
      const bytes = out.error.bytes ?? entry?.files?.[failed.key]?.bytes;
      const size = bytes ? ` (${count(bytes)} bytes)` : '';
      out = {
        error: {
          ...out.error,
          message: `The ${entry?.title ?? failed.id} file ${failed.key}${size} is not cached.`,
          hint: options.allowDownload
            ? 'The verified download from the geoprims asset origin failed. Check the connection, then try again.'
            : 'Restart the server with --allow-asset-download to download and verify it.',
        },
      };
    }
    return out;
  };
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
  opts = {
    ...opts,
    assets: opts.assets === undefined ? fileAssets(wasmDir, opts.assetDir, {
      allowDownload: opts.allowAssetDownload,
      assetOrigin: opts.assetOrigin,
      cacheDir: opts.assetCacheDir,
    }) : opts.assets,
  };
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
