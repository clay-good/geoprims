// MCP client setup snippets (add-local-mcp-server, "Setup snippets"): one
// source for the README, the site's "Use with agents" page, and the smoke
// test that launches the server exactly as each snippet says.

/** Placeholder for the clone's path in every clone-path snippet. */
export const CLONE_PATH = '/abs/path/geoprims';
export const NPX_PACKAGE = '@geoprims/mcp';

/** The command and arguments that start the server for one install path. */
export function launch(path, how = 'clone') {
  return how === 'npx' ? { command: 'npx', args: ['-y', NPX_PACKAGE] } : { command: 'node', args: [`${path}/mcp/server.mjs`] };
}

export const CLIENTS = [
  { id: 'claude-code', name: 'Claude Code', where: 'Run in a terminal' },
  { id: 'claude-desktop', name: 'Claude Desktop', where: 'Settings, Developer, Edit Config: claude_desktop_config.json', key: 'mcpServers' },
  { id: 'vscode', name: 'VS Code', where: '.vscode/mcp.json in your workspace', key: 'servers', entry: { type: 'stdio' } },
  { id: 'cursor', name: 'Cursor', where: '.cursor/mcp.json in your project, or ~/.cursor/mcp.json', key: 'mcpServers' },
  { id: 'windsurf', name: 'Windsurf', where: '~/.codeium/windsurf/mcp_config.json', key: 'mcpServers' },
];

/** The snippet text for a client: a shell command for Claude Code, JSON for the rest. */
export function snippet(client, how = 'clone', path = CLONE_PATH) {
  const { command, args } = launch(path, how);
  if (client.id === 'claude-code') return `claude mcp add geoprims -- ${[command, ...args].join(' ')}`;
  return JSON.stringify({ [client.key]: { geoprims: { ...client.entry, command, args } } }, null, 2);
}

/** Recovers { command, args } from a snippet, as the client would read it. */
export function parseSnippet(client, text) {
  if (client.id === 'claude-code') {
    const [command, ...args] = text.split(' -- ')[1].split(' ');
    return { command, args };
  }
  const { command, args } = JSON.parse(text)[client.key].geoprims;
  return { command, args };
}
