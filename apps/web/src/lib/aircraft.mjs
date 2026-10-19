// Aircraft profiles (aviation/fuel-and-loading, "Aircraft profiles saved
// locally"): the aircraft kind of the equipment profiles in profiles.mjs,
// with the interface the aircraft panel was written against. Storage, the
// file format, and the fields kept are profiles.mjs's and data/profiles.json's;
// old `.aircraft.json` files still import.
import * as profiles from './profiles.mjs';

const KIND = 'aircraft';

/** The inputs a profile keeps for each tool: aircraft data, not flight data. */
export const PROFILE_FIELDS = profiles.KINDS[KIND].tools;
/** The kept inputs that are tables (one object per row); the rest are single values. */
export const TABLES = new Set(profiles.KINDS[KIND].tables);
export const MAX_BYTES = profiles.MAX_BYTES;

export const takesProfile = (toolId) => profiles.fieldsFor(KIND, toolId).length > 0;
export const blank = (name, savedOn) => profiles.blank(KIND, name, savedOn);
export const { saveTool, valuesFor, toFile, fileName } = profiles;

/** Reads a profile file, refusing one of another kind. */
export function fromFile(text) {
  const r = profiles.fromFile(text);
  if (r.ok && r.profile.kind !== KIND) return { ok: false, message: `This is a ${profiles.KINDS[r.profile.kind].title.toLowerCase()} profile, not an aircraft profile.` };
  return r;
}

/** Saved aircraft profiles, by name. */
export const all = () => profiles.ofKind(KIND);
/** Saves a profile; false when it lasts for this page only (or the kind is full). */
export const put = (profile) => profiles.put(profile) === 'saved';
export const remove = (name) => profiles.remove(KIND, name);
