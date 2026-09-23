// Build-time access to the catalog and the core (Node). Pages are rendered with
// real answers from the same Wasm modules the browser loads.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { nodeHost } from '../../../../packages/runtime/src/node.mjs';
import { runJourney } from './journeys.mjs';
import { vectorFor } from './worked.mjs';
import { readSignoffs, reviewSentence } from '../../../../tools/trust/signoffs.mjs';
import { entriesFor, KINDS, readChangelog } from '../../../../tools/trust/changelog.mjs';
import { verificationReport } from '../../../../tools/trust/verification.mjs';
import { readLedger, rowFor } from '../../../../tools/trust/ledger.mjs';
import { buildDate } from './build-date.mjs';

const root = join(process.cwd(), '../..');
export const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const codes = JSON.parse(readFileSync(join(root, 'data/codes.json'), 'utf8'));
const ledgerRows = Object.values(JSON.parse(readFileSync(join(root, 'data/sources-ledger.json'), 'utf8'))).find(Array.isArray);
const sourceById = new Map(ledgerRows.map((r) => [r.id, r]));
const signoffs = readSignoffs(root);
const buildDay = buildDate();
/** The honest review line for a domain, or for one tool when `id` is given. */
export const reviewLine = (domain, id = null) => reviewSentence(signoffs, domain, buildDay, id);

const ledger = readLedger(root);
/** The ledger rows a tool cites, deduplicated (trust/freshness). */
export const sourceRowsFor = (t) => [...new Map(t.references.map((r) => rowFor(ledger, r)).filter(Boolean).map((r) => [r.id, r])).values()];

/**
 * The worked example as "You enter / You get" (contracts/page-chrome region 9,
 * discovery/search-pages content minimums): the example's inputs with their
 * labels and units, and the answer the core produced for them.
 */
export function youEnterYouGet(t, result) {
  const ex = primaryExample(t);
  // The same unit spellings a reader types, as the tool island shows them.
  const readable = (text) => text.replace(/(\d)\s*deg([CF])\b/g, '$1 °$2');
  const label = (side, k) => t[side].properties[k]?.title ?? k;
  const unitOf = (side, k) => {
    const u = t[side].properties[k]?.['x-unit'];
    return u && u !== '1' ? u : '';
  };
  // A list input reads as its rows, one per line and in the column order the
  // manifest declares, the way the form takes them ("0, 300"), never as JSON.
  const listText = (k, rows) => {
    const cols = Object.keys(t.inputs.properties[k]?.items?.properties ?? {});
    return rows.map((row) => (cols.length ? cols : Object.keys(row)).filter((c) => row[c] !== undefined).map((c) => row[c]).join(', ')).join('\n');
  };
  const valueText = (k, v) => (Array.isArray(v) ? listText(k, v) : typeof v === 'object' && v !== null ? JSON.stringify(v) : String(v));
  const enter = Object.entries(ex.input)
    .filter(([k]) => k !== 'options')
    .map(([k, v]) => ({ label: label('inputs', k), value: readable(valueText(k, v)), unit: typeof v === 'number' ? unitOf('inputs', k) : '', rows: Array.isArray(v) }));
  const get = result?.ok
    ? Object.entries(result.display ?? {}).map(([k, v]) => ({ label: label('outputs', k), value: v, unit: '' }))
    : [];
  return { title: ex.title, source: ex.source, enter, get };
}

/** The day a maintainer last confirmed the sources behind a tool, or null. */
export function lastVerifiedFor(t) {
  const days = sourceRowsFor(t).map((r) => r.lastVerified).filter(Boolean).sort();
  return days[0] ?? null;
}

/** Severity of each warning a tool may emit (codes registry), for ordering and styling. */
export const severities = (t) => Object.fromEntries(t.warnings.map((c) => [c, codes.warnings[c]?.severity ?? 'info']));
const host = nodeHost(join(root, 'dist/wasm'));

export const changelog = readChangelog(root);
export const CHANGE_KINDS = KINDS;
/** Changelog entries that name a tool. */
export const changesFor = (id) => entriesFor(changelog, id);
const glossary = JSON.parse(readFileSync(join(root, 'data/glossary.json'), 'utf8'));
/** The sources ledger and the row a citation matches, for the page renderer. */
export { ledger };
export const ledgerRowFor = (rows, ref) => rowFor(rows, ref);

