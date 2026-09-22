// Browser compute worker (compute-core "Execution off the main thread"). Loads
// the same Wasm modules as the MCP server through packages/runtime.
import { assetProvider } from '../../../../packages/runtime/src/assets.mjs';
import { loadModule } from '../../../../packages/runtime/src/module.mjs';
import { detectValues } from './detect.js';
import { prefillActions } from './prefill.js';
import { readCoordinate } from './coordinate.mjs';
import { readFile, readFileBytes } from './import.mjs';
import { NO_WASM } from './messages.js';

// Data assets come from the same origin, whole files only, checked against the
// registry's SHA-256 before use (data-assets "Integrity verification").
let registry = null;
const repaired = new Set();
const assetPath = ({ id, version, key }) => `/assets/${id}/${version}/${key}`;
async function discardCachedAsset(path) {
  if (!('caches' in self)) return;
  for (const name of await caches.keys()) {
    if (name.startsWith('gp-app-') || name.startsWith('gp-pack-')) {
      await (await caches.open(name)).delete(path);
    }
  }
  repaired.add(path);
}

const assets = async (want) => {
  registry ??= fetch('/assets/registry.json').then((r) => (r.ok ? r.json() : { assets: [] }));
  const get = assetProvider(await registry, async (id, version, key) => {
    const r = await fetch(assetPath({ id, version, key }), { cache: 'reload' });
    return r.ok ? r.arrayBuffer() : null;
  });
  const out = await get(want);
  const path = assetPath(want);
  if (out.error?.code === 'ASSET_INTEGRITY') {
    try {
      await discardCachedAsset(path);
    } catch {
      return { error: { ...out.error, hint: 'Clear offline data before retrying.' } };
    }
  } else if (out.bytes && repaired.delete(path)) {
    // The verified replacement remains available offline after the retry.
    try { await (await caches.open('gp-pack-verified-assets')).put(path, new Response(out.bytes)); } catch { /* The result is still verified. */ }
  }
  return out;
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

// The command palette ranks with the same core search as geoprims_search,
// indexed once from the catalog on first use.
let searchIndex = null;
let tools = new Map();
const searcher = () =>
  (searchIndex ??= Promise.all([get('search'), fetch('/catalog/v1.json').then((r) => r.json())]).then(async ([m, catalog]) => {
    await m.callString('gp_search_load', JSON.stringify(catalog.tools));
    tools = new Map(catalog.tools.map((t) => [t.id, t]));
    return m;
  }));

const encode = async (state) => JSON.parse(await (await get('link')).callString('gp_link_encode', JSON.stringify(state)));
// The core search, with "Open with these values" links for what the question filled in.
const searchWithPrefill = async (request) => {
  const out = JSON.parse(await (await searcher()).callString('gp_search', request));
  const top = out.ok ? out.result.results[0] : null;
  if (top) top.open = await prefillActions(top, tools.get(top.id), encode);
  return JSON.stringify(out);
};
const run = async (id, input) => JSON.parse(await (await get(moduleFor(id))).invoke(id, JSON.stringify(input)));
// The coordinate field: any notation the catalog can decode, read by the same
// core code the palette uses.
const readAnyCoordinate = async (text) => {
  const m = await searcher();
  const out = await readCoordinate(text, {
    candidates: async (q) => JSON.parse(await m.callString('gp_detect', JSON.stringify({ query: q }))).result.candidates,
    run,
  });
  return JSON.stringify(out);
};

const detect = async (query) => {
  const m = await searcher();
  const out = await detectValues(query, {
    candidates: async (q) => JSON.parse(await m.callString('gp_detect', JSON.stringify({ query: q }))).result.candidates,
    run,
    encode,
    tools,
  });
  return JSON.stringify(out);
};

// app-shell "Error resilience": say why nothing computes, and what does work.
const failure = (message, code = 'UNSUPPORTED') => JSON.stringify({ ok: false, error: { code, message } });

self.onmessage = async ({ data: { seq, method, args } }) => {
  if (typeof WebAssembly !== 'object') return self.postMessage({ seq, out: failure(NO_WASM) });
  try {
    let out;
    if (method === 'invoke') out = await (await get(moduleFor(args[0]))).invoke(args[0], args[1]);
    else if (method === 'search') out = await searchWithPrefill(args[0]);
    else if (method === 'detect') out = await detect(args[0]);
    else if (method === 'readCoordinate') out = await readAnyCoordinate(args[0]);
    // A file a reader brought is parsed here, off the page, and nothing it
    // points at is fetched (web/io-formats "Safe parsing").
    else if (method === 'readFile') out = JSON.stringify(readFile(args[0], args[1]));
    else if (method === 'readFileBytes') out = JSON.stringify(await readFileBytes(args[0], args[1]));
    // Batch mode: one chunk of rows through the core's own batch entry point.
    else if (method === 'invokeBatch') out = await (await get(moduleFor(args[0]))).invokeBatch(args[0], args[1]);
    else out = await (await get(args[0])).callString(args[1], args[2]);
    self.postMessage({ seq, out });
  } catch (e) {
    modules.clear();
    searchIndex = null;
    // A policy can leave WebAssembly defined but refuse to compile it.
    const blocked = e instanceof WebAssembly.CompileError || /WebAssembly|wasm-unsafe-eval/i.test(String(e?.message));
    self.postMessage({ seq, out: blocked ? failure(NO_WASM) : failure(`The calculator could not load: ${e.message}`, 'INTERNAL') });
  }
};
