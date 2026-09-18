// Node host that runs every call in a worker thread, so a runaway call can be
// stopped: on timeout the worker is terminated and replaced, the call returns
// LIMIT_EXCEEDED, and later calls keep working (MCP "Resource limits and
// robustness"). Calls run one at a time, in order.
import { Worker } from 'node:worker_threads';

const workerUrl = new URL('./worker-main.mjs', import.meta.url);

const envelope = (code, message, hint) => JSON.stringify({ ok: false, error: { code, message, ...(hint ? { hint } : {}) } });

export function workerHost(wasmDir, { timeoutMs = 10_000, maxBytes } = {}) {
  let worker;
  let seq = 0;
  let pending = null; // { id, resolve, timer }
  const queue = [];
  let searchIndex = null; // re-sent to a fresh worker after a restart

  const start = () => {
    worker = new Worker(workerUrl, { workerData: { wasmDir, maxBytes } });
    worker.unref();
    worker.on('message', ({ id, out }) => {
      if (!pending || pending.id !== id) return;
      clearTimeout(pending.timer);
      const { resolve } = pending;
      pending = null;
      resolve(out);
      pump();
    });
    worker.on('error', () => fail(envelope('INTERNAL', 'The compute worker failed. It has been restarted. Please report it.')));
  };
  const fail = (out) => {
    if (pending) {
      clearTimeout(pending.timer);
      const { resolve } = pending;
      pending = null;
      resolve(out);
    }
    worker.terminate();
    start();
    if (searchIndex) worker.postMessage({ id: -1, method: 'searchLoad', args: [searchIndex] });
    pump();
  };
  const pump = () => {
    if (pending || !queue.length) return;
    const { method, args, resolve } = queue.shift();
    const id = ++seq;
    const timer = setTimeout(
      () => fail(envelope('LIMIT_EXCEEDED', `The call took longer than the ${timeoutMs} ms timeout and was stopped.`, 'Try a smaller input, or raise --timeout.')),
      timeoutMs,
    );
    pending = { id, resolve, timer };
    worker.postMessage({ id, method, args });
  };
  const call = (method, ...args) =>
    new Promise((resolve) => {
      queue.push({ method, args, resolve });
      pump();
    });

  start();
  return {
    invoke: (id, input) => call('invoke', id, input),
    invokeBatch: (id, inputs) => call('invokeBatch', id, inputs),
    searchLoad: (index) => {
      searchIndex = index;
      return call('searchLoad', index);
    },
    search: (request) => call('search', request),
    close: () => worker.terminate(),
    /** For tests: call any worker method by name. */
    _call: call,
  };
}
