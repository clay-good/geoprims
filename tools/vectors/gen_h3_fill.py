#!/usr/bin/env python3
"""polygonToCells differential fixture from H3 C (h3-py), all containment
modes. Random star-shaped polygons, some with a hole, some across the
antimeridian, at resolutions 3-9, then fixed polygons that enclose a pole.

    gen_h3_fill.py OUT.jsonl COUNT SEED

The random polygons come first and draw from the seeded stream alone, so
adding the pole cases after them leaves those rows unchanged."""
import json
import math
import random
import sys

import h3

out, count, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
rng = random.Random(seed)
MODES = ["center", "full", "overlap"]
# Rings that enclose a pole, and bands that reach towards one over most of the
# longitudes. H3 reads a ring in latitude and longitude with straight edges, so
# a ring at one latitude encloses no area at all and the cap cases return
# nothing; the point of having them is that we return nothing too.
POLES = [
    [(88.0, 0.0), (88.0, 90.0), (88.0, 180.0), (88.0, -90.0)],
    [(80.0, 0.0), (80.0, 45.0), (80.0, 90.0), (80.0, 135.0), (80.0, 180.0), (80.0, -135.0), (80.0, -90.0), (80.0, -45.0)],
    [(-85.0, 0.0), (-85.0, -90.0), (-85.0, 180.0), (-85.0, 90.0)],
    [(85.0, -180.0), (85.0, -90.0), (85.0, 0.0), (85.0, 90.0), (89.9, 90.0), (89.9, -180.0)],
    [(-80.0, -180.0), (-80.0, -60.0), (-80.0, 60.0), (-89.5, 60.0), (-89.5, -60.0), (-89.5, -180.0)],
]


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

    # Polygons enclosing a pole, where longitude stops ordering the corners and
    # a ring's east-west span covers the whole sphere. Fixed rather than random,
    # so the cases stay the ones worth having.
    for outer in POLES:
        for res in (2, 3, 4):
            poly = h3.LatLngPoly(outer)
            rec = {"res": res, "outer": outer, "holes": [], "pole": True}
            for m in MODES:
                rec[m] = sorted(h3.polygon_to_cells_experimental(poly, res, contain=m))
            f.write(json.dumps(rec) + "\n")
