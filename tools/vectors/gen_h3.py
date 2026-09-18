#!/usr/bin/env python3
"""Golden vectors for indexing.h3.* from H3 C through the `h3` Python
bindings (run from a virtualenv with `pip install h3`), plus the
add-spatial-indexing-and-raster scenarios."""
import json
import sys
from pathlib import Path

import h3

SRC = f"H3 C {h3.versions()['c']} via h3-py {h3.__version__}"
VER = f"H3 {h3.versions()['c']}"
SPEC = "add-spatial-indexing-and-raster scenarios"
PTS = [(40.446111, -79.982222, 9), (-33.8688, 151.2093, 7), (51.5074, -0.1278, 12), (64.8378, -147.7164, 5), (-0.5, 179.999, 3), (89.9, 45.0, 6)]


def vec(i, inp, exp, src=SRC, ver=VER, tol=5e-11):
    e = dict(exp)
    e.setdefault("ok", True)
    # h3o and H3 C agree within 1e-12° below 88° latitude, 5e-11° nearer the poles.
    t = {k: {"rel": 1e-11, "abs": tol} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def cells(cs):
    return [{"cell": c} for c in cs]


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    f = {}
    to_cell, info, disk, ring, path, parent, children, compact, uncompact, edges, chooser = ([] for _ in range(11))
    for i, (la, lo, r) in enumerate(PTS, 1):
        c = h3.latlng_to_cell(la, lo, r)
        clat, clng = h3.cell_to_latlng(c)
        src = SPEC if i == 1 else SRC
        to_cell.append(vec(i, {"lat": la, "lon": lo, "resolution": r}, {"result.cell": c, "result.center_lat.value": clat, "result.center_lon.value": clng,
                                                                     "result.area.value": h3.cell_area(c, "km^2")}, src, tol=1e-9))
        b = h3.cell_to_boundary(c)
        info.append(vec(i, {"cell": c}, {"result.lat.value": clat, "result.lon.value": clng, "result.resolution": float(r),
                                         "result.base_cell": float(h3.get_base_cell_number(c)), "result.pentagon": "yes" if h3.is_pentagon(c) else "no",
                                         "result.class3": "yes" if h3.is_res_class_III(c) else "no", "result.decimal": str(h3.str_to_int(c)),
                                         "result.boundary.0.lat": b[0][0], "result.boundary.0.lon": b[0][1]}, src))
        k = [1, 2, 0, 3, 1, 2][i - 1]
        d = h3.grid_disk(c, k)
        disk.append(vec(i, {"cell": c, "k": k}, {"result.count": float(len(d)), "result.cells.0.cell": c}))
        rg = h3.grid_ring(c, max(k, 1))
        ring.append(vec(i, {"cell": c, "k": max(k, 1)}, {"result.count": float(len(rg))}))
        far = h3.grid_ring(c, 3)[4]
        p = h3.grid_path_cells(c, far)
        path.append(vec(i, {"from": c, "to": far}, {"result.distance": float(h3.grid_distance(c, far)), "result.cells.1.cell": p[1], "result.cells.3.cell": p[3]}))
        pr = max(r - 3, 0)
        parent.append(vec(i, {"cell": c, "resolution": pr}, {"result.parent": h3.cell_to_parent(c, pr), "result.child_position": float(h3.cell_to_child_pos(c, pr))},
                          SPEC if i == 1 else SRC))
        cr = min(r + 1, 15)
        kids = sorted(h3.cell_to_children(c, cr))
        children.append(vec(i, {"cell": c, "resolution": cr}, {"result.count": float(len(kids)), "result.center_child": h3.cell_to_center_child(c, cr)},
                            SPEC if i == 1 else SRC))
        compact.append(vec(i, {"cells": cells(kids)},
                           {"result.count": float(len(h3.compact_cells(kids))), "result.cells.0.cell": h3.compact_cells(kids)[0]}))
        uncompact.append(vec(i, {"cells": [{"cell": c}], "resolution": cr}, {"result.count": float(len(kids))}))
        es = h3.origin_to_directed_edges(c)
        edges.append(vec(i, {"cell": c}, {"result.edge_count": float(len(es)), "result.edges.0.edge": es[0],
                                          "result.edges.0.length.value": h3.edge_length(es[0], "m")}, tol=1e-6))
    pent = "85080003fffffff"
    disk.append(vec(7, {"cell": pent, "k": 1}, {"result.count": 6.0, "meta.warnings.1.code": "PENTAGON_DISTORTION"}, SPEC, "2026"))
    to_cell.append(vec(7, {"lat": 0, "lon": 0, "resolution": 16}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    info.append(vec(7, {"cell": 617741122143780863}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    info.append(vec(8, {"cell": "617741122143780863"}, {"result.decimal": "617741122143780863", "result.resolution": 9.0}, SPEC, "2026"))
    info.append(vec(9, {"cell": "0x892A8471487FFFF"}, {"result.resolution": 9.0}, SPEC, "2026"))
    import math

    def nearest(values, t):
        return min(range(16), key=lambda r: abs(math.log(values(r) / t)))
    for i, (area, km2) in enumerate([("1 km2", 1.0), ("0.1 km2", 0.1), ("100 km2", 100.0), ("1 m2", 1e-6), ("5000000 km2", 5e6)], 1):
        chooser.append(vec(i, {"target_area": area}, {"result.resolution": float(nearest(lambda r: h3.average_hexagon_area(r, "km^2"), km2))},
                           SPEC if i == 1 else "H3 average hexagon areas (h3.average_hexagon_area)", VER))
    chooser.append(vec(6, {"target_edge": "150 m"}, {"result.resolution": float(nearest(lambda r: h3.average_hexagon_edge_length(r, "km"), 0.15))},
                       "H3 average hexagon edge lengths (h3.average_hexagon_edge_length)", VER))
    f = {"indexing.h3.lat-lng-to-cell": to_cell, "indexing.h3.cell-info": info, "indexing.h3.grid-disk": disk, "indexing.h3.grid-ring": ring,
         "indexing.h3.grid-path": path, "indexing.h3.parent": parent, "indexing.h3.children": children, "indexing.h3.compact": compact,
         "indexing.h3.uncompact": uncompact, "indexing.h3.edges": edges, "indexing.h3.resolution-chooser": chooser}
    for tool, vs in f.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
