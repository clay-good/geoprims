// Search and prefill accuracy (discovery/natural-language-prefill "Measured
// ranking quality"). Questions are generated from every tool's worked
// example: its title or an alias followed by the example's numeric values,
// as a person would type them. There is no search log yet, so these stand in
// for real phrasings alongside the hand-written data/prefill-fixture.json.

const MAX_ALIASES = 3;

/** A value as it would be typed, or null when it is not a number or a number with a unit. */
function typed(v) {
  if (typeof v === 'number') return String(v);
  if (typeof v === 'string' && /^[-+]?\d/.test(v) && v.length <= 24) return v;
  return null;
}

/** [{query, id, expect: {field: value}}] for every tool and phrasing. */
export function generate(tools) {
  const out = [];
  for (const t of tools) {
    const ex = (t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0])?.input ?? {};
    // In input order (the manifest's prefill list), as a person would list them.
    const order = (t.prefill ?? []).map((p) => p.input);
    const values = Object.entries(ex)
      .map(([k, v]) => [k, typed(v)])
      .filter(([, v]) => v !== null)
      .sort(([a], [b]) => order.indexOf(a) - order.indexOf(b));
    const expect = Object.fromEntries(values);
    const tail = values.map(([, v]) => v).join(' ');
    for (const name of [t.title, ...t.aliases.slice(0, MAX_ALIASES)]) {
      // "with" keeps the title's last word from reading as the first value's name.
      out.push({ query: tail ? `${name} with ${tail}` : name, id: t.id, expect });
    }
  }
  return out;
}

/** "29.80 inHg" and 29.8 compare equal when the unit is the field's own. */
function same(got, want, unit) {
  const parse = (v) => {
    const m = /^([-+]?[\d.]+)\s*(.*)$/.exec(String(v).trim());
    return m ? [Number(m[1]), m[2] || unit || ''] : [NaN, String(v)];
  };
  const spelling = { '°C': 'degC', '℃': 'degC', '°F': 'degF', '℉': 'degF' };
  const canon = (u) => (spelling[u] ?? u).toLowerCase();
  const [a, ua] = parse(got);
  const [b, ub] = parse(want);
  return a === b && canon(ua) === canon(ub);
}

/** Runs every question through `search(request)` and scores ranking and prefill. */
export async function measure(cases, tools, search) {
  const byId = new Map(tools.map((t) => [t.id, t]));
  let top1 = 0, top3 = 0, filled = 0, right = 0, expected = 0;
  const misses = [];
  const wrongFills = [];
  for (const c of cases) {
    const out = JSON.parse(await search(JSON.stringify({ query: c.query, limit: 3, includeExperimental: true })));
    const ids = out.ok ? out.result.results.map((r) => r.id) : [];
    if (ids[0] === c.id) top1++;
    if (ids.includes(c.id)) top3++;
    else misses.push(c.query);
    if (ids[0] !== c.id) continue;
    const props = byId.get(c.id).inputs.properties;
    const got = out.result.results[0].prefill ?? {};
    expected += Object.keys(c.expect).length;
    for (const [k, v] of Object.entries(got)) {
      filled++;
      if (k in c.expect && same(v, c.expect[k], props[k]?.['x-unit'])) right++;
      else wrongFills.push(`${c.query}: ${k} = ${JSON.stringify(v)}`);
    }
  }
  const pct = (n, d) => (d ? Math.round((1000 * n) / d) / 10 : 100);
  return {
    queries: cases.length,
    top1: pct(top1, cases.length),
    top3: pct(top3, cases.length),
    prefillPrecision: pct(right, filled),
    prefillRecall: pct(right, expected),
    misses,
    wrongFills,
  };
}
