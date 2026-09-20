// Derived catalog artifacts. Keep these projections small: the Rust manifests
// remain the source of truth for schemas, search fields, and tool descriptions.
import { DIRECT_OUTPUT_SCHEMA, directDescription, directName } from '../../mcp/toolsets.mjs';

function tsType(schema) {
  if (schema.enum) return schema.enum.map((value) => JSON.stringify(value)).join(' | ');
  const kinds = Array.isArray(schema.type) ? schema.type : [schema.type];
  return kinds.map((kind) => {
    if (kind === 'object') {
      const required = new Set(schema.required ?? []);
      const fields = Object.entries(schema.properties ?? {}).map(([key, value]) =>
        `${JSON.stringify(key)}${required.has(key) ? '' : '?'}: ${tsType(value)};`);
      if (schema.additionalProperties && typeof schema.additionalProperties === 'object') {
        fields.push(`[key: string]: ${tsType(schema.additionalProperties)};`);
      }
      return `{ ${fields.join(' ')} }`;
    }
    if (kind === 'array') return `(${tsType(schema.items)})[]`;
    if (['string', 'number', 'boolean', 'null'].includes(kind)) return kind;
    throw new Error(`Unsupported manifest schema type: ${kind}`);
  }).join(' | ');
}

export function artifactsForTool(tool) {
  return {
    type: `  ${JSON.stringify(tool.id)}: ${tsType(tool.inputs)};`,
    outputType: `  ${JSON.stringify(tool.id)}: ${tsType(tool.outputs)};`,
    mcp: {
      id: tool.id,
      name: directName(tool.id),
      title: tool.title,
      description: directDescription(tool),
      inputSchema: tool.inputs,
      outputSchema: DIRECT_OUTPUT_SCHEMA,
    },
    search: Object.fromEntries(['id', 'title', 'summary', 'domain', 'group', 'aliases', 'keywords', 'stability', 'inputs', 'prefill']
      .map((key) => [key, tool[key]])),
  };
}

export function buildArtifacts(catalog) {
  const rows = catalog.tools.map(artifactsForTool);
  return {
    types: `// Generated from the built Wasm manifests. Do not edit.\nexport interface ToolInputs {\n${rows.map((r) => r.type).join('\n')}\n}\n\nexport interface ToolOutputs {\n${rows.map((r) => r.outputType).join('\n')}\n}\n\nexport type ToolId = keyof ToolInputs;\n`,
    mcp: rows.map((r) => r.mcp),
    search: rows.map((r) => r.search),
  };
}
