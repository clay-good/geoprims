#!/usr/bin/env python3
"""Planar hull edges in geometry.shape.enclosing (1.1.0): GEOS's convex hull
through shapely on longitude and latitude as written, the way GIS software
computes it on EPSG:4326 data, with its area from geographiclib on the hull
cut into 0.01° straight pieces.

Appends vectors to core/vectors/geometry.shape.enclosing.jsonl (existing
lines left byte for byte). Requires shapely and geographiclib."""
import json
import math
from pathlib import Path

import shapely
from geographiclib.geodesic import Geodesic
from shapely.geometry import MultiPoint

PATH = Path("core/vectors/geometry.shape.enclosing.jsonl")
TAG = "gen_hull_planar.py"


def area(ring):
    pa = Geodesic.WGS84.Polygon()
    for (x0, y0), (x1, y1) in zip(ring, ring[1:]):
        n = max(1, math.ceil(max(abs(x1 - x0), abs(y1 - y0)) / 0.01))
        for k in range(n):
            pa.AddPoint(y0 + (y1 - y0) * k / n, x0 + (x1 - x0) * k / n)
    return abs(pa.Compute(False, True)[2])


CASES = [
    # The straight top edge runs along 60° N; the geodesic between its ends
    # bows to 60.38° (tan φ = tan 60° / cos 10°), so (60.2, 10) is a corner here and inside the
    # great-circle hull.
    ("a point between a straight edge and its geodesic", [(0, 60), (20, 60), (10, 55), (10, 60.2)]),
    ("a scatter across the antimeridian, written past 180", [(178, -17), (181.5, -16.2), (183, -15), (179, -14.5), (180.2, -15.8)]),
    ("seven points in Colorado", [(-105.0, 40.0), (-104.996, 40.004), (-104.99, 40.001), (-104.993, 39.997), (-104.994, 40.002), (-104.998, 39.999), (-104.992, 40.006)]),
]


def main():
    lines = PATH.read_text().splitlines()
    kept = [l for l in lines if TAG not in l]
    n0, new = len(kept), []
    for title, pts in CASES:
        hull = MultiPoint(pts).convex_hull
        ring = list(hull.exterior.coords)
        new.append({
            "id": f"v{n0 + len(new) + 1:03d}",
            "input": {"points": [{"lat": y, "lon": x} for x, y in pts], "edges": "planar"},
            "expect": {"result.hull_count": len(ring) - 1, "result.hull_area.value": area(ring) / 1e6, "ok": True},
            "source": f"GEOS {shapely.geos_version_string} (shapely {shapely.__version__}) convex hull on longitude and latitude, {title}; area by geographiclib on 0.01° pieces ({TAG})",
            "sourceVersion": f"GEOS {shapely.geos_version_string}",
            "tolerance": {"result.hull_count": {"abs": 0}, "result.hull_area.value": {"rel": 1e-9}},
        })
    PATH.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print(n0, "->", n0 + len(new))


if __name__ == "__main__":
    main()
