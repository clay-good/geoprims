// Paste-to-detect (web/command-palette): the core names every plausible
// reading of a value by syntax (gp_detect); each is confirmed by running its
// decoder, failures are dropped, and the tools worth opening get links
// pre-filled with the value over their worked example. Shared by the compute
// worker and the tests.
const route = (id) => '/' + id.split('.').join('/') + '/';
const at = (o, path) => path.split('.').reduce((x, k) => x?.[k], o);

/**
 * `candidates(query)` returns gp_detect's candidates, `run(id, input)` a tool
 * result, `encode(state)` a gp_link_encode result, and `tools` maps id to manifest.
 */
export async function detectValues(query, { candidates, run, encode, tools }) {
  const found = [];
  for (const c of await candidates(query)) {
    const decoded = await run(c.decoder.id, c.decoder.input);
    if (!decoded.ok) continue;
    const actions = [];
    for (const o of c.offers) {
      const t = tools.get(o.id);
      if (!t) continue;
      const example = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0])?.input ?? {};
      const i = { ...example, ...o.input };
      for (const [field, path] of Object.entries(o.from)) i[field] = at(decoded, path);
      const link = await encode({ state: { i }, flags: [] });
      actions.push({ id: o.id, title: t.title, input: i, href: route(o.id) + (link.ok ? `#${link.result.fragment}` : '') });
    }
    found.push({ kind: c.kind, label: c.label, value: c.value, summary: decoded.summary, actions });
  }
  return { ok: true, result: { found } };
}
