// Wasm module checks shared by the build and its tests: the import-section lint
// (numeric-determinism: no host Math) and compressed size budgets (compute-core).
import { brotliCompressSync, constants } from 'node:zlib';

/** Imports a geoprims module may declare. Empty: the core does no host I/O. */
export const IMPORT_ALLOW_LIST = [];

/** Returns [{module, name}] for every entry in the module's import section. */
export function listImports(bytes) {
  const mod = new WebAssembly.Module(bytes);
  return WebAssembly.Module.imports(mod).map(({ module, name }) => ({ module, name }));
}

/** Throws if any import is outside the allow-list, naming each offender. */
export function lintImports(bytes, label) {
  const bad = listImports(bytes).filter(
    (i) => !IMPORT_ALLOW_LIST.some((a) => a.module === i.module && a.name === i.name),
  );
  if (bad.length) {
    const list = bad.map((i) => `${i.module}.${i.name}`).join(', ');
    throw new Error(`${label}: forbidden Wasm imports (${list}); the core must not call the host`);
  }
}

export function brotliSize(bytes) {
  return brotliCompressSync(bytes, {
    params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_SIZE_HINT]: bytes.length },
  }).length;
}

/** Throws if the Brotli size exceeds the budget, naming module, size, and budget. */
export function checkBudget(bytes, label, budget) {
  const size = brotliSize(bytes);
  if (size > budget) {
    throw new Error(`${label}: ${size} bytes Brotli exceeds its budget of ${budget} bytes`);
  }
  return size;
}
