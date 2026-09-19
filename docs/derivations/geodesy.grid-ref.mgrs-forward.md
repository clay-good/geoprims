<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# MGRS encoder (`geodesy.grid-ref.mgrs-forward`)

## Method

Project the WGS 84 point to UTM (Karney's Krüger series, the same as the stable UTM tool) or, poleward of 84°N and 80°S, to UPS. Pick the latitude band letter and apply the Norway (32V) and Svalbard (31X–37X) zone exceptions. Name the 100 km square with the NGA AA lettering: column letters cycle through three sets by zone, and row letters repeat every 2,000 km, offset by 500 km in even zones. Then write the easting and northing within the square, truncated, never rounded, to the requested precision, as the standard requires.

## Equations

- Column letter: set (zone − 1) mod 3 of `ABCDEFGH`, `JKLMNPQR`, `STUVWXYZ`, indexed by ⌊E / 100 km⌋ − 1.
- Row letter: `ABCDEFGHJKLMNPQRSTUV` indexed by (⌊N / 100 km⌋ + 5 × [zone even]) mod 20.
- Digits for precision 10^(5−d) m: ⌊(E mod 100 km) / 10^(5−d)⌋ and ⌊(N mod 100 km) / 10^(5−d)⌋, each d digits.
- UPS: the zone letters A, B (south) and Y, Z (north) with their own letter tables.

## Symbols and units

E, N UTM or UPS easting and northing (m), d digits per axis (0 to 8, for 100 km down to 1 mm). Latitude and longitude are WGS 84 degrees.

## Domain

All latitudes and longitudes. UTM covers 80°S to 84°N and UPS covers the caps. Points on a zone or band edge go to the zone or band the standard assigns.

## Approximations

None. The projection is exact to nanometers (checked for the UTM tool), and truncation names the square that contains the point. Precision finer than 1 m exceeds what WGS 84 coordinates usually mean. The tool allows it for special applications, as the USNG standard does.

## Worked example

- sourcePublisher: Federal Geographic Data Committee
- sourceTitle: FGDC-STD-011-2001, United States National Grid
- sourceEdition: December 2001
- sourceLocator: Section 5.2.2 and table 1 (the Washington Monument, NAD 83 UTM zone 18 E 323,483.168 m, N 4,306,479.498 m = 18SUJ2348306479, truncated to 18SUJ23480647, 18SUJ234064, and 18SUJ2306); its latitude and longitude from PROJ 9.3.0
- independent: yes
- inputs: 38.889467309501576, −77.0352402156242 at 1 m, 10 m, 100 m, and 1 km
- outputs: 18SUJ2348306479, 18SUJ23480647, 18SUJ234064, 18SUJ2306
- tolerance: identical strings
- verifiedBy: golden vectors v022 to v025, run by the core on every build
- verifiedOn: 2026-09-19

## Differential tests

- `core/crates/gp-geodesy/tests/mgrs_parity.rs`: 2,000 points against NGA GEOTRANS (the mgrs 1.5.4 package), one in five in the Norway and Svalbard exceptions and one in seven in the UPS caps. All match. In one case GEOTRANS's 1.5 cm polar projection error flips the last digit, and the fixture takes that digit from PROJ's exact grid coordinates.
- `tools/vectors/gen_mgrs_diff.py`: regenerates that fixture
- `core/vectors/geodesy.grid-ref.mgrs-forward.jsonl`: 25 vectors, including 16 GEOTRANS references across the exceptions, both poles, the antimeridian, and every precision

## Invariants

- `core/crates/gp-geodesy/tests/mgrs_parity.rs` `mgrs_invariants`: coarser references keep the leading digits of each axis, and every square's corner lies within 1.5 square sizes of the point
