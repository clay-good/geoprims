// Equipment profiles (web/equipment-profiles): the numbers that belong to a
// reader's equipment rather than to one job, saved in this browser and moved
// between browsers as a JSON file. Nothing is uploaded. Which tool inputs each
// kind keeps is data, in data/profiles.json; a profile holds them per tool, in
// the same JSON the tool takes, so a profile file is readable on its own.
import PROFILES from '../../../../data/profiles.json' with { type: 'json' };

export const KINDS = PROFILES.kinds;
export const FORMAT = PROFILES.format;
export const VERSION = PROFILES.version;
export const MAX_BYTES = PROFILES.limits.maxBytes;
export const MAX_NAME = PROFILES.limits.maxName;
export const MAX_PER_KIND = PROFILES.limits.perKind;
/** Every profile lives under this one key, by kind and then by name. */
export const KEY = 'gp-profiles';
/** Where aircraft profiles lived before kinds; read once, then removed. */
export const LEGACY_KEY = 'gp-aircraft';
const LEGACY_FORMAT = 'geoprims-aircraft';

/** The inputs `kind` keeps for `toolId`, or [] when it keeps none. */
export const fieldsFor = (kind, toolId) => KINDS[kind]?.tools[toolId] ?? [];

/** The kinds whose profiles fill this tool, in data order. */
export const kindsFor = (toolId) => Object.keys(KINDS).filter((k) => fieldsFor(k, toolId).length > 0);

export const takesProfile = (toolId) => kindsFor(toolId).length > 0;

const today = () => new Date().toISOString().slice(0, 10);

/** A new, empty profile. */
export const blank = (kind, name, savedOn = today()) => ({ format: FORMAT, version: VERSION, kind, name, savedOn, tools: {} });

/** The profile with this tool's kept inputs replaced by the ones given. */
export function saveTool(profile, toolId, inputs) {
  const kept = {};
  for (const f of fieldsFor(profile.kind, toolId)) if (inputs[f] !== undefined) kept[f] = inputs[f];
  return { ...profile, tools: { ...profile.tools, [toolId]: kept } };
}

/** This tool's saved inputs, or an empty object when the profile has none. */
export const valuesFor = (profile, toolId) => profile.tools?.[toolId] ?? {};

/** The file a profile exports to. */
export const toFile = (profile) => JSON.stringify(profile, null, 2) + '\n';

/** A download name: "Zenmuse P1" as a camera is "zenmuse-p1.camera.json". */
export const fileName = (profile) =>
  (profile.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || profile.kind) + `.${profile.kind}.json`;

const plain = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);
const scalar = (v) => typeof v === 'string' || (typeof v === 'number' && Number.isFinite(v)) || typeof v === 'boolean';
const row = (v) => plain(v) && Object.values(v).every(scalar);
const isoDate = (v) => typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v);

/**
 * Reads an exported profile: this format, or an aircraft file from before
 * kinds (`geoprims-aircraft` version 1). Returns { ok: true, profile, dropped }
 * or { ok: false, message }. Tools and inputs the kind does not keep are
 * dropped and listed in `dropped`, so an edited or older file loads whatever
 * it still has right and says what it left out.
 */
