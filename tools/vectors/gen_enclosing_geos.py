#!/usr/bin/env python3
"""Golden vectors for geometry.shape.enclosing from GEOS.

Three shapes come back from this tool and each has its own reference:

- the smallest enclosing circle, from GEOS's `minimum_bounding_circle` and
  `minimum_bounding_radius` -- its own implementation of Welzl's algorithm;
- the convex hull, from GEOS's `convex_hull`, with its area measured back on
  the ellipsoid by geographiclib's PolygonArea rather than in the plane, since
  the azimuthal equidistant projection keeps distances from its centre but not
  areas;
- the smallest rotated rectangle, from GEOS's `minimum_rotated_rectangle`, with
  its two side lengths read off in the plane where they are true.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_enclosing_geos.py
"""
import json
import math
import random
from pathlib import Path

from geographiclib.geodesic import Geodesic
from geographiclib.polygonarea import PolygonArea
from pyproj import CRS, Transformer
from shapely import minimum_bounding_circle, minimum_bounding_radius
from shapely.geometry import MultiPoint

G = Geodesic.WGS84
OUT = Path(__file__).resolve().parents[2] / "core/vectors/geometry.shape.enclosing.jsonl"
SRC = (
    "GEOS 3.11.4 through shapely on PROJ's azimuthal equidistant plane: "
    "minimum_bounding_circle, convex_hull and minimum_rotated_rectangle, with the "
    "hull's area measured on the ellipsoid by geographiclib's PolygonArea"
)
VER = "GEOS 3.11.4 / PROJ 9.3.0 / geographiclib 2.1"


def plane_for(pts):
    lat0 = sum(p[0] for p in pts) / len(pts)
    base = pts[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in pts) / len(pts)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    return (
        Transformer.from_crs("EPSG:4326", crs, always_xy=True),
        Transformer.from_crs(crs, "EPSG:4326", always_xy=True),
    )


def enclosing(pts):
    fwd, inv = plane_for(pts)
    mp = MultiPoint([fwd.transform(lo, la) for la, lo in pts])

    circle = minimum_bounding_circle(mp).centroid
    clon, clat = inv.transform(circle.x, circle.y)

    # Two points, or points that happen to be collinear in the plane, give a
    # hull that is a line rather than a polygon, and it has no `exterior`.
    hull = mp.convex_hull
    if hull.geom_type == "Polygon":
        corners = list(hull.exterior.coords)[:-1]
    else:
        corners = list(hull.coords)
    pa = PolygonArea(G)
    for x, y in corners:
        lon, lat = inv.transform(x, y)
        pa.AddPoint(lat, lon)

    box = mp.minimum_rotated_rectangle
    if box.geom_type == "Polygon":
        rect = list(box.exterior.coords)[:-1]
        sides = sorted(math.dist(rect[i], rect[(i + 1) % 4]) for i in range(4))
        sides = [sides[0], sides[-1]]
    else:
        coords = list(box.coords)
        sides = [0.0, math.dist(coords[0], coords[-1])]

    return {
        "circle_lat": clat,
        "circle_lon": clon,
        "circle_radius": minimum_bounding_radius(mp),
        "hull_count": len(corners),
        "hull_area": abs(pa.Compute(False, True)[2]) / 1e6,
        "rect_width": sides[0],
        "rect_length": sides[1],
    }


def cluster(lat, lon, spread_deg, n, seed):
    rnd = random.Random(seed)
    return [
        (lat + rnd.uniform(-spread_deg, spread_deg), lon + rnd.uniform(-spread_deg, spread_deg))
        for _ in range(n)
    ]


CASES = [
    ("a quadrilateral", [(40.0, -105.0), (40.01, -104.99), (40.005, -104.97), (39.995, -104.98)]),
    ("a triangle", [(-20.0, 30.0), (-20.05, 30.12), (-20.11, 30.03)]),
    ("two points", [(51.5, -0.12), (48.86, 2.35)]),
    ("three points in a line", [(10.0, 20.0), (10.01, 20.01), (10.02, 20.02)]),
    ("a tight cluster", cluster(35.0, 139.0, 0.004, 9, 11)),
    ("a loose cluster", cluster(35.0, 139.0, 0.08, 12, 12)),
    ("a scatter at the equator", cluster(0.0, 0.0, 0.05, 8, 13)),
    ("a scatter at 70 north", cluster(70.0, 25.0, 0.06, 8, 14)),
    ("a southern scatter", cluster(-33.87, 151.2, 0.03, 10, 15)),
    ("points either side of the antimeridian", [(-17.0, 179.9), (-17.05, -179.92), (-16.95, -179.97), (-17.02, 179.95)]),
    ("a long thin spread", [(40.0, -105.0), (40.001, -104.9), (40.002, -104.8), (40.0015, -104.7)]),
    ("a square, where the rectangle is the hull", [(5.0, 100.0), (5.0, 100.05), (5.05, 100.05), (5.05, 100.0)]),
]


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 10:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows = []
    for i, (name, pts) in enumerate(CASES, start=start + 1):
        r = enclosing(pts)
        expect = {"ok": True, "result.hull_count": r["hull_count"]}
        tolerance = {"result.hull_count": {"abs": 0}}
        # The circle's centre is the loosest of these: over a twenty-kilometre
        # scatter the two implementations place it within 7.2e-6 deg, under a
        # metre, which is Welzl's algorithm converging from different starting
        # points rather than either being wrong.
        for k in ("circle_lat", "circle_lon"):
            expect[f"result.{k}.value"] = r[k]
            tolerance[f"result.{k}.value"] = {"abs": 5e-5}
        fields = ["circle_radius", "hull_area"]
        # A triangle's smallest-area enclosing rectangle is not unique: every
        # rectangle flush with one of its three edges has exactly twice the
        # triangle's area, so all three are minimal and two correct
        # implementations can return different ones. Pinning the sides there
        # would be pinning a choice between equals; the invariant test checks
        # the property that does hold -- the area -- instead.
        if r["hull_count"] != 3:
            fields += ["rect_width", "rect_length"]
        for k in fields:
            expect[f"result.{k}.value"] = r[k]
            tolerance[f"result.{k}.value"] = {"rel": 1e-5, "abs": 1e-9}
        rows.append({
            "id": f"v{i:03d}",
            "input": {
                "points": [{"lat": la, "lon": lo} for la, lo in pts],
                "options": {
                    "outputUnits": {
                        "circle_radius": "m",
                        "hull_area": "km2",
                        "rect_width": "m",
                        "rect_length": "m",
                    }
                },
            },
            "expect": expect,
            "source": f"{SRC}: {name}",
            "sourceVersion": VER,
            "tolerance": tolerance,
        })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
