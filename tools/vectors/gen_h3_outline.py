#!/usr/bin/env python3
"""Golden vectors for indexing.h3.cells-to-polygon from H3 C through the `h3`
Python bindings (pip install h3==4.4.2).

    gen_h3_outline.py [core/vectors]

Counts, enclosed area, and the latitude span are pinned; the geometry itself
is held by the 400-set differential test (tools/vectors/gen_h3_outline_diff.py).
"""
import json
import sys
from pathlib import Path

import h3

SRC = f"H3 C {h3.versions()['c']} via h3-py {h3.__version__}"
VER = f"H3 {h3.versions()['c']}"


def vec(i, inp, exp):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 1e-9, "abs": 1e-9} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": SRC, "sourceVersion": VER, "tolerance": t}


def case(cells):
    """What the tool should answer for this set, from H3 C."""
    gj = h3.cells_to_h3shape(cells).__geo_interface__
    polys = gj["coordinates"] if gj["type"] == "MultiPolygon" else [gj["coordinates"]]
    lats = [p[1] for poly in polys for ring in poly for p in ring]
    return {
        "result.polygon_count": float(len(polys)),
        "result.hole_count": float(sum(len(poly) - 1 for poly in polys)),
        "result.point_count": float(sum(len(ring) for poly in polys for ring in poly)),
        "result.area.value": sum(h3.cell_area(c, "km^2") for c in cells),
        "result.south.value": min(lats),
        "result.north.value": max(lats),
    }


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    origin = h3.latlng_to_cell(40.446111, -79.982222, 9)
    sets = []
    # One cell, then a cell with its six neighbors: the worked example.
    sets.append([origin])
    sets.append(sorted(h3.grid_disk(origin, 1)))
    # A disk with its middle taken out, which has to come back as a hole.
    donut = sorted(set(h3.grid_disk(origin, 2)) - {origin})
    sets.append(donut)
    # Two patches far apart, which have to stay separate polygons.
    far = h3.latlng_to_cell(-33.8688, 151.2093, 9)
    sets.append(sorted(set(h3.grid_disk(origin, 1)) | set(h3.grid_disk(far, 1))))
    # Around a pentagon, where cells carry extra boundary points.
    sets.append(sorted(h3.grid_disk(h3.get_pentagons(5)[0], 1)))
    # Across the antimeridian.
    sets.append(sorted(h3.grid_disk(h3.latlng_to_cell(-0.5, 179.99, 7), 2)))
    # Coarse and fine, for the range of resolutions.
    sets.append(sorted(h3.grid_disk(h3.latlng_to_cell(64.8378, -147.7164, 3), 2)))
    sets.append(sorted(h3.grid_disk(h3.latlng_to_cell(51.5074, -0.1278, 12), 3)))

    rows = [vec(i, {"cells": [{"cell": c} for c in cs]}, case(cs)) for i, cs in enumerate(sets, 1)]
    path = out / "indexing.h3.cells-to-polygon.jsonl"
    path.write_text("".join(json.dumps(r) + "\n" for r in rows))
    print(f"{path}: {len(rows)} vectors")


if __name__ == "__main__":
    main()
