<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Isogonic lines (`geodesy.magnetic.isogonic`)

## Method

An isogonic line joins the places where a compass is off true north by the same amount. The tool evaluates the magnetic model's declination at sea level on a grid of latitude and longitude over the region, or the whole globe, and draws a contour at every multiple of the interval by marching squares, placing each crossing between two grid points by linear interpolation. It also marks the zones where the horizontal field is too weak for a compass to be relied on, and stops the lines there.

## Equations

- Declination at each grid point: the WMM2025 (or IGRF-14) main field at the date, at sea level, as the declination tool computes it.
- Grid step: the square root of the area in square degrees over 8,000, and no finer than 0.25°.
- A line at level L crosses a grid edge between values a and b at the fraction (L − a) ÷ (b − a) along it.
- Lines are left out where the horizontal field is under 6,000 nT, and where declination nears ±180° and wraps.

## Symbols and units

Latitude, longitude, the interval (2° by default, 0.5° to 30°), and declination in degrees, east positive. The date is a calendar date or a decimal year inside the model's window.

## Domain

All four edges of the region, or none for the whole globe; north above south, and east of west with no region crossing 180°. A date outside the model's window is OUT_OF_DOMAIN. A region needing more than twice the grid budget at 0.25° is LIMIT_EXCEEDED.

## Approximations

A contour is as good as its grid. Between grid points the field is taken as linear, so a line can sit a little off the model's own value: a few thousandths of a degree where the grid is fine and the field smooth, and tenths of a degree on a whole-globe map, most near the magnetic poles where declination turns fastest. The model is the main field only, without local crustal anomalies, and is itself good to about half a degree at mid latitudes. For one exact value the declination tool is the one to use.

## Worked example

- sourcePublisher: pygeomag (open-source Python implementation of the World Magnetic Model)
- sourceTitle: pygeomag 1.1.0, GeoMag.calculate with the WMM2025 coefficients
- sourceEdition: pygeomag 1.1.0, run 2026-10-10
- sourceLocator: tools/diff/isogonic.test.mjs: every point of every line over six regions and the whole globe, about 15,000 points, each handed to pygeomag for its declination there
- independent: yes
- inputs: Colorado at 1°; the conterminous US at 2°; Europe at 1°; Australia at 2°; southern Africa at 2°; Alaska at 5°; and the whole globe at 2°
- outputs: over the regions every line point is within 0.12° of the line's value in pygeomag (0.0002° over Colorado); over the globe the median is 0.012°, 95% are within 0.21°, and the worst, beside the south magnetic pole, is 0.79°
- tolerance: 0.2° for a region; for the globe a median under 0.05°, 95% under 0.3°, and none over 1°
- verifiedBy: automated differential test (below), which runs wherever pygeomag is installed
- verifiedOn: 2026-10-10

The lowest line of each of 14 regions is also predicted from pygeomag alone, by scanning its declination on a 0.1° grid, and held as golden vectors v007 through v020.

## Differential tests

- `tools/diff/isogonic.test.mjs`: pygeomag's declination at every line point against the line's value; skipped where no Python with pygeomag is installed
- `tools/vectors/gen_isogonic.py`: the lowest line in 14 regions and dates from pygeomag's own scan of the field, kept clear of cases where a coarser grid could fairly start a line later
- `core/vectors/geodesy.magnetic.isogonic.jsonl`: those vectors, a regression of the conterminous-US overlay, and the refused inputs, run through the core on every build

## Invariants

- `core/crates/gp-geodesy/tests/magnetic.rs` `isogonic_lines_lie_where_the_model_has_their_declination`: every point of every line over the conterminous US, evaluated by the declination tool itself, has the line's declination to within the grid's interpolation, and every level is a multiple of the interval
- `core/crates/gp-geodesy/tests/magnetic.rs` `isogonic_zones_ring_the_magnetic_poles_and_lines_stop_there`: the weak-field zones surround the magnetic poles and no line point lies inside them
