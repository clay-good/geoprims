<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# UTM zone and grid zone (`geodesy.utm.zone`)

## Method

The UTM zone of a point is ⌊(longitude + 180) / 6⌋ + 1 — sixty zones, six degrees each, numbered east from the antimeridian. That is the whole rule everywhere except two places, and the two exceptions are the only reason this needs a tool rather than a line of arithmetic.

**Norway.** Between 56° and 64° north, zone 32 is widened westward to 3° east. A point at 4° east and 60° north is in zone 32, not the zone 31 the formula gives. The exception exists so that the Norwegian mainland does not fall across a zone boundary.

**Svalbard.** Between 72° and 84° north, zones 32, 34 and 36 do not exist and 31, 33, 35 and 37 are widened to take their place. The boundaries there fall at 9°, 21° and 33° east rather than every 6°.

The latitude band is the second letter of an MGRS grid zone: twenty bands of 8° from C at 80° south to X, which is 12° tall and reaches 84° north. I and O are skipped, because they read as 1 and 0. Outside the UTM range the point is in one of the polar UPS zones, reported as zone 0 with a band of A, B, Y or Z.

The central meridian follows from the zone alone: 6z − 183.

## Equations

- Zone = ⌊(λ + 180) / 6⌋ + 1, before exceptions.
- Norway: 56° ≤ φ < 64° and 3° ≤ λ < 12° gives zone 32.
- Svalbard: 72° ≤ φ < 84°, with boundaries at 0°, 9°, 21°, 33° and 42° east giving zones 31, 33, 35 and 37.
- Band letter: index ⌊(φ + 80) / 8⌋ into `CDEFGHJKLMNPQRSTUVWX`, with X covering 72° to 84°.
- Central meridian = 6z − 183.

## Symbols and units

`lat` and `lon` in degrees. Out come the `zone` (0 in the polar regions), the `grid_zone` as zone and band, and the `central_meridian`.

## Domain

Any point. Between 80° south and 84° north the answer is a UTM zone; beyond either, it is a UPS zone and the zone number is 0.

## Approximations

None. Every rule here is an integer boundary.

## Worked example

- sourcePublisher: Charles Karney (GeographicLib); National Geospatial-Intelligence Agency
- sourceTitle: GeographicLib's `GeoConvert`; the UTM and MGRS definitions with the Norway and Svalbard exceptions
- sourceEdition: GeographicLib 2.7
- sourceLocator: `GeoConvert -u` for the zone and `-m` for the MGRS grid zone, both implementing the exceptions
- independent: yes
- inputs: 39 points, including twelve ordinary ones around the world, eight straddling the Norway exception on all four sides, eight in and around Svalbard at every widened boundary, both antimeridian sides, and the band edges
- outputs: the zone, the grid zone and the central meridian
- tolerance: exact
- verifiedBy: golden vectors v001 to v039, run by the core on every build
- verifiedOn: 2026-09-23

GeographicLib implements both exceptions, so the reference reads its answers rather than restating the rules — which matters, because restating them is exactly where a transcription error would hide. The cases a degree outside each exception on every side are the ones that catch a boundary written with the wrong comparison.

One case is left out on purpose. At exactly 84° north the core keeps the point in UTM, symmetric with the 80° south edge where the two agree; GeographicLib switches to UPS there, so that UTM and UPS tile without overlapping. Both are defensible readings of "80° S to 84° N", so the difference is written into the limitations instead of pinned to one implementation's choice.

## Differential tests

- `tools/vectors/gen_utm_zone.py`: 33 of the 39 vectors, from GeographicLib's `GeoConvert`
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.utm.zone.jsonl`: 39 points, weighted toward the two exceptions

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `utm_zone_invariants`: away from the exceptions the zone is exactly ⌊(λ + 180)/6⌋ + 1, checked every two degrees round the equator, so the ordinary case is the formula and nothing else; the Norway exception applies inside its box and not a degree outside it on any of the four sides; in the Svalbard band the zone is one of 31, 33, 35, 37 and never 32, 34 or 36, while a degree below the band those even zones return; the central meridian is 6z − 183 and is within three degrees of the longitude asked about, which is what makes it the zone's own meridian; the band letter never has an I or an O in it; and the polar regions report zone 0 with an A, B, Y or Z
