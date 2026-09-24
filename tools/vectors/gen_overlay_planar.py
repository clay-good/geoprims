#!/usr/bin/env python3
"""Planar edges in the boolean overlay (geometry.overlay.boolean 1.2.0):
GEOS through shapely on longitude and latitude as they are written, the way
GIS software treats EPSG:4326 data, with the area of each GEOS result from
geographiclib's PolygonArea on its rings cut into 0.01° straight pieces.

Appends vectors to core/vectors/geometry.overlay.boolean.jsonl (existing
lines left byte for byte). Requires shapely and geographiclib."""
import json
import math
from pathlib import Path

import shapely
from geographiclib.geodesic import Geodesic
from shapely.geometry import Polygon

PATH = Path("core/vectors/geometry.overlay.boolean.jsonl")
TAG = "gen_overlay_planar.py"
G = Geodesic.WGS84


def dense(ring):
    out = []
    for (x0, y0), (x1, y1) in zip(ring, ring[1:]):
        n = max(1, math.ceil(max(abs(x1 - x0), abs(y1 - y0)) / 0.01))
        out += [(x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n) for k in range(n)]
    return out


def area(geom):
    total = 0.0
    polys = [geom] if geom.geom_type == "Polygon" else list(getattr(geom, "geoms", []))
    for p in polys:
        for k, ring in enumerate([p.exterior, *p.interiors]):
            pa = G.Polygon()
            for x, y in dense(list(ring.coords)):
                pa.AddPoint(y, x)
            a = abs(pa.Compute(False, True)[2])
            total += a if k == 0 else -a
    return total, len(polys)


def rows(poly):
    return [{"lat": y, "lon": x} for x, y in list(poly.exterior.coords)[:-1]]


CASES = [
    # Two one-degree boxes at 60° N: the straight top edges are 20 km north
    # of the geodesics' paths, so planar and geodesic answers differ.
    ("boxes at 60° N", Polygon([(10, 60), (12, 60), (12, 61), (10, 61)]), Polygon([(11, 60.5), (13, 60.5), (13, 61.5), (11, 61.5)])),
    # A shape across the antimeridian, written with longitudes past 180.
    ("across the antimeridian", Polygon([(178, -17), (182, -17), (182, -15), (178, -15)]), Polygon([(180, -16.5), (184, -16.5), (184, -14), (180, -14)])),
]


def main():
    lines = PATH.read_text().splitlines()
    kept = [l for l in lines if TAG not in l]
    n0, new = len(kept), []
    for title, a, b in CASES:
        for op, fn in (("intersection", a.intersection), ("union", a.union), ("difference", a.difference), ("symmetric-difference", a.symmetric_difference)):
            res = fn(b)
            m2, parts = area(res)
            new.append({
                "id": f"v{n0 + len(new) + 1:03d}",
                "input": {"polygon_a": rows(a), "polygon_b": rows(b), "operation": op, "edges": "planar"},
                "expect": {"result.parts": parts, "result.area.value": m2 / 1e6, "ok": True},
                "source": f"GEOS {shapely.geos_version_string} (shapely {shapely.__version__}) on longitude and latitude, {title}; area by geographiclib on 0.01° pieces ({TAG})",
                "sourceVersion": f"GEOS {shapely.geos_version_string}",
                "tolerance": {"result.area.value": {"rel": 1e-9}, "result.parts": {"abs": 0}},
            })
    PATH.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print(n0, "->", n0 + len(new))


if __name__ == "__main__":
    main()
