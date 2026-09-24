#!/usr/bin/env python3
"""Polygon repair against GEOS on a grid, for geometry.validity.make-valid (1.1.0).

Writes core/crates/gp-geometry/tests/data/make_valid_geos.json: 600 rings of
4 to 8 corners on a 7 x 7 grid that GEOS calls invalid (crossing, touching,
and retracing themselves), each with its repair by the even-odd rule, which
is the rule the tool states. The reference is built from GEOS's own parts:
the ring's linework noded by GEOS (unary_union), cut into faces by GEOS
(polygonize), the faces kept whose interior point the ring encloses an odd
number of times (a ray count written here), and the kept faces merged by
GEOS. GEOS's make_valid itself is not the reference: its "linework" method
reads a ring that runs back over itself differently from the even-odd rule.

The fixture holds the area in grid squares and the number of parts; the
area on the ellipsoid, with the grid placed at 40° N, 105° W at a step of
0.00001°, is checked by core/crates/gp-geometry/tests/make_valid_parity.rs
through one grid square's area. Requires shapely and geographiclib."""
import json
import random
from pathlib import Path

from geographiclib.geodesic import Geodesic
from shapely.geometry import LineString, Polygon
from shapely.ops import polygonize, unary_union

STEP = 1e-5


def even_odd(pts):
    ring = [(x, y) for y, x in pts]

    def inside(p):
        px, py, ins = p.x, p.y, False
        for (ax, ay), (bx, by) in zip(ring, ring[1:] + ring[:1]):
            if (ay > py) != (by > py) and px < ax + (py - ay) * (bx - ax) / (by - ay):
                ins = not ins
        return ins

    faces = [f for f in polygonize(unary_union(LineString(ring + [ring[0]]))) if inside(f.representative_point())]
    u = unary_union(faces)
    parts = [g for g in (u.geoms if hasattr(u, "geoms") else [u]) if g.geom_type == "Polygon" and g.area > 0]
    return sum(g.area for g in parts), len(parts)


def main():
    rng = random.Random(11)
    cases = []
    while len(cases) < 600:
        n = rng.randint(4, 8)
        pts = [(rng.randint(0, 6), rng.randint(0, 6)) for _ in range(n)]
        if any(pts[i] == pts[(i + 1) % n] for i in range(n)) or len(set(pts)) != n:
            continue
        if Polygon([(x, y) for y, x in pts]).is_valid:
            continue
        area, parts = even_odd(pts)
        cases.append({"ring": pts, "area": area, "parts": parts})
    p = Geodesic.WGS84.Polygon()
    for y, x in [(3, 3), (3, 4), (4, 4), (4, 3)]:
        p.AddPoint(40 + y * STEP, -105 + x * STEP)
    out = {"cell_m2": abs(p.Compute()[2]), "cases": cases}
    Path("core/crates/gp-geometry/tests/data/make_valid_geos.json").write_text(json.dumps(out, separators=(",", ":")) + "\n")
    print(len(cases), "rings")


if __name__ == "__main__":
    main()
