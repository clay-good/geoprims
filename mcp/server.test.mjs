// End-to-end tests of the MCP server over stdio (run `npm run build` first).
import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { copyFileSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, before, test } from 'node:test';
import assert from 'node:assert/strict';
import { nodeHost } from '../packages/runtime/src/node.mjs';
import { TOOLS } from './meta.mjs';
import { directName } from './toolsets.mjs';

const here = new URL('.', import.meta.url).pathname;
const root = join(here, '..');

class Client {
  constructor(args = []) {
    this.proc = spawn(process.execPath, [join(here, 'server.mjs'), ...args], { stdio: ['pipe', 'pipe', 'pipe'] });
    this.seq = 0;
    this.waiting = new Map();
    this.stderr = '';
    let buf = '';
    this.proc.stdout.setEncoding('utf8');
    this.proc.stdout.on('data', (d) => {
      buf += d;
      let nl;
      while ((nl = buf.indexOf('\n')) >= 0) {
        const msg = JSON.parse(buf.slice(0, nl));
        buf = buf.slice(nl + 1);
        this.waiting.get(msg.id)?.(msg);
        this.waiting.delete(msg.id);
      }
    });
    this.proc.stderr.on('data', (d) => (this.stderr += d));
  }
  raw(line, id) {
    return new Promise((resolve) => {
      this.waiting.set(id, resolve);
      this.proc.stdin.write(line + '\n');
    });
  }
  request(method, params) {
    const id = ++this.seq;
    return this.raw(JSON.stringify({ jsonrpc: '2.0', id, method, params }), id);
  }
  async call(name, args) {
    const r = await this.request('tools/call', { name, arguments: args });
    assert.ok(r.result, `tools/call ${name} failed: ${JSON.stringify(r)}`);
    return r.result;
  }
  close() {
    this.proc.stdin.end();
  }
}

let c;
before(() => {
  c = new Client();
});
after(() => c.close());

test('negotiates protocol versions', async () => {
  for (const v of ['2025-06-18', '2025-11-25', '2026-07-28']) {
    const r = await c.request('initialize', { protocolVersion: v, capabilities: {}, clientInfo: { name: 'test', version: '1' } });
    assert.equal(r.result.protocolVersion, v);
    assert.equal(r.result.serverInfo.name, 'geoprims');
  }
  const old = await c.request('initialize', { protocolVersion: '2024-11-05', capabilities: {} });
  assert.equal(old.result.protocolVersion, '2025-11-25');
  const d = await c.request('server/discover', {});
  assert.deepEqual(d.result.supportedVersions, ['2026-07-28', '2025-11-25', '2025-06-18']);
});

test('lists the meta-tools in order, annotated, within the token budget', async () => {
  const r = await c.request('tools/list', {});
  assert.deepEqual(
    r.result.tools.map((t) => t.name),
    ['geoprims_search', 'geoprims_describe', 'geoprims_run', 'geoprims_pipeline', 'geoprims_convert_units', 'geoprims_report_problem'],
  );
  for (const t of r.result.tools) {
    assert.deepEqual(
      [t.annotations.readOnlyHint, t.annotations.destructiveHint, t.annotations.idempotentHint, t.annotations.openWorldHint],
      [true, false, true, false],
    );
    assert.ok(t.title && t.outputSchema && t.inputSchema);
  }
  const chars = JSON.stringify(r.result).length;
  assert.ok(chars / 4 <= 6000, `tools/list is ~${Math.ceil(chars / 4)} tokens`);
});

test('golden surface file matches (UPDATE_SURFACE=1 to regenerate)', async () => {
  const surface = {
    tools: (await c.request('tools/list', {})).result.tools,
    resources: (await c.request('resources/list', {})).result.resources,
    resourceTemplates: (await c.request('resources/templates/list', {})).result.resourceTemplates,
    prompts: (await c.request('prompts/list', {})).result.prompts,
  };
  const file = join(here, 'surface.json');
  const text = JSON.stringify(surface, null, 2) + '\n';
  if (process.env.UPDATE_SURFACE) writeFileSync(file, text);
  assert.equal(text, readFileSync(file, 'utf8'), 'the MCP surface changed; review and run with UPDATE_SURFACE=1');
});

