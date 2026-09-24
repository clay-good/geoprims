#!/usr/bin/env python3
"""Point in polygon with planar edges against GEOS, exactly
(geometry.predicate.point-in-polygon 1.1.0).

Writes core/crates/gp-geometry/tests/data/pip_planar_geos.json: 300 polygons
(some with a hole) with corners on a small integer grid, each with 12 points
on the grid and on half steps, so many sit exactly on an edge or a corner.
GEOS decides each on the same doubles with exact predicates: on the boundary
when the polygon's boundary meets the point, inside when the polygon
contains it, outside otherwise. Grid coordinates are used as longitude
(x) and latitude (y) directly. Requires shapely."""
import json
import random
import sys
from pathlib import Path

from shapely.geometry import Point, Polygon

sys.path.insert(0, str(Path(__file__).parent))
from gen_relate_geos import rand_polygon  # noqa: E402


def main():
    rng = random.Random(3)
    cases = []
    for _ in range(300):
        pa, ha = rand_polygon(rng)
        poly = Polygon([(x, y) for y, x in pa], [[(x, y) for y, x in h] for h in ha])
        pts = [(rng.randint(-1, 9) + rng.choice([0, 0, 0.5]), rng.randint(-1, 9) + rng.choice([0, 0, 0.5])) for _ in range(12)]
        exp = []
        for y, x in pts:
            p = Point(x, y)
            exp.append("on-boundary" if poly.boundary.intersects(p) else ("inside" if poly.contains(p) else "outside"))
        cases.append({"outline": pa, "holes": ha, "points": pts, "expect": exp})
    out = Path("core/crates/gp-geometry/tests/data/pip_planar_geos.json")
    out.write_text(json.dumps(cases, separators=(",", ":")) + "\n")
    print(len(cases), "polygons,", sum(len(c["points"]) for c in cases), "points,", sum(e == "on-boundary" for c in cases for e in c["expect"]), "on the boundary")


if __name__ == "__main__":
    main()
