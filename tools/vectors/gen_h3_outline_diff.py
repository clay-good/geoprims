#!/usr/bin/env python3
"""cellsToMultiPolygon differential fixture from H3 C (h3-py). Cell sets that
exercise the cases an outline has to get right: a solid patch, a patch with a
hole punched out, several separate patches, a set touching a pentagon, and a
set across the antimeridian, at resolutions 3-11. Coarser than that a handful
of cells spans much of the globe and a loop can degenerate to two points,
which H3Shape cannot express; such a set is skipped and reported.

    gen_h3_outline_diff.py OUT.jsonl COUNT SEED"""
import json
import random
import sys

import h3

out, count, seed = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
rng = random.Random(seed)

# The twelve pentagons of a resolution, so some sets are made around one.
def pentagons(res):
    return list(h3.get_pentagons(res))


def rings(shape):
    """The polygons of an H3Shape, as lists of closed (lat, lon) rings."""
    gj = shape.__geo_interface__
    polys = gj["coordinates"] if gj["type"] == "MultiPolygon" else [gj["coordinates"]]
    # GeoJSON is (lon, lat); the fixture keeps (lat, lon) as the tools do.
    return [[[(round(p[1], 9), round(p[0], 9)) for p in ring] for ring in poly] for poly in polys]


def make(kind, res):
    if kind == "pentagon":
        return list(h3.grid_disk(rng.choice(pentagons(res)), rng.randint(1, 3)))
    if kind == "antimeridian":
        center = h3.latlng_to_cell(rng.uniform(-60, 60), 179.97, res)
        return list(h3.grid_disk(center, rng.randint(1, 3)))
    center = h3.latlng_to_cell(rng.uniform(-75, 75), rng.uniform(-179, 179), res)
    if kind == "solid":
        return list(h3.grid_disk(center, rng.randint(1, 4)))
    if kind == "holed":
        # A disk with its middle removed, which must come back as a hole.
        cells = set(h3.grid_disk(center, rng.randint(2, 4)))
        for c in h3.grid_disk(center, rng.randint(0, 1)):
            cells.discard(c)
        return list(cells)
    if kind == "scattered":
        # Several patches far enough apart to stay separate polygons.
        cells = []
        for _ in range(rng.randint(2, 3)):
            c = h3.latlng_to_cell(rng.uniform(-75, 75), rng.uniform(-179, 179), res)
            cells.extend(h3.grid_disk(c, rng.randint(0, 2)))
        return list(set(cells))
    raise SystemExit(f"unknown kind {kind}")


KINDS = ["solid", "holed", "scattered", "pentagon", "antimeridian"]

with open(out, "w") as f:
    f.write(json.dumps({"comment": f"H3 C {h3.versions()['c']} via h3-py {h3.__version__}; seed {seed}"}) + "\n")
    for i in range(count):
        kind = KINDS[i % len(KINDS)]
        res = rng.randint(3, 11)
        cells = sorted(make(kind, res))
        if not cells:
            continue
        try:
            shape = h3.cells_to_h3shape(cells)
        except (ValueError, h3.H3BaseException) as e:
            # A degenerate loop the reference cannot hold, or a set H3 C itself
            # declines. Reported rather than swallowed, so a run says what it
            # could not cover.
            print(f"skipped {kind} at res {res} ({len(cells)} cells): {type(e).__name__} {e}", file=sys.stderr)
            continue
        f.write(json.dumps({"kind": kind, "res": res, "cells": cells, "polys": rings(shape)}) + "\n")