test('search hides experimental tools unless asked', async () => {
  const hidden = await c.call('geoprims_search', { query: 'knots to mph' });
  assert.ok(hidden.structuredContent.result.results.every((r) => r.stability === 'stable'));
  assert.ok(!hidden.structuredContent.result.results.some((r) => r.id === 'units.speed.kt-to-mph'));
  assert.ok(hidden.structuredContent.result.hiddenExperimental > 0);
  const shown = await c.call('geoprims_search', { query: 'knots to mph', includeExperimental: true });
  assert.equal(shown.structuredContent.result.results[0].id, 'units.speed.kt-to-mph');
  const typo = await c.call('geoprims_search', { query: 'fahrenhiet to celsius', includeExperimental: true });
  assert.equal(typo.structuredContent.result.results[0].id, 'units.temperature.f-to-c');
});

test('run returns byte-identical results to the runtime', async () => {
  const r = await c.call('geoprims_run', { id: 'units.speed.kt-to-mph', args: { value: 100 } });
  const direct = await nodeHost(join(root, 'dist/wasm')).invoke('units.speed.kt-to-mph', '{"value":100}');
  assert.equal(r.content[0].text, direct);
  assert.equal(JSON.stringify(r.structuredContent), direct);
  assert.equal(r.structuredContent.result.converted.value, 115.07794480235425);
  assert.equal(r.structuredContent.summary, '100 kt is 115.07794 mph.');
  assert.equal(r.isError, undefined);
});

test('run with no args runs the worked example; units selects a profile', async () => {
  const r = await c.call('geoprims_run', { id: 'units.fuel.convert' });
  assert.equal(r.structuredContent.result.mass.value, 300);
  const si = await c.call('geoprims_run', { id: 'units.fuel.convert', units: 'si' });
  assert.equal(si.structuredContent.result.mass.unit, 'kg');
});

test('wrong id is a recoverable error with suggestions', async () => {
  const r = await c.call('geoprims_run', { id: 'units.kt-to-mph', args: { value: 1 } });
  assert.equal(r.isError, true);
  assert.equal(r.structuredContent.error.code, 'UNSUPPORTED');
  assert.ok(r.structuredContent.error.suggestions.includes('units.speed.kt-to-mph'), JSON.stringify(r.structuredContent));
  const bad = await c.call('geoprims_run', { id: 'units.speed.convert', args: { value: '1 ft', to: 'mph' } });
  assert.equal(bad.isError, true);
  assert.equal(bad.structuredContent.error.code, 'UNIT_MISMATCH');
});

test('describe at each detail level, with unknown ids flagged', async () => {
  const s = await c.call('geoprims_describe', { ids: ['units.speed.convert'], detail: 'summary' });
  assert.equal(s.structuredContent.result.tools[0].inputs, undefined);
  const sc = await c.call('geoprims_describe', { ids: ['units.speed.convert', 'nope.x.y'], detail: 'schema' });
  const [tool, missing] = sc.structuredContent.result.tools;
  assert.equal(tool.inputs.properties.value['x-quantity'], 'speed');
  assert.ok(tool.references.length > 0);
  assert.equal(missing.error.code, 'UNSUPPORTED');
  const ex = await c.call('geoprims_describe', { ids: ['units.speed.convert'], detail: 'examples' });
  assert.ok(ex.structuredContent.result.tools[0].examples.length > 0);
  const tooMany = await c.call('geoprims_describe', { ids: Array(21).fill('x') });
  assert.equal(tooMany.isError, true);
});

test('pipeline chains with unit-carrying bindings and rejects forward references', async () => {
  const r = await c.call('geoprims_pipeline', {
    steps: [
      { id: 'units.speed.convert', args: { value: 100, to: 'm/s' } },
      { id: 'units.speed.convert', args: { to: 'mph' }, bind: { '/value': '0:/result/converted' } },
    ],
  });
  assert.equal(r.structuredContent.ok, true);
  const mph = r.structuredContent.result.steps[1].result.converted.value;
  assert.ok(Math.abs(mph - 115.07794480235425) < 1e-12, String(mph));
  const fwd = await c.call('geoprims_pipeline', {
    steps: [{ id: 'units.speed.convert', args: { to: 'mph' }, bind: { '/value': '1:/result/converted' } }, { id: 'units.speed.convert', args: { value: 1, to: 'kt' } }],
  });
  assert.equal(fwd.isError, true);
  assert.match(fwd.structuredContent.error.message, /has not run yet/);
  const failing = await c.call('geoprims_pipeline', { steps: [{ id: 'units.speed.convert', args: { value: '1 ft', to: 'mph' } }] });
  assert.equal(failing.structuredContent.error.step, 0);
});