/** A sources-ledger row by id, for anything that cites one. */
export const sourceRow = (id) => sourceById.get(id) ?? null;

/** Glossary entries for the abbreviations a tool shows (data/glossary.json), alphabetical. */
export const termsFor = (id) =>
  glossary.entries.filter((e) => e.relatedTools.includes(id)).sort((a, b) => a.term.localeCompare(b.term));
let report = null;
/** This release's verification report, computed once per build from the vectors. */
export const verification = () => (report ??= verificationReport({ root, catalog, host }));
/** Plain display of an observed error and its share of the declared tolerance. */
export function errorShare(worst) {
  if (!worst) return { error: 'no numeric checks', share: '' };
  const tol = [worst.abs && `±${worst.abs}`, worst.rel && `±${worst.rel} relative`].filter(Boolean).join(' + ') || 'exact';
  const share = Number.isFinite(worst.used) ? `${Math.round(worst.used * 100)}%` : 'over';
  return { error: worst.error === 0 ? '0' : worst.error.toPrecision(2), tol, share, path: worst.path.replace(/^result\./, '').replace(/\.value$/, '') };
}
const modules = JSON.parse(readFileSync(join(root, 'dist/wasm/modules.json'), 'utf8')).modules;

/**
 * Every Wasm module in this release with its full SHA-256, so a reader can
 * check the bytes this site served (platform/verification, "Reproducible
 * builds"). The same source gives the same digests: `npm run
 * verify:reproducible` builds twice and compares them.
 */
export const moduleDigests = () =>
  modules.map(({ module, bytes, sha256 }) => ({ module, bytes, sha256 })).sort((a, b) => a.module.localeCompare(b.module));

/** The first 16 hex digits of the SHA-256 of the tool's Wasm module: which build made the answer. */
export const buildHashOf = (t) => (modules.find((m) => m.module === (t.domain === 'units' ? 'base' : t.domain))?.sha256 ?? '').slice(0, 16);

export const DOMAIN_TITLES = {
  geodesy: 'Geodesy', navigation: 'Navigation', geometry: 'Geometry', aviation: 'Aviation', drone: 'Drone',
  survey: 'Survey', indexing: 'Spatial indexing', raster: 'Raster', time: 'Time', units: 'Units',
};

/** One friendly line per domain for the home page. */
export const DOMAIN_BLURBS = {
  geodesy: 'Coordinates, datums, projections, grid references, and the shape of the Earth.',
  navigation: 'Distances, bearings, routes, and what you can see from where.',
  geometry: 'Areas and shapes on the curved Earth.',
  aviation: 'Density altitude, winds, airspeeds, performance, and weather codes.',
  drone: 'Ground sample distance, flight plans, batteries, and the rules.',
  survey: 'Traverses, curves, earthwork, and legal land descriptions.',
  indexing: 'H3 cells, geohashes, map tiles, and Plus Codes.',
  raster: 'Elevation and terrain from grids.',
  time: 'Time scales, time zones, sunrise, sunset, and the sun’s position.',
  units: 'Exact conversions for every unit these tools use.',
};

/** Group labels that are acronyms or need their own spelling. */
const GROUP_TITLES = {
  ifr: 'IFR', spcs: 'State plane coordinates', ups: 'UPS', utm: 'UTM', h3: 'H3', 'grid-ref': 'Grid references',
  los: 'Line of sight', cogo: 'Coordinate geometry', 'plus-code': 'Plus Code',
};

/** A group's heading: its own spelling, else the slug in sentence case. */
export const groupTitle = (group) => GROUP_TITLES[group] ?? group[0].toUpperCase() + group.slice(1).replaceAll('-', ' ');

/** The plain word for each domain, for the home page's description sentence. */
export const DOMAIN_NOUNS = {
  geodesy: 'geodesy', navigation: 'navigation', geometry: 'geometry', aviation: 'aviation', drone: 'drones',
  survey: 'surveying', indexing: 'spatial indexing', raster: 'terrain', time: 'time', units: 'units',
};

/** "a, b, and c" from a list of words. */
export const sentenceList = (words) =>
  words.length < 3 ? words.join(' and ') : `${words.slice(0, -1).join(', ')}, and ${words.at(-1)}`;

