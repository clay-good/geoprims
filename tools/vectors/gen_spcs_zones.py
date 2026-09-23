#!/usr/bin/env python3
"""Golden vectors for geodesy.spcs.zone-lookup, from the EPSG registry.

The tool answers two questions: which SPCS83 zones belong to a state or match
a name, and which zones' area of use covers a point. Both are lookups into the
same published table, and PROJ ships that table -- so `pyproj.database
.query_crs_info` gives the zones and their EPSG area-of-use boxes without
transcribing anything.

Only the metre-based NAD83 zones are counted. Each has a foot-based twin with
its own EPSG code and the same extent, and counting both would double every
answer; the tool reports the zone once with its feet unit as a property.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_spcs_zones.py
"""
import json
import re
import sys
from pathlib import Path

from pyproj.database import query_crs_info

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
SRC = ("The EPSG registry through pyproj: NAD83 State Plane projected CRSs and their "
       "areas of use (tools/vectors/gen_spcs_zones.py)")
VER = "PROJ 9.3.0 EPSG database"

STATE_QUERIES = [
    "Colorado", "Texas", "California", "Alaska", "Florida", "New York",
    "Montana", "Hawaii", "Pennsylvania", "Wyoming", "Utah", "Oregon",
]
NAME_QUERIES = ["Colorado Central", "Texas North Central", "California Zone 3", "Alaska zone 1"]
POINTS = [
    (39.7392, -104.9903), (40.4406, -79.9959), (29.7604, -95.3698),
    (34.0522, -118.2437), (61.2181, -149.9003), (25.7617, -80.1918),
    (47.6062, -122.3321), (21.3069, -157.8583),
]


def zones():
    """The metre-based NAD83 State Plane zones, with their EPSG extents."""
    out = []
    for r in query_crs_info(auth_name="EPSG", pj_types=["PROJECTED_CRS"]):
        # "NAD83 / Colorado Central" is the metre zone; "(ftUS)" and "(ft)" are
        # its twins, and the zone is the same ground either way.
        if not r.name.startswith("NAD83 / ") or "(" in r.name:
            continue
        if not r.area_of_use:
            continue
        name = r.name[len("NAD83 / "):]
        # The NAD83 prefix is shared by UTM and by a statewide projection per
        # state -- Texas Centric Albers, California Albers, Florida GDL Albers,
        # the Texas State Mapping System. Those cover a whole state in one
        # piece and are not SPCS zones, so a lookup that counted them would
        # report Texas as eight zones instead of its five.
        # No SPCS zone name contains "Lambert": the zones are named North,
        # South, Central, East, West, or "zone N". Anything Lambert here is a
        # statewide or continental system -- Wyoming Lambert, Canada Atlas
        # Lambert, Statistics Canada Lambert -- whose area of use is wide
        # enough to cover points in several states at once.
        if re.search(r"UTM zone|Albers|Centric|Conus|State Mapping|Lambert|"
                     r"Canada|Maine 2000|\(CORS|Oregon GIC", name):
            continue
        out.append((name, int(r.code), r.area_of_use.bounds))
    return out


ZONES = zones()


def matches(query, name):
    """Every word of the query appears as a whole word in the zone name.

    Whole words, not a prefix: "Alaska zone 1" names one zone, and a prefix
    match would hand back zone 10 with it."""
    words = re.findall(r"\w+", name.lower())
    return all(w in words for w in re.findall(r"\w+", query.lower()))


def expectation(codes):
    """What to pin for a set of matching zones.

    The count is the answer. Which zone is listed first is a presentation
    choice -- the core does not order by EPSG code -- so the code is pinned
    only when there is exactly one zone and the order cannot differ."""
    expect = {"ok": True, "result.count": float(len(codes))}
    tol = {"result.count": {"abs": 0}}
    if len(codes) == 1:
        expect["result.zones.0.epsg"] = float(codes[0])
        tol["result.zones.0.epsg"] = {"abs": 0}
    return expect, tol


def main():
    path = ROOT / "geodesy.spcs.zone-lookup.jsonl"
    existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start > 8:
        raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
    rows = []
    i = start
    for q in STATE_QUERIES + NAME_QUERIES:
        i += 1
        want = sorted(code for name, code, _ in ZONES if matches(q, name))
        rows.append((i, {"query": q}, *expectation(want)))
    for lat, lon in POINTS:
        i += 1
        want = sorted(code for _, code, (w, s, e, n) in ZONES if s <= lat <= n and w <= lon <= e)
        rows.append((i, {"lat": lat, "lon": lon}, *expectation(want)))
    with path.open("a") as f:
        for i, inp, expect, tol in rows:
            f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                "source": SRC, "sourceVersion": VER, "tolerance": tol}) + "\n")
    print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
