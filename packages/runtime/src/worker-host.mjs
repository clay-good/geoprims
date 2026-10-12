// Node host that runs every call in a worker thread, so a runaway call can be
// stopped: on timeout the worker is terminated and replaced, the call returns
// LIMIT_EXCEEDED, and later calls keep working (MCP "Resource limits and
// robustness"). Calls run one at a time, in order.
import { Worker } from 'node:worker_threads';
import { fileBytes } from './files.mjs';

const workerUrl = new URL('./worker-main.mjs', import.meta.url);

const envelope = (code, message, hint) => JSON.stringify({ ok: false, error: { code, message, ...(hint ? { hint } : {}) } });

export function workerHost(wasmDir, { timeoutMs = 10_000, maxBytes } = {}) {
  let worker;
  let seq = 0;
  let pending = null;
  let closed = false;
  let closing;
  const queue = [];
  let searchIndex = null; // re-sent to a fresh worker after a restart
  const files = new Map(); // private file bytes re-sent after a worker restart

  const moduleFor = (tool) => (tool.startsWith('units.') ? 'base' : tool.split('.')[0]);
  const fileKey = (tool, handle) => `${moduleFor(tool)}\0${handle}`;
  const restoreState = () => {
    if (searchIndex) worker.postMessage({ id: -1, method: 'searchLoad', args: [searchIndex] });
    for (const file of files.values()) {
      worker.postMessage({
        id: -1,
        method: file.path ? 'openFilePath' : 'putFile',
        args: file.path ? [file.tool, file.handle, file.path] : [file.tool, file.handle, file.bytes],
      });
    }
  };

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
      if (closed || worker !== current) return;
      if (!pending || pending.id !== id) return;
      const job = pending;
      pending = null;
      settle(job, out);
      pump();
    });
    current.on('error', () => {
      if (!closed && worker === current) fail(envelope('INTERNAL', 'The compute worker failed. It has been restarted. Please report it.'));
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
    restoreState();
    pump();
  };
  const pump = () => {
    if (closed || pending || !queue.length) return;
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
      if (closed || signal?.aborted) return resolve(null);
      const job = { method, args, resolve, signal, onProgress };
      job.abort = () => {
        if (pending === job) {
          pending = null;
          settle(job, null);
          worker.terminate();
          start();
          restoreState();
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
    putFile: async (tool, handle, bytes) => {
      bytes = fileBytes(bytes);
      const out = await call('putFile', tool, handle, bytes);
      if (out && JSON.parse(out).ok) files.set(fileKey(tool, handle), { module: moduleFor(tool), tool, handle, bytes });
      return out;
    },
    openFilePath: async (tool, handle, path) => {
      const out = await call('openFilePath', tool, handle, path);
      if (out && JSON.parse(out).ok) files.set(fileKey(tool, handle), { module: moduleFor(tool), tool, handle, path });
      return out;
    },
    inspectFile: (tool, handle, options) => submit('inspectFile', [tool, handle], options),
    readFileWindow: (tool, handle, window, maxSamples, options) => (
      submit('readFileWindow', [tool, handle, window, maxSamples], options)
    ),
    removeFile: async (tool, handle) => {
      const out = await call('removeFile', tool, handle);
      files.delete(fileKey(tool, handle));
      return out;
    },
    clearFiles: async (tool) => {
      const out = await call('clearFiles', tool);
      const module = moduleFor(tool);
      for (const [key, file] of files) if (file.module === module) files.delete(key);
      return out;
    },
    searchLoad: (index) => {
      searchIndex = index;
      return call('searchLoad', index);
    },
    search: (request) => call('search', request),
    /** Calls a one-string export of any module, e.g. ('link', 'gp_link_encode', json). */
    callExport: (module, exportName, input) => call('callExport', module, exportName, input),
    close: () => {
      if (closed) return closing;
      closed = true;
      if (pending) {
        settle(pending, null);
        pending = null;
      }
      for (const job of queue.splice(0)) settle(job, null);
      files.clear();
      closing = worker.terminate();
      return closing;
    },
    /** For tests: call any worker method by name. */
    _call: call,
    _callWithOptions: submit,
  };
}
