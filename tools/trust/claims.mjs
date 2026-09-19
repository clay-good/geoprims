// Claims honesty gate (trust/correctness-program): public statements must match
// the build. Checks counts, "experimental" statements, "checked against"
// statements, review statements, and offline claims, sentence by sentence, and
// quotes each failing sentence.
import { existsSync } from 'node:fs';
import { join } from 'node:path';

const DOMAIN_WORDS = {
  geodesy: /\bgeodesy\b/i, navigation: /\bnavigation\b/i, geometry: /\bgeometry\b/i, aviation: /\baviation\b/i,
  drone: /\bdrone\b/i, survey: /\bsurvey\b/i, indexing: /\bindexing\b/i, raster: /\braster\b/i, time: /\btime domain\b/i,
  units: /\bunits domain\b/i,
};

// Reference implementations a page may say results are checked against, with
// the differential suites that must exist for the claim to stand.
export const REFERENCES = {
  GeographicLib: ['tools/vectors/gen_navigation.py', 'tools/vectors/gen_geodesy.py', 'core/crates/gp-geodesy/tests/geoid.rs'],
  GeoidEval: ['core/crates/gp-geodesy/tests/geoid.rs'],
  PROJ: ['core/crates/gp-geodesy/tests/spcs.rs', 'core/crates/gp-geodesy/tests/data/spcs83_diff.csv'],
  'H3 C': ['core/crates/gp-indexing/tests/h3_c_parity.rs'],
  pvlib: ['tools/vectors/gen_sun.py'],
  zoneinfo: ['tools/vectors/gen_tz_diff.py'],
  'WMM2025 test values': ['core/crates/gp-geodesy/tests/data/WMM2025_TestValues.txt'],
  ppigrf: ['tools/vectors/gen_magnetic.py'],
  GeodTest: ['core/crates/gp-navigation/tests/data/GeodTest-sample.dat'],
  TMcoords: ['core/crates/gp-geodesy/tests/data/TMcoords-sample.dat'],
  ambiance: ['tools/vectors/gen_aviation.py'],
};

const NEGATION = /\b(not yet|no practitioner|no one|pending|until|will be|only when|unless)\b/i;

/** Plain-text sentences from markdown or HTML; list items and table rows stand alone. */
export function sentences(text) {
  const plain = text
    .replace(/\n\s*\n|\n(?=\s*(?:[-*|#]|\d+\.) )/g, ' ¶¶ ')
    .replace(/<script[\s\S]*?<\/script>|<style[\s\S]*?<\/style>/g, ' ')
    .replace(/<[^>]+>/g, ' ')
    .replace(/&[a-z#0-9]+;/gi, ' ')
    .replace(/[*`|]/g, ' ')
    .replace(/\s+/g, ' ');
  return plain
    .split(/ ?¶¶ ?|(?<=[.!?;])\s+(?=[A-Z0-9"'(])/)
    .map((s) => s.trim())
    .filter(Boolean);
}

const num = (s) => Number(s.replaceAll(',', ''));

/**
 * Problems in `text` (from `where`) given the build: `catalog`, the parsed
 * sign-offs, `root` for evidence files, and `offline` (a service worker ships).
 */
export function claimProblems(text, where, { catalog, signoffs, root, offline }) {
  const problems = [];
  const fail = (s, why) => problems.push(`${where}: ${why}: "${s.length > 200 ? s.slice(0, 200) + '…' : s}"`);
  const domainOf = (s) =>
    catalog.tools.find((t) => t.domain === /geoprims\.com\/([a-z]+)\/\)/.exec(s)?.[1])?.domain ?? Object.keys(DOMAIN_WORDS).find((d) => DOMAIN_WORDS[d].test(s));
  const toolsIn = (d) => catalog.tools.filter((t) => !d || t.domain === d);
  for (const s of sentences(text)) {
    const d = domainOf(s);
    const tools = toolsIn(d);
    const ops = tools.filter((t) => t.composedOf.length === 0).length;
    // Counts of what is built: "135 operations are built", "has 135 operations and 170 tool ids".
    for (const m of s.matchAll(/\b(\d[\d,]*) operations (?=and\b|are built|built\b)/g)) {
      if (num(m[1]) !== ops) fail(s, `says ${m[1]} operations, the build has ${ops}`);
    }
    for (const m of s.matchAll(/\b(\d[\d,]*) tool ids\b/g)) {
      if (num(m[1]) !== tools.length) fail(s, `says ${m[1]} tool ids, the build has ${tools.length}`);
    }
    const stable = tools.filter((t) => t.stability === 'stable').length;
    if (/\b(all|every)\b[^.]{0,200}\bexperimental\b|\bnone has met the stable\b/i.test(s) && stable > 0 && !/\bstill\b|\brest\b/i.test(s)) {
      fail(s, `claims everything is experimental, but ${stable} ${d ?? ''} tools are stable`.replace('  ', ' '));
    }
    if (/\b(every|all)\b (tools?|calculators?|results?)\b[^.]{0,200}\b(checked|verified|tested|validated) against\b/i.test(s)) {
      fail(s, 'claims every tool is checked against a reference; only some have differential suites');
    }
    if (/\bagainst\b/i.test(s)) {
      for (const [ref, files] of Object.entries(REFERENCES)) {
        if (s.includes(ref) && !files.every((f) => existsSync(join(root, f)))) fail(s, `names ${ref}, but its differential suite is missing`);
      }
    }
    // Practitioner review, not a source's "Reviewed 2026-09-18" check date.
    if (/\breviewed by\b|\bsigned off\b|\bpractitioner[- ]reviewed\b/i.test(s) && !NEGATION.test(s)) {
      const named = signoffs.records.filter((r) => s.includes(r.reviewer));
      if (!named.length) fail(s, 'implies a practitioner review that docs/review-signoffs.md does not record');
    }
    if (/\bworks offline\b|\boffline[- ]ready\b|\bavailable offline\b/i.test(s) && !offline) {
      fail(s, 'claims offline use, but the build ships no service worker');
    }
  }
  return problems;
}
