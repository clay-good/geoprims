#!/usr/bin/env python3
"""polygonToCells differential fixture from H3 C (h3-py), all containment
modes. Random star-shaped polygons, some with a hole, some across the
antimeridian, at resolutions 3-9.

    gen_h3_fill.py OUT.jsonl COUNT SEED"""
import json
import math
import random
import sys

import h3

out, count, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
rng = random.Random(seed)
MODES = ["center", "full", "overlap"]


def star(clat, clng, radius, n, wobble):
    pts = []
    for k in range(n):
        a = 2 * math.pi * k / n
        r = radius * (1 - wobble * rng.random())
        lat = clat + r * math.sin(a)
        lng = clng + r * math.cos(a) / max(math.cos(math.radians(clat)), 0.2)
        lng = (lng + 180) % 360 - 180
        pts.append((round(lat, 9), round(lng, 9)))
    return pts


with open(out, "w") as f:
    f.write(json.dumps({"comment": f"H3 C {h3.versions()['c']} via h3-py {h3.__version__}; seed {seed}"}) + "\n")
    for i in range(count):
        res = rng.randint(3, 9)
        # Cell edge in degrees, so polygons span a few to a few hundred cells.
        edge = h3.average_hexagon_edge_length(res, "km") / 111.0
        radius = edge * rng.uniform(2, 12)
        clat = rng.uniform(-70, 70)
        clng = 179.9 - radius * 0.3 if i % 5 == 0 else rng.uniform(-179, 179)
        outer = star(clat, clng, radius, rng.randint(3, 12), rng.uniform(0, 0.6))
        holes = [star(clat, clng, radius * 0.3, rng.randint(3, 6), 0.2)] if i % 3 == 0 else []
        poly = h3.LatLngPoly(outer, *holes)
        rec = {"res": res, "outer": outer, "holes": holes}
        for m in MODES:
            rec[m] = sorted(h3.polygon_to_cells_experimental(poly, res, contain=m))
        f.write(json.dumps(rec) + "\n")
