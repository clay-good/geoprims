<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# MGRS decoder (`geodesy.grid-ref.mgrs-inverse`)

## Method

Parse the reference, spaced or not, in MGRS or USNG form. Recover the easting and northing of the square's south-west corner from the zone, band, AA letters, and digits. The row letter repeats every 2,000 km, so the band's latitude range picks the right cycle. Then invert UTM or UPS to latitude and longitude. The tool returns the corner, the square's center, and its size, because a reference names a square, not a point. A 100 km square that does not occur in the reference's zone and band is refused with INVALID_INPUT. A square that straddles a band edge is accepted with the BAND_ADJUSTED warning.

## Equations

- E = (column index + 1) × 100 km + digits × 10^(5−d) m.
- N = row index × 100 km (with the even-zone 500 km offset) + 2,000 km × k + digits × 10^(5−d) m, with k chosen so N falls in the band.
- Center = corner + half a square in E and N, inverted separately.

## Symbols and units

d digits per axis (0 to 8), E and N in meters, output in WGS 84 degrees. The square size is 10^(5−d) m.

## Domain

Valid MGRS and USNG references in any case, with or without spaces. The letters I and O, uneven digit counts, and squares that do not exist in a zone are refused with INVALID_INPUT.

## Approximations

None. The corner is exact. The true point is anywhere in the square, which the square size states. A square cut by a zone edge can have its center beyond that edge (60NZF at the antimeridian, for example).

## Worked example

- sourcePublisher: Federal Geographic Data Committee
- sourceTitle: FGDC-STD-011-2001, United States National Grid
- sourceEdition: December 2001
- sourceLocator: Section 5.2.2 and table 1 (18SUJ2348316806479498 is the Washington Monument at NAD 83 UTM zone 18 E 323,483.168 m, N 4,306,479.498 m); latitude and longitude of that corner from PROJ 9.3.0
- independent: yes
- inputs: 18SUJ2348316806479498
- outputs: corner 38.889467309501576, −77.0352402156242; square 0.001 m
- tolerance: 1e-10°
- verifiedBy: golden vector v022, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/mgrs_parity.rs`: the corners of 2,000 references against NGA GEOTRANS (mgrs 1.5.4), within 2 cm. GEOTRANS's own projection series is off by up to 1.5 cm near the UPS edge and far out in Svalbard's widened zones, where PROJ puts our corners exactly on the grid (BBB714175 at E 2,171,400.000000 N 917,500.000000; 33XUJ at E 300,000.000000 N 8,800,000.000000; pinned as vectors v023 and v024).
- `tools/vectors/gen_mgrs_diff.py`: regenerates that fixture
- `core/vectors/geodesy.grid-ref.mgrs-inverse.jsonl`: 24 vectors, including 16 GEOTRANS corners, the FGDC millimeter reference, the two PROJ-exact corners, and invalid references
- `core/crates/gp-geodesy/tests/mgrs_al_parity.rs`: the AL lettering (1.1.0) on the Clarke 1866 and Bessel 1841 ellipsoids, 600 references from 10 km to 1 m against NGA GEOTRANS with those ellipsoid codes: grid zone and square letters identical, digits within one in the last place (GEOTRANS's projection series), corners within 2 cm, and each corner re-encoding to GEOTRANS's reference; `tools/vectors/gen_mgrs_al.py` regenerates it

## Invariants

- `core/crates/gp-geodesy/tests/mgrs_parity.rs` `mgrs_invariants`: at every precision the decoded corner of the point's reference lies within 1.5 square sizes of the point, and coarser references nest inside finer ones
