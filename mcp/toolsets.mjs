// Direct toolsets (add-local-mcp-server, "Optional direct toolsets"): opt-in
// groups of stable catalog tools exposed as first-class MCP tools, named
// gp_<id with dots as underscores>. Only stable tools join, so a toolset
// grows as tools are promoted.
import { createHash } from 'node:crypto';

/** Each toolset lists id prefixes; members are the stable tools that match, in catalog order. */
export const TOOLSETS = {
  'geodesy-core': ['geodesy.'],
  navigation: ['navigation.', 'geometry.'],
  e6b: ['aviation.airspeed.', 'aviation.altimetry.', 'aviation.wind.', 'aviation.performance.'],
  atmosphere: ['aviation.atmosphere.', 'aviation.altimetry.', 'time.sun.'],
  'drone-mapping': ['drone.'],
  'survey-cogo': ['survey.'],
  indexing: ['indexing.'],
};
const MAX_PER_TOOLSET = 40;
const NAME = /^[a-zA-Z0-9_-]{1,64}$/;
export const DIRECT_OUTPUT_SCHEMA = { type: 'object', properties: { ok: { type: 'boolean' } }, required: ['ok'] };

/** gp_ + the id with dots as underscores; over 64 characters, a deterministic hash suffix. */
export function directName(id) {
  const name = `gp_${id.replaceAll('.', '_')}`;
  if (name.length <= 64) return name;
  const hash = createHash('sha256').update(id).digest('hex').slice(0, 8);
  return `${name.slice(0, 55)}_${hash}`;
}

export function directDescription(t) {
  return `${t.summary} Model: ${t.model}. Accuracy: ${t.accuracy}. Same as geoprims_run with id "${t.id}". Planning aid; relay warnings.`;
}

/** Throws with the valid names when any requested toolset is unknown. */
export function parseToolsets(value) {
  const names = String(value ?? '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  const bad = names.filter((n) => !(n in TOOLSETS));
  if (!names.length || bad.length) {
    throw new Error(`Unknown toolset ${bad.join(', ') || '(none given)'}. Valid toolsets: ${Object.keys(TOOLSETS).join(', ')}.`);
  }
  return names;
}

/** The MCP tool definitions for the named toolsets, each carrying its catalog `id`. */
export function directTools(catalog, names, annotations) {
  const picked = new Map();
  for (const n of names) {
    const members = catalog.tools.filter((t) => t.stability === 'stable' && TOOLSETS[n].some((p) => t.id.startsWith(p)));
    for (const t of members.slice(0, MAX_PER_TOOLSET)) picked.set(t.id, t);
  }
  const seen = new Map();
  return [...picked.values()].map((t) => {
    const name = directName(t.id);
    if (!NAME.test(name)) throw new Error(`Direct tool name ${name} for ${t.id} is not a valid MCP tool name.`);
    if (seen.has(name)) throw new Error(`Direct tool names collide: ${t.id} and ${seen.get(name)} both map to ${name}.`);
    seen.set(name, t.id);
    return {
      id: t.id,
      name,
      title: t.title,
      description: directDescription(t),
      inputSchema: t.inputs,
      outputSchema: DIRECT_OUTPUT_SCHEMA,
      annotations: { title: t.title, ...annotations },
    };
  });
}
