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
      const r = await c.call('geoprims_run', { id, args: v.input });
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
