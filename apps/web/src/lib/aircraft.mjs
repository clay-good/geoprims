// Aircraft profiles (aviation/fuel-and-loading, "Aircraft profiles saved
// locally"): the numbers that belong to one airplane rather than one flight,
// saved in this browser and moved between browsers as a JSON file. Nothing
// is uploaded. A profile holds, per tool, the inputs listed below, in the
// same JSON the tool takes, so a profile file is readable on its own.

export const KEY = 'gp-aircraft';
export const FORMAT = 'geoprims-aircraft';
export const VERSION = 1;
/** Larger than any real POH's tables, small enough to refuse a wrong file. */
export const MAX_BYTES = 1_000_000;
const MAX_NAME = 60;

/** The inputs a profile keeps for each tool: aircraft data, not flight data. */
export const PROFILE_FIELDS = {
  'aviation.loading.weight-balance': ['stations', 'envelope', 'fuel_arm', 'lemac', 'mac'],
  'aviation.loading.fuel-plan': ['category', 'usable_fuel', 'taxi', 'climb', 'cruise_burn'],
  'aviation.loading.table-interpolate': ['table', 'names', 'corrections'],
  'aviation.airspeed.cas-to-tas': ['calibration', 'vs0', 'vs1', 'vfe', 'vno', 'vne'],
  'aviation.airspeed.tas-to-cas': ['vs0', 'vs1', 'vfe', 'vno', 'vne'],
};

/** The kept inputs that are tables (one object per row); the rest are single values. */
export const TABLES = new Set(['stations', 'envelope', 'table', 'corrections', 'calibration']);

export const takesProfile = (toolId) => Object.hasOwn(PROFILE_FIELDS, toolId);

/** A new, empty profile. */
export const blank = (name) => ({ format: FORMAT, version: VERSION, name, tools: {} });

/** The profile with this tool's aircraft inputs replaced by the ones given. */
export function saveTool(profile, toolId, inputs) {
  const kept = {};
  for (const f of PROFILE_FIELDS[toolId] ?? []) if (inputs[f] !== undefined) kept[f] = inputs[f];
  return { ...profile, tools: { ...profile.tools, [toolId]: kept } };
}

/** This tool's saved inputs, or an empty object when the profile has none. */
export const valuesFor = (profile, toolId) => profile.tools?.[toolId] ?? {};

/** The file a profile exports to. */
export const toFile = (profile) => JSON.stringify(profile, null, 2) + '\n';

/** A download name from the profile's name: "N12345 C172" becomes "n12345-c172.aircraft.json". */
export const fileName = (profile) =>
  (profile.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'aircraft') + '.aircraft.json';

const plain = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);
const scalar = (v) => typeof v === 'string' || (typeof v === 'number' && Number.isFinite(v));
const row = (v) => plain(v) && Object.values(v).every(scalar);

/**
 * Reads an exported profile. Returns { ok: true, profile } or { ok: false,
 * message }. Tools and inputs the site does not keep are dropped, so an
 * edited or older file loads whatever it still has right.
 */
export function fromFile(text) {
  if (typeof text !== 'string' || text.length > MAX_BYTES) return { ok: false, message: 'This file is too large to be an aircraft profile.' };
  let p;
  try {
    p = JSON.parse(text);
  } catch {
    return { ok: false, message: 'This file is not JSON, so it is not an aircraft profile.' };
  }
  if (!plain(p) || p.format !== FORMAT) return { ok: false, message: 'This JSON file is not a geoprims aircraft profile.' };
  if (p.version !== VERSION) return { ok: false, message: `This profile is version ${p.version}; this site reads version ${VERSION}.` };
  const name = typeof p.name === 'string' ? p.name.trim() : '';
  if (name === '' || name.length > MAX_NAME) return { ok: false, message: `The profile needs a name of 1 to ${MAX_NAME} characters.` };
  const tools = {};
  for (const [id, inputs] of Object.entries(plain(p.tools) ? p.tools : {})) {
    if (!takesProfile(id) || !plain(inputs)) continue;
    const kept = {};
    for (const f of PROFILE_FIELDS[id]) {
      const v = inputs[f];
      if (TABLES.has(f) ? Array.isArray(v) && v.every(row) : scalar(v)) kept[f] = v;
    }
    tools[id] = kept;
  }
  return { ok: true, profile: { format: FORMAT, version: VERSION, name, tools } };
}

// Storage: every profile under one key, by name. Storage can be blocked (a
// private window) or full; then profiles last for this page only.
let memory = {};
export function all() {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? 'null');
    if (plain(v)) memory = v;
  } catch {
    /* storage blocked: use what this page saved */
  }
  return memory;
}
function store(next) {
  memory = next;
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
    return true;
  } catch {
    return false;
  }
}
/** Saves a profile under its name; false when the browser would not keep it. */
export const put = (profile) => store({ ...all(), [profile.name]: profile });
export function remove(name) {
  const next = { ...all() };
  delete next[name];
  return store(next);
}
