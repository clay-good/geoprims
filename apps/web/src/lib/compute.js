// Main-thread client for the compute worker. Only the latest request per key
// resolves; older ones are dropped as stale (web "stale-result" behavior).
let worker;
let seq = 0;
const waiting = new Map();
const latest = new Map();

function ensure() {
  if (!worker) {
    const current = new Worker(new URL('./compute.worker.js', import.meta.url), { type: 'module' });
    worker = current;
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
export const encodeLink = (state, flags = []) => call('callString', ['link', 'gp_link_encode', JSON.stringify({ state, flags })], 'encode');
export const decodeLink = (fragment) => call('callString', ['link', 'gp_link_decode', fragment]);
export const search = (request) => call('search', [JSON.stringify(request)], 'search');
export const detect = (query) => call('detect', [query], 'detect');
/** Reads a pasted coordinate in any notation; only the latest read resolves. */
export const readCoordinate = (text) => call('readCoordinate', [text], 'coordinate');
/** Parses a file's text in the worker: GeoJSON, KML, GPX, WKT, CSV, or TSV. */
export const readFile = (name, text) => call('readFile', [name, text]);
/** One chunk of a batch: the core's array of envelopes, in row order. No key,
 *  so a keystroke on the form never supersedes or cancels a running batch. */
export const invokeBatch = (id, inputsJson) => call('invokeBatch', [id, inputsJson]);
