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
  let pending = null;
  const queue = [];
  let searchIndex = null; // re-sent to a fresh worker after a restart

  const settle = (job, out) => {
    clearTimeout(job.timer);
    clearInterval(job.progress);
    job.signal?.removeEventListener('abort', job.abort);
    job.resolve(out);
  };
  const start = () => {
    const current = new Worker(workerUrl, { workerData: { wasmDir, maxBytes } });
    worker = current;
    current.unref();
    current.on('message', ({ id, out }) => {
      if (worker !== current) return;
      if (!pending || pending.id !== id) return;
      const job = pending;
      pending = null;
      settle(job, out);
      pump();
    });
    current.on('error', () => {
      if (worker === current) fail(envelope('INTERNAL', 'The compute worker failed. It has been restarted. Please report it.'));
    });
  };
  const fail = (out) => {
    if (pending) {
      const job = pending;
      pending = null;
      settle(job, out);
    }
    worker.terminate();
    start();
    if (searchIndex) worker.postMessage({ id: -1, method: 'searchLoad', args: [searchIndex] });
    pump();
  };
  const pump = () => {
    if (pending || !queue.length) return;
    const job = queue.shift();
    if (job.signal?.aborted) {
      settle(job, null);
      return pump();
    }
    job.id = ++seq;
    job.timer = setTimeout(
      () => fail(envelope('LIMIT_EXCEEDED', `The call took longer than the ${timeoutMs} ms timeout and was stopped.`, 'Try a smaller input, or raise --timeout.')),
      timeoutMs,
    );
    const started = performance.now();
    if (job.onProgress) job.progress = setInterval(() => job.onProgress(performance.now() - started), 250);
    pending = job;
    worker.postMessage({ id: job.id, method: job.method, args: job.args });
  };
  const submit = (method, args, { signal, onProgress } = {}) =>
    new Promise((resolve) => {
      if (signal?.aborted) return resolve(null);
      const job = { method, args, resolve, signal, onProgress };
      job.abort = () => {
        if (pending === job) {
          pending = null;
          settle(job, null);
          worker.terminate();
          start();
          if (searchIndex) worker.postMessage({ id: -1, method: 'searchLoad', args: [searchIndex] });
          pump();
        } else {
          const index = queue.indexOf(job);
          if (index >= 0) {
            queue.splice(index, 1);
            settle(job, null);
          }
        }
      };
      signal?.addEventListener('abort', job.abort, { once: true });
      queue.push(job);
      pump();
    });
  const call = (method, ...args) => submit(method, args);

  start();
  return {
    invoke: (id, input, options) => submit('invoke', [id, input], options),
    invokeBatch: (id, inputs, options) => submit('invokeBatch', [id, inputs], options),
    searchLoad: (index) => {
      searchIndex = index;
      return call('searchLoad', index);
    },
    search: (request) => call('search', request),
    /** Calls a one-string export of any module, e.g. ('link', 'gp_link_encode', json). */
    callExport: (module, exportName, input) => call('callExport', module, exportName, input),
    close: () => worker.terminate(),
    /** For tests: call any worker method by name. */
    _call: call,
    _callWithOptions: submit,
  };
}
