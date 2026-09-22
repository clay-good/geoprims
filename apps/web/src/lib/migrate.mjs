// Old permalinks, still readable (build-web-experience 3.5, "migration
// hooks"). A link holds a tool's inputs by field name; when a field is renamed
// or removed, data/link-migrations.json says so, and a link made before the
// change opens with its values in the right places. Anything the tool no
// longer takes is dropped and named, never silently ignored.

/**
 * A decoded link's inputs for `tool`, migrated: { inputs, dropped, renamed }.
 * `migrations` is the "tools" table of data/link-migrations.json.
 */
export function migrateState(tool, inputs, migrations = {}) {
  let out = { ...(inputs ?? {}) };
  const renamed = [];
  for (const step of migrations[tool.id] ?? []) {
    for (const [from, to] of Object.entries(step.rename ?? {})) {
      if (from in out && !(to in out)) {
        out[to] = out[from];
        renamed.push([from, to]);
      }
      delete out[from];
    }
    for (const gone of step.drop ?? []) delete out[gone];
  }
  const known = tool.inputs?.properties ?? {};
  const dropped = Object.keys(out).filter((k) => !(k in known));
  for (const k of dropped) delete out[k];
  return { inputs: out, dropped, renamed };
}
