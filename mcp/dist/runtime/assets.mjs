// Asset providers (data-assets "Integrity verification", compute-core "Asset
// provided by host"). The core names a missing asset as {id, version, key};
// a provider finds it in the registry, reads the bytes, checks the SHA-256,
// and returns them, or an error envelope body. Works in browsers and Node:
// it needs only WebCrypto.

const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');

/**
 * `registry` is assets/registry.json; `read(id, version, key)` resolves to the
 * file's bytes (Uint8Array or ArrayBuffer) or null when it cannot be loaded.
 */
export function assetProvider(registry, read) {
  const entries = new Map(registry.assets.map((a) => [`${a.id}@${a.version}`, a]));
  return async ({ id, version, key }) => {
    const file = entries.get(`${id}@${version}`)?.files?.[key];
    if (!file) {
      return { error: { code: 'ASSET_UNAVAILABLE', message: `The ${id} data (${version}, ${key}) is not in the asset registry.` } };
    }
    let bytes = null;
    try {
      bytes = await read(id, version, key);
    } catch {
      bytes = null;
    }
    if (!bytes) {
      return {
        error: {
          code: 'ASSET_UNAVAILABLE',
          message: `The ${id} data (${version}, ${key}) could not be loaded.`,
          hint: 'Check the connection, or install the offline data, then try again.',
        },
      };
    }
    bytes = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    if (hex(await crypto.subtle.digest('SHA-256', bytes)) !== file.sha256) {
      return {
        error: {
          code: 'ASSET_INTEGRITY',
          message: `The ${id} data (${key}) failed its integrity check and was discarded. Try again to download a fresh copy.`,
        },
      };
    }
    return { bytes };
  };
}
