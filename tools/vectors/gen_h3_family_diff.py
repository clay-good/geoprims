#!/usr/bin/env python3
"""H3 family differential fixture from H3 C via h3-py:

  pip install h3==4.4.2
  python tools/vectors/gen_h3_family_diff.py

Writes core/crates/gp-indexing/tests/data/h3_family_diff.jsonl: for seeded
cells at every resolution (one in eight a pentagon), H3 C's cell details,
grid ring, grid path to a nearby cell, parent and child position, children,
directed edges and vertexes, and a compact / uncompact round trip.
Seeded, so rerunning reproduces the file.
"""
import json
import random
from pathlib import Path

import h3

N = 300


def main():
    rnd = random.Random(33)
    rows = [{"note": f"H3 C {h3.versions()['c']} via h3-py {h3.__version__}; seed 33"}]
    while len(rows) <= N:
        k = len(rows)
        r = rnd.randint(0, 15)
        if k % 8 == 0:
            c = h3.get_pentagons(r)[rnd.randint(0, 11)]
        else:
            c = h3.latlng_to_cell(rnd.uniform(-89.5, 89.5), rnd.uniform(-180, 180), r)
        lat, lng = h3.cell_to_latlng(c)
        ring_k = rnd.randint(1, 4)
        try:
            ring = sorted(h3.grid_ring(c, ring_k))
        except Exception:
            ring = None  # H3 C cannot ring through a pentagon's distortion
        far = sorted(h3.grid_disk(c, 5))[rnd.randint(0, 90) % len(h3.grid_disk(c, 5))]
        try:
            path = h3.grid_path_cells(c, far)
        except Exception:
            path = None
        pr = rnd.randint(0, r)
        parent = h3.cell_to_parent(c, pr)
        cr = min(15, r + rnd.randint(0, 2))
        children = h3.cell_to_children(c, cr)
        disk = h3.grid_disk(c, 2)
        compacted = sorted(h3.compact_cells(sorted(set(h3.cell_to_children(h3.cell_to_parent(c, max(0, r - 1)), r)) | set(disk))))
        rows.append({
            "cell": c, "resolution": r, "lat": lat, "lon": lng, "base_cell": h3.get_base_cell_number(c),
            "pentagon": h3.is_pentagon(c), "class3": h3.is_res_class_III(c), "area_km2": h3.cell_area(c, "km^2"),
            "boundary": [list(v) for v in h3.cell_to_boundary(c)],
            "ring_k": ring_k, "ring": ring, "path_to": far, "path": path,
            "parent_res": pr, "parent": parent, "child_pos": h3.cell_to_child_pos(c, pr),
            "children_res": cr, "children_count": len(children), "center_child": h3.cell_to_center_child(c, cr),
            "edges": sorted(h3.origin_to_directed_edges(c)), "vertexes": sorted(h3.cell_to_vertexes(c)),
            "compact_input": sorted(set(h3.cell_to_children(h3.cell_to_parent(c, max(0, r - 1)), r)) | set(disk)),
            "compacted": compacted,
        })
    out = Path("core/crates/gp-indexing/tests/data/h3_family_diff.jsonl")
    out.write_text("".join(json.dumps(x) + "\n" for x in rows))
    print(len(rows) - 1, "cells", sum(1 for x in rows[1:] if x["ring"] is None), "rings skipped", sum(1 for x in rows[1:] if x["path"] is None), "paths skipped")


if __name__ == "__main__":
    main()
