// Build-time access to the catalog and the core (Node). Pages are rendered with
// real answers from the same Wasm modules the browser loads.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../../../packages/runtime/src/node.mjs';

const root = join(process.cwd(), '../..');
export const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const codes = JSON.parse(readFileSync(join(root, 'data/codes.json'), 'utf8'));

/** Severity of each warning a tool may emit (codes registry), for ordering and styling. */
export const severities = (t) => Object.fromEntries(t.warnings.map((c) => [c, codes.warnings[c]?.severity ?? 'info']));
const host = nodeHost(join(root, 'dist/wasm'));
const modules = JSON.parse(readFileSync(join(root, 'dist/wasm/modules.json'), 'utf8')).modules;

/** The first 16 hex digits of the SHA-256 of the tool's Wasm module: which build made the answer. */
export const buildHashOf = (t) => (modules.find((m) => m.module === (t.domain === 'units' ? 'base' : t.domain))?.sha256 ?? '').slice(0, 16);

export const DOMAIN_TITLES = {
  geodesy: 'Geodesy', navigation: 'Navigation', geometry: 'Geometry', aviation: 'Aviation', drone: 'Drone',
  survey: 'Survey', indexing: 'Spatial indexing', raster: 'Raster', time: 'Time', units: 'Units',
};

export const route = (id) => '/' + id.split('.').join('/') + '/';
export const tool = (id) => catalog.tools.find((t) => t.id === id);

/** A tool's canonical page: generated endpoints point at their parent operation. */
export const canonicalOf = (t) => (t.composedOf.length ? route(t.composedOf[0]) : route(t.id));

/** Only stable tools are indexable (experimental pages lack their full content). */
export const indexable = (t) => t.stability === 'stable' && t.composedOf.length === 0;

export const primaryExample = (t) => t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];

/** Runs the tool's primary example so the answer is in the HTML. */
export async function exampleResult(t) {
  return JSON.parse(await host.invoke(t.id, JSON.stringify(primaryExample(t).input)));
}

export function groups(domain) {
  const byGroup = new Map();
  for (const t of catalog.tools.filter((x) => x.domain === domain)) {
    if (!byGroup.has(t.group)) byGroup.set(t.group, []);
    byGroup.get(t.group).push(t);
  }
  return byGroup;
}
