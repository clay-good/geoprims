// Direct toolsets (add-local-mcp-server, "Optional direct toolsets"): opt-in
// groups of stable catalog tools exposed as first-class MCP tools, named
// gp_<id with dots as underscores>. Only stable tools join, so a toolset
// grows as tools are promoted.
import { createHash } from 'node:crypto';

/** Each toolset lists id prefixes; members are the stable tools that match, in catalog order. */
export const TOOLSETS = {
  // Everyday geodesy; the projections with user-set parameters, and State
  // Plane, are their own toolset, since together the two pass the 40-tool cap.
  // Coordinate-notation parsing and formatting is its own toolset too, since
  // with it the core passes the cap.
  'geodesy-core': [
    'geodesy.grid-ref.gars-',
    'geodesy.grid-ref.georef-',
    'geodesy.grid-ref.grid-zone-lookup',
    'geodesy.grid-ref.maidenhead-',
    'geodesy.grid-ref.mgrs-',
    'geodesy.grid-ref.usng-',
    'geodesy.utm.',
    'geodesy.ups.',
    'geodesy.datum.helmert',
    'geodesy.datum.itrf',
    'geodesy.datum.legacy',
    'geodesy.datum.nad83',
    'geodesy.datum.nadcon5',
    'geodesy.datum.plate-motion',
    'geodesy.datum.transform',
    'geodesy.ellipsoid.',
    'geodesy.frame.',
  ],
  'coordinate-notation': ['geodesy.parse.'],
  // Heights and magnetic models are their own toolsets, since with them the
  // core passes the 40-tool cap.
  heights: ['geodesy.geoid.', 'geodesy.height.'],
  magnetic: ['geodesy.magnetic.'],
  // The projections you set the parameters of, beside UTM, UPS, and State
  // Plane; the national grids are their own toolset, since with them this
  // passes the 40-tool cap.
  projections: [
    'geodesy.projection.albers-',
    'geodesy.projection.arc-to-chord',
    'geodesy.projection.azimuthal-equidistant-',
    'geodesy.projection.convergence',
    'geodesy.projection.equidistant-cylindrical-',
    'geodesy.projection.gnomonic-',
    'geodesy.projection.hotine-',
    'geodesy.projection.lcc-',
    'geodesy.projection.orthographic-',
    'geodesy.projection.polar-stereographic-',
    'geodesy.projection.scale-factor',
    'geodesy.projection.tm-',
    'geodesy.projection.web-mercator-',
    'geodesy.utm.',
    'geodesy.ups.',
    'geodesy.spcs.',
  ],
  // Other countries' grids and the official transformations onto them:
  // Britain, Ireland, Europe, France, Switzerland, Japan, South Africa,
  // Australia, New Zealand, and the Netherlands.
  'national-grids': [
    'geodesy.grid-ref.bng-',
    'geodesy.grid-ref.latlon-to-bng',
    'geodesy.grid-ref.irish-',
    'geodesy.grid-ref.itm-',
    'geodesy.datum.etrf',
    'geodesy.datum.gda2020',
    'geodesy.datum.ostn15',
    'geodesy.datum.osi-polynomial',
    'geodesy.projection.etrs89-',
    'geodesy.projection.jgd2011-',
    'geodesy.projection.lambert93-',
    'geodesy.projection.lo-',
    'geodesy.projection.lv95-',
    'geodesy.projection.mga-',
    'geodesy.projection.nztm-',
    'geodesy.datum.rdnaptrans',
  ],
  // Navigation and geometry together pass the cap, so each is its own toolset;
  // and the marine tools (sailings, piloting, celestial, tides, and the
  // vessel) are their own beside navigation, since together they pass it.
  navigation: ['navigation.geodesic.', 'navigation.route.', 'navigation.los.', 'navigation.vector.', 'navigation.rhumb.'],
  marine: ['navigation.sailing.', 'navigation.piloting.', 'navigation.celestial.', 'navigation.tide.', 'navigation.vessel.'],
  geometry: ['geometry.'],
  e6b: ['aviation.airspeed.', 'aviation.altimetry.', 'aviation.wind.', 'aviation.performance.'],
  atmosphere: ['aviation.atmosphere.', 'aviation.altimetry.', 'time.sun.'],
  // Mapping flight planning; power, ops, sensors, and links are their own
  // toolset, since together with photogrammetry and mission they pass the cap.
  'drone-mapping': ['drone.photogrammetry.', 'drone.mission.'],
  'drone-ops': ['drone.power.', 'drone.ops.', 'drone.sensors.', 'drone.links.'],
  // Office computation, and the field tools (GNSS, land records, and
  // astronomic azimuths) beside it, since together they pass the cap.
  'survey-cogo': ['survey.cogo.', 'survey.curves.', 'survey.reduction.', 'survey.earthwork.'],
  'survey-field': ['survey.gnss.', 'survey.land.', 'survey.astro.'],
  // H3 alone passes the cap, so it is its own toolset beside the other indexes.
  indexing: ['indexing.s2.', 'indexing.tile.', 'indexing.geohash.', 'indexing.plus-code.', 'indexing.a5.', 'indexing.convert.'],
  h3: ['indexing.h3.'],
};
export const MAX_PER_TOOLSET = 40;
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
