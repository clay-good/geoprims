#!/usr/bin/env python3
"""Golden vectors for the focused H3 operations, from H3 C through the `h3`
Python bindings (run from a virtualenv with `pip install h3`), independent of
the h3o library the core links."""
import json
import random
import sys
from pathlib import Path

import h3

SRC = f"H3 C {h3.versions()['c']} via h3-py {h3.__version__} (tools/vectors/gen_h3_views.py)"
VER = f"H3 {h3.versions()['c']}"
SPEC = ("add-spatial-indexing-and-raster scenarios", "2026")
# h3o and H3 C agree within 1e-12° below 88° latitude; areas within 2e-8 relative at resolution 15.
ANGLE, REL = {"abs": 1e-11}, {"rel": 1e-8, "abs": 1e-15}


def vec(i, inp, exp, tol=None, src=(SRC, VER)):
    e = dict(exp)
    e.setdefault("ok", True)
    tolerance = {}
    for k, v in e.items():
        if isinstance(v, float):
            tolerance[k] = tol(k) if callable(tol) else (tol or {"abs": 0})
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src[0], "sourceVersion": src[1], "tolerance": tolerance}


def refusal(i, inp, code="INVALID_INPUT", field=None):
    e = {"ok": False, "error.code": code}
    if field:
        e["error.field"] = field
    return vec(i, inp, e, src=SPEC)


def cells(seed, n, res_range=(0, 15), first=()):
    rng = random.Random(seed)
    out = list(first)
    while len(out) < n:
        out.append(h3.latlng_to_cell(rng.uniform(-80, 80), rng.uniform(-180, 180), rng.randint(*res_range)))
    return out


PENTAGONS = [h3.get_pentagons(r)[i % 12] for i, r in enumerate([0, 3, 5, 8, 11, 14])]


def angle_tol(k):
    return ANGLE if k.endswith(("lat", "lon", "lat.value", "lon.value")) else {"abs": 0}


