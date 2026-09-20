// What the /licenses page renders (platform/data-assets, "Licensing and
// attribution"). The datasets come from the asset registry, so a new dataset
// cannot ship without its license and attribution appearing here.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

// The build runs from apps/web, the same way catalog.mjs resolves the repo.
const web = process.cwd();
const root = join(web, '../..');

export const siteLicense = { spdx: 'MIT', file: 'LICENSE' };

/** Every registered dataset, alphabetical by title. */
export const datasets = JSON.parse(readFileSync(join(root, 'assets/registry.json'), 'utf8')).assets
  .map(({ id, title, issuer, license, attribution, sourceUrl }) => ({ id, title, issuer, license, attribution, sourceUrl }))
  .sort((a, b) => a.title.localeCompare(b.title));

/** The build dependencies, read from the website's own package.json. */
export const buildTools = Object.entries(
  JSON.parse(readFileSync(join(web, 'package.json'), 'utf8')).devDependencies ?? {},
)
  .map(([name, version]) => ({ name, version, license: 'MIT' }))
  .sort((a, b) => a.name.localeCompare(b.name));

/**
 * Datasets deliberately not used, with the reason
 * (platform/data-assets, "Exclusions").
 */
export const excluded = [
  {
    name: 'what3words',
    reason: 'Its address grid is proprietary and its licence forbids reimplementation, so there is no way to compute it here and show the work.',
  },
  {
    name: 'Enhanced Magnetic Model (EMM)',
    reason: 'Its high-resolution coefficients are far larger than the World Magnetic Model for a difference almost no practitioner needs, and it would push the offline download past its budget.',
  },
  {
    name: 'Copyrighted design tables (AASHTO, ASTM, and similar)',
    reason: 'Reproducing the tables would infringe. Tools that need a value from one cite the table and take the value as an input instead.',
  },
  {
    name: 'Commercial basemap tiles',
    reason: 'Every tile request would tell a third party where you are looking. The map is drawn from public-domain Natural Earth data served from this site.',
  },
];
