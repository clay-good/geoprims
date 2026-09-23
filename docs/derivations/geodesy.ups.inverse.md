<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# UPS inverse (`geodesy.ups.inverse`)

## Method

This undoes `geodesy.ups.forward`: a Universal Polar Stereographic easting and northing back to latitude and longitude. The hemisphere has to be given, because the two caps use the same numbers — (2000000, 2000000) is the north pole and the south pole both — and nothing in the coordinates themselves says which.

The inverse of a polar stereographic is direct rather than iterative in one step and iterative in the other. Subtracting the false origin gives the radius ρ and the bearing from grid north; the longitude follows from the bearing immediately. The latitude comes from ρ through the conformal latitude χ, and then from χ to the geodetic latitude φ by the series that inverts the isometric latitude — the same series the forward direction uses in reverse, converging in a few terms because the eccentricity is small.

## Equations

- North: ΔE = E − 2,000,000, ΔN = N − 2,000,000; ρ = √(ΔE² + ΔN²); λ = atan2(ΔE, −ΔN).
- South: λ = atan2(ΔE, ΔN).
- t = ρ √((1+e)^(1+e) (1−e)^(1−e)) / (2 a k₀); χ = π/2 − 2 arctan t.
- φ from χ by the inverse conformal-latitude series, then negated in the southern cap.
- ρ = 0 is the pole: latitude ±90°, and the longitude is not defined.

## Symbols and units

`hemisphere` is `N` or `S`; `easting` and `northing` in metres, with an optional `ellipsoid` or explicit `a` and `inverse_flattening`. Out come `lat` and `lon` in degrees.

## Domain

Any easting and northing within the cap. At the pole the longitude has no meaning and the coordinates are exactly (2,000,000, 2,000,000).

## Approximations

None beyond double precision and the convergence of the conformal-latitude series, which is exact to the last bit for any Earth-like ellipsoid.

## Worked example

- sourcePublisher: PROJ contributors; Charles Karney (GeographicLib)
- sourceTitle: PROJ, through pyproj, as EPSG:32661 and EPSG:32761 (WGS 84 / UPS North and South); GeographicLib's `GeoConvert`
- sourceEdition: PROJ 9.3.0 / pyproj 3.6.1; GeographicLib 2.7
- sourceLocator: the UPS definition — polar stereographic, k₀ = 0.994, false easting and northing 2,000,000 m
- independent: yes
- inputs: 26 coordinate pairs, the forward images of points around the longitude circle at both limits and up to 89.99° and −89.9°
- outputs: the latitude and longitude recovered in each case
- tolerance: 1e-9° on latitude; on longitude, 1e-9° divided by cos(latitude)
- verifiedBy: golden vectors v001 to v026, run by the core on every build
- verifiedOn: 2026-09-23

The longitude tolerance is the part worth explaining. Near the pole a longitude is a very short distance on the ground: at 89.99° a metre of easting is five thousand times the longitude it is at the equator, so a fixed angular bound would be either far too loose at 84° or impossible at 89.99°. The bound is therefore divided by cos(latitude), which is exactly the conditioning factor, and the same physical accuracy is demanded everywhere. Two cases at the antimeridian leave the longitude unpinned entirely, since ±180° is one angle written two ways.

## Differential tests

- `tools/vectors/gen_ups.py`: 21 of the 26 vectors, from PROJ through pyproj
- `core/crates/gp-geodesy/tests/`: the catalog lint, the examples, and the vectors, on every build
- `core/vectors/geodesy.ups.inverse.jsonl`: 26 vectors over both caps

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `ups_invariants`: forward then inverse returns the point it started from, to under a micrometre of ground, at every latitude from the cap edge to a tenth of a degree from the pole; the false origin inverts to the pole in both hemispheres; the hemisphere is not guessed — the same easting and northing read as N and as S give latitudes of opposite sign; and moving away from the origin lowers the latitude monotonically, so the inverse does not fold back on itself