test('convert_units finds the quantity from the units', async () => {
  const a = await c.call('geoprims_convert_units', { value: 100, from: 'kt', to: 'mph' });
  assert.equal(a.structuredContent.result.converted.value, 115.07794480235425);
  const b = await c.call('geoprims_convert_units', { value: '29.92 inHg', to: 'hPa' });
  assert.equal(b.structuredContent.result.converted.unit, 'hPa');
  const t = await c.call('geoprims_convert_units', { value: 15, from: 'degC', to: 'degF' });
  assert.equal(t.structuredContent.result.converted.value, 59);
  const bad = await c.call('geoprims_convert_units', { value: 1, from: 'kt', to: 'ft' });
  assert.equal(bad.structuredContent.error.code, 'UNIT_MISMATCH');
});

test('protocol errors', async () => {
  assert.equal((await c.request('tools/call', { name: 'geoprims_nope', arguments: {} })).error.code, -32602);
  assert.equal((await c.request('bogus/method', {})).error.code, -32601);
  assert.equal((await c.raw('{not json', null)).error.code, -32700);
  const extra = await c.call('geoprims_search', { query: 'x', color: 'red' });
  assert.equal(extra.isError, true);
});

test('resources: catalog index and a tool manifest with vectors', async () => {
  const cat = await c.request('resources/read', { uri: 'geoprims://catalog' });
  const body = JSON.parse(cat.result.contents[0].text);
  assert.ok(body.tools.some((t) => t.id === 'units.speed.kt-to-mph'));
  const tool = JSON.parse((await c.request('resources/read', { uri: 'geoprims://tool/units.speed.kt-to-mph' })).result.contents[0].text);
  assert.ok(tool.accuracy && tool.references.length && tool.examples.length && tool.vectors.length >= 3);
  assert.equal((await c.request('resources/read', { uri: 'geoprims://tool/nope' })).error.code, -32002);
});

test('opens no listening socket', { skip: spawnSync('lsof', ['-v']).error ? 'lsof not available' : false }, async () => {
  await c.request('ping', {});
  const out = spawnSync('lsof', ['-a', '-p', String(c.proc.pid), '-i']).stdout.toString();
  assert.equal(out.trim(), '', out);
});

test('an untagged checkout without built files explains itself and exits 1', () => {
  const dir = mkdtempSync(join(tmpdir(), 'gp-mcp-'));
  for (const f of ['server.mjs', 'meta.mjs', 'toolsets.mjs', 'prompts.mjs', 'package.json']) copyFileSync(join(here, f), join(dir, f));
  const r = spawnSync(process.execPath, [join(dir, 'server.mjs')], { input: '' });
  assert.equal(r.status, 1);
  assert.equal(r.stderr.toString().trim(), 'Built files missing: check out a release tag (git checkout vX.Y.Z) or run npm run build (requires Rust)');
});

test('unknown options fail fast; an unknown toolset lists the valid ones', () => {
  const r = spawnSync(process.execPath, [join(here, 'server.mjs'), '--toolsets=bogus'], { input: '' });
  assert.equal(r.status, 2);
  assert.match(String(r.stderr), /Unknown toolset bogus\. Valid toolsets: geodesy-core, navigation, e6b, atmosphere, drone-mapping, survey-cogo, indexing\./);
  assert.equal(spawnSync(process.execPath, [join(here, 'server.mjs'), '--wat'], { input: '' }).status, 2);
  assert.equal(spawnSync(process.execPath, [join(here, 'server.mjs'), '--no-meta'], { input: '' }).status, 2);
});

