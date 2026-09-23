#!/usr/bin/env python3
"""Containment fixture for indexing.s2.covering, from s2sphere.

A covering is not unique. S2's own RegionCoverer uses a priority-queue
heuristic; this core refines from the six faces. Both produce *valid*
coverings of the same region, and they are different sets -- so pinning one
implementation's cell list against the other's would be pinning a choice
between equals, and would fail for a reason that is not a defect.

What is not a choice is what a covering has to *be*: every cell inside the
level range, no more cells than the budget, and the union containing the
region. The last is the one worth an outside opinion, so this writes, for a
set of regions, sample points s2sphere places inside them together with the
token of every ancestor cell of each point in the level range. A covering
contains a point exactly when it holds one of that point's ancestors, which is
a set intersection the test can check without trusting the core's own idea of
where a cell is.

    python3 tools/vectors/gen_s2_cover.py OUT.jsonl
"""
import json
import math
import random
import sys

import s2sphere as s2

# (kind, params, min_level, max_level, max_cells) -- rectangles and caps over
# every face of the cube, spanning the antimeridian and both poles.
CASES = [
    ("rect", (40.43, 40.46, -80.01, -79.96), 10, 16, 8),
    ("rect", (40.43, 40.46, -80.01, -79.96), 12, 18, 32),
    ("rect", (-34.0, -33.8, 151.1, 151.3), 8, 14, 12),
    ("rect", (51.4, 51.6, -0.3, 0.1), 10, 16, 20),
    ("rect", (35.6, 35.8, 139.6, 139.8), 11, 17, 24),
    ("rect", (-1.0, 1.0, -1.0, 1.0), 4, 10, 8),
    ("rect", (-5.0, 5.0, 175.0, -175.0), 4, 10, 96),
    ("rect", (80.0, 89.0, -30.0, 30.0), 3, 9, 12),
    ("rect", (-89.0, -80.0, 100.0, 160.0), 3, 9, 12),
    ("rect", (0.0, 45.0, 90.0, 135.0), 2, 8, 10),
    ("cap", (40.44, -79.99, 5000.0), 8, 14, 12),
    ("cap", (0.0, 0.0, 100000.0), 4, 10, 16),
    ("cap", (-33.87, 151.21, 2000.0), 10, 16, 20),
    ("cap", (89.0, 0.0, 50000.0), 5, 11, 12),
    ("cap", (-45.0, -135.0, 20000.0), 7, 13, 16),
    ("cap", (25.2, 55.27, 1000.0), 12, 18, 24),
]
EARTH_M = 6_371_010.0


def region(kind, params):
    if kind == "rect":
        s, n, w, e = params
        if w <= e:
            return s2.LatLngRect.from_point_pair(
                s2.LatLng.from_degrees(s, w), s2.LatLng.from_degrees(n, e)
            )
        # Across the antimeridian: build the interval explicitly, since
        # from_point_pair would read it the short way round.
        return s2.LatLngRect(
            s2.LineInterval(math.radians(s), math.radians(n)),
            s2.SphereInterval(math.radians(w), math.radians(e)),
        )
    lat, lon, radius_m = params
    centre = s2.LatLng.from_degrees(lat, lon).to_point()
    return s2.Cap.from_axis_angle(centre, s2.Angle.from_radians(radius_m / EARTH_M))


def samples(kind, params, n, rnd):
    """Points s2sphere agrees are inside the region."""
    r = region(kind, params)
    out = []
    guard = 0
    while len(out) < n and guard < 20000:
        guard += 1
        if kind == "rect":
            s, nn, w, e = params
            lat = rnd.uniform(s, nn)
            lon = rnd.uniform(w, e) if w <= e else (rnd.uniform(w, e + 360.0) + 540.0) % 360.0 - 180.0
        else:
            lat0, lon0, radius_m = params
            ang = radius_m / EARTH_M * rnd.uniform(0.0, 0.95)
            brg = rnd.uniform(0.0, 2 * math.pi)
            p0 = math.radians(lat0)
            lat = math.asin(math.sin(p0) * math.cos(ang) + math.cos(p0) * math.sin(ang) * math.cos(brg))
            lon = math.radians(lon0) + math.atan2(
                math.sin(brg) * math.sin(ang) * math.cos(p0),
                math.cos(ang) - math.sin(p0) * math.sin(lat),
            )
            lat, lon = math.degrees(lat), (math.degrees(lon) + 540.0) % 360.0 - 180.0
        ll = s2.LatLng.from_degrees(lat, lon)
        # A rectangle takes a LatLng; a cap takes a point on the sphere.
        inside = r.contains(ll) if kind == "rect" else r.contains(ll.to_point())
        if inside:
            out.append((lat, lon))
    if len(out) < n:
        raise SystemExit(f"only {len(out)} points inside {kind} {params}")
    return out


def main():
    out = sys.argv[1]
    rnd = random.Random(20260923)
    rows = []
    for kind, params, lo, hi, budget in CASES:
        pts = []
        for lat, lon in samples(kind, params, 8, rnd):
            cid = s2.CellId.from_lat_lng(s2.LatLng.from_degrees(lat, lon))
            pts.append({
                "lat": lat,
                "lon": lon,
                # A covering holds this point exactly when it holds one of
                # these: the point's own cell at each level it may use.
                "ancestors": [cid.parent(lv).to_token() for lv in range(lo, hi + 1)],
            })
        rows.append({
            "kind": kind, "params": list(params),
            "min_level": lo, "max_level": hi, "max_cells": budget,
            "points": pts,
        })
    with open(out, "w") as f:
        f.write(f"# s2sphere {s2.__name__} 0.2.5: points inside each region and their ancestor tokens\n")
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"{len(rows)} regions, {sum(len(r['points']) for r in rows)} points -> {out}")


if __name__ == "__main__":
    main()
