// Loads one geoprims Wasm module and wraps its raw ABI (design D1). The same
// code runs in browsers and Node: it needs only WebAssembly and TextEncoder.
import { checkJson } from './harden.mjs';

const enc = new TextEncoder();
const dec = new TextDecoder();

/**
 * Instantiates module bytes. `name` labels errors. A Wasm trap never escapes:
 * it becomes an INTERNAL error envelope, and the instance is recreated so
 * later calls keep working (compute-core "Trap is contained").
 */
export async function loadModule(bytes, name, { maxBytes } = {}) {
  const compiled = await WebAssembly.compile(bytes);
  let inst = await WebAssembly.instantiate(compiled, {});

  const text = (ptr) => dec.decode(new Uint8Array(inst.exports.memory.buffer, ptr, inst.exports.gp_out_len()));
  const put = (s) => {
    const b = enc.encode(s);
    const p = inst.exports.gp_alloc(b.length);
    new Uint8Array(inst.exports.memory.buffer, p, b.length).set(b);
    return [p, b.length];
  };
  const guarded = async (id, fn) => {
    try {
      return fn();
    } catch (e) {
      inst = await WebAssembly.instantiate(compiled, {});
      return JSON.stringify({
        ok: false,
        error: {
          code: 'INTERNAL',
          message: `The ${name} module stopped while running ${id} (${e instanceof WebAssembly.RuntimeError ? 'trap' : 'error'}). Please report it.`,
        },
      });
    }
  };
  const call = (exportName, id, input) =>
    guarded(id, () => {
      const [ip, il] = put(id);
      const [xp, xl] = put(input);
      try {
        return text(inst.exports[exportName](ip, il, xp, xl));
      } finally {
        inst.exports.gp_free(ip, il);
        inst.exports.gp_free(xp, xl);
      }
    });

  return {
    name,
    version: () => text(inst.exports.gp_version()),
    manifest: () => JSON.parse(text(inst.exports.gp_manifest())),
    /** Returns the result envelope as a JSON string (the exact core bytes). */
    async invoke(id, inputJson) {
      const bad = checkJson(inputJson, maxBytes);
      if (bad) return JSON.stringify(bad);
      return call('gp_invoke', id, inputJson);
    },
    async invokeBatch(id, inputsJson) {
      const bad = checkJson(inputsJson, maxBytes);
      if (bad) return JSON.stringify(bad);
      return call('gp_invoke_batch', id, inputsJson);
    },
  };
}