def generate(tool):
    out = []
    if tool == "cell-to-latlng":
        for i, c in enumerate(cells(77101, 20, first=["8a2a1072b59ffff", PENTAGONS[1]]), 1):
            la, lo = h3.cell_to_latlng(c)
            out.append(vec(i, {"cell": c}, {"result.lat.value": la, "result.lon.value": lo, "result.resolution": float(h3.get_resolution(c))}, angle_tol))
    elif tool == "cell-boundary":
        for i, c in enumerate(cells(77102, 20, first=["873933676ffffff", PENTAGONS[2]]), 1):
            b = h3.cell_to_boundary(c)
            e = {"result.vertex_count": float(len(b))}
            for j in sorted({0, len(b) - 1}):
                e[f"result.boundary.{j}.lat"], e[f"result.boundary.{j}.lon"] = b[j]
            out.append(vec(i, {"cell": c}, e, angle_tol))
    elif tool == "resolution":
        for i, c in enumerate(cells(77103, 20, first=["8c2a1072b59adff"]), 1):
            r = h3.get_resolution(c)
            out.append(vec(i, {"cell": c}, {"result.resolution": float(r), "result.average_area.value": h3.average_hexagon_area(r, "km^2")},
                           lambda k: REL if "area" in k else {"abs": 0}))
    elif tool == "base-cell":
        for i, c in enumerate(cells(77104, 20, first=["8966e42888fffff", PENTAGONS[3]]), 1):
            base = h3.cell_to_parent(c, 0)
            out.append(vec(i, {"cell": c}, {"result.base_cell": float(h3.get_base_cell_number(c)), "result.base_pentagon": "yes" if h3.is_pentagon(base) else "no",
                                            "result.base_index": base}))
    elif tool == "is-pentagon":
        for i, c in enumerate(cells(77105, 20, first=["830800fffffffff", *PENTAGONS]), 1):
            p = h3.is_pentagon(c)
            out.append(vec(i, {"cell": c}, {"result.pentagon": "yes" if p else "no", "result.resolution": float(h3.get_resolution(c)), "result.neighbors": 5.0 if p else 6.0}))
    elif tool == "is-class-iii":
        for i, c in enumerate(cells(77106, 20, first=["89283082803ffff"]), 1):
            out.append(vec(i, {"cell": c}, {"result.class3": "yes" if h3.is_res_class_III(c) else "no", "result.resolution": float(h3.get_resolution(c))}))
    elif tool == "validate":
        good = cells(77107, 10, first=["892a8471487ffff"])
        bad = ["8f2a1072b59a5f7", "892a8471487fff", "zz2a8471487ffff", "1192a8471487ffff", "0", "ffffffffffffffff", "892a8471487ffffffff", "202a8471487ffff", "8f2a1072b59a5ff", "hello"]
        for c in [bad[0]] + good + bad[1:]:
            try:
                v = h3.is_valid_cell(c) if all(ch in "0123456789abcdefABCDEF" for ch in c) else False
            except OverflowError:  # more than 64 bits
                v = False
            e = {"result.valid": "yes" if v else "no"}
            if v:
                e["result.resolution"] = float(h3.get_resolution(c))
            out.append(vec(len(out) + 1, {"cell": c}, e))
    elif tool == "cell-area":
        for i, c in enumerate(cells(77108, 20, first=["88ad361519fffff", PENTAGONS[4]]), 1):
            r = h3.get_resolution(c)
            a, avg = h3.cell_area(c, "km^2"), h3.average_hexagon_area(r, "km^2")
            out.append(vec(i, {"cell": c}, {"result.area.value": a, "result.average_area.value": avg, "result.ratio": a / avg}, REL))
    elif tool == "grid-distance":
        rng = random.Random(77109)
        pairs = [("892a8471487ffff", "892a84710cbffff")]
        while len(pairs) < 20:
            a = h3.latlng_to_cell(rng.uniform(-70, 70), rng.uniform(-180, 180), rng.randint(3, 12))
            ring = h3.grid_ring(a, rng.randint(0, 6))
            b = ring[rng.randrange(len(ring))]
            if not any(h3.is_pentagon(x) for x in h3.grid_disk(a, 7)):
                pairs.append((a, b))
        for i, (a, b) in enumerate(pairs, 1):
            out.append(vec(i, {"from": a, "to": b}, {"result.distance": float(h3.grid_distance(a, b))}))
        out.append(refusal(len(out) + 1, {"from": "892a8471487ffff", "to": "8a2a1072b59ffff"}, field="/to"))
    elif tool == "center-child":
        rng = random.Random(77110)
        pairs = [("872a10729ffffff", 11)] + [(c, min(15, h3.get_resolution(c) + rng.randint(0, 4))) for c in cells(77111, 19, (0, 12))]
        for i, (c, r) in enumerate(pairs, 1):
            child = h3.cell_to_center_child(c, r)
            la, lo = h3.cell_to_latlng(child)
            out.append(vec(i, {"cell": c, "resolution": r}, {"result.center_child": child, "result.lat.value": la, "result.lon.value": lo}, angle_tol))
        out.append(refusal(len(out) + 1, {"cell": "872a10729ffffff", "resolution": 5}, field="/resolution"))
    elif tool == "child-position":
        rng = random.Random(77112)
        pairs = [("8a2a1072b59ffff", 7)] + [(c, rng.randint(0, h3.get_resolution(c))) for c in cells(77113, 17, (1, 15))]
        pairs += [(h3.cell_to_center_child(PENTAGONS[0], 4), 0), (h3.cell_to_children(PENTAGONS[1], 6)[-1], 3)]
        for i, (c, r) in enumerate(pairs, 1):
            parent = h3.cell_to_parent(c, r)
            out.append(vec(i, {"cell": c, "resolution": r}, {"result.position": float(h3.cell_to_child_pos(c, r)), "result.parent": parent,
                                                             "result.siblings": float(len(h3.cell_to_children(parent, h3.get_resolution(c))) if h3.get_resolution(c) - r <= 6 else 7 ** (h3.get_resolution(c) - r) - (0 if not h3.is_pentagon(parent) else 0))}))
        out.append(refusal(len(out) + 1, {"cell": "872a10729ffffff", "resolution": 9}, field="/resolution"))
    elif tool == "directed-edges":
        for i, c in enumerate(cells(77114, 20, first=["882f5a363bfffff", PENTAGONS[2]]), 1):
            # Ascending index order, the order the core lists them in.
            edges = sorted(h3.origin_to_directed_edges(c))
            e = {"result.edge_count": float(len(edges))}
            for j in sorted({0, len(edges) - 1}):
                e[f"result.edges.{j}.edge"] = edges[j]
                e[f"result.edges.{j}.to"] = h3.get_directed_edge_destination(edges[j])
            out.append(vec(i, {"cell": c}, e))
    elif tool == "vertexes":
        for i, c in enumerate(cells(77115, 20, first=["867a6e42fffffff", PENTAGONS[3]]), 1):
            vs = h3.cell_to_vertexes(c)
            e = {"result.vertex_count": float(len(vs))}
            for j in sorted({0, len(vs) - 1}):
                e[f"result.vertexes.{j}.vertex"] = vs[j]
                e[f"result.vertexes.{j}.lat"], e[f"result.vertexes.{j}.lon"] = h3.vertex_to_latlng(vs[j])
            out.append(vec(i, {"cell": c}, e, angle_tol))
    elif tool == "edge-length":
        rng = random.Random(77116)
        edges = ["1192a8471487ffff"]
        for c in cells(77117, 19, (0, 15)):
            es = h3.origin_to_directed_edges(c)
            edges.append(es[rng.randrange(len(es))])
        for i, e in enumerate(edges, 1):
            out.append(vec(i, {"edge": e}, {"result.length.value": h3.edge_length(e, "m"), "result.origin": h3.get_directed_edge_origin(e),
                                            "result.destination": h3.get_directed_edge_destination(e)}, REL))
        out.append(refusal(len(out) + 1, {"edge": "892a8471487ffff"}, field="/edge"))
    if tool not in ("grid-distance", "center-child", "child-position", "edge-length", "validate"):
        out.append(refusal(len(out) + 1, {"cell": "not a cell"}, field="/cell"))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    tools = sys.argv[2:] or ["cell-to-latlng", "cell-boundary", "resolution", "base-cell", "is-pentagon", "is-class-iii", "validate", "cell-area",
                             "grid-distance", "center-child", "child-position", "directed-edges", "vertexes", "edge-length"]
    for tool in tools:
        (out / f"indexing.h3.{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in generate(tool)))


if __name__ == "__main__":
    main()