export const route = (id) => '/' + id.split('.').join('/') + '/';
export const tool = (id) => catalog.tools.find((t) => t.id === id);

/**
 * A tool's canonical page: a deprecated tool points at its replacement, and a
 * generated endpoint at the operation it composes.
 */
export const canonicalOf = (t) =>
  t.deprecation ? route(t.deprecation.replacement) : t.composedOf.length ? route(t.composedOf[0]) : route(t.id);

/** Only stable tools are indexable (experimental and deprecated pages are not). */
export const indexable = (t) => t.stability === 'stable' && t.composedOf.length === 0 && !t.deprecation;

export const primaryExample = (t) => t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];

/** Runs the tool's primary example so the answer is in the HTML. */
export async function exampleResult(t) {
  return JSON.parse(await host.invoke(t.id, JSON.stringify(primaryExample(t).input)));
}

/**
 * The tool's own work for its primary example, the same trace an agent gets
 * from `explain: true` (trust/proof-display, "Show your work").
 *
 * A decoder's work is not a formula but a reading, so it has none of these;
 * `exampleGroups` gives that instead.
 */
export async function exampleTrace(t) {
  const input = { ...primaryExample(t).input, options: { ...(primaryExample(t).input.options ?? {}), explain: true } };
  const r = JSON.parse(await host.invoke(t.id, JSON.stringify(input)));
  return r.ok ? (r.trace ?? []) : [];
}

/**
 * A decoder's work: every coded group of the report beside what it means.
 * Empty for a tool that decodes nothing.
 */
export async function exampleGroups(t) {
  const r = await exampleResult(t);
  const groups = r.ok ? r.result?.groups : null;
  return Array.isArray(groups) ? groups.filter((g) => g?.group && g?.meaning) : [];
}

export function groups(domain) {
  const byGroup = new Map();
  for (const t of catalog.tools.filter((x) => x.domain === domain)) {
    if (!byGroup.has(t.group)) byGroup.set(t.group, []);
    byGroup.get(t.group).push(t);
  }
  return byGroup;
}

const SITE = 'https://geoprims.com';

/** BreadcrumbList JSON-LD for a trail of [name, route] pairs. */
export const breadcrumbs = (trail) => ({
  '@context': 'https://schema.org',
  '@type': 'BreadcrumbList',
  itemListElement: trail.map(([name, path], i) => ({ '@type': 'ListItem', position: i + 1, name, item: SITE + path })),
});

/** WebApplication JSON-LD for a tool page: free, no ratings or reviews. */
export const webApplication = (t) => ({
  '@context': 'https://schema.org',
  '@type': 'WebApplication',
  name: t.title,
  description: t.summary,
  url: SITE + route(t.id),
  applicationCategory: 'UtilitiesApplication',
  operatingSystem: 'Any (runs in the browser)',
  isAccessibleForFree: true,
  offers: { '@type': 'Offer', price: 0, priceCurrency: 'USD' },
});

/** CollectionPage with an ItemList for a hub. */
export const collection = (name, path, tools) => ({
  '@context': 'https://schema.org',
  '@type': 'CollectionPage',
  name,
  url: SITE + path,
  mainEntity: {
    '@type': 'ItemList',
    itemListElement: tools.map((t, i) => ({ '@type': 'ListItem', position: i + 1, name: t.title, url: SITE + route(t.id) })),
  },
});

/** The input table for the developer block: field, unit, and allowed values. */
export const inputRows = (t) =>
  Object.entries(t.inputs.properties)
    .filter(([k]) => k !== 'options')
    .map(([k, s]) => ({
      name: k,
      required: t.inputs.required.includes(k),
      unit: s['x-unit'] && s['x-unit'] !== '1' ? s['x-unit'] : '',
      range: s.enum ? s.enum.join(', ') : s['x-angle-range'] ?? (s.minimum !== undefined || s.maximum !== undefined ? `${s.minimum ?? ''} to ${s.maximum ?? ''}` : s.type === 'array' ? 'list of rows' : ''),
    }));

/**
 * Where each status output's threshold comes from, resolved at build time so
 * the answer card can always cite it (ux/glanceable-results status phrases).
 */
