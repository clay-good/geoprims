// The geoprims meta-tools (add-local-mcp-server, "Default meta-tool surface").
// Descriptions are static text: they never interpolate user input.

const ANNOTATIONS = { readOnlyHint: true, destructiveHint: false, idempotentHint: true, openWorldHint: false };

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
      'Find geoprims calculators (geodesy, navigation, aviation, drone, survey, indexing, time, units) by plain words, abbreviations, or tool id. Returns ranked ids with summaries. Experimental tools are hidden unless includeExperimental is true; hiddenExperimental counts them.',
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
      'Run one tool by id. args follow the tool\'s input schema (see geoprims_describe); numbers may carry units as strings, like "145 kts". Omit args to run the worked example. units picks an output unit profile: si, aviation, aviation-hpa, us-customary, survey-metric, or survey-us. Results are planning aids, not certified for navigation; relay meta.warnings to the user.',
    inputSchema: {
      type: 'object',
      properties: {
        id: { type: 'string' },
        args: { type: 'object' },
        units: { type: 'string', enum: ['si', 'aviation', 'aviation-hpa', 'us-customary', 'survey-metric', 'survey-us'] },
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
].map((t) => ({ ...t, annotations: { title: t.title, ...ANNOTATIONS } }));

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

/** A bound quantity {value, unit} travels as a unit-tagged string, so the core converts it. */
const bindValue = (v) =>
  v && typeof v === 'object' && typeof v.value === 'number' && typeof v.unit === 'string' ? `${v.value} ${v.unit}` : v;

export function metaHandlers({ host, catalog }) {
  const byId = new Map(catalog.tools.map((t) => [t.id, t]));
  const run = async (id, args) => JSON.parse(await host.invoke(id, JSON.stringify(args)));
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

    geoprims_run: async ({ id, args, units }) => {
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
      return run(id, input);
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
