// The equipment-profile gate (web/equipment-profiles, "Profile kinds are
// data"). Every field a profile kind keeps is a real input of its tool, is not
// private, and belongs to the equipment rather than the job: a profile that
// kept a location, a date, the wind, or a load would quietly carry one day's
// conditions into the next plan.

/** Input names that describe the job, never the equipment. */
const JOB = /^(lat|lon|latitude|longitude|.*_lat|.*_lon|waypoints|position|date|time|datetime|start|end|when|gust|headwind|tailwind|payload|load|cargo|passengers)$/;

/** Why `field` is a job value, or null when it may belong to equipment. */
export function jobValue(field) {
  if (JOB.test(field)) return 'it describes the job (a place, a time, or a load)';
  // Wind is the day's; a wind rating or a maximum crosswind is the airframe's.
  if (/wind/.test(field) && !/rating/.test(field) && !/^max_/.test(field)) return 'wind is a condition of the day, not of the equipment';
  return null;
}

/** Problems with `profiles` (data/profiles.json) against the catalog. */
export function profileProblems(profiles, catalog) {
  const byId = new Map(catalog.tools.map((t) => [t.id, t]));
  const problems = [];
  for (const [kind, def] of Object.entries(profiles.kinds ?? {})) {
    if (!/^[a-z]+(-[a-z]+)*$/.test(kind)) problems.push(`kind ${kind}: ids are lowercase words joined by hyphens`);
    const tables = new Set(def.tables ?? []);
    for (const [id, fields] of Object.entries(def.tools ?? {})) {
      const tool = byId.get(id);
      if (!tool) {
        problems.push(`${kind}: ${id} is not a tool`);
        continue;
      }
      for (const f of fields) {
        const p = tool.inputs.properties[f];
        if (!p) {
          problems.push(`${kind}: ${id} has no input ${f}`);
          continue;
        }
        if (p['x-private']) problems.push(`${kind}: ${id} ${f} is private`);
        const job = jobValue(f);
        if (job) problems.push(`${kind}: ${id} ${f} cannot be kept: ${job}`);
        if ((p.type === 'array') !== tables.has(f)) {
          problems.push(`${kind}: ${id} ${f} is ${p.type === 'array' ? 'a table but not listed in tables' : 'listed in tables but not an array'}`);
        }
      }
    }
  }
  for (const ex of profiles.examples ?? []) {
    const fields = profiles.kinds?.[ex.kind]?.tools ?? null;
    if (!fields) {
      problems.push(`example ${ex.name}: no kind ${ex.kind}`);
      continue;
    }
    for (const [id, values] of Object.entries(ex.tools)) {
      for (const f of Object.keys(values)) {
        if (!(fields[id] ?? []).includes(f)) problems.push(`example ${ex.name}: ${ex.kind} keeps no ${f} for ${id}`);
      }
    }
  }
  return problems;
}

/**
 * Problems with workflow inputs that declare `profile: {kind, field}`
 * (web/workflows, "Workflow inputs filled from profiles"): the kind exists,
 * and its map keeps that field for the input's own tool.
 */
export function workflowProfileProblems(workflows, profiles) {
  const problems = [];
  for (const w of workflows) {
    for (const i of w.inputs) {
      if (i.profile === undefined) continue;
      const { kind, field } = i.profile ?? {};
      const def = profiles.kinds?.[kind];
      if (!def) problems.push(`${w.slug}: input ${i.name} names no profile kind "${kind}"`);
      else if (!(def.tools[i.tool] ?? []).includes(field)) problems.push(`${w.slug}: input ${i.name} declares kind ${kind}, whose map keeps no ${field} for ${i.tool}`);
    }
  }
  return problems;
}
