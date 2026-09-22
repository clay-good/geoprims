#!/usr/bin/env python3
"""Differential data for geometry.mesh.delaunay and voronoi, from Qhull via
SciPy (independent of the tool's own hull): planar sets on GeographicLib's
azimuthal equidistant projection (GeodesicProj -z) at the points' center,
and global sets on the unit sphere (ConvexHull and SphericalVoronoi).

Writes core/crates/gp-geometry/tests/data/mesh_diff.json: for each case the
points, the triangles as sorted point numbers (1-based), and the Voronoi cell
corners of every spherical point and every planar point whose cell is bounded
(the tool closes the others at a box). Random points have no four on a circle,
so the triangulation is unique. Also writes golden vectors with the counts."""
import json
import math
import random
import subprocess
from pathlib import Path

import numpy as np
from scipy.spatial import ConvexHull, Delaunay, SphericalVoronoi, Voronoi

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "core/crates/gp-geometry/tests/data/mesh_diff.json"
SRC, VER = "Qhull via SciPy (tools/vectors/gen_mesh_diff.py)", "SciPy 1.x"


def center(pts):
    x = sum(math.cos(math.radians(a)) * math.cos(math.radians(b)) for a, b in pts)
    y = sum(math.cos(math.radians(a)) * math.sin(math.radians(b)) for a, b in pts)
    z = sum(math.sin(math.radians(a)) for a, _ in pts)
    return math.degrees(math.atan2(z, math.hypot(x, y))), math.degrees(math.atan2(y, x))


def aeqd(pts, c, reverse=False):
    cmd = ["GeodesicProj", "-z", f"{c[0]:.15f}", f"{c[1]:.15f}", "-p", "12"] + (["-r"] if reverse else [])
    out = subprocess.run(cmd, input="".join(f"{a:.15f} {b:.15f}\n" for a, b in pts), capture_output=True, text=True, check=True).stdout
    return [tuple(float(v) for v in l.split()[:2]) for l in out.splitlines()]


def unit(pts):
    return np.array([[math.cos(math.radians(a)) * math.cos(math.radians(b)), math.cos(math.radians(a)) * math.sin(math.radians(b)), math.sin(math.radians(a))] for a, b in pts])


def ll_of(v):
    return [math.degrees(math.atan2(v[2], math.hypot(v[0], v[1]))), math.degrees(math.atan2(v[1], v[0]))]


def planar(pts):
    c = center(pts)
    xy = np.array(aeqd(pts, c))
    tri = sorted(sorted(int(i) + 1 for i in s) for s in Delaunay(xy).simplices)
    vor = Voronoi(xy)
    # The tool closes cells at the points' box padded by a tenth of its larger
    # side; only cells wholly inside that box can be compared.
    lo, hi = xy.min(axis=0), xy.max(axis=0)
    pad = 0.1 * max(hi[0] - lo[0], hi[1] - lo[1], 1.0)
    inside = lambda v: lo[0] - pad < v[0] < hi[0] + pad and lo[1] - pad < v[1] < hi[1] + pad
    cells = {}
    for i, r in enumerate(vor.point_region):
        region = vor.regions[r]
        if region and -1 not in region and all(inside(vor.vertices[k]) for k in region):
            cells[str(i + 1)] = [list(p) for p in aeqd([tuple(vor.vertices[k]) for k in region], c, reverse=True)]
    return tri, cells


def spherical(pts):
    u = unit(pts)
    tri = sorted(sorted(int(i) + 1 for i in s) for s in ConvexHull(u).simplices)
    sv = SphericalVoronoi(u, radius=1, center=np.zeros(3))
    cells = {str(i + 1): [ll_of(sv.vertices[k]) for k in region] for i, region in enumerate(sv.regions)}
    return tri, cells


def main():
    rnd = random.Random(20260922)
    cases = []
    for n, spread in [(6, 0.05), (25, 0.2), (60, 1.5), (200, 3.0)]:
        pts = [(40 + rnd.uniform(-spread, spread), -105 + rnd.uniform(-spread, spread)) for _ in range(n)]
        tri, cells = planar(pts)
        cases.append({"surface": "planar", "points": pts, "triangles": tri, "cells": cells})
    for n in [8, 40, 150]:
        pts = [(math.degrees(math.asin(rnd.uniform(-1, 1))), rnd.uniform(-180, 180)) for _ in range(n)]
        tri, cells = spherical(pts)
        cases.append({"surface": "spherical", "points": pts, "triangles": tri, "cells": cells})
    OUT.write_text(json.dumps(cases, separators=(",", ":")) + "\n")
    # Golden vectors: the counts and the surface chosen, for the smaller cases.
    for tool, key in [("geometry.mesh.delaunay", "triangle_count"), ("geometry.mesh.voronoi", "cell_count")]:
        rows = []
        for c in cases[:3] + cases[4:6]:
            inp = {"points": [{"lat": a, "lon": b} for a, b in c["points"]], "surface": "auto"}
            val = float(len(c["triangles"]) if key == "triangle_count" else len(c["points"]))
            rows.append({"id": f"v{len(rows) + 1:03d}", "input": inp, "expect": {f"result.{key}": val, "result.surface_used": c["surface"], "ok": True},
                         "source": SRC, "sourceVersion": VER, "tolerance": {f"result.{key}": {"abs": 0}}})
        (ROOT / f"core/vectors/{tool}.jsonl").write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
    print(len(cases), "cases")


if __name__ == "__main__":
    main()
