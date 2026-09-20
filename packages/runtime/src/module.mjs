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
export async function loadModule(bytes, name, { maxBytes, assets, onInvoke } = {}) {
  const compiled = await WebAssembly.compile(bytes);
  let inst = await WebAssembly.instantiate(compiled, {});

  const text = (ptr) => dec.decode(new Uint8Array(inst.exports.memory.buffer, ptr, inst.exports.gp_out_len()));
  const put = (s) => {
    const b = enc.encode(s);
    const p = inst.exports.gp_alloc(b.length);
    new Uint8Array(inst.exports.memory.buffer, p, b.length).set(b);
    return [p, b.length];
  };
  // Assets supplied so far, re-supplied if a trap forces a fresh instance.
  const supplied = new Map();
  const supply = (key, data) => {
    const [kp, kl] = put(key);
    const dp = inst.exports.gp_alloc(data.length);
    new Uint8Array(inst.exports.memory.buffer, dp, data.length).set(data);
    try {
      inst.exports.gp_asset_put(kp, kl, dp, data.length);
    } finally {
      inst.exports.gp_free(kp, kl);
      inst.exports.gp_free(dp, data.length);
    }
  };
  const guarded = async (id, fn) => {
    try {
      return fn();
    } catch (e) {
      inst = await WebAssembly.instantiate(compiled, {});
      for (const [k, d] of supplied) supply(k, d);
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
      // The optional benchmark observer times the synchronous ABI call. It
      // excludes worker messages, async scheduling, and asset downloads.
      const started = onInvoke && exportName === 'gp_invoke' ? performance.now() : null;
      const [ip, il] = put(id);
      const [xp, xl] = put(input);
      try {
        return text(inst.exports[exportName](ip, il, xp, xl));
      } finally {
        inst.exports.gp_free(ip, il);
        inst.exports.gp_free(xp, xl);
        if (started !== null) onInvoke(id, performance.now() - started);
      }
    });

  /** Calls an export that takes one string and returns one string (e.g. gp_search). */
  const callString = (exportName, input) =>
    guarded(exportName, () => {
      const [p, l] = put(input);
      try {
        return text(inst.exports[exportName](p, l));
      } finally {
        inst.exports.gp_free(p, l);
      }
    });

  /**
   * Runs a call; when the core reports ASSET_UNAVAILABLE for a named asset and
   * a provider is configured, fetches and verifies it, supplies it, and retries.
   */
  const withAssets = async (run) => {
    for (let attempt = 0; attempt < 8; attempt++) {
      const out = await run();
      if (!assets || !out.includes('"ASSET_UNAVAILABLE"')) return out;
      const parsed = JSON.parse(out);
      const missing = (Array.isArray(parsed) ? parsed.map((r) => r.error) : [parsed.error]).find(
        (e) => e?.code === 'ASSET_UNAVAILABLE' && e.asset,
      );
      if (!missing) return out;
      const key = `${missing.asset.id}@${missing.asset.version}/${missing.asset.key}`;
      if (supplied.has(key)) return out;
      const got = await assets(missing.asset);
      if (got.error) return Array.isArray(parsed) ? out : JSON.stringify({ ok: false, error: { ...got.error, asset: missing.asset } });
      supply(key, got.bytes);
      supplied.set(key, got.bytes);
    }
    return run();
  };

  return {
    name,
    callString,
    version: () => text(inst.exports.gp_version()),
    manifest: () => JSON.parse(text(inst.exports.gp_manifest())),
    /** Returns the result envelope as a JSON string (the exact core bytes). */
    async invoke(id, inputJson) {
      const bad = checkJson(inputJson, maxBytes);
      if (bad) return JSON.stringify(bad);
      return withAssets(() => call('gp_invoke', id, inputJson));
    },
    async invokeBatch(id, inputsJson) {
      const bad = checkJson(inputsJson, maxBytes);
      if (bad) return JSON.stringify(bad);
      return withAssets(() => call('gp_invoke_batch', id, inputsJson));
    },
  };
}
