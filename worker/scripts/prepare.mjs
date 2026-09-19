#!/usr/bin/env node
// Copies the tool ids and versions from the site's catalog build into the
// Worker bundle (spec "Isolated server component": the Worker imports the
// catalog from the same build as the site). Run after `npm run build`.
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const catalog = JSON.parse(readFileSync(join(root, 'dist/catalog/v1.json'), 'utf8'));
const tools = catalog.tools.map((t) => ({ id: t.id, version: t.version })).sort((a, b) => a.id.localeCompare(b.id));
writeFileSync(join(root, 'worker/src/catalog-tools.json'), JSON.stringify(tools) + '\n');
console.log(`worker/src/catalog-tools.json: ${tools.length} tools`);
