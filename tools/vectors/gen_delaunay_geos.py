#!/usr/bin/env python3
"""Golden vectors and a fixture for geometry.mesh.delaunay, from GEOS.

A Delaunay triangulation of points in general position is unique, so this is
one of the few things in the catalog where a reference does not merely have to
agree closely -- it has to agree exactly, triangle for triangle. GEOS's
`delaunay_triangles` provides that second opinion.

Two outputs:

- vectors appended to the frozen file, pinning the triangle count;
- `core/crates/gp-geometry/tests/data/delaunay_geos.json`, the full triangle
  sets, which the Rust differential test compares as sets. The count alone
  would pass for a triangulation with the right number of wrong triangles.

Cocircular points are avoided on purpose: four points on a circle can be split
either way and neither is wrong, so the rings here have their radii nudged.

    python3 tools/vectors/gen_delaunay_geos.py
"""
import json
import math
import random
from pathlib import Path

from pyproj import CRS, Transformer
from shapely import delaunay_triangles
from shapely.geometry import MultiPoint

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/vectors/geometry.mesh.delaunay.jsonl"
FIXTURE = ROOT / "core/crates/gp-geometry/tests/data/delaunay_geos.json"
SRC = "GEOS 3.11.4 through shapely, delaunay_triangles on PROJ's azimuthal equidistant plane"
VER = "GEOS 3.11.4 / PROJ 9.3.0"


def plane_for(pts):
    lat0 = sum(p[0] for p in pts) / len(pts)
    base = pts[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in pts) / len(pts)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    return Transformer.from_crs("EPSG:4326", crs, always_xy=True)


def triangles(pts):
    """The triangulation as sorted one-based index triples."""
    fwd = plane_for(pts)
    xy = [fwd.transform(lo, la) for la, lo in pts]

    def idx(p, tol=1e-6):
        for i, q in enumerate(xy):
            if abs(p[0] - q[0]) < tol and abs(p[1] - q[1]) < tol:
                return i + 1
        raise KeyError(p)

    found = {
        tuple(sorted(idx(p) for p in list(g.exterior.coords)[:-1]))
        for g in delaunay_triangles(MultiPoint(xy)).geoms
    }
    return sorted(found)


def scatter(lat, lon, spread, n, seed):
    r = random.Random(seed)
    return [(lat + r.uniform(-spread, spread), lon + r.uniform(-spread, spread)) for _ in range(n)]


def jittered_grid(lat, lon, step, k, seed):
    r = random.Random(seed)
    return [
        (lat + i * step + r.uniform(-step / 6, step / 6),
         lon + j * step + r.uniform(-step / 6, step / 6))
        for i in range(k) for j in range(k)
    ]


def ring(lat, lon, n, wobble):
    return [
        (lat + (0.02 + wobble * (k % 3)) * math.cos(math.radians(360 / n * k)),
         lon + (0.026 + wobble * (k % 3)) * math.sin(math.radians(360 / n * k)))
        for k in range(n)
    ]


CASES = [
    ("five points", [(40.0, -105.0), (40.01, -104.99), (40.0, -104.98),
                     (39.99, -104.992), (40.004, -104.995)]),
    ("a small scatter", scatter(40.0, -105.0, 0.02, 7, 21)),
    ("a wider scatter", scatter(40.0, -105.0, 0.08, 10, 22)),
    ("a dozen points", scatter(35.0, 139.0, 0.05, 12, 23)),
    ("a jittered 3x3 grid", jittered_grid(40.0, -105.0, 0.01, 3, 24)),
    ("a jittered 4x4 grid", jittered_grid(40.0, -105.0, 0.008, 4, 25)),
    ("a southern scatter", scatter(-33.87, 151.2, 0.03, 9, 26)),
    ("an equatorial scatter", scatter(0.0, 0.0, 0.04, 8, 27)),
    ("a high-latitude scatter", scatter(70.0, 25.0, 0.05, 8, 28)),
    ("a scatter across the antimeridian", scatter(-17.0, 179.98, 0.03, 8, 29)),
    ("a long thin scatter", [(40 + 0.0002 * i, -105 + 0.01 * i + (0.0005 if i % 2 else -0.0005))
                             for i in range(8)]),
    ("two clusters", scatter(40.0, -105.0, 0.005, 5, 30) + scatter(40.05, -104.95, 0.005, 5, 31)),
    ("a nudged ring", ring(40.0, -105.0, 8, 0.0008)),
    ("a nudged ring with a centre", [(40.0, -105.0)] + ring(40.0, -105.0, 6, 0.0009)),
    ("sixteen scattered points", scatter(48.85, 2.35, 0.06, 16, 32)),
]


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 6:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows, fixture = [], []
    for i, (name, pts) in enumerate(CASES, start=start + 1):
        tris = triangles(pts)
        rows.append({
            "id": f"v{i:03d}",
            "input": {"points": [{"lat": la, "lon": lo} for la, lo in pts], "surface": "planar"},
            "expect": {"result.triangle_count": len(tris), "result.surface_used": "planar", "ok": True},
            "source": f"{SRC}: {name}",
            "sourceVersion": VER,
            "tolerance": {"result.triangle_count": {"abs": 0}, "result.surface_used": {"abs": 0}},
        })
        fixture.append({
            "name": name,
            "points": [{"lat": la, "lon": lo} for la, lo in pts],
            "triangles": [list(t) for t in tris],
        })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    FIXTURE.write_text(json.dumps(fixture, indent=1) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")
    print(f"wrote {FIXTURE.relative_to(ROOT)} with {len(fixture)} triangle sets")


if __name__ == "__main__":
    main()