export function fromFile(text) {
  if (typeof text !== 'string' || text.length > MAX_BYTES) return { ok: false, message: 'This file is too large to be an equipment profile.' };
  let p;
  try {
    p = JSON.parse(text);
  } catch {
    return { ok: false, message: 'This file is not JSON, so it is not an equipment profile.' };
  }
  if (!plain(p) || (p.format !== FORMAT && p.format !== LEGACY_FORMAT)) return { ok: false, message: 'This JSON file is not a geoprims equipment profile.' };
  const legacy = p.format === LEGACY_FORMAT;
  if (p.version !== VERSION) return { ok: false, message: `This profile is version ${p.version}; this site reads version ${VERSION}.` };
  const kind = legacy ? 'aircraft' : p.kind;
  if (!Object.hasOwn(KINDS, kind)) return { ok: false, message: `This site keeps no "${kind}" profiles; it keeps ${Object.keys(KINDS).join(', ')}.` };
  const name = typeof p.name === 'string' ? p.name.trim() : '';
  if (name === '' || name.length > MAX_NAME) return { ok: false, message: `The profile needs a name of 1 to ${MAX_NAME} characters.` };
  const tables = new Set(KINDS[kind].tables);
  const tools = {};
  const dropped = [];
  for (const [id, inputs] of Object.entries(plain(p.tools) ? p.tools : {})) {
    const fields = fieldsFor(kind, id);
    if (!fields.length || !plain(inputs)) {
      dropped.push(id);
      continue;
    }
    const kept = {};
    for (const [f, v] of Object.entries(inputs)) {
      const ok = fields.includes(f) && (tables.has(f) ? Array.isArray(v) && v.every(row) : scalar(v));
      if (ok) kept[f] = v;
      else dropped.push(`${id} ${f}`);
    }
    tools[id] = kept;
  }
  const savedOn = isoDate(p.savedOn) ? p.savedOn : today();
  return { ok: true, profile: { format: FORMAT, version: VERSION, kind, name, savedOn, tools }, dropped };
}

// Storage: every profile under one key, { kind: { name: profile } }. Storage
// can be blocked (a private window) or full; then profiles last for this page.
let memory = {};

/** Aircraft profiles saved before kinds, as profiles of kind aircraft. */
function migrate(store) {
  let old;
  try {
    old = JSON.parse(localStorage.getItem(LEGACY_KEY) ?? 'null');
  } catch {
    return store;
  }
  if (!plain(old)) return store;
  const aircraft = { ...(store.aircraft ?? {}) };
  for (const p of Object.values(old)) {
    const r = fromFile(JSON.stringify(p));
    if (r.ok && !aircraft[r.profile.name]) aircraft[r.profile.name] = r.profile;
  }
  const next = { ...store, aircraft };
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
    localStorage.removeItem(LEGACY_KEY);
  } catch {
    /* storage blocked: migrate again next time */
  }
  return next;
}

/** Every saved profile, { kind: { name: profile } }. */
export function all() {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? 'null');
    memory = migrate(plain(v) ? v : {});
  } catch {
    /* storage blocked: use what this page saved */
  }
  return memory;
}

/** The saved profiles of one kind, { name: profile }. */
export const ofKind = (kind) => all()[kind] ?? {};

function write(next) {
  memory = next;
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
    return true;
  } catch {
    return false;
  }
}

/** Whether a profile of this kind and name can be saved without passing the per-kind limit. */
export const hasRoom = (kind, name) => Object.hasOwn(ofKind(kind), name) || Object.keys(ofKind(kind)).length < MAX_PER_KIND;

/**
 * Saves a profile under its kind and name. Returns 'saved', 'page' when the
 * browser would not keep it (it lasts for this page), or 'full' when the kind
 * already holds MAX_PER_KIND other profiles.
 */
export function put(profile) {
  if (!hasRoom(profile.kind, profile.name)) return 'full';
  const store = all();
  return write({ ...store, [profile.kind]: { ...(store[profile.kind] ?? {}), [profile.name]: profile } }) ? 'saved' : 'page';
}

export function remove(kind, name) {
  const store = all();
  const mine = { ...(store[kind] ?? {}) };
  delete mine[name];
  return write({ ...store, [kind]: mine });
}

/** Renames a profile; false when the new name is taken, empty, or too long. */
export function rename(kind, from, to) {
  const name = to.trim();
  const mine = ofKind(kind);
  if (!mine[from] || name === '' || name.length > MAX_NAME || (name !== from && mine[name])) return false;
  const next = { ...mine };
  delete next[from];
  next[name] = { ...mine[from], name };
  return write({ ...all(), [kind]: next });
}