export const statusSources = (t) =>
  Object.fromEntries(
    Object.entries(t.outputs?.properties ?? {})
      .filter(([, f]) => f['x-status'])
      .map(([name, f]) => {
        const { kind, source } = f['x-status'];
        if (source === 'user') return [name, { kind, label: 'the limit you entered', url: '' }];
        const row = sourceById.get(source);
        return [name, { kind, label: row?.name ?? source, url: row?.freeAccessUrl ?? '' }];
      }),
  );

/** The slice of a tool that ToolApp needs in the browser (tool page and the home page's featured copy). */
export const clientTool = (t) => ({
  id: t.id,
  title: t.title,
  route: route(t.id),
  visualization: t.visualization,
  timeline: t.timeline ?? null,
  version: t.version,
  inputs: t.inputs,
  outputs: t.outputs,
  severity: severities(t),
  statusSources: statusSources(t),
  coreVersion: catalog.coreVersion,
  buildHash: buildHashOf(t),
  // Whether the answer carries a compact picture of itself.
  'x-diagram-inline': t['x-diagram-inline'] ?? false,
});

// Group hubs (web/tool-docs, "Domain and group index pages"): the tasks a
// group serves, each with its tools, and a short guide where they form a
// sequence. Unit groups have one task, converting. A step's tool is an
// operation in the group or a full id elsewhere.
const hubs = JSON.parse(readFileSync(join(root, 'data/hubs.json'), 'utf8')).hubs;
/** A domain hub's intro paragraph, from data/hubs.json keyed by the domain. */
export const domainIntro = (domain) => hubs[domain]?.intro ?? '';

export function hubFor(domain, group) {
  const inGroup = catalog.tools.filter((t) => t.domain === domain && t.group === group);
  const find = (ref) => catalog.tools.find((t) => t.id === (ref.includes('.') ? ref : `${domain}.${group}.${ref}`));
  const hub = hubs[`${domain}.${group}`];
  // A hub may carry only an intro (the units groups), and then lists its tools as one task.
  if (!hub?.tasks) return { summary: hub?.summary ?? '', intro: hub?.intro ?? '', tasks: [{ want: `convert ${group.replaceAll('-', ' ')}`, tools: inGroup }], guide: null };
  return {
    summary: hub.summary,
    intro: hub.intro ?? '',
    tasks: hub.tasks.map((t) => ({ want: t.want, tools: t.tools.map(find).filter(Boolean) })),
    guide: hub.guide ? { title: hub.guide.title, steps: hub.guide.steps.map((s) => ({ text: s.text, tool: s.tool ? find(s.tool) : null })) } : null,
  };
}

// Learning guides (web/tool-docs): each journey in data/journeys.json run
// through the core at build time, every step with the permalink that opens its
// tool holding those inputs and naming the step it came from. A step that
// fails stops the build (runJourney throws).
export const JOURNEYS = JSON.parse(readFileSync(join(root, 'data/journeys.json'), 'utf8')).journeys;
let guides;
export function journeyGuides() {
  guides ??= (async () => {
    const link = await host.module('link');
    const invoke = async (id, input) => JSON.parse(await host.invoke(id, JSON.stringify(input)));
    const out = [];
    for (const j of JOURNEYS) {
      const steps = await runJourney(j, invoke);
      for (const s of steps) {
        const came = s.carried.length ? steps[s.carried[0].from].tool : undefined;
        const enc = JSON.parse(await link.callString('gp_link_encode', JSON.stringify({ state: { i: s.input, ...(came ? { c: came } : {}) } })));
        if (!enc.ok) throw new Error(`${j.slug}: ${s.tool}: ${enc.error.message}`);
        s.href = `${route(s.tool)}#${enc.result.fragment}`;
        s.title = catalog.tools.find((t) => t.id === s.tool).title;
      }
      out.push({ ...j, steps, route: `/journeys/${j.slug}/` });
    }
    return out;
  })();
  return guides;
}
/** The journeys a tool appears in, for hub and home links. */
export const journeysWith = (ids) => JOURNEYS.filter((j) => j.steps.some((s) => ids.includes(s.tool)));

// The golden vector a tool's worked example is, if it is one (9.1); the build
// gate (scripts/examples.mjs) has already checked the tool agrees with it.
export function workedVector(t) {
  const vectors = readFileSync(join(root, t.vectors), 'utf8').trim().split('\n').map((l) => JSON.parse(l));
  return vectorFor(primaryExample(t).input, vectors);
}
