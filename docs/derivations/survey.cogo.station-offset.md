<!-- Derivation note (trust/correctness-program layer A). -->
# Station and offset from a line (`survey.cogo.station-offset`)

## Method

The tool ties a point to one straight survey line by station and offset, or works the other way and sets out a point from a station and offset. The line starts at a known northing and easting with a known station, and runs along a direction (bearing or azimuth) or toward a second point. Given a point, it projects the point onto the line: the distance along the line is added to the starting station, and the perpendicular distance is the offset. Given a station and offset instead, it walks along the line to that station and then steps off at right angles. Offsets are positive to the right when facing up-station. `survey.cogo.perpendicular-foot` and `survey.cogo.point-from-station-offset` are its two directions under their own names, run by the same code.

## Equations

For start point S, unit tangent **t** = (cos α, sin α) in (north, east), and right-hand unit normal **n** = (−sin α, cos α):

- Projection: along = (P − S) · **t**, offset = (P − S) · **n**, station = start station + along, foot = S + along · **t**
- Setout: along = station − start station, foot = S + along · **t**, P = foot + offset · **n**

When an end point E defines the line, α = atan2(E_east − S_east, E_north − S_north).

## Symbols and units

Northings, eastings, along distances, and offsets share one length unit: meters, international feet, or US survey feet. α is an azimuth clockwise from grid north. Stations are strings: 12+34.56 is 1,234.56 ft and 1+234.567 is 1,234.567 m. Results are rounded for display to 0.01 ft or 0.001 m in the station string; the numeric outputs are not rounded.

## Domain

The line needs a nonzero direction: a bearing or azimuth, or an end point different from the start. Give either a point (northing and easting) or a station and offset, not neither. All lengths must be in compatible units. The line extends without limit both ways, so a point before the start gets a station below the start station.

## Approximations

Exact plane geometry, up to binary floating point and the display rounding of the station string. It does not apply grid-to-ground scale, Earth curvature, or follow curves, spirals, or a route with several PIs.

## Worked example

- sourcePublisher: Autodesk
- sourceTitle: Civil 3D User Guide, To Create a Point From a Survey Baseline Using a Station and Offset
- sourceEdition: Civil 3D 2024 online help
- sourceLocator: help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-5D79BE01-65E7-4A68-A807-D05B7D8CA155.htm (BL TP 3 220 15.24 and BL TP 4 270 -15.24)
- independent: yes
- inputs: baseline from N 1,000, E 1,000 at station 0+000, azimuth 100° (the page's ZD 2 100 121.92 shot); point 3 at station 220, offset 15.24 right; point 4 at station 270, offset 15.24 left
- outputs: point 3 N 946.7889, E 1,214.0113; point 4 N 968.1235, E 1,268.5445; projecting those printed coordinates back gives stations 220 and 270 and offsets +15.24 and −15.24 to the printed precision
- tolerance: 1e-4 m (the printed precision) against the page; 1e-9 relative against the Python evaluation in the golden vectors
- verifiedBy: golden vectors v007 to v010 of `core/vectors/survey.cogo.station-offset.jsonl`
- verifiedOn: 2026-10-19

The page prints the azimuth only as the shot that made point 2; solving its two printed points for the line direction gives 100.0000°, which confirms the reading.

## Differential tests

- `core/vectors/survey.cogo.station-offset.jsonl` holds 22 vectors. v001 to v006 come from `tools/vectors/gen_survey.py`; v007 to v022 come from a separate Python evaluation of the equations above, covering both directions, lines by azimuth and by end point, all four quadrants, meters, feet, and US survey feet, a zero offset that reads as on the line, and points behind the start.
- The integration test `named_direction_and_station_modes_match_their_general_tools` holds the two focused views to this tool exactly.

## Invariants

- `core/crates/gp-survey/tests/cogo_views_invariants.rs` `station_offset_invariants`: at five azimuths, along² + offset² equals the squared distance to the start; moving the whole figure leaves station and offset unchanged; turning the line around flips the offset's sign and the along distance; and setting out the reported station and offset returns the point.
- `core/crates/gp-survey/tests/cogo_views_invariants.rs` `station_offset_views_invariants`: the foot-to-point vector is perpendicular to the line, and a point set out from a station and offset projects back to them.
