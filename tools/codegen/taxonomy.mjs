// Catalog-wide taxonomy gate (platform/tool-catalog). The Rust manifest lint
// checks each crate; this catches collisions and taxonomy drift across crates.
const DOMAINS = ['geodesy', 'navigation', 'geometry', 'aviation', 'drone', 'survey', 'indexing', 'raster', 'time', 'units'];
const SEGMENT = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export function taxonomyProblems(tools, taxonomy) {
  const problems = [];
  const domains = taxonomy?.domains ?? {};
  for (const domain of DOMAINS) {
    if (!domains[domain]) problems.push(`missing domain ${domain}`);
  }
  for (const [domain, row] of Object.entries(domains)) {
    if (!DOMAINS.includes(domain)) problems.push(`unknown domain ${domain}`);
    if (!row.title?.trim()) problems.push(`${domain}: missing title`);
    if (!Array.isArray(row.groups) || row.groups.length === 0) {
      problems.push(`${domain}: no groups`);
      continue;
    }
    const seen = new Set();
    for (const group of row.groups) {
      if (typeof group !== 'string' || !SEGMENT.test(group)) problems.push(`${domain}: invalid group ${group}`);
      if (seen.has(group)) problems.push(`${domain}: group ${group} is declared twice`);
      seen.add(group);
    }
  }

  const ids = new Set();
  for (const tool of tools) {
    const parts = tool.id?.split('.') ?? [];
    if (parts.length !== 3 || parts.some((part) => !SEGMENT.test(part))) {
      problems.push(`${tool.id}: id must be domain.group.operation`);
      continue;
    }
    const [domain, group] = parts;
    if (ids.has(tool.id)) problems.push(`${tool.id}: id appears twice`);
    ids.add(tool.id);
    if (tool.domain !== domain || tool.group !== group) problems.push(`${tool.id}: domain or group disagrees with the id`);
    const matches = domains[domain]?.groups?.filter((name) => name === group).length ?? 0;
    if (matches !== 1) problems.push(`${tool.id}: belongs to ${matches} groups in ${domain}`);
  }
  return problems;
}
