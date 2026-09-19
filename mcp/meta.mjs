// The geoprims meta-tools (add-local-mcp-server, "Default meta-tool surface").
// Descriptions are static text: they never interpolate user input.

export const ANNOTATIONS = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false };

const envelopeSchema = {
  type: 'object',
  properties: { ok: { type: 'boolean' } },
  required: ['ok'],
};

export const TOOLS = [
  {
    name: 'geoprims_search',
    title: 'Search geoprims tools',
    description:
      'Find geoprims calculators (geodesy, navigation, aviation, drone, survey, indexing, time, units) by plain words, abbreviations, or tool id. Returns ranked ids with summaries. Numbers in the query fill inputs: prefill on the top result holds args for geoprims_run ("density altitude 5000 ft 30C 29.80"), and ambiguous lists values it would not guess. Experimental tools are hidden unless includeExperimental is true; hiddenExperimental counts them.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'What to calculate, e.g. "density altitude" or "knots to mph".' },
        domain: { type: 'string', description: 'Limit to one domain, e.g. "aviation".' },
        limit: { type: 'integer', minimum: 1, maximum: 50, default: 10 },
        includeExperimental: { type: 'boolean', default: false },
      },
      required: ['query'],
      additionalProperties: false,
    },
    outputSchema: envelopeSchema,
  },
  {
    name: 'geoprims_describe',
    title: 'Describe geoprims tools',
    description:
      'Get manifests for up to 20 tool ids. detail "summary" gives title and summary; "schema" adds input and output JSON Schemas, units, accuracy, model, references, warnings, and related tools; "examples" adds the worked example.',
    inputSchema: {
      type: 'object',
      properties: {
        ids: { type: 'array', items: { type: 'string' }, minItems: 1, maxItems: 20 },
        detail: { type: 'string', enum: ['summary', 'schema', 'examples'], default: 'schema' },
      },
      required: ['ids'],
      additionalProperties: false,
    },
    outputSchema: envelopeSchema,
  },
  {
    name: 'geoprims_run',
    title: 'Run a geoprims tool',
    description:
      'Run one tool by id. args follow the tool\'s input schema (see geoprims_describe); numbers may carry units as strings, like "145 kts". Omit args to run the worked example. units picks an output unit profile: si, aviation, aviation-hpa, us-customary, survey-metric, or survey-us. Lists longer than output.maxItems (default 1,000) come back one page at a time; page gives the total, offset, and truncated flag of each list. Results are planning aids, not certified for navigation; relay meta.warnings to the user.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string' },
        args: { type: 'object' },
        units: { type: 'string', enum: ['si', 'aviation', 'aviation-hpa', 'us-customary', 'survey-metric', 'survey-us'] },
        output: {
          type: 'object',
          properties: { maxItems: { type: 'integer', minimum: 1, maximum: 10000, default: 1000 }, offset: { type: 'integer', minimum: 0, default: 0 } },
          additionalProperties: false,
        },
      },
      required: ['id'],
      additionalProperties: false,
    },
    outputSchema: envelopeSchema,
  },
  {
    name: 'geoprims_pipeline',
    title: 'Run a chain of geoprims tools',
    description:
      'Run up to 20 tools in order without round trips. Each step is {id, args, bind}; bind maps an input JSON Pointer to "<earlierStepIndex>:<pointer into that step\'s result>", e.g. {"/value": "0:/result/converted"}. A bound {value, unit} is passed with its unit, so units convert automatically. Stops at the first failing step.',
    inputSchema: {
      type: 'object',
      properties: {
        steps: {
          type: 'array',
          minItems: 1,
          maxItems: 20,
          items: {
            type: 'object',
            properties: {
              id: { type: 'string' },
              args: { type: 'object' },
              bind: { type: 'object', additionalProperties: { type: 'string' } },
            },
            required: ['id'],
            additionalProperties: false,
          },
        },
      },
      required: ['steps'],
      additionalProperties: false,
    },
    outputSchema: envelopeSchema,
  },
  {
    name: 'geoprims_convert_units',
    title: 'Convert units',
    description:
      'Convert a value between units with exact definitions, e.g. {value: 100, from: "kt", to: "mph"}. value may carry its unit instead of from, like "29.92 inHg". Absolute temperatures are assumed; for differences such as ISA deviation, run units.temperature-difference.convert.',
    inputSchema: {
      type: 'object',
      properties: {
        value: { type: ['number', 'string'] },
        from: { type: 'string' },
        to: { type: 'string' },
      },
      required: ['value', 'to'],
      additionalProperties: false,
    },
    outputSchema: envelopeSchema,
  },
  {
    name: 'geoprims_report_problem',
    title: 'Prepare a problem report',
    description:
      'Prepare, but never send, a report about a geoprims result that looks wrong. Returns the payload, a geoprims.com link that reopens the tool with these inputs and the report form filled in, and a GitHub issue link. Nothing leaves this machine: show the link to the user so they can review and send it themselves.',
    inputSchema: {
      type: 'object',
      properties: {
        toolId: { type: 'string' },
        args: { type: 'object', description: 'The inputs that produced the result.' },
        observed: { type: 'string', description: 'The result that looks wrong.' },
        expected: { type: 'string', description: 'What you expected instead.' },
        source: { type: 'string', description: 'A published source for the expected value.' },
        note: { type: 'string' },
      },
      required: ['toolId', 'args', 'observed'],
      additionalProperties: false,
    },
    outputSchema: envelopeSchema,
  },
].map((t) => ({ ...t, annotations: { title: t.title, ...ANNOTATIONS } }));

