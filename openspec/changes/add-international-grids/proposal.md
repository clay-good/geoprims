## Why

geoprims converts coordinates to centimeter truth inside the United States and to a 7-parameter approximation everywhere else. British National Grid is "labeled low accuracy" (a few meters); Irish, Australian, New Zealand, Canadian, Swiss, Dutch, German, and French users get a UTM zone and nothing national. Yet the official transformation grids for those countries are published under open licenses, PROJ already redistributes them, and each issuer publishes test points. The same engine that runs NADCON5 and GEOID18 (float32 tiles, the issuer's interpolation, signed indexes, 1° privacy tiles) can run OSTN15, GDA2020, NZGD2000, and the rest.

Depends on: `add-geodesy-suite` (projections, grid transformations, CRS registry), `establish-platform-foundation` (asset pipeline, tiling, licensing page).

## What Changes

National grids and their official transformations as first-class, cited, test-pointed tools, in the `geodesy` domain:

| Country | Grid and datum tools | Transformation and geoid assets (license) |
|---|---|---|
| Great Britain | OS National Grid letters ↔ eastings and northings; OSGB36 ↔ ETRS89 by OSTN15; ODN heights by OSGM15 | OSTN15 and OSGM15 (BSD, Ordnance Survey 2016) |
| Ireland and Northern Ireland | Irish Grid (TM75) and ITM letters and numbers; Irish Grid ↔ ETRS89 by the OSi/LPS polynomial; Malin and Belfast heights by OSGM15 | Polynomial coefficients and OSGM15 Malin/Belfast grids (BSD, three-agency notice) |
| Australia | MGA2020 and MGA94 zones; GDA94 ↔ GDA2020 by the conformal and conformal-plus-distortion grids; AHD heights by AUSGeoid2020 | ICSM grids and AUSGeoid2020 (CC BY 4.0) |
| New Zealand | NZTM2000; NZGD1949 ↔ NZGD2000 by the distortion grid; NZVD2016 heights by NZGeoid2016; the NZGD2000 deformation model for epochs | LINZ grids and model (CC BY 4.0) |
| Canada | MTM zones; NAD83(CSRS) epoch propagation by the v7 velocity grid; CGG2013a heights | NRCan grids (Open Government Licence – Canada) |
| Europe | ETRS89 realizations (ETRF2000, ETRF2014, ETRF2020) ↔ ITRF by EUREF Technical Note 1 (March 2024); ETRS89-LAEA and LCC | Parameters only (facts; EPSG 10572 and related) |
| Switzerland, Netherlands, Germany, France, South Africa | LV95 by swisstopo's exact formulas with CHENyx06; RD with RDNAPTRANS2018; Gauss-Krüger ↔ UTM with BETA2007; Lambert-93 with the IGN grids and RAF20; Lo zones with SAGEOID2010 | CC0, CC BY 4.0, free redistribution, Licence Ouverte, CC BY 4.0 |

Every transformation names the grid, version, and issuer, states the issuer's accuracy, and carries the required attribution. Japan's plane rectangular zones are added as projections only: GSI's TKY2JGD and PatchJGD parameter files need Survey Act approval and are not redistributed.

## Capabilities

### New Capabilities

- `geodesy/national-grids`: National grid references, national projections, and the official grid-based datum and height transformations for the countries above.

### Modified Capabilities

None (the CRS registry and grid-transformation requirements already describe the mechanism; this change adds the data and the tools that use it).

## Non-goals

- Japan's datum transformation grids (approval-only), India (no open grid), and any grid whose license is unclear.
- A general "any EPSG to any EPSG" engine. Each country's path is explicit.
- Real-time deformation beyond the NZGD2000 model and the Canadian velocity grid.

## Impact

- New assets, each registered with license, attribution, digest, and load policy: `ostn15`, `osgm15` (GB, Belfast, Malin), `osi-polynomial` (tiny, bundled), `gda2020-conformal`, `gda2020-distortion`, `ausgeoid2020`, `nzgd2kgrid`, `nzgd2000-deformation`, `nzgeoid2016`, `nad83csrs-v7vg`, `cgg2013a`, `chenyx06`, `rdnaptrans2018` (three files), `beta2007`, `fr-ign` (ntf_r93, gr3df97a, RAF20), `sageoid2010`. Regional grids are tiled at 1° or kept whole when under 1 MB.
- The CRS registry gains the national CRSs (OSGB36 BNG, TM75, ITM, GDA2020 and MGA2020, NZGD2000 and NZTM2000, NAD83(CSRS) MTM, ETRS89 LAEA and LCC, LV95, RD, DHDN GK and ETRS89 UTM, RGF93 Lambert-93, Hartebeesthoek94 Lo, JGD2011 zones).
- `/licenses` renders every new attribution. The `geodesy` rollup grows by about 24 operations and the generated-pair allow-list by a few high-intent pairs (`bng-to-latlon`, `mga-to-latlon`).
