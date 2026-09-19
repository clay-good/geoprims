// Main-thread client for the compute worker. Only the latest request per key
// resolves; older ones are dropped as stale (web "stale-result" behavior).
let worker;
let seq = 0;
const waiting = new Map();
const latest = new Map();

function ensure() {
  if (!worker) {
    worker = new Worker(new URL('./compute.worker.js', import.meta.url), { type: 'module' });
    worker.onmessage = ({ data: { seq: s, out } }) => {
      const w = waiting.get(s);
      waiting.delete(s);
      w?.(out);
    };
  }
  return worker;
}

function call(method, args, key) {
  const s = ++seq;
  if (key) latest.set(key, s);
  return new Promise((resolve) => {
    waiting.set(s, (out) => {
      if (key && latest.get(key) !== s) return resolve(null); // superseded
      resolve(JSON.parse(out));
    });
    ensure().postMessage({ seq: s, method, args });
  });
}

export const invoke = (id, args) => call('invoke', [id, JSON.stringify(args)], 'invoke');
export const encodeLink = (state, flags = []) => call('callString', ['link', 'gp_link_encode', JSON.stringify({ state, flags })], 'encode');
export const decodeLink = (fragment) => call('callString', ['link', 'gp_link_decode', fragment]);
export const search = (request) => call('search', [JSON.stringify(request)], 'search');
export const detect = (query) => call('detect', [query], 'detect');
