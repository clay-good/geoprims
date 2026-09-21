// A list of rows in the answer card (web/tool-app "Result panel"). A decoder's
// periods, a lookup's matches, and a forecast's winds are the answer, not a
// count of it. A long list stays a count, because a thousand cells is a
// download rather than a table.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { MAX_COLUMNS, MAX_ROWS, cellText, rowTables } from '../src/lib/rows.js';

const web = new URL('..', import.meta.url).pathname;
const root = join(web, '../..');
const dist = join(web, 'dist');
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const route = (id) => '/' + id.split('.').join('/') + '/';
const page = (r) => readFileSync(join(dist, r, 'index.html'), 'utf8');
const tables = (html) => [...html.matchAll(/<div class="table-scroll rows">([\s\S]*?)<\/table>/g)].map((m) => m[1]);
const text = (s) => s.replace(/<[^>]+>/g, ' ').replace(/&#39;/g, "'").replace(/&deg;/g, '°').replace(/\s+/g, ' ');

test('a forecast shows its periods, not how many there are', () => {
  const html = page(route('aviation.weather.taf-decode'));
  const [t] = tables(html);
  assert.ok(t, 'the TAF decoder shows no periods');
  const body = text(t);
  for (const heading of ['Change', 'From', 'Wind', 'Visibility', 'Flight category']) {
    assert.ok(body.includes(heading), `the periods table has no ${heading} column`);
  }
  assert.ok(body.includes('300°'), 'the first period’s wind is not shown');
});

test('winds aloft show the level, direction, speed, and temperature', () => {
  const [t] = tables(page(route('aviation.weather.fb-winds-decode')));
  assert.ok(t, 'the winds-aloft decoder shows no winds');
  const body = text(t);
  for (const cell of ['34,000 ft', '230 deg', '119 kt', '-60 degC']) {
    assert.ok(body.includes(cell), `the table does not show ${cell}`);
  }
});

test('a long list stays a count, and a wide row is not squeezed into a table', () => {
  // No worked example is this big, so the rule is checked where it lives.
  const rows = (n) => Array.from({ length: n }, (_, i) => ({ cell: `8928308280fffff-${i}` }));
  assert.equal(rowTables({ ok: true, result: { cells: rows(MAX_ROWS) } }).length, 1, 'the cap itself is shown');
  assert.deepEqual(rowTables({ ok: true, result: { cells: rows(MAX_ROWS + 1) } }), [], 'one row over the cap');
  const wide = Object.fromEntries(Array.from({ length: MAX_COLUMNS + 1 }, (_, i) => [`c${i}`, i]));
  assert.deepEqual(rowTables({ ok: true, result: { wide: [wide] } }), [], 'too many columns to read');
  assert.equal(rowTables({ ok: true, result: { ok: [Object.fromEntries(Array.from({ length: MAX_COLUMNS }, (_, i) => [`c${i}`, i]))] } }).length, 1);
});

test('a quantity reads with its unit, and a failed result has no tables', () => {
  const r = { ok: true, result: { winds: [{ speed: { value: 119, unit: 'kt' }, text: 'fast' }] } };
  const [t] = rowTables(r);
  assert.deepEqual(t.columns, ['speed', 'text']);
  assert.equal(cellText(t.rows[0].speed), '119 kt');
  assert.equal(cellText(t.rows[0].text), 'fast');
  assert.equal(cellText(null), '');
  // A nested object is not a cell, so that list is not a table.
  assert.deepEqual(rowTables({ ok: true, result: { deep: [{ a: { b: { c: 1 } } }] } }), []);
  assert.deepEqual(rowTables({ ok: false, error: { code: 'X' } }), []);
  assert.deepEqual(rowTables(null), []);
});

test('a tool with no list of rows shows no table', () => {
  assert.deepEqual(tables(page(route('aviation.altimetry.density-altitude'))), []);
});

test('every rendered table names its columns from the manifest', () => {
  const problems = [];
  for (const t of catalog.tools) {
    for (const body of tables(page(route(t.id)))) {
      const caption = /<caption>([^<]*)<\/caption>/.exec(body)?.[1];
      if (!caption) problems.push(`${t.id}: a table with no caption`);
      const headings = [...body.matchAll(/<th scope="col">([^<]*)<\/th>/g)].map((m) => m[1]);
      if (headings.length === 0) problems.push(`${t.id}: a table with no column headings`);
      // A heading that is still the raw field name means the manifest lacks a title.
      for (const h of headings) {
        if (/^[a-z][a-z0-9_]*$/.test(h)) problems.push(`${t.id}: column "${h}" has no title in the manifest`);
      }
    }
  }
  assert.deepEqual(problems, []);
});

test('a table cell reads to the precision its column declares', async () => {
  const { cellText, formatNumber } = await import('../src/lib/rows.js');
  // The traverse's adjusted points: three decimals, grouped, not 13.
  assert.equal(cellText({ value: 5299.974325199917, unit: 'ft' }, { 'x-display-precision': { decimals: 3 } }), '5,299.974 ft');
  assert.equal(cellText(2, { 'x-display-precision': { decimals: 0 } }), '2');
  assert.equal(formatNumber(-1234.5, { decimals: 1 }), '-1,234.5');
  assert.equal(formatNumber(2436, { decimals: 0, grouping: false }), '2436');
  assert.equal(formatNumber(0.000123456, { significant: 3 }), '0.000123');
  // A column that declares nothing prints what the core gave.
  assert.equal(cellText({ value: 1.23456789, unit: 'm' }), '1.23456789 m');
  assert.equal(cellText('N 0°00\'13" W'), 'N 0°00\'13" W');
});

test('every table cell on a page reads exactly as its column declares', async () => {
  // Each cell the page printed is compared with what the column's own
  // precision gives for the same value, so float noise cannot slip through
  // and a coordinate declared to nine decimals keeps its nine.
  const { nodeHost } = await import('../../../packages/runtime/src/node.mjs');
  const host = nodeHost(join(root, 'dist/wasm'));
  const unescape = (x) => x.replace(/&#39;/g, "'").replace(/&quot;/g, '"').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>');
  const problems = [];
  let cells = 0;
  for (const t of catalog.tools) {
    const shown = tables(page(route(t.id)));
    if (!shown.length) continue;
    const ex = t.examples.find((e) => e.id === t['x-primary-example']) ?? t.examples[0];
    const result = JSON.parse(await host.invoke(t.id, JSON.stringify(ex.input)));
    const expected = rowTables(result).flatMap((table) =>
      table.rows.flatMap((row) => table.columns.map((c) => String(cellText(row[c], t.outputs.properties[table.key]?.items?.properties?.[c])))),
    );
    const printed = shown.flatMap((html) => [...html.matchAll(/<td>([^<]*)<\/td>/g)].map((m) => unescape(m[1])));
    cells += printed.length;
    if (JSON.stringify(printed) !== JSON.stringify(expected)) problems.push(`${t.id}: ${printed.slice(0, 3).join(' | ')} vs ${expected.slice(0, 3).join(' | ')}`);
  }
  assert.deepEqual(problems, []);
  assert.ok(cells > 50, `only ${cells} table cells checked`);
});
