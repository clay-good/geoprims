#!/usr/bin/env python3
"""Containment fixture for indexing.s2.covering of polygons.

Like tools/vectors/gen_s2_cover.py: a covering is not unique, so this checks
what it has to be, a set of cells whose union holds the region. For each
polygon it writes sample points inside it, with the token of each point's own
cell at every level the covering may use (from s2sphere); a covering holds a
point exactly when it holds one of them.

s2sphere has no polygons, so which points are inside is decided here, by the
winding of the polygon's great-circle edges seen from the point, written
independently of the core. Half the points are scattered over the polygon and
half sit just inside an edge (0.2 m from it), where a covering that drops a
cell it should keep shows its hole.

    python3 tools/vectors/gen_s2_cover_polygon.py
"""
import json
import math
import random
from pathlib import Path

import s2sphere as s2

OUT = Path("core/crates/gp-indexing/tests/data/s2_cover_polygon.jsonl")
EARTH_M = 6_371_010.0


def vec(lat, lon):
    a, o = math.radians(lat), math.radians(lon)
    return (math.cos(a) * math.cos(o), math.cos(a) * math.sin(o), math.sin(a))


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def unit(v):
    n = math.sqrt(dot(v, v))
    return tuple(x / n for x in v)


def latlon(v):
    return math.degrees(math.asin(max(-1, min(1, v[2])))), math.degrees(math.atan2(v[1], v[0]))


def winding(ring, p):
    total = 0.0
    for a, b in zip(ring, ring[1:] + ring[:1]):
        ta = tuple(x - dot(a, p) * y for x, y in zip(a, p))
        tb = tuple(x - dot(b, p) * y for x, y in zip(b, p))
        total += math.atan2(dot(p, cross(ta, tb)), dot(ta, tb))
    return round(total / (2 * math.pi))


def inside(rings, p):
    w = abs(winding(rings[0], p))
    for h in rings[1:]:
        w -= abs(winding(h, p))
    return w > 0


def star(rnd, lat, lon, radius_m, n, dent):
    """A star-shaped polygon: n corners at random bearings, some pulled in."""
    pts = []
    for k in range(n):
        brg = 2 * math.pi * (k + rnd.uniform(0.1, 0.9)) / n
        r = radius_m * (rnd.uniform(0.3, 0.55) if rnd.random() < dent else rnd.uniform(0.8, 1.0))
        pts.append(destination(lat, lon, brg, r))
    return pts


def destination(lat, lon, brg, dist_m):
    ang = dist_m / EARTH_M
    p0 = math.radians(lat)
    la = math.asin(math.sin(p0) * math.cos(ang) + math.cos(p0) * math.sin(ang) * math.cos(brg))
    lo = math.radians(lon) + math.atan2(math.sin(brg) * math.sin(ang) * math.cos(p0), math.cos(ang) - math.sin(p0) * math.sin(la))
    return math.degrees(la), (math.degrees(lo) + 540.0) % 360.0 - 180.0


# (center lat, lon, radius m, corners, dent share, hole, min, max, budget)
CASES = [
    (40.44, -79.99, 3000, 7, 0.4, False, 10, 16, 12),
    (40.44, -79.99, 3000, 9, 0.5, True, 12, 18, 40),
    (-33.87, 151.21, 20000, 12, 0.5, False, 8, 14, 20),
    (51.5, -0.1, 500, 5, 0.3, True, 14, 20, 24),
    (0.0, 179.9, 60000, 8, 0.5, False, 6, 12, 16),
    (88.5, 30.0, 150000, 10, 0.4, False, 5, 11, 20),
    (-45.0, -135.0, 8000, 6, 0.6, True, 9, 15, 16),
    (35.68, 139.77, 1500, 11, 0.5, False, 12, 18, 32),
    (0.0, 0.0, 400000, 14, 0.5, True, 3, 9, 24),
    (25.2, 55.27, 200, 8, 0.5, False, 16, 22, 30),
]


def main():
    rnd = random.Random(20260924)
    rows = []
    for lat, lon, radius, n, dent, hole, lo, hi, budget in CASES:
        outline = star(rnd, lat, lon, radius, n, dent)
        rings_ll = [outline]
        if hole:
            rings_ll.append(list(reversed(star(rnd, lat, lon, radius * 0.2, 5, 0.0))))
        rings = [[vec(*p) for p in r] for r in rings_ll]
        pts = []
        guard = 0
        while len(pts) < 12 and guard < 50000:
            guard += 1
            p = destination(lat, lon, rnd.uniform(0, 2 * math.pi), radius * math.sqrt(rnd.random()))
            if inside(rings, vec(*p)):
                pts.append(p)
        # Just inside an edge: a point along it, stepped 0.2 m toward the
        # polygon's inside (tried either way; kept when the step lands in).
        while len(pts) < 24 and guard < 100000:
            guard += 1
            ring = rings[rnd.randrange(len(rings))]
            i = rnd.randrange(len(ring))
            a, b = ring[i], ring[(i + 1) % len(ring)]
            f = rnd.uniform(0.05, 0.95)
            m = unit(tuple(x * (1 - f) + y * f for x, y in zip(a, b)))
            nrm = unit(cross(a, b))
            step = 0.2 / EARTH_M
            for sgn in (1, -1):
                q = unit(tuple(x + sgn * step * y for x, y in zip(m, nrm)))
                if inside(rings, q):
                    pts.append(latlon(q))
                    break
        points = []
        for plat, plon in pts:
            cid = s2.CellId.from_lat_lng(s2.LatLng.from_degrees(plat, plon))
            points.append({"lat": plat, "lon": plon, "ancestors": [cid.parent(lv).to_token() for lv in range(lo, hi + 1)]})
        polygon = [{"lat": a, "lon": b} for a, b in rings_ll[0]]
        for k, h in enumerate(rings_ll[1:], 1):
            polygon += [{"lat": a, "lon": b, "ring": k} for a, b in h]
        rows.append({"polygon": polygon, "min_level": lo, "max_level": hi, "max_cells": budget, "points": points})
    with OUT.open("w") as f:
        f.write("# s2sphere ancestor tokens; inside decided by great-circle winding written here\n")
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(len(rows), "polygons,", sum(len(r["points"]) for r in rows), "points")


if __name__ == "__main__":
    main()
