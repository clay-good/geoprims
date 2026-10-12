// Main-thread client for the compute worker. Only the latest request per key
// resolves; older ones are dropped as stale (web "stale-result" behavior).
import { fileBytes, rangeSource } from '../../../../packages/runtime/src/files.mjs';
import { processNormalizedDifferenceScene } from '../../../../packages/runtime/src/raster.mjs';

let worker;
let seq = 0;
const waiting = new Map();
const latest = new Map();
const files = new Map();
const moduleFor = (tool) => (tool.startsWith('units.') ? 'base' : tool.split('.')[0]);
const fileKey = (tool, handle) => `${moduleFor(tool)}\0${handle}`;

function ensure() {
  if (!worker) {
    const current = new Worker(new URL('./compute.worker.js', import.meta.url), { type: 'module' });
    worker = current;
    for (const { tool, handle, bytes, source } of files.values()) {
      current.postMessage({
        seq: 0,
        method: source ? 'openFile' : 'putFile',
        args: source ? [tool, handle, source] : [tool, handle, bytes],
      });
    }
    current.onmessage = ({ data: { seq: s, out } }) => {
      if (worker !== current) return; // an interrupted worker may have queued a reply
      const w = waiting.get(s);
      waiting.delete(s);
      w?.resolve(out);
    };
  }
  return worker;
}

// Stop a synchronous Wasm call by ending its worker. Other pending calls are
// replayed on the replacement, so a map or search request is not lost.
export function cancel(key = 'invoke') {
  if (![...waiting.values()].some((w) => w.key === key)) return;
  worker?.terminate();
  worker = undefined;
  for (const [s, w] of waiting) {
    if (w.key === key) {
      waiting.delete(s);
      w.resolve(null);
    }
  }
  if (waiting.size) {
    const replacement = ensure();
    for (const [s, w] of waiting) replacement.postMessage({ seq: s, method: w.method, args: w.args });
  }
}

function call(method, args, key, onProgress) {
  if (method === 'invoke' && key) cancel(key);
  const s = ++seq;
  if (key) latest.set(key, s);
  return new Promise((resolve) => {
    const started = performance.now();
    const timer = onProgress && setInterval(() => onProgress(performance.now() - started), 250);
    waiting.set(s, {
      method, args, key,
      resolve: (out) => {
        if (timer !== undefined) clearInterval(timer);
        if (key && latest.get(key) !== s) return resolve(null); // superseded
        resolve(out === null ? null : JSON.parse(out));
      },
    });
    ensure().postMessage({ seq: s, method, args });
  });
}

// `key` groups requests: only the latest per key resolves (the map uses its own).
export const invoke = (id, args, key = 'invoke', onProgress) => call('invoke', [id, JSON.stringify(args)], key, onProgress);
/** Compiles this tool's module in the worker without running a calculation. */
export const warm = (id) => call('warm', [id]);
/** A quick call where only the newest answer matters (the map readout): an
 *  older one resolves null, and nothing is cancelled, so the worker stays warm. */
export const invokeLatest = (id, args, key) => call('invokeLatest', [id, JSON.stringify(args)], key);
export const encodeLink = (state, flags = []) => call('callString', ['link', 'gp_link_encode', JSON.stringify({ state, flags })], 'encode');
export const decodeLink = (fragment) => call('callString', ['link', 'gp_link_decode', fragment]);
export const search = (request) => call('search', [JSON.stringify(request)], 'search');
export const detect = (query) => call('detect', [query], 'detect');
/** Reads a pasted coordinate in any notation; only the latest read resolves. */
export const readCoordinate = (text) => call('readCoordinate', [text], 'coordinate');
/** Parses a file's text in the worker: GeoJSON, KML, GPX, WKT, CSV, or TSV. */
export const readFile = (name, text) => call('readFile', [name, text]);
/** Parses a file from its bytes, so a KMZ can be unzipped: any format above, and KMZ. */
export const readFileBytes = (name, bytes) => call('readFileBytes', [name, bytes]);
/** One chunk of a batch: the core's array of envelopes, in row order. No key,
 *  so a keystroke on the form never supersedes or cancels a running batch. */
export const invokeBatch = (id, inputsJson) => call('invokeBatch', [id, inputsJson]);
/** Keeps a user-selected file in memory under a private handle. */
export async function putFile(tool, handle, bytes) {
  bytes = fileBytes(bytes);
  const out = await call('putFile', [tool, handle, bytes]);
  if (out?.ok) files.set(fileKey(tool, handle), { module: moduleFor(tool), tool, handle, bytes });
  return out;
}
/** Keeps a user-selected large file available for bounded range reads. */
export async function openFile(tool, handle, source) {
  source = rangeSource(source);
  const out = await call('openFile', [tool, handle, source]);
  if (out?.ok) files.set(fileKey(tool, handle), { module: moduleFor(tool), tool, handle, source });
  return out;
}
export const inspectFile = (tool, handle) => call('inspectFile', [tool, handle]);
export const readFileWindow = (tool, handle, window, maxSamples) => (
  call('readFileWindow', [tool, handle, window, maxSamples])
);
export const processFileWindow = (tool, leftHandle, leftBand, rightHandle, rightBand, window, maxSamples) => (
  call('processFileWindow', [tool, leftHandle, leftBand, rightHandle, rightBand, window, maxSamples])
);
export const processFileScene = (options) => (
  processNormalizedDifferenceScene({ inspectFile, processFileWindow }, options)
);
export async function removeFile(tool, handle) {
  const out = await call('removeFile', [tool, handle]);
  files.delete(fileKey(tool, handle));
  return out;
}
export async function clearFiles(tool) {
  const out = await call('clearFiles', [tool]);
  const module = moduleFor(tool);
  for (const [key, file] of files) if (file.module === module) files.delete(key);
  return out;
}
