// Asset providers (data-assets "Integrity verification", compute-core "Asset
// provided by host"). The core names a missing asset as {id, version, key};
// a provider finds it in the registry, reads the bytes, checks the SHA-256,
// and returns them, or an error envelope body. Works in browsers and Node:
// it needs only WebCrypto.

const hex = (buf) => [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
const enc = new TextEncoder();

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`;
  }
  const encoded = JSON.stringify(value);
  if (encoded === undefined) throw new Error('unsupported value in signed index');
  return encoded;
}

const decodeBase64 = (value) => Uint8Array.from(atob(value), (c) => c.charCodeAt(0));

async function verifySignedIndex(bytes, entry) {
  const signed = JSON.parse(new TextDecoder().decode(bytes));
  if (signed.algorithm !== 'Ed25519'
    || signed.keyId !== entry.tiling.keyId
    || signed.publicKey !== entry.tiling.publicKey
    || signed.index?.assetId !== entry.id
    || signed.index?.version !== entry.version
    || !Array.isArray(signed.index?.tiles)) return null;
  const key = await crypto.subtle.importKey('spki', decodeBase64(signed.publicKey), { name: 'Ed25519' }, false, ['verify']);
  const valid = await crypto.subtle.verify(
    { name: 'Ed25519' },
    key,
    decodeBase64(signed.signature),
    enc.encode(canonicalJson(signed.index)),
  );
  if (!valid) return null;
  const tiles = new Map();
  for (const tile of signed.index.tiles) {
    if (!tile || typeof tile.file !== 'string' || !tile.file || tile.file.includes('/')
      || !Number.isSafeInteger(tile.bytes) || tile.bytes <= 0
      || !/^[a-f0-9]{64}$/.test(tile.sha256) || tiles.has(tile.file)) return null;
    tiles.set(tile.file, tile);
  }
  return tiles;
}

/**
 * `registry` is assets/registry.json; `read(id, version, key)` resolves to the
 * file's bytes (Uint8Array or ArrayBuffer) or null when it cannot be loaded.
 * `verified(asset, bytes)` may persist a replacement after every applicable
 * digest, signature, and pinned-key check succeeds.
 */
export function assetProvider(registry, read, verified = async () => {}) {
  const entries = new Map(registry.assets.map((a) => [`${a.id}@${a.version}`, a]));
  const indexes = new Map();
  const unavailable = (id, version, key) => ({
    error: {
      code: 'ASSET_UNAVAILABLE',
      message: `The ${id} data (${version}, ${key}) could not be loaded.`,
      hint: 'Check the connection, or install the offline data, then try again.',
      asset: { id, version, key },
    },
  });
  const integrity = (id, version, key) => ({
    error: {
      code: 'ASSET_INTEGRITY',
      message: `The ${id} data (${key}) failed its integrity check and was discarded. Try again to download a fresh copy.`,
      asset: { id, version, key },
    },
  });
  const load = async (entry, key, file) => {
    let bytes = null;
    try {
      bytes = await read(entry.id, entry.version, key);
    } catch {
      bytes = null;
    }
    if (!bytes) return unavailable(entry.id, entry.version, key);
    bytes = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
    if (bytes.length !== file.bytes || hex(await crypto.subtle.digest('SHA-256', bytes)) !== file.sha256) {
      return integrity(entry.id, entry.version, key);
    }
    return { bytes };
  };
  const tileIndex = async (entry) => {
    const identity = `${entry.id}@${entry.version}`;
    if (!indexes.has(identity)) {
      indexes.set(identity, (async () => {
        const key = entry.tiling.index;
        const file = entry.files?.[key];
        if (!file) return integrity(entry.id, entry.version, key);
        const loaded = await load(entry, key, file);
        if (loaded.error) return loaded;
        try {
          const tiles = await verifySignedIndex(loaded.bytes, entry);
          if (!tiles) return integrity(entry.id, entry.version, key);
          try { await verified({ id: entry.id, version: entry.version, key }, loaded.bytes); } catch { /* Verification still succeeded. */ }
          return { tiles };
        } catch {
          return integrity(entry.id, entry.version, key);
        }
      })());
    }
    const result = await indexes.get(identity);
    if (result.error) indexes.delete(identity);
    return result;
  };
  return async ({ id, version, key }) => {
    const entry = entries.get(`${id}@${version}`);
    let file = entry?.files?.[key];
    if (!file && entry?.tiling && typeof entry.tiling === 'object') {
      const index = await tileIndex(entry);
      if (index.error) return index;
      file = index.tiles.get(key);
    }
    if (!file) {
      return { error: { code: 'ASSET_UNAVAILABLE', message: `The ${id} data (${version}, ${key}) is not in the asset registry.` } };
    }
    const loaded = await load(entry, key, file);
    if (!loaded.error) {
      try { await verified({ id, version, key }, loaded.bytes); } catch { /* Verification still succeeded. */ }
    }
    return loaded;
  };
}
