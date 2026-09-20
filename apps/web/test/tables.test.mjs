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
  for (const cell of ['34000 ft', '230 deg', '119 kt', '-60 degC']) {
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
