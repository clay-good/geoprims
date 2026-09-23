#!/usr/bin/env python3
"""Golden vectors and a fixture for geometry.mesh.voronoi, from GEOS.

A Voronoi diagram is determined by its points, but what comes back depends on
where it is cut off: the cells of the outer points are unbounded and have to be
clipped. So this reproduces the tool's stated box -- the points' extent padded
by a tenth of their spread -- and then asks GEOS for the diagram inside it.

Getting that box right is the whole difficulty. Padding each axis by a tenth of
its OWN spread rather than of the larger one gives a quite different box for a
long thin set, and disagreed with the tool by 633% on one cell before it was
fixed.

Two outputs: vectors pinning the cell count, and a fixture of per-cell corner
counts and areas that the Rust differential test compares, since a cell count
alone is just the number of points.

    python3 tools/vectors/gen_voronoi_geos.py
"""
import json
from pathlib import Path

from geographiclib.geodesic import Geodesic
from geographiclib.polygonarea import PolygonArea
from pyproj import CRS, Transformer
from shapely import voronoi_polygons
from shapely.geometry import MultiPoint, Point, box

import gen_delaunay_geos as delaunay

G = Geodesic.WGS84
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/vectors/geometry.mesh.voronoi.jsonl"
FIXTURE = ROOT / "core/crates/gp-geometry/tests/data/voronoi_geos.json"
SRC = "GEOS 3.11.4 through shapely, voronoi_polygons in the tool's own clip box on PROJ's azimuthal equidistant plane"
VER = "GEOS 3.11.4 / PROJ 9.3.0 / geographiclib 2.1"


def cells(pts):
    """Per generator: how many corners its cell has and what area it covers."""
    lat0 = sum(p[0] for p in pts) / len(pts)
    base = pts[0][1]
    lon0 = base + sum(((p[1] - base + 180.0) % 360.0) - 180.0 for p in pts) / len(pts)
    crs = CRS.from_proj4(
        f"+proj=aeqd +ellps=WGS84 +lat_0={lat0} +lon_0={lon0} +units=m +no_defs"
    )
    fwd = Transformer.from_crs("EPSG:4326", crs, always_xy=True)
    inv = Transformer.from_crs(crs, "EPSG:4326", always_xy=True)
    xy = [fwd.transform(lo, la) for la, lo in pts]
    xs = [p[0] for p in xy]
    ys = [p[1] for p in xy]
    # A tenth of the larger spread, on both axes -- the tool's convention.
    pad = max(max(xs) - min(xs), max(ys) - min(ys)) / 10
    env = box(min(xs) - pad, min(ys) - pad, max(xs) + pad, max(ys) + pad)

    out = {}
    for g in voronoi_polygons(MultiPoint(xy), extend_to=env).geoms:
        cell = g.intersection(env)
        if cell.is_empty or cell.geom_type != "Polygon":
            continue
        owners = [i + 1 for i, p in enumerate(xy) if cell.contains(Point(p))]
        if len(owners) != 1:
            continue
        ring = list(cell.exterior.coords)[:-1]
        pa = PolygonArea(G)
        for x, y in ring:
            lon, lat = inv.transform(x, y)
            pa.AddPoint(lat, lon)
        out[owners[0]] = {"corners": len(ring), "area": abs(pa.Compute(False, True)[2])}
    return out


def main():
    existing = [json.loads(line) for line in OUT.read_text().splitlines() if line.strip()]
    start = max(int(v["id"][1:]) for v in existing)
    if start >= 6:
        raise SystemExit(f"{OUT.name} already carries {start} vectors; this script appends once")
    rows, fixture = [], []
    for i, (name, pts) in enumerate(delaunay.CASES, start=start + 1):
        c = cells(pts)
        if len(c) != len(pts):
            raise SystemExit(f"{name}: GEOS gave {len(c)} cells for {len(pts)} points")
        rows.append({
            "id": f"v{i:03d}",
            "input": {"points": [{"lat": la, "lon": lo} for la, lo in pts], "surface": "planar"},
            "expect": {"result.cell_count": len(c), "result.surface_used": "planar", "ok": True},
            "source": f"{SRC}: {name}",
            "sourceVersion": VER,
            "tolerance": {"result.cell_count": {"abs": 0}, "result.surface_used": {"abs": 0}},
        })
        fixture.append({
            "name": name,
            "points": [{"lat": la, "lon": lo} for la, lo in pts],
            "cells": [{"point": k, "corners": v["corners"], "area": v["area"]}
                      for k, v in sorted(c.items())],
        })
    with OUT.open("a") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    FIXTURE.write_text(json.dumps(fixture, indent=1) + "\n")
    print(f"appended {len(rows)} vectors to {OUT.name} (now {len(existing) + len(rows)})")
    print(f"wrote {FIXTURE.relative_to(ROOT)} with {len(fixture)} diagrams")


if __name__ == "__main__":
    main()
