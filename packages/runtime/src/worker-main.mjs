// The worker side of worker-host.mjs.
import { parentPort, workerData } from 'node:worker_threads';
import { nodeHost } from './node.mjs';

const host = nodeHost(workerData.wasmDir, {
  maxBytes: workerData.maxBytes,
  assetDir: workerData.assetDir,
  allowAssetDownload: workerData.allowAssetDownload,
  assetOrigin: workerData.assetOrigin,
  assetCacheDir: workerData.assetCacheDir,
});

const methods = {
  invoke: (id, input) => host.invoke(id, input),
  invokeBatch: (id, inputs) => host.invokeBatch(id, inputs),
  putFile: (id, handle, bytes) => host.putFile(id, handle, bytes),
  openFilePath: (id, handle, path) => host.openFilePath(id, handle, path),
  inspectFile: (id, handle) => host.inspectFile(id, handle),
  readFileWindow: (id, handle, window, maxSamples) => host.readFileWindow(id, handle, window, maxSamples),
  processFileWindow: (id, leftHandle, leftBand, rightHandle, rightBand, window, maxSamples, resampling, parameters) => (
    host.processFileWindow(id, leftHandle, leftBand, rightHandle, rightBand, window, maxSamples, resampling, parameters)
  ),
  processFileSources: (id, sources, window, maxSamples, parameters) => (
    host.processFileSources(id, sources, window, maxSamples, parameters)
  ),
  exportFilePlan: (id, handle, tileWidth, tileHeight) => (
    host.exportFilePlan(id, handle, tileWidth, tileHeight)
  ),
  exportFileTile: (id, request) => host.exportFileTile(id, request),
  removeFile: (id, handle) => host.removeFile(id, handle),
  clearFiles: (id) => host.clearFiles(id),
  searchLoad: async (index) => (await host.module('search')).callString('gp_search_load', index),
  search: async (request) => (await host.module('search')).callString('gp_search', request),
  callExport: async (module, exportName, input) => (await host.module(module)).callString(exportName, input),
  // Test hook: a call that never returns, to exercise the timeout.
  spin: () => {
    for (;;);
  },
};

// Messages are handled strictly in order, even though methods are async.
let chain = Promise.resolve();
parentPort.on('message', ({ id, method, args }) => {
  chain = chain.then(async () => {
    const out = await methods[method](...args);
    if (id >= 0) parentPort.postMessage({ id, out });
  });
});
