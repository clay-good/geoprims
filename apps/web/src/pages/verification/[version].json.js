import { catalog, verification } from '../../lib/catalog.mjs';

export function getStaticPaths() {
  return [{ params: { version: catalog.coreVersion } }];
}

export async function GET() {
  return new Response(JSON.stringify(await verification(), null, 2) + '\n', { headers: { 'Content-Type': 'application/json' } });
}
