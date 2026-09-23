<!-- Derivation note (trust/correctness-program layer A). The promotion gate checks these sections. -->
# State Plane zone lookup (`geodesy.spcs.zone-lookup`)

## Method

The State Plane Coordinate System of 1983 divides the United States into about 125 zones, each a Lambert conformal conic or transverse Mercator fitted to one part of one state so that scale error stays under about one part in ten thousand. Before you can project anything you have to know which zone, and that is a lookup rather than a computation.

Two ways in. **By name**: give a state, or a zone's own name, and the answer is every zone whose name contains every word you gave. Whole words, not a prefix — "Alaska zone 1" names one zone, and a prefix match would hand back zone 10 with it. **By point**: give a latitude and longitude and the answer is every zone whose area of use covers it, which is often two, because the published extents are rectangles and neighbouring zones overlap at their shared edge.

Each zone comes back with its NGS zone code, its EPSG code, which projection it uses, and which foot — US survey or international — its foot-based twin is defined in. That last one matters: the same zone exists twice in EPSG, once in metres and once in feet, and the two feet differ by two parts per million.

## Equations

None. This is a table.

## Symbols and units

Either `query`, a state or zone name, or `lat` and `lon`. Out come `zones` — each with `zone_code`, `zone_name`, `epsg`, `projection` and `feet` — the `count`, and a `note` saying which rule was applied.

## Domain

The SPCS83 zones of the United States and its territories. A point outside them returns nothing, which is the answer rather than an error.

## Approximations

The point lookup uses each zone's published rectangular area of use, not the state boundary. A rectangle around an irregular state includes ground the zone is not meant for, so a point just outside a state can match its zone, and a point near a zone boundary matches both. The count is a shortlist, not a decision.

## Worked example

- sourcePublisher: IOGP (the EPSG registry); PROJ contributors
- sourceTitle: The EPSG geodetic parameter registry, through `pyproj.database.query_crs_info`
- sourceEdition: PROJ 9.3.0's EPSG database
- sourceLocator: the NAD83 State Plane projected CRSs and their areas of use
- independent: yes
- inputs: 30 lookups — twelve states, four zone names including the one whose number is a prefix of another, and eight points from Denver to Honolulu
- outputs: the zone count, and the EPSG code where only one zone matches
- tolerance: exact
- verifiedBy: golden vectors v001 to v030, run by the core on every build
- verifiedOn: 2026-09-23

PROJ ships the EPSG table, so the reference enumerates the zones and their extents rather than transcribing them. Building it needed one filter worth recording: the `NAD83 /` prefix is shared with a statewide projection per state — Texas Centric Albers, California Albers, Florida GDL Albers, the Texas State Mapping System, Wyoming Lambert — and with continental ones like Canada Atlas Lambert whose extent covers Denver. None of those is an SPCS zone, and counting them made Texas come out as eight zones instead of its five. No SPCS zone name contains "Albers" or "Lambert"; the zones are North, South, Central, East, West, or "zone N".

Which zone is listed first is a presentation choice rather than a fact — the tool does not order by EPSG code — so the code is pinned only where a single zone matches and no order exists to disagree about.

## Differential tests

- `tools/vectors/gen_spcs_zones.py`: 24 of the 30 vectors, from the EPSG registry through pyproj
- `core/crates/gp-geodesy/tests/spcs.rs`: the zone parameters themselves against the published table
- `core/vectors/geodesy.spcs.zone-lookup.jsonl`: 30 lookups by name and by point

## Invariants

- `core/crates/gp-geodesy/tests/geodesy.rs` `spcs_zone_lookup_invariants`: every zone returned carries a zone code, an EPSG code, a projection and a feet unit, so no row is half filled; a zone found by name can be found again by a point inside its own extent; a more specific query returns a subset of a less specific one, so "Colorado Central" is among the zones "Colorado" returns; the count matches the number of zones listed; every EPSG code is one `geodesy.spcs.spcs83-forward` will accept, which is what makes the lookup useful rather than merely informative; and a point in the middle of the Pacific returns nothing rather than a nearest guess
