#!/usr/bin/env python3
"""Golden vectors for geometry.shape.bbox from geographiclib and the RFC.

Two things have to be right and they fail differently.

The latitude extremes are not the endpoints: a geodesic bulges poleward, and
the box has to reach its vertex. The core finds that vertex with Clairaut's
relation; this finds it by bisecting on where Karney's geographiclib reports
the azimuth passing due east or due west, which is the same condition solved
by a different library and a different method.

The longitude box is the complement of the largest gap on the circle, which is
RFC 7946 section 5.2's rule, implemented here from the text rather than from
the core.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_bbox_geodesic.py
"""
import json
from pathlib import Path

from geographiclib.geodesic import Geodesic

G = Geodesic.WGS84
OUT = Path(__file__).resolve().parents[2] / "core/vectors/geometry.shape.bbox.jsonl"
SRC = (
    "latitude extremes by bisecting Karney's geographiclib for where the azimuth "
    "passes due east or west; longitude box by RFC 7946 section 5.2's largest-gap rule"
)
VER = "geographiclib 2.1; RFC 7946"


def edge_lat_extremes(a, b):
    """The least and greatest latitude reached along the geodesic from a to b."""
    inv = G.Inverse(a[0], a[1], b[0], b[1])
    line = G.Line(a[0], a[1], inv["azi1"])
    s13 = inv["s12"]
    lo_lat, hi_lat = min(a[0], b[0]), max(a[0], b[0])

    def azi(s):
        return line.Position(s)["azi2"]

    # The azimuth runs monotonically through 90 (northern vertex) or 270 /
    # -90 (southern vertex) at most once on a geodesic shorter than half the
    # Earth. Bisect for each crossing that is bracketed.
    for target in (90.0, -90.0):
        f0, f1 = azi(0.0) - target, azi(s13) - target
        if f0 * f1 >= 0.0:
            continue
        lo, hi = 0.0, s13
        for _ in range(200):
            m = (lo + hi) / 2.0
            if (azi(m) - target) * f0 > 0.0:
                lo = m
            else:
                hi = m
        lat = line.Position((lo + hi) / 2.0)["lat2"]
        lo_lat, hi_lat = min(lo_lat, lat), max(hi_lat, lat)
    return lo_lat, hi_lat


def arcs_of(points, shape):
    """The arcs of longitude the shape covers, as (start, length) in degrees."""
    arcs = [((p[1] + 360.0) % 360.0, 0.0) for p in points]
    if shape in ("line", "polygon"):
        n = len(points)
        last = n - 1 if shape == "line" else n
        for i in range(last):
            a, b = points[i], points[(i + 1) % n]
            d = ((b[1] - a[1] + 180.0) % 360.0) - 180.0  # the shorter way
            start = (a[1] + 360.0) % 360.0
            arcs.append((start if d >= 0 else (start + d) % 360.0, abs(d)))
    return arcs


def longitude_box(points, shape):
    """(west, east, span), the complement of the largest uncovered gap."""
    arcs = arcs_of(points, shape)
    # Merge the covered arcs on the circle by sweeping.
    events = []
    for start, length in arcs:
        events.append((start, start + length))
    events.sort()
    merged = []
    for s, e in events:
        if merged and s <= merged[-1][1] + 1e-12:
            merged[-1] = (merged[-1][0], max(merged[-1][1], e))
        else:
            merged.append((s, e))
    # Wrap the last into the first if they meet across 360.
    if len(merged) > 1 and merged[-1][1] >= merged[0][0] + 360.0 - 1e-12:
        merged[0] = (merged[-1][0] - 360.0, merged[0][1])
        merged.pop()
    if len(merged) == 1:
        west, east = merged[0]
        if east - west >= 360.0 - 1e-12:
            return -180.0, 180.0, 360.0
    else:
        gaps = []
        for i, (s, e) in enumerate(merged):
            nxt = merged[(i + 1) % len(merged)][0] + (360.0 if i == len(merged) - 1 else 0.0)
            gaps.append((nxt - e, e, nxt))
        gap, gap_start, gap_end = max(gaps)
        west, east = gap_end, gap_start + 360.0
    span = east - west
    return ((west + 180.0) % 360.0) - 180.0, ((east + 180.0) % 360.0) - 180.0, span


def box(points, shape):
    south = min(p[0] for p in points)
    north = max(p[0] for p in points)
    if shape in ("line", "polygon"):
        n = len(points)
        last = n - 1 if shape == "line" else n
        for i in range(last):
            lo, hi = edge_lat_extremes(points[i], points[(i + 1) % n])
            south, north = min(south, lo), max(north, hi)
    west, east, span = longitude_box(points, shape)
    return {"west": west, "south": south, "east": east, "north": north, "lon_span": span}


CASES = [
    ("a long mid-latitude line, which bulges poleward", "line", [(60.0, -60.0), (60.0, 60.0)]),
    ("the same pair as bare points, which does not", "points", [(60.0, -60.0), (60.0, 60.0)]),
    ("a southern-hemisphere line", "line", [(-45.0, 10.0), (-45.0, 80.0)]),
    ("a transatlantic line", "line", [(40.6413, -73.7781), (51.47, -0.4543)]),
    ("a triangle in the northern mid-latitudes", "polygon", [(45.0, -100.0), (48.0, -80.0), (42.0, -90.0)]),
    ("a box across the antimeridian", "polygon", [(-17.0, 178.0), (-17.0, -178.0), (-15.0, -178.0), (-15.0, 178.0)]),
    ("points either side of the antimeridian", "points", [(10.0, 179.0), (12.0, -179.0), (11.0, 175.0)]),
    # Spread over more than half the world. The longitudes are chosen so the
    # largest gap is unique (120 deg, from 50 to 170): with three equal gaps
    # the rule has nothing to choose between them and two correct
    # implementations pick different boxes of the same width, which is an
    # ambiguity in the rule and not something a vector should pin.
    ("points spread over more than half the world", "points", [(0.0, -170.0), (0.0, -55.0), (0.0, 50.0), (0.0, 170.0)]),
    ("a narrow equatorial strip", "polygon", [(-0.5, 20.0), (-0.5, 24.0), (0.5, 24.0), (0.5, 20.0)]),
    ("a high-latitude quadrilateral", "polygon", [(78.0, 15.0), (78.0, 35.0), (80.0, 35.0), (80.0, 15.0)]),
]


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 13:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows = []
    for i, (name, shape, pts) in enumerate(CASES, start=start + 1):
        b = box(pts, shape)
        rows.append({
            "id": f"v{i:03d}",
            "input": {
                "points": [{"lat": a, "lon": o} for a, o in pts],
                "shape": shape,
            },
            "expect": {
                "result.west.value": b["west"],
                "result.south.value": b["south"],
                "result.east.value": b["east"],
                "result.north.value": b["north"],
                "result.lon_span.value": b["lon_span"],
                "ok": True,
            },
            "source": f"{SRC}: {name}",
            "sourceVersion": VER,
            # The bisection converges to the last bit, so these are held at a
            # nanodegree -- about 0.1 mm -- rather than at a round figure.
            "tolerance": {
                "result.west.value": {"abs": 1e-9},
                "result.south.value": {"abs": 1e-9},
                "result.east.value": {"abs": 1e-9},
                "result.north.value": {"abs": 1e-9},
                "result.lon_span.value": {"abs": 1e-9},
            },
        })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
