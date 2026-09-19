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
    disk.append(vec(7, {"cell": pent, "k": 1}, {"result.count": 6.0, "meta.warnings.0.code": "PENTAGON_DISTORTION"}, SPEC, "2026"))
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
    fillv = []
    shapes = [
        ([(40.4406, -80.0020), (40.4406, -79.9900), (40.4480, -79.9900), (40.4480, -80.0020)], [], 9, "center"),
        ([(40.4406, -80.0020), (40.4406, -79.9900), (40.4480, -79.9900), (40.4480, -80.0020)], [], 9, "overlapping"),
        ([(-33.80, 151.10), (-33.80, 151.30), (-33.95, 151.30), (-33.95, 151.10)], [[(-33.85, 151.15), (-33.85, 151.25), (-33.90, 151.20)]], 7, "full"),
        ([(10.0, 179.0), (10.0, -179.0), (12.0, -179.0), (12.0, 179.0)], [], 5, "center"),
        ([(51.40, -0.30), (51.60, -0.30), (51.60, 0.10), (51.40, 0.10)], [], 8, "overlapping"),
    ]
    for i, (outer, holes, res, mode) in enumerate(shapes, 1):
        cells_ = sorted(h3.polygon_to_cells_experimental(h3.LatLngPoly(outer, *holes), res, contain={"overlapping": "overlap"}.get(mode, mode)))
        if holes or i == 4:
            inp = {"geojson": json.dumps({"type": "Polygon", "coordinates": [[[lo, la] for la, lo in r + [r[0]]] for r in [outer] + holes]}), "resolution": res, "containment": mode}
        else:
            inp = {"points": [{"lat": la, "lon": lo} for la, lo in outer], "resolution": res, "containment": mode}
        fillv.append(vec(i, inp, {"result.count": float(len(cells_)), "result.cells.0.cell": cells_[0], "result.containment": mode}))
    fillv.append(vec(6, {"points": [{"lat": -35, "lon": 110}, {"lat": -35, "lon": 155}, {"lat": -10, "lon": 155}, {"lat": -10, "lon": 110}], "resolution": 12},
                     {"ok": False, "error.code": "LIMIT_EXCEEDED"}, SPEC, "2026"))
    # More points for the stable bar (20+): seeded, every resolution, both hemispheres.
    import random
    rnd = random.Random(3)
    for _ in range(18):
        la, lo, r = rnd.uniform(-89, 89), rnd.uniform(-180, 180), rnd.randint(0, 15)
        c = h3.latlng_to_cell(la, lo, r)
        clat, clng = h3.cell_to_latlng(c)
        to_cell.append(vec(len(to_cell) + 1, {"lat": la, "lon": lo, "resolution": r}, {"result.cell": c, "result.center_lat.value": clat,
                                                                                       "result.center_lon.value": clng, "result.area.value": h3.cell_area(c, "km^2")}, tol=1e-9))
    # Grid disks for the stable bar: seeded cells at every resolution, and pentagons, where the disk has fewer cells.
    for _ in range(10):
        c = h3.latlng_to_cell(rnd.uniform(-89, 89), rnd.uniform(-180, 180), rnd.randint(0, 15))
        k = rnd.randint(0, 5)
        disk.append(vec(len(disk) + 1, {"cell": c, "k": k}, {"result.count": float(len(h3.grid_disk(c, k))), "result.cells.0.cell": c}))
    for r, k in [(0, 1), (3, 2), (7, 1), (10, 3), (15, 2)]:
        c = h3.get_pentagons(r)[r % 12]
        disk.append(vec(len(disk) + 1, {"cell": c, "k": k}, {"result.count": float(len(h3.grid_disk(c, k))), "result.cells.0.cell": c}))
    # The rest of the family to 20+ vectors: seeded cells at every resolution, one in five a pentagon.
    rnd2 = random.Random(34)
    for k in range(14):
        r = rnd2.randint(1, 15)
        c = h3.get_pentagons(r)[k % 12] if k % 5 == 0 else h3.latlng_to_cell(rnd2.uniform(-85, 85), rnd2.uniform(-180, 180), r)
        clat, clng = h3.cell_to_latlng(c)
        b = h3.cell_to_boundary(c)
        info.append(vec(len(info) + 1, {"cell": c}, {"result.lat.value": clat, "result.lon.value": clng, "result.resolution": float(r),
                                                    "result.base_cell": float(h3.get_base_cell_number(c)), "result.pentagon": "yes" if h3.is_pentagon(c) else "no",
                                                    "result.boundary.0.lat": b[0][0], "result.boundary.0.lon": b[0][1]}))
        rk = 1 + k % 3
        ring.append(vec(len(ring) + 1, {"cell": c, "k": rk}, {"result.count": float(len(h3.grid_ring(c, rk)))}))
        far = sorted(h3.grid_disk(c, 4))[k * 7 % len(h3.grid_disk(c, 4))]
        try:
            pc = h3.grid_path_cells(c, far)
            path.append(vec(len(path) + 1, {"from": c, "to": far}, {"result.distance": float(len(pc) - 1), "result.cells.0.cell": pc[0], f"result.cells.{len(pc) - 1}.cell": pc[-1]}))
        except Exception:
            path.append(vec(len(path) + 1, {"from": c, "to": far}, {"ok": False}))
        pr = rnd2.randint(0, r)
        parent.append(vec(len(parent) + 1, {"cell": c, "resolution": pr}, {"result.parent": h3.cell_to_parent(c, pr), "result.child_position": float(h3.cell_to_child_pos(c, pr))}))
        cr = min(15, r + 1 + k % 2)
        kids = sorted(h3.cell_to_children(c, cr))
        children.append(vec(len(children) + 1, {"cell": c, "resolution": cr}, {"result.count": float(len(kids)), "result.center_child": h3.cell_to_center_child(c, cr)}))
        group = sorted(set(h3.cell_to_children(h3.cell_to_parent(c, r - 1), r)) | set(h3.grid_disk(c, 1)))
        packed = sorted(h3.compact_cells(group))
        compact.append(vec(len(compact) + 1, {"cells": cells(group)}, {"result.count": float(len(packed))}))
        uncompact.append(vec(len(uncompact) + 1, {"cells": cells(packed), "resolution": r}, {"result.count": float(len(group))}))
        es = sorted(h3.origin_to_directed_edges(c))
        edges.append(vec(len(edges) + 1, {"cell": c}, {"result.edge_count": float(len(es))}))
    # The chooser and polygon fill to 20+ vectors: areas and edges across the range, and seeded boxes in every containment mode.
    for area, km2 in [("10 km2", 10.0), ("1000 km2", 1e3), ("50 m2", 5e-5), ("1 ha", 0.01), ("0.5 mi2", 0.5 * 2.589988110336), ("250000 km2", 2.5e5),
                      ("20000 km2", 2e4), ("3 km2", 3.0), ("1 ac", 0.0040468564224)]:
        chooser.append(vec(len(chooser) + 1, {"target_area": area}, {"result.resolution": float(nearest(lambda r: h3.average_hexagon_area(r, "km^2"), km2))},
                           "H3 average hexagon areas (h3.average_hexagon_area)", VER))
    for edge, km in [("1 km", 1.0), ("10 m", 0.01), ("100 km", 100.0), ("2 mi", 3.218688), ("500 ft", 0.1524), ("1 m", 0.001)]:
        chooser.append(vec(len(chooser) + 1, {"target_edge": edge}, {"result.resolution": float(nearest(lambda r: h3.average_hexagon_edge_length(r, "km"), km))},
                           "H3 average hexagon edge lengths (h3.average_hexagon_edge_length)", VER))
    rnd3 = random.Random(71)
    for k in range(15):
        r = rnd3.randint(3, 10)
        la, lo = rnd3.uniform(-70, 70), rnd3.uniform(-179, 179)
        span = 3.0 * h3.average_hexagon_edge_length(r, "km") / 111.0
        outer = [(la, lo), (la, lo + span * 1.5), (la + span, lo + span * 1.5), (la + span, lo)]
        mode = ["center", "full", "overlapping"][k % 3]
        got = sorted(h3.polygon_to_cells_experimental(h3.LatLngPoly(outer), r, contain={"overlapping": "overlap"}.get(mode, mode)))
        exp = {"result.count": float(len(got)), "result.containment": mode}
        if got:
            exp["result.cells.0.cell"] = got[0]
        fillv.append(vec(len(fillv) + 1, {"points": [{"lat": a, "lon": b} for a, b in outer], "resolution": r, "containment": mode}, exp))
    # Published examples: the H3 resolution table (res 9 averages 0.105332513 km2 over 4,842,432,842 cells), and the
    # San Francisco polygon in H3 C's testPolygonToCells.c (1,253 cells at res 9, 1,214 with the triangular hole).
    chooser.append(vec(len(chooser) + 1, {"target_area": "0.105332513 km2"}, {"result.resolution": 9.0, "result.average_area.value": 0.105332513,
                       "result.table.9.cells": 4842432842.0}, "H3 documentation, Tables of cell statistics across resolutions", "h3geo.org, retrieved 2026-09-19", tol=5e-10))
    sf = [(0.659966917655, -2.1364398519396), (0.6595011102219, -2.1359434279405), (0.6583348114025, -2.1354884206045),
          (0.6581220034068, -2.1382437718946), (0.6594479998527, -2.1384597563896), (0.6599990002976, -2.1376771158464)]
    hole = [(0.6595072188743, -2.1371053983433), (0.6591482046471, -2.1373141048153), (0.6592295020837, -2.1365222838402)]
    deg = lambda ring: [[math.degrees(lo), math.degrees(la)] for la, lo in ring + [ring[0]]]
    for rings, n in [([sf], 1253), ([sf, hole], 1214)]:
        gj = json.dumps({"type": "Polygon", "coordinates": [deg(r_) for r_ in rings]})
        fillv.append(vec(len(fillv) + 1, {"geojson": gj, "resolution": 9, "containment": "center"}, {"result.count": float(n)},
                         "H3 C test suite, testPolygonToCells.c (sfGeoPolygon, holeGeoPolygon)", "H3 v4.4.1"))
    f = {"indexing.h3.lat-lng-to-cell": to_cell, "indexing.h3.cell-info": info, "indexing.h3.grid-disk": disk, "indexing.h3.grid-ring": ring,
         "indexing.h3.grid-path": path, "indexing.h3.parent": parent, "indexing.h3.children": children, "indexing.h3.compact": compact,
         "indexing.h3.uncompact": uncompact, "indexing.h3.edges": edges, "indexing.h3.resolution-chooser": chooser, "indexing.h3.polygon-to-cells": fillv}
    for tool, vs in f.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
