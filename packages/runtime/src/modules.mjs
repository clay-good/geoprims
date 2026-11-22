// Which Wasm module a tool lives in (compute-core, "A tool page SHALL load
// only `base` plus the modules its tool declares"). Every host needs this —
// the Node host, the worker host, the website's main thread and its compute
// worker — so it lives here once rather than four times, which is how a
// module boundary came to be wrong in three of them at the same time.
//
// A tool's module is its domain, with the exceptions below, each of which is
// a boundary that does not follow the domain: the `units` domain is linked
// into `base`, and the national grids are their own module inside the geodesy
// domain, because geodesy reached its 400 KB compressed budget. The ids keep
// their domain, so only this list moves.
//
// `moduleForMatchesCatalog` in runtime.test.mjs holds the list to the built
// catalog's own module field for every tool, so it cannot drift.
export const MODULE_EXCEPTIONS = [
  ['units.', 'base'],
  ['geodesy.datum.beta2007', 'national-grids'],
  ['geodesy.datum.ch1903plus', 'national-grids'],
  ['geodesy.datum.chenyx06', 'national-grids'],
  ['geodesy.datum.nzgd2000', 'national-grids'],
  ['geodesy.datum.osi-polynomial', 'national-grids'],
  ['geodesy.datum.ostn15', 'national-grids'],
  ['geodesy.datum.rdnaptrans', 'national-grids'],
];

export function moduleFor(toolId) {
  for (const [prefix, module] of MODULE_EXCEPTIONS) {
    if (toolId.startsWith(prefix)) return module;
  }
  return toolId.split('.')[0];
}
