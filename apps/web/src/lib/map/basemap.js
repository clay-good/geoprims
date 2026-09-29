// Natural Earth base-map loading (web/map-canvas). Both resolutions pass
// through the shared asset provider, so bytes are checked against the asset
// registry before JSON parsing. The 50m map is one whole-file request.
import { assetProvider } from '../../../../../packages/runtime/src/assets.mjs';
import { decode } from './projection.js';

export const DETAIL_AT = 60;
export const GENERALIZED_AT = 180;

const request = {
  '110m': { id: 'ne-110m', version: '5.1.2', key: 'ne-110m.json' },
  '50m': { id: 'ne-50m', version: '5.1.2', key: 'ne-50m.json' },
};
const pathOf = ({ id, version, key }) => id === 'ne-110m'
  ? '/basemap/ne-110m.json'
  : `/assets/${id}/${version}/${key}`;

export function unpack(ne) {
  return {
    land: ne.land.map(decode),
    lakes: ne.lakes.map(decode),
    borders: ne.borders.map(decode),
    states: (ne.states ?? []).map(decode),
    places: (ne.places ?? []).map(([name, lon, lat, minZoom]) => ({ name, lon: lon / 100, lat: lat / 100, minZoom })),
  };
}

export function createBasemapLoader(fetcher = fetch) {
  let registry;
  let provider;
  return async (scale) => {
    const want = request[scale];
    if (!want) throw new Error(`unknown Natural Earth scale: ${scale}`);
    registry ??= fetcher('/assets/registry.json').then((response) => {
      if (!response.ok) throw new Error(`asset registry: HTTP ${response.status}`);
      return response.json();
    });
    provider ??= assetProvider(await registry, async (id, version, key) => {
      const response = await fetcher(pathOf({ id, version, key }));
      return response.ok ? response.arrayBuffer() : null;
    });
    const loaded = await provider(want);
    if (loaded.error) throw new Error(`${loaded.error.code}: ${loaded.error.message}`);
    return unpack(JSON.parse(new TextDecoder().decode(loaded.bytes)));
  };
}