test('direct toolsets list stable tools beside the meta-tools, and run like geoprims_run', async () => {
  const d = new Client(['--toolsets=geodesy-core,e6b']);
  try {
    const names = (await d.request('tools/list', {})).result.tools.map((t) => t.name);
    assert.deepEqual(names.slice(0, 6), TOOLS.map((t) => t.name));
    const direct = names.slice(6);
    assert.ok(direct.includes('gp_geodesy_utm_forward') && direct.length <= 80, names.join());
    assert.ok(direct.every((n) => /^gp_[a-zA-Z0-9_-]{1,61}$/.test(n)));
    const args = { lat: 40, lon: -105 };
    const r = await d.call('gp_geodesy_utm_forward', args);
    const viaRun = await d.call('geoprims_run', { id: 'geodesy.utm.forward', args });
    assert.equal(r.content[0].text, viaRun.content[0].text);
    const bad = await d.call('gp_geodesy_utm_forward', { lat: 95, lon: 0 });
    assert.equal(bad.isError, true);
    assert.equal(bad.structuredContent.error.field, '/lat');
  } finally {
    d.close();
  }
  const only = new Client(['--toolsets=navigation', '--no-meta']);
  try {
    const names = (await only.request('tools/list', {})).result.tools.map((t) => t.name);
    assert.ok(names.length > 0 && names.every((n) => n.startsWith('gp_navigation_') || n.startsWith('gp_geometry_')), names.join());
    assert.equal((await only.request('tools/call', { name: 'geoprims_search', arguments: { query: 'x' } })).error.code, -32602);
  } finally {
    only.close();
  }
});

test('long direct names shorten deterministically under 64 characters', () => {
  const id = 'aviation.performance.a-very-long-operation-name-that-keeps-going-past-the-limit';
  const n = directName(id);
  assert.equal(n.length, 64);
  assert.equal(n, directName(id));
  assert.notEqual(n, directName(id + 'x'));
});

test('has zero runtime dependencies', () => {
  const pkg = JSON.parse(readFileSync(join(here, 'package.json'), 'utf8'));
  assert.deepEqual(pkg.dependencies, {});
  assert.equal(execFileSync('git', ['ls-files', 'mcp/node_modules'], { cwd: root, encoding: 'utf8' }), '');
});

test('a 4-step pipeline carries units end to end', async () => {
  const r = await c.call('geoprims_pipeline', {
    steps: [
      { id: 'units.length.convert', args: { value: '1 NM', to: 'm' } },
      { id: 'units.length.convert', args: { to: 'ft' }, bind: { '/value': '0:/result/converted' } },
      { id: 'units.length.convert', args: { to: 'mi' }, bind: { '/value': '1:/result/converted' } },
      { id: 'units.length.convert', args: { to: 'NM' }, bind: { '/value': '2:/result/converted' } },
    ],
  });
  assert.equal(r.structuredContent.ok, true);
  const nm = r.structuredContent.result.steps[3].result.converted.value;
  assert.ok(Math.abs(nm - 1) < 1e-15, String(nm));
});

test('convert_units agrees with every units golden vector', async () => {
  const dir = join(root, 'core/vectors');
  let n = 0;
  for (const f of readdirSync(dir).filter((x) => /^units\.[a-z-]+\.convert\.jsonl$/.test(x) && !x.includes('temperature-difference'))) {
    for (const line of readFileSync(join(dir, f), 'utf8').split('\n').filter(Boolean)) {
      const v = JSON.parse(line);
      if (!('to' in v.input) || !('result.converted.value' in v.expect)) continue;
      const r = await c.call('geoprims_convert_units', { value: v.input.value, to: v.input.to });
      const want = v.expect['result.converted.value'];
      const got = r.structuredContent.result?.converted?.value;
      assert.ok(Math.abs(got - want) <= v.tolerance['result.converted.value'].rel * Math.abs(want), `${f} ${v.id}: ${JSON.stringify(r.structuredContent)}`);
      n++;
    }
  }
  assert.ok(n > 80, `${n} vectors`);
});

