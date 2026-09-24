#!/usr/bin/env python3
"""Polygon overlay against GEOS on a grid, for geometry.overlay.boolean (1.1.0).

Writes core/crates/gp-geometry/tests/data/overlay_geos.json: 400 pairs of
polygons on a small integer grid (a third with a hole, 40% of the second
shapes made from the first by a one-step shift or a reversal), so shared
edges, corners on edges, and shapes touching at a point are common. For each
of the four operations it holds GEOS's result on the grid itself (exact) as
the area in grid squares and the number of parts, and the same result's area
measured on the ellipsoid by GeographicLib once the grid is placed at 40° N,
105° W with a step of 0.00001° (about a meter).

core/crates/gp-geometry/tests/overlay_parity.rs checks the planar overlay
exactly against the grid answers, again with every corner moved by up to
0.2 µm, and the geodesic tool against the part counts and areas.

Also appends regression vectors to core/vectors/geometry.overlay.boolean.jsonl
for cases that failed before 1.1.0. Requires shapely and geographiclib."""
import json
import random
import sys
from pathlib import Path

import shapely
from geographiclib.geodesic import Geodesic
from shapely.geometry import Polygon

sys.path.insert(0, str(Path(__file__).parent))
from gen_relate_geos import rand_polygon  # noqa: E402

G = Geodesic.WGS84
VER = f"shapely {shapely.__version__}, GEOS {'.'.join(map(str, shapely.geos_version))}"
OPS = ["intersection", "union", "difference", "symmetric-difference"]
STEP = 1e-5


def poly(pts, holes):
    return Polygon([(x, y) for y, x in pts], [[(x, y) for y, x in h] for h in holes])


def parts_of(g):
    return [q for q in (g.geoms if hasattr(g, "geoms") else [g]) if q.geom_type == "Polygon" and q.area > 0]


def ellipsoid_area(q):
    total = 0.0
    for k, ring in enumerate([q.exterior, *q.interiors]):
        p = G.Polygon()
        for x, y in list(ring.coords)[:-1]:
            p.AddPoint(40 + y * STEP, -105 + x * STEP)
        a = abs(p.Compute()[2])
        total += a if k == 0 else -a
    return total


def result(a, b, op):
    g = getattr(a, op.replace("-", "_"))(b)
    ps = parts_of(g)
    return {"grid_area": sum(q.area for q in ps), "parts": len(ps), "m2": sum(ellipsoid_area(q) for q in ps)}


def rows(pts, holes):
    out = [{"lat": 40 + y * STEP, "lon": -105 + x * STEP} for y, x in pts]
    for k, h in enumerate(holes, 1):
        out += [{"lat": 40 + y * STEP, "lon": -105 + x * STEP, "ring": k} for y, x in h]
    return out


def main():
    rng = random.Random(5)
    cases = []
    for _ in range(400):
        pa, ha = rand_polygon(rng)
        if rng.random() < 0.4:
            dy, dx = rng.choice([(0, 0), (0, 1), (1, 0), (1, 1), (0, 2)])
            pb, hb = [(y + dy, x + dx) for y, x in pa], []
            if rng.random() < 0.5:
                pb.reverse()
        else:
            pb, hb = rand_polygon(rng)
        a, b = poly(pa, ha), poly(pb, hb)
        cases.append({"a": [pa, ha], "b": [pb, hb], "results": {op: result(a, b, op) for op in OPS}})
    Path("core/crates/gp-geometry/tests/data/overlay_geos.json").write_text(json.dumps(cases, separators=(",", ":")) + "\n")

    # Regression vectors: cases the tool got wrong before 1.1.0 (an empty
    # union, a lost difference, and pieces touching at a point returned as one
    # ring that touched itself).
    # Published vectors are frozen byte for byte: keep the file's lines as they
    # are and append only the regressions it does not have yet.
    path = Path("core/vectors/geometry.overlay.boolean.jsonl")
    lines = path.read_text().splitlines()
    kept = [l for l in lines if "Regression (1.1.0)" not in l]
    vecs = [json.loads(l) for l in kept]
    new = []
    for i, op in [(269, "union"), (195, "difference"), (8, "symmetric-difference"), (9, "symmetric-difference"), (0, "intersection")]:
        c = cases[i]
        r = c["results"][op]
        new.append({
            "id": f"v{len(vecs) + len(new) + 1:03d}",
            "input": {"polygon_a": rows(*c["a"]), "polygon_b": rows(*c["b"]), "operation": op},
            "expect": {"result.parts": r["parts"], "result.area.value": r["m2"] / 1e6, "ok": True},
            "source": f"Regression (1.1.0): GEOS on the grid, areas measured on the ellipsoid by GeographicLib (tools/vectors/gen_overlay_grid.py, pair {i})",
            "sourceVersion": VER,
            "tolerance": {"result.parts": {"rel": 0, "abs": 1e-9}, "result.area.value": {"rel": 5e-5, "abs": 0}},
        })
    path.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print(len(cases), "pairs;", len(vecs) + len(new), "vectors")


if __name__ == "__main__":
    main()
