<!-- Derivation note (trust/correctness-program layer A). -->
# Grid zone and 100 km square in MGRS (`geodesy.grid-ref.grid-zone-lookup`)

## Method

Every MGRS reference begins with a grid zone designator, the UTM zone number and latitude band letter (or a single UPS letter near the poles), and two letters naming the 100 km square inside it. The operation projects the point to UTM or UPS with the same code as `geodesy.grid-ref.mgrs-forward` and writes the reference at its two coarsest precisions: the designator alone, and with the square.

## Equations

- Zone and band from the point, with the Norway and Svalbard exceptions; UPS above 84° N and below 80° S
- Square column letter from the easting's 100 km digit and the zone's set; row letter from the northing's 100 km digit, offset in even zones (AA lettering) or by the older AL scheme for Clarke 1866 and Bessel 1841

## Symbols and units

Latitude and longitude in degrees; the outputs are text.

## Domain

Latitude from −90° to 90°. Ellipsoids too flattened for the series method are refused.

## Approximations

None: the zone, band, and square follow exactly from the grid coordinates.

## Worked example

- sourcePublisher: National Geospatial-Intelligence Agency (GeoTrans), through the mgrs Python package
- sourceTitle: GeoTrans MGRS conversion
- sourceEdition: mgrs 1.5 (GeoTrans 3.x)
- sourceLocator: `MGRS().toMGRS(lat, lon, MGRSPrecision=0)`, split into the designator and the last two letters
- independent: yes
- inputs: Longyearbyen, 78.2232° N, 15.6267° E
- outputs: 33XWG: grid zone 33X under the Svalbard exception, square WG
- tolerance: exact text
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-14

## Differential tests

- `tools/vectors/gen_mgrs_views.py` `zone_vectors`: 20 points, uniform on the sphere plus the Norway and Svalbard exceptions, both UPS caps, and the equator and prime meridian, each written by GeoTrans rather than the core.
- `core/vectors/geodesy.grid-ref.grid-zone-lookup.jsonl`: those vectors and one refusal case.
- `core/crates/gp-geodesy/tests/geodesy.rs` `mgrs_zone_and_precision_match_the_mgrs_tools` requires each answer to begin the forward tool's reference.

## Invariants

- `core/crates/gp-geodesy/tests/geodesy_views_invariants.rs` `mgrs_views_invariants`: the designator and square begin every MGRS reference at the point, at 1, 3, and 5 digits; a point a meter away in the same square gets the same answer; and above 84° N the designator is Y or Z, below 80° S A or B.