test('report_problem prepares a payload and link without sending anything', async () => {
  const r = await c.call('geoprims_report_problem', {
    toolId: 'units.fuel.convert',
    args: { volume: 50, fuel: 'avgas-100ll', options: { outputUnits: { mass: 'kg' } } },
    observed: '136.08 kg',
    expected: '136.08 kg is right; testing',
    source: 'FAA-H-8083-25C',
  });
  assert.equal(r.structuredContent.ok, true, JSON.stringify(r.structuredContent));
  const { link, issueUrl, payload } = r.structuredContent.result;
  assert.match(link, /^https:\/\/geoprims\.com\/units\/fuel\/convert\/#v1:[A-Za-z0-9_-]+;report$/);
  assert.match(issueUrl, /template=wrong-answer\.yml$/);
  const limits = JSON.parse(readFileSync(join(root, 'data/report-limits.json'), 'utf8'));
  assert.deepEqual(Object.keys(payload), ['apiVersion', 'toolId', 'toolVersion', 'coreVersion', 'buildHash', 'assetVersions', 'kind', 'pagePath', 'inputs', 'outputs', 'warnings', 'display', 'note', 'token']);
  assert.equal(payload.kind, 'wrong-result');
  assert.equal(payload.token, null);
  assert.deepEqual(payload.inputs, [
    { field: 'volume', label: 'Volume', value: '50', unit: 'galUS' },
    { field: 'fuel', label: 'Fuel type', value: 'avgas-100ll', unit: '' },
  ]);
  assert.equal(payload.outputs.find((o) => o.field === 'mass').unit, 'kg');
  assert.ok(payload.warnings.includes('NOMINAL_VALUE_USED'));
  assert.ok(payload.note.length <= limits.noteChars && payload.note.startsWith('Observed: 136.08 kg'));
  // The link's fragment decodes back to the inputs.
  const frag = link.split('#')[1];
  const { nodeHost: nh } = await import('../packages/runtime/src/node.mjs');
  const link2 = await nh(join(root, 'dist/wasm')).module('link');
  const back = JSON.parse(await link2.callString('gp_link_decode', frag));
  assert.deepEqual(back.result.state, { i: { fuel: 'avgas-100ll', volume: 50 }, u: { mass: 'kg' } });
  assert.deepEqual(back.result.flags, ['report']);
});

test('report_problem truncates long notes to the shared limit', async () => {
  const r = await c.call('geoprims_report_problem', { toolId: 'units.speed.convert', args: { value: 1, to: 'mph' }, observed: 'x'.repeat(1000) });
  assert.equal(r.structuredContent.result.payload.note.length, 280);
  const bad = await c.call('geoprims_report_problem', { toolId: 'nope.x.y', args: {}, observed: 'x' });
  assert.equal(bad.structuredContent.error.code, 'UNSUPPORTED');
});

test('every golden vector gives the same bytes through the server as the runtime', async () => {
  // mcp "Same result as the website": the site and the server share one core.
  const host = nodeHost(join(root, 'dist/wasm'));
  const dir = join(root, 'core/vectors');
  let n = 0;
  for (const file of readdirSync(dir).filter((f) => f.endsWith('.jsonl'))) {
    const id = file.replace(/\.jsonl$/, '');
    for (const line of readFileSync(join(dir, file), 'utf8').split('\n').filter(Boolean)) {
      const v = JSON.parse(line);
      if (v.supersededBy) continue;
      const r = await c.call('geoprims_run', { id, args: v.input, output: { maxItems: 10000 } });
      assert.equal(r.content[0].text, await host.invoke(id, JSON.stringify(v.input)), `${id} ${v.id}`);
      n++;
    }
  }
  assert.ok(n > 1000, `ran ${n} vectors`);
});

test('operational results carry their caveats in meta', async () => {
  const r = await c.call('geoprims_run', { id: 'geodesy.magnetic.declination', args: { lat: 40, lon: -105, date: '2026-09-19' } });
  const m = r.structuredContent.meta;
  assert.equal(m.model, 'WMM2025 main field, degree 12');
  assert.ok(m.accuracy);
  assert.deepEqual(Object.keys(m.context), ['model', 'epoch', 'validFrom', 'validTo', 'declinationUncertaintyDeg', 'compassZone']);
  assert.equal(m.notice, 'Planning and education aid. Not for primary navigation.');
  const isa = await c.call('geoprims_run', { id: 'aviation.atmosphere.isa', args: { altitude: '5000 ft' } });
  assert.equal(isa.structuredContent.meta.notice, m.notice);
  const kt = await c.call('geoprims_run', { id: 'units.speed.kt-to-mph', args: { value: 1 } });
  assert.equal(kt.structuredContent.meta.notice, undefined);
});

test('each workflow prompt produces a pipeline that runs end to end', async () => {
  const examples = {
    'preflight-performance': { elevation: '5000 ft', altimeter: '29.80 inHg', temperature: '30 degC', runway: '27', wind_direction: '300 deg', wind_speed: '15 kt', max_crosswind: '15 kt' },
    'photogrammetry-mission': { target_gsd: '2 cm', sensor_width: '13.2 mm', sensor_height: '8.8 mm', focal_length: '8.8 mm', image_width: '5472', image_height: '3648', groundspeed: '10 m/s', front_overlap: '75', side_overlap: '65' },
    'traverse-closure': { courses: JSON.stringify([{ direction: '0', distance: 300 }, { direction: '90', distance: 400.02 }, { direction: '180', distance: 299.95 }, { direction: '270.01', distance: 400 }]) },
    'coordinate-conversion-audit': { coordinate: `40°26'46"N 79°58'56"W` },
    'h3-resolution-choice': { target_area: '1 km2', lat: '40.4461', lon: '-79.9822' },
  };
  const list = (await c.request('prompts/list', {})).result.prompts;
  assert.deepEqual(list.map((p) => p.name), Object.keys(examples));
  for (const [name, args] of Object.entries(examples)) {
    const got = (await c.request('prompts/get', { name, arguments: args })).result;
    const text = got.messages[0].content.text;
    const call = JSON.parse(/```json\n([\s\S]*?)\n```/.exec(text)[1]);
    const r = await c.call('geoprims_pipeline', call);
    assert.equal(r.structuredContent.ok, true, `${name}: ${r.content[0].text}`);
    assert.equal(r.structuredContent.result.steps.length, call.steps.length);
  }
  const audit = (await c.call('geoprims_pipeline', JSON.parse(/```json\n([\s\S]*?)\n```/.exec((await c.request('prompts/get', { name: 'coordinate-conversion-audit', arguments: examples['coordinate-conversion-audit'] })).result.messages[0].content.text)[1]))).structuredContent;
  assert.ok(Math.abs(audit.result.steps[2].result.lat.value - 40.446111) < 1e-6);
  assert.equal((await c.request('prompts/get', { name: 'h3-resolution-choice', arguments: { lat: '1' } })).error.code, -32602);
  assert.equal((await c.request('prompts/get', { name: 'nope', arguments: {} })).error.code, -32602);
});

test('large polyfill: one page of cells with the total, covered area, and bounds', async () => {
  // mcp "Large polyfill": a county-sized square at resolution 10.
  const points = [{ lat: 40.0, lon: -80.35 }, { lat: 40.0, lon: -79.55 }, { lat: 40.5, lon: -79.55 }, { lat: 40.5, lon: -80.35 }];
  const r = (await c.call('geoprims_run', { id: 'indexing.h3.polygon-to-cells', args: { points, resolution: 10 } })).structuredContent;
  assert.equal(r.ok, true, JSON.stringify(r.error));
  assert.equal(r.result.cells.length, 1000);
  assert.ok(r.result.count > 240000, String(r.result.count));
  assert.deepEqual(r.page.cells, { total: r.result.count, offset: 0, returned: 1000, truncated: true });
  assert.ok(r.result.area.value > 3700 && r.result.area.value < 3800, String(r.result.area.value));
  assert.ok(r.result.south.value < 40 && r.result.north.value > 40.5 && r.result.west.value < -80.35 && r.result.east.value > -79.55);
  const last = (await c.call('geoprims_run', { id: 'indexing.h3.polygon-to-cells', args: { points, resolution: 10 }, output: { maxItems: 10000, offset: r.result.count - 5 } })).structuredContent;
  assert.deepEqual(last.page.cells, { total: r.result.count, offset: r.result.count - 5, returned: 5, truncated: false });
});

test('other long lists are sliced by the server with the same page fields', async () => {
  const args = { cell: '892a8471487ffff', resolution: 13 };
  const all = (await c.call('geoprims_run', { id: 'indexing.h3.children', args, output: { maxItems: 10000 } })).structuredContent;
  assert.equal(all.page, undefined);
  const n = all.result.cells.length;
  const tail = (await c.call('geoprims_run', { id: 'indexing.h3.children', args, output: { offset: 2000 } })).structuredContent;
  assert.deepEqual(tail.page.cells, { total: n, offset: 2000, returned: n - 2000, truncated: false });
  assert.deepEqual(tail.result.cells, all.result.cells.slice(2000));
  const bad = (await c.call('geoprims_run', { id: 'indexing.h3.children', args, output: { maxItems: 0 } })).structuredContent;
  assert.equal(bad.error.field, '/output/maxItems');
});

test('search then describe then run', async () => {
  // mcp "Search then run": each call succeeds and the run carries model and accuracy.
  const s = (await c.call('geoprims_search', { query: 'density altitude', includeExperimental: true })).structuredContent;
  const id = s.result.results[0].id;
  assert.equal(id, 'aviation.altimetry.density-altitude');
  const d = (await c.call('geoprims_describe', { ids: [id], detail: 'schema' })).structuredContent;
  assert.ok(d.result.tools[0].inputs.required.includes('elevation'));
  const r = (await c.call('geoprims_run', { id, args: { elevation: '5000 ft', altimeter: '29.80 inHg', temperature: '30 degC' } })).structuredContent;
  assert.equal(r.ok, true);
  assert.ok(r.meta.model && r.meta.accuracy);
});

test('every setup snippet launches the server; README and package agree', async () => {
  const { CLIENTS, NPX_PACKAGE, parseSnippet, snippet } = await import('./clients.mjs');
  const readme = readFileSync(join(here, 'README.md'), 'utf8');
  const pkg = JSON.parse(readFileSync(join(here, 'package.json'), 'utf8'));
  assert.equal(pkg.name, NPX_PACKAGE);
  assert.ok(pkg.bin['geoprims-mcp']);
  // mcp "VS Code key": VS Code reads "servers", the others "mcpServers".
  const vscode = JSON.parse(snippet(CLIENTS.find((x) => x.id === 'vscode')));
  assert.deepEqual(Object.keys(vscode), ['servers']);
  assert.equal(vscode.servers.geoprims.type, 'stdio');
  for (const client of CLIENTS) {
    assert.ok(readme.includes(snippet(client)), `README is missing the ${client.name} snippet`);
    assert.deepEqual(parseSnippet(client, snippet(client, 'npx')), { command: 'npx', args: ['-y', NPX_PACKAGE] });
    const { command, args } = parseSnippet(client, snippet(client, 'clone', root.replace(/\/$/, '')));
    const r = spawnSync(command, args, {
      input: JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {} } }) + '\n',
      encoding: 'utf8',
      timeout: 30_000,
    });
    assert.equal(JSON.parse(r.stdout.split('\n')[0]).result.serverInfo.name, 'geoprims', `${client.name}: ${r.stderr}`);
  }
});

