// Approved GitHub mirroring for confirmed problem reports
// (feedback/triage-and-corrections, "GitHub mirroring only after approval").
// Reporter prose is deliberately absent from both the query and this API.

export const MIRROR_QUERY = `SELECT id, tool_id, tool_version, core_version, build_hash, asset_versions_json,
  kind, page_path, inputs_json, outputs_json, status
FROM problem_reports WHERE id = ?`;

const CONTROL = /[\u0000-\u001f\u007f]/;

function text(value, name, max = 1000) {
  if (typeof value !== 'string' || !value.trim() || value.length > max || CONTROL.test(value)) {
    throw new Error(`${name} must be 1-${max} characters with no control characters`);
  }
  return value.trim();
}

function json(value, name) {
  try {
    return JSON.parse(value);
  } catch {
    throw new Error(`${name} is not valid JSON`);
  }
}

const codeBlock = (value) => JSON.stringify(value, null, 2).split('\n').map((line) => `    ${line}`).join('\n');

/** Build the only issue shape the mirror may send. */
export function buildIssue(row, summary, siteOrigin = 'https://geoprims.com') {
  if (!row || row.status !== 'confirmed') throw new Error('only a confirmed report may be mirrored');
  const tool = text(row.tool_id, 'tool id', 80);
  const maintainerSummary = text(summary, 'maintainer summary');
  const pagePath = text(row.page_path, 'page path', 2048);
  if (!/^\/(?!\/)/.test(pagePath)) throw new Error('page path must be a same-site absolute path');
  const permalink = new URL(pagePath, siteOrigin).href;
  const assets = json(row.asset_versions_json, 'asset versions');
  const inputs = json(row.inputs_json, 'inputs');
  const outputs = json(row.outputs_json, 'outputs');
  const title = `Confirmed report: ${tool}`;
  const body = [
    '## Maintainer summary',
    '',
    maintainerSummary,
    '',
    '## Report',
    '',
    `- Tool: \`${tool}\``,
    `- Kind: \`${text(row.kind, 'kind', 40)}\``,
    `- Permalink: <${permalink}>`,
    `- Tool version: \`${text(row.tool_version, 'tool version', 64)}\``,
    `- Core version: \`${text(row.core_version, 'core version', 64)}\``,
    `- Build: \`${text(row.build_hash, 'build hash', 64)}\``,
    `- Asset versions: ${Object.keys(assets).length ? '' : 'none'}`,
    ...(Object.keys(assets).length ? ['', codeBlock(assets)] : []),
    '',
    '## Inputs',
    '',
    codeBlock(inputs),
    '',
    '## Outputs',
    '',
    codeBlock(outputs),
    '',
    `Mirrored from confirmed report \`${text(row.id, 'report id', 128)}\` after maintainer approval.`,
  ].join('\n');
  return { title, body, labels: ['correctness'] };
}

/** Create one issue with a fine-grained token supplied by the operator. */
export async function createIssue({ repository, token, issue, fetchImpl = fetch, apiOrigin = 'https://api.github.com' }) {
  const match = /^([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+)$/.exec(repository ?? '');
  if (!match) throw new Error('repository must be owner/name');
  text(token, 'GitHub token', 4096);
  const response = await fetchImpl(`${apiOrigin}/repos/${match[1]}/${match[2]}/issues`, {
    method: 'POST',
    headers: {
      Accept: 'application/vnd.github+json',
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
      'User-Agent': 'geoprims-report-mirror',
      'X-GitHub-Api-Version': '2022-11-28',
    },
    body: JSON.stringify(issue),
  });
  if (!response.ok) {
    const detail = (await response.text()).slice(0, 500);
    throw new Error(`GitHub issue creation failed (${response.status})${detail ? `: ${detail}` : ''}`);
  }
  const created = await response.json();
  if (!created.html_url || !created.number) throw new Error('GitHub returned no issue URL or number');
  return { number: created.number, url: created.html_url };
}