const SITE = 'https://geoprims.com';
const ISSUE_URL = 'https://github.com/clay-good/geoprims/issues/new?template=wrong-answer.yml';

const cut = (s, n) => {
  const t = String(s ?? '');
  return t.length <= n ? t : t.slice(0, n - 1) + '…';
};

const fail = (code, message, extra = {}) => ({ ok: false, error: { code, message, ...extra } });

function schemaCheck(tool, args) {
  const s = tool.inputSchema;
  if (args === null || typeof args !== 'object' || Array.isArray(args)) return 'arguments must be an object';
  for (const k of Object.keys(args)) if (!(k in s.properties)) return `unknown argument ${k}`;
  for (const k of s.required) if (!(k in args)) return `missing argument ${k}`;
  return null;
}

/** Reads a JSON Pointer from an object. */
function getPointer(obj, ptr) {
  if (ptr === '' || ptr === '/') return obj;
  return ptr
    .split('/')
    .slice(1)
    .map((p) => p.replaceAll('~1', '/').replaceAll('~0', '~'))
    .reduce((o, k) => (o == null ? undefined : o[k]), obj);
}

function setPointer(obj, ptr, value) {
  const parts = ptr.split('/').slice(1).map((p) => p.replaceAll('~1', '/').replaceAll('~0', '~'));
  let o = obj;
  for (const p of parts.slice(0, -1)) o = o[p] ??= {};
  o[parts.at(-1)] = value;
}

/** The bounding box of list items that carry lat and lon (plain or {value, unit}), before slicing. */
function bounds(list) {
  const num = (v) => (typeof v === 'number' ? v : typeof v?.value === 'number' ? v.value : NaN);
  const pts = list.map((r) => [num(r?.lat), num(r?.lon)]).filter(([a, b]) => Number.isFinite(a) && Number.isFinite(b));
  if (pts.length !== list.length || !pts.length) return {};
  const lats = pts.map((p) => p[0]);
  const lons = pts.map((p) => p[1]);
  return { bbox: { south: Math.min(...lats), west: Math.min(...lons), north: Math.max(...lats), east: Math.max(...lons) } };
}

/** A bound quantity {value, unit} travels as a unit-tagged string, so the core converts it. */
const bindValue = (v) =>
  v && typeof v === 'object' && typeof v.value === 'number' && typeof v.unit === 'string' ? `${v.value} ${v.unit}` : v;