test('an over-10 MB request is refused and the server keeps serving', async () => {
  const big = JSON.stringify({ jsonrpc: '2.0', id: 999, method: 'tools/call', params: { name: 'geoprims_run', arguments: { id: 'x', args: { pad: 'x'.repeat(10_500_000) } } } });
  const r = await c.raw(big, null);
  assert.equal(r.error.code, -32600);
  assert.match(r.error.message, /over 10000000 bytes/);
  const ok = await c.call('geoprims_run', { id: 'units.speed.kt-to-mph', args: { value: 1 } });
  assert.equal(ok.structuredContent.ok, true);
});

test('logs go to stderr and never include argument values', async () => {
  const marker = 'secret-marker-4d1f';
  await c.call('geoprims_run', { id: 'geodesy.parse.coordinates', args: { text: marker } });
  await c.call('geoprims_search', { query: marker });
  assert.match(c.stderr, /<- tools\/call geoprims_run/);
  assert.ok(!c.stderr.includes(marker), 'argument value leaked into the log');
});

test('search prefill matches the web palette and runs as-is', async () => {
  // natural-language-prefill "Surface parity": the same query gives the same
  // top result and prefill on both surfaces (one Wasm search module).
  const query = 'density altitude 5000 ft 30C 29.80';
  const s = (await c.call('geoprims_search', { query, includeExperimental: true })).structuredContent;
  const { workerHost } = await import('../packages/runtime/src/worker-host.mjs');
  const web = workerHost(join(root, 'dist/wasm'));
  await web.searchLoad(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
  const palette = JSON.parse(await web.search(JSON.stringify({ query, limit: 10, includeExperimental: true })));
  web.close();
  assert.deepEqual(s.result.results[0], palette.result.results[0]);
  const top = s.result.results[0];
  assert.deepEqual(top.prefill, { elevation: '5000 ft', altimeter: '29.80 inHg', temperature: '30 degC' });
  const r = (await c.call('geoprims_run', { id: top.id, args: top.prefill })).structuredContent;
  assert.equal(r.ok, true);
  assert.ok(Math.abs(r.result.density_altitude.value - 7932) < 1, String(r.result.density_altitude.value));
});
