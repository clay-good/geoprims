<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# Coordinate system finder (`geodesy.crs.search`)

## Method

There is no arithmetic here, only lookup, so the method is the index and the matching rules. The index is generated from the EPSG dataset, v13.102 through PROJ 9.9.0, by `tools/codegen/crs_registry.py`: the four geographic systems (WGS 84, NAD83, NAD83(2011), NAD27), Web Mercator, every UTM zone on WGS 84, NAD83, and NAD83(2011), and every SPCS83 zone on NAD83 and NAD83(2011) in meters and in each foot EPSG defines for it, 624 systems in all, each with EPSG's own name, unit, and area-of-use box and tied to its UTM zone or NGS zone code. The SPCS2022 beta zones come from the NGS asset the SPCS2022 tools use, and are read only when a name or a point could match one. A code matches exactly, with or without an EPSG: prefix; a name matches when every word given is a word of the system's name or of its kind's common names (state plane, UTM, geographic); a point matches every system whose box contains it. Given both, a system must match both. Rows come most local first: State Plane 1983, State Plane 2022, UTM, Web Mercator, then geographic, by code within each, with a system whose whole name was typed placed first.

## Equations

- Box test: s ≤ φ ≤ n, and w ≤ λ ≤ e, or, for a box that crosses 180° (w > e), λ ≥ w or λ ≤ e.
- Word match: every lowercase alphanumeric word of the query is among the words of the name and kind.
- Order: (rank of kind, code), with an exact name match moved to the front.

## Symbols and units

φ and λ are latitude and longitude in degrees; w, s, e, n the west, south, east, and north edges of an area of use in degrees, as EPSG and NGS publish them.

## Domain

Any query text up to 80 characters, any point on the globe, or both. At most 200 rows are returned, with the full count and a note to narrow the search.

## Approximations

Areas of use are rectangles, not the state, county, or national boundaries behind them, so near a line more than one system covers a point, and a point just outside a state can match its zone. EPSG's boxes are published to two decimal places. The index holds only what the converters here produce, and the SPCS2022 zones are beta.

## Worked example

- sourcePublisher: IOGP (the EPSG dataset) through PROJ; NGS for SPCS2022
- sourceTitle: `projinfo --list-crs --bbox` with PROJ's own area-of-use test, and the NGS SPCS2022 beta bounds
- sourceEdition: EPSG v13.102 in PROJ 9.9.0; SPCS2022 beta of 2026-06-01
- sourceLocator: the systems intersecting (39.74, −104.99), kept to the families the index holds
- independent: yes
- inputs: latitude 39.74, longitude −104.99 (downtown Denver)
- outputs: 19 systems: SPCS83 Colorado Central and North on NAD83 and NAD83(2011) in meters and US survey feet, SPCS2022 Colorado, Colorado Rocky Mountain, and Colorado Metro Denver, UTM 13N on NAD83, NAD83(2011), and WGS 84, Web Mercator, and the four geographic systems
- tolerance: exact
- verifiedBy: golden vector v001, run by the core on every build
- verifiedOn: 2026-10-09

The expected list is PROJ's answer to its own spatial query, filtered to the index's families by name, not the index's answer: when the two disagreed at a test point, PROJ showed that five SPCS83 boxes in the zone table were older than EPSG's, and the table was regenerated.

## Differential tests

- `tools/vectors/gen_crs_search.py`: 24 vectors from PROJ 9.9.0's area-of-use query at eleven points (Denver, Honolulu, the Aleutians across 180°, the open Atlantic, a point on a UTM zone line, and others), EPSG codes named by projinfo, word searches, and SPCS2022 codes from the NGS bounds

## Invariants

- `core/crates/gp-geodesy/tests/crs.rs` `every_system_is_found_by_its_code_and_its_name`: each of the 624 systems is the one answer to its code and among the answers to its name
- `core/crates/gp-geodesy/tests/crs.rs` `a_code_lookup_does_not_read_the_zone_file`: a code lookup leaves the SPCS2022 asset alone; a name search reads it
- `core/crates/gp-geodesy/tests/crs.rs` `every_system_covers_the_middle_of_its_area`: a point at the middle of each box finds that system, across 180° too
- `core/crates/gp-geodesy/tests/crs.rs` `the_state_plane_answers_agree_with_the_zone_lookup`: on a grid over the US, the SPCS83 zones listed are exactly the zone lookup's
- `core/crates/gp-geodesy/tests/crs.rs` `every_point_on_land_or_sea_has_its_wgs84_utm_zone`: the standard WGS 84 UTM zone is always among the answers
- `core/crates/gp-geodesy/tests/crs.rs` `nothing_is_left_unsaid`: no input is refused with the field named, an unknown code says nothing matched, and every row names its converter or is WGS 84 itself