export function metaHandlers({ host, catalog, modules = [], limits }) {
  const byId = new Map(catalog.tools.map((t) => [t.id, t]));
  const run = async (id, args) => JSON.parse(await host.invoke(id, JSON.stringify(args)));

  // Output size control: a tool with offset and limit inputs pages at the
  // source (its count is the total); any other list is sliced here. Results
  // that fit come back unchanged.
  const paged = async (m, input, maxItems, offset) => {
    const props = m.inputs.properties ?? {};
    const atSource = 'offset' in props && 'limit' in props && !('offset' in input) && !('limit' in input);
    const out = await run(m.id, atSource ? { ...input, offset, limit: maxItems } : input);
    if (!out.ok || !out.result || typeof out.result !== 'object') return out;
    const page = {};
    for (const [k, list] of Object.entries(out.result)) {
      if (!Array.isArray(list)) continue;
      const total = atSource && typeof out.result.count === 'number' ? out.result.count : list.length;
      const shown = atSource ? list : list.slice(offset, offset + maxItems);
      const from = Math.min(offset, total);
      if (total <= maxItems && from === 0) continue;
      out.result[k] = shown;
      page[k] = { total, offset: from, returned: shown.length, truncated: from + shown.length < total, ...bounds(list) };
    }
    if (Object.keys(page).length) out.page = page;
    return out;
  };
  const searchRaw = async (req) => JSON.parse(await host.search(JSON.stringify(req)));
  const suggest = async (id) => {
    const out = await searchRaw({ query: id.replaceAll('.', ' ').replaceAll('-', ' '), limit: 3, includeExperimental: true });
    return out.ok ? out.result.results.map((r) => r.id) : [];
  };

  const describe = (m, detail) => {
    const base = { id: m.id, title: m.title, summary: m.summary, domain: m.domain, group: m.group, stability: m.stability, version: m.version };
    if (detail === 'summary') return base;
    const schema = {
      ...base,
      aliases: m.aliases,
      inputs: m.inputs,
      outputs: m.outputs,
      model: m.model,
      accuracy: m.accuracy,
      references: m.references,
      warnings: m.warnings,
      errors: m.errors,
      related: m.related,
      composedOf: m.composedOf,
      preset: m.preset,
      limits: m.limits,
      vectorCount: m.vectorCount,
    };
    if (detail === 'schema') return schema;
    return { ...schema, examples: m.examples, primaryExample: m['x-primary-example'] };
  };

  return {
    geoprims_search: async (a) => searchRaw(a),

    geoprims_describe: async ({ ids, detail = 'schema' }) => {
      if (!Array.isArray(ids) || ids.length < 1 || ids.length > 20) return fail('INVALID_INPUT', 'ids must list 1 to 20 tool ids.', { field: '/ids' });
      if (!['summary', 'schema', 'examples'].includes(detail)) return fail('INVALID_INPUT', 'detail must be summary, schema, or examples.', { field: '/detail' });
      const tools = [];
      for (const id of ids) {
        const m = byId.get(id);
        tools.push(m ? describe(m, detail) : { id, error: { code: 'UNSUPPORTED', message: `There is no tool with id "${id}".`, suggestions: await suggest(id) } });
      }
      return { ok: true, result: { tools } };
    },

    geoprims_run: async ({ id, args, units, output }) => {
      const m = byId.get(id);
      if (!m) {
        return fail('UNSUPPORTED', `There is no tool with id "${id}".`, {
          field: '/id',
          hint: 'Use geoprims_search to find the right id.',
          suggestions: await suggest(id),
        });
      }
      let input = args;
      if (input === undefined) {
        const ex = m.examples.find((e) => e.id === m['x-primary-example']) ?? m.examples[0];
        input = structuredClone(ex.input);
      }
      if (units) input = { ...input, options: { ...(input.options ?? {}), profile: units } };
      const maxItems = output?.maxItems ?? 1000;
      const offset = output?.offset ?? 0;
      if (!Number.isInteger(maxItems) || maxItems < 1 || maxItems > 10000) return fail('INVALID_INPUT', 'output.maxItems is a whole number from 1 to 10,000.', { field: '/output/maxItems' });
      if (!Number.isInteger(offset) || offset < 0) return fail('INVALID_INPUT', 'output.offset is a whole number, 0 or more.', { field: '/output/offset' });
      return paged(m, input, maxItems, offset);
    },

    geoprims_pipeline: async ({ steps }) => {
      if (!Array.isArray(steps) || steps.length < 1 || steps.length > 20) return fail('INVALID_INPUT', 'steps must list 1 to 20 steps.', { field: '/steps' });
      const results = [];
      for (const [i, step] of steps.entries()) {
        const args = structuredClone(step.args ?? {});
        for (const [target, source] of Object.entries(step.bind ?? {})) {
          const m = /^(\d+):(\/.*|)$/.exec(source);
          const field = `/steps/${i}/bind/${target.replaceAll('~', '~0').replaceAll('/', '~1')}`;
          if (!m || !target.startsWith('/')) return fail('INVALID_INPUT', `Binding "${source}" must look like "0:/result/field".`, { field });
          const from = Number(m[1]);
          if (from >= i) return fail('INVALID_INPUT', `Step ${i} binds to step ${from}, which has not run yet; bind only to earlier steps.`, { field });
          const value = getPointer(results[from], m[2]);
          if (value === undefined) return fail('INVALID_INPUT', `Step ${from} has nothing at ${m[2]}.`, { field });
          setPointer(args, target, bindValue(value));
        }
        const out = await run(step.id, args);
        results.push(out);
        if (!out.ok) return { ok: false, error: { ...out.error, step: i }, result: { steps: results } };
      }
      return { ok: true, result: { steps: results } };
    },

    geoprims_report_problem: async ({ toolId, args, observed, expected, source, note }) => {
      const m = byId.get(toolId);
      if (!m) {
        return fail('UNSUPPORTED', `There is no tool with id "${toolId}".`, { field: '/toolId', suggestions: await suggest(toolId) });
      }
      const L = limits;
      const { options = {}, ...inputs } = args;
      const run = JSON.parse(await host.invoke(toolId, JSON.stringify(args)));
      const row = (field, label, value, unit) => ({
        field: cut(field, L.fieldChars),
        label: cut(label, L.labelChars),
        value: cut(typeof value === 'string' ? value : JSON.stringify(value), L.valueChars),
        unit: cut(unit ?? '', L.unitChars),
      });
      const props = m.inputs.properties;
      const inputRows = Object.entries(inputs)
        .slice(0, L.inputRows)
        .map(([k, v]) => row(k, props[k]?.title ?? k, v, typeof v === 'number' ? props[k]?.['x-unit'] : ''));
      const outputRows = run.ok
        ? Object.entries(run.result)
            .slice(0, L.outputRows)
            .map(([k, v]) =>
              v && typeof v === 'object' && 'value' in v ? row(k, m.outputs.properties[k]?.title ?? k, v.value, v.unit) : row(k, m.outputs.properties[k]?.title ?? k, v, ''),
            )
        : [];
      const warnings = run.ok ? run.meta.warnings.map((w) => w.code).slice(0, L.warningCodes) : [];
      const noteText = [
        `Observed: ${observed}`,
        expected && `Expected: ${expected}`,
        source && `Source: ${source}`,
        !run.ok && `Run error: ${run.error.code}`,
        note,
      ]
        .filter(Boolean)
        .join('; ');
      const [domain, group, op] = toolId.split('.');
      const pagePath = `/${domain}/${group}/${op}/`;
      const moduleName = domain === 'units' ? 'base' : domain;
      const state = { i: Object.fromEntries(Object.entries(inputs).filter(([, v]) => typeof v === 'string' || typeof v === 'number')) };
      if (options.outputUnits) state.u = options.outputUnits;
      const enc = JSON.parse(await host.callExport('link', 'gp_link_encode', JSON.stringify({ state, flags: ['report'] })));
      if (!enc.ok) return enc;
      const payload = {
        apiVersion: L.apiVersion,
        toolId,
        toolVersion: m.version,
        coreVersion: catalog.coreVersion,
        buildHash: (modules.find((x) => x.module === moduleName)?.sha256 ?? '').slice(0, 16),
        assetVersions: {},
        kind: 'wrong-result',
        pagePath,
        inputs: inputRows,
        outputs: outputRows,
        warnings,
        display: { theme: 'none', unitProfile: options.profile ?? 'default', viewportClass: 'agent' },
        note: cut(noteText, L.noteChars),
        token: null,
      };
      const bytes = new TextEncoder().encode(JSON.stringify(payload)).length;
      if (bytes > L.bodyBytes) return fail('LIMIT_EXCEEDED', `The report is ${bytes} bytes; the limit is ${L.bodyBytes}.`);
      return {
        ok: true,
        result: {
          link: `${SITE}${pagePath}#${enc.result.fragment}`,
          issueUrl: ISSUE_URL,
          payload,
          instructions:
            'Show the link to the user. Opening it restores these inputs and the report form; the user reviews and sends it. Nothing was sent.',
        },
      };
    },

    geoprims_convert_units: async ({ value, from, to }) => {
      const converters = catalog.tools.filter((t) => t.group !== 'temperature-difference' && /^units\.[a-z-]+\.convert$/.test(t.id) && t.inputs.properties.to);
      for (const t of converters) {
        const out = await run(t.id, { value, to, ...(from === undefined ? {} : { from }) });
        if (out.ok || out.error.code !== 'UNIT_MISMATCH') return out;
      }
      const named = [from ?? (typeof value === 'string' ? value : undefined), to].filter(Boolean).join(' and ');
      return fail('UNIT_MISMATCH', `No unit family accepts ${named} together.`, {
        hint: 'Check the unit symbols (they are case-sensitive, e.g. mbar, not Mbar) and that both measure the same thing.',
      });
    },
  };
}

export { schemaCheck };
