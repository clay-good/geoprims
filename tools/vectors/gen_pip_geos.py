#!/usr/bin/env python3
"""Golden vectors for geometry.predicate.point-in-polygon from GEOS.

The core answers by geodesic winding number: it sums the turn in azimuth seen
from the point as it goes round the outline, by Karney's inverse problem. GEOS
answers by a planar point-in-polygon test. Those are different algorithms on
different surfaces, and for shapes small enough that the projection is faithful
they must agree on every point that is not sitting on the boundary.

The distance to the nearest edge is checked too, against GEOS's own planar
distance on the azimuthal equidistant plane, where distance from the centre is
true.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_pip_geos.py
"""
import json
import random
from pathlib import Path

from geographiclib.geodesic import Geodesic
from pyproj import CRS, Transformer
from shapely.geometry import Point, Polygon

G = Geodesic.WGS84
OUT = Path(__file__).resolve().parents[2] / "core/vectors/geometry.predicate.point-in-polygon.jsonl"
SRC = "GEOS 3.11.4 through shapely on PROJ's azimuthal equidistant plane"
VER = "GEOS 3.11.4 / PROJ 9.3.0 / geographiclib 2.1"
STEP_M = 5_000.0


def plane_for(points):
    lat0 = sum(p[0] for p in points) / len(points)
    base = points[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in points) / len(points)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    return Transformer.from_crs("EPSG:4326", crs, always_xy=True)


def densified(pts):
    out = []
    for i, a in enumerate(pts):
        b = pts[(i + 1) % len(pts)]
        line = G.InverseLine(a[0], a[1], b[0], b[1])
        n = max(1, int(line.s13 / STEP_M) + 1)
        for k in range(n):
            p = line.Position(line.s13 * k / n)
            out.append((p["lat2"], p["lon2"]))
    return out


def classify(ring, points):
    """GEOS's verdict and edge distance for each point."""
    fwd = plane_for(ring)
    poly = Polygon([fwd.transform(lo, la) for la, lo in densified(ring)])
    out = []
    for lat, lon in points:
        p = Point(*fwd.transform(lon, lat))
        out.append({
            "inside": "inside" if poly.contains(p) else "outside",
            "distance": poly.exterior.distance(p),
        })
    return out


# A square, an L and a triangle, each with points well inside, well outside,
# and near but not on the edges. Nothing is placed within a metre of a
# boundary: there the two methods are entitled to disagree, and the tool has
# its own on-boundary verdict that this reference cannot speak to.
SQUARE = [(40.0, -105.0), (40.0, -104.9), (40.08, -104.9), (40.08, -105.0)]
L_SHAPE = [(40.0, -105.0), (40.0, -104.9), (40.04, -104.9), (40.04, -104.95),
           (40.08, -104.95), (40.08, -105.0)]
TRIANGLE = [(-20.0, 30.0), (-20.4, 30.9), (-20.9, 30.2)]

CASES = [
    ("a square", SQUARE, [(40.04, -104.95), (40.01, -104.99), (40.07, -104.91)]),
    ("a square, points outside", SQUARE, [(40.04, -105.2), (39.9, -104.95), (40.2, -104.95)]),
    ("an L, points in the arm and in the notch", L_SHAPE,
     [(40.02, -104.93), (40.06, -104.92), (40.06, -104.97)]),
    ("a triangle", TRIANGLE, [(-20.3, 30.4), (-20.05, 30.1), (-20.8, 30.3)]),
    ("a triangle, points outside", TRIANGLE, [(-20.5, 30.8), (-19.9, 30.5), (-21.0, 30.7)]),
]


def scattered(ring, n, seed):
    """Points over the ring's box, keeping clear of its edges."""
    rnd = random.Random(seed)
    fwd = plane_for(ring)
    poly = Polygon([fwd.transform(lo, la) for la, lo in densified(ring)])
    lo_lat = min(p[0] for p in ring) - 0.02
    hi_lat = max(p[0] for p in ring) + 0.02
    lo_lon = min(p[1] for p in ring) - 0.02
    hi_lon = max(p[1] for p in ring) + 0.02
    out = []
    while len(out) < n:
        lat = rnd.uniform(lo_lat, hi_lat)
        lon = rnd.uniform(lo_lon, hi_lon)
        if poly.exterior.distance(Point(*fwd.transform(lon, lat))) > 1.0:
            out.append((lat, lon))
    return out


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 9:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")

    cases = list(CASES)
    for i, (name, ring) in enumerate(
        [("a square", SQUARE), ("an L", L_SHAPE), ("a triangle", TRIANGLE)]
    ):
        for k in range(3):
            cases.append((f"{name}, scattered points {k + 1}", ring,
                          scattered(ring, 4, 1000 + 10 * i + k)))

    rows = []
    for i, (name, ring, points) in enumerate(cases, start=start + 1):
        verdicts = classify(ring, points)
        expect = {"ok": True, "result.inside_count": sum(
            1 for v in verdicts if v["inside"] == "inside")}
        tolerance = {"result.inside_count": {"abs": 0}}
        for k, v in enumerate(verdicts):
            expect[f"result.results.{k}.even_odd"] = v["inside"]
            expect[f"result.results.{k}.nonzero"] = v["inside"]
            expect[f"result.results.{k}.distance.value"] = v["distance"]
            # The verdicts must match exactly, and do over all 51 points. The
            # distance is two measurements of the same thing by different
            # means -- GEOS in the plane, the core along the geodesic -- and
            # the worst seen is 7.4e-6 relative, so this sits about seven
            # times above it.
            tolerance[f"result.results.{k}.distance.value"] = {"rel": 5e-5, "abs": 1e-3}
        rows.append({
            "id": f"v{i:03d}",
            "input": {
                "polygon": [{"lat": la, "lon": lo} for la, lo in ring],
                "points": [{"lat": la, "lon": lo} for la, lo in points],
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
