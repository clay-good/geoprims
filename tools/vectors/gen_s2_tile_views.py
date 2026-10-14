#!/usr/bin/env python3
"""Golden vectors for the focused S2 and map-tile operations. S2 answers come
from s2sphere (a Python port of the S2 library, run from a virtualenv with
`pip install s2sphere`), independent of the core's own S2 port; tile answers
from the slippy-map and quadkey definitions written out here."""
import json
import math
import random
import sys
from pathlib import Path

import s2sphere as s2

S2_SRC = ("s2sphere (Python port of the S2 geometry library) (tools/vectors/gen_s2_tile_views.py)", f"s2sphere {getattr(s2, '__version__', '0.2')}")
TILE_SRC = ("Slippy-map XYZ, TMS, and Bing Maps quadkey definitions, evaluated in Python (tools/vectors/gen_s2_tile_views.py)", "definition")
SPEC = ("add-spatial-indexing-and-raster scenarios", "2026")
R = 6371008.8


def vec(i, inp, exp, src, tol=None):
    e = dict(exp)
    e.setdefault("ok", True)
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src[0], "sourceVersion": src[1],
            "tolerance": {k: (tol(k) if callable(tol) else (tol or {"abs": 0})) for k, v in e.items() if isinstance(v, float)}}


def refusal(i, inp, field):
    return vec(i, inp, {"ok": False, "error.code": "INVALID_INPUT", "error.field": field}, SPEC)


def cells(seed, n, first=()):
    rng = random.Random(seed)
    out = [s2.CellId.from_token(t) for t in first]
    while len(out) < n:
        ll = s2.LatLng.from_degrees(math.degrees(math.asin(rng.uniform(-1, 1))), rng.uniform(-180, 180))
        out.append(s2.CellId.from_lat_lng(ll).parent(rng.randint(0, 30)))
    return out


def written(c, i):
    """Alternate the token with the decimal id, the two forms the tools accept."""
    return str(c.id()) if i % 3 == 0 and len(str(c.id())) > 16 else c.to_token()


def generate(tool):
    out = []
    if tool == "token":
        for i, c in enumerate(cells(88101, 20, ["8085809c"]), 1):
            out.append(vec(i, {"cell": written(c, i)}, {"result.cell": c.to_token(), "result.cell_id": str(c.id()), "result.level": float(c.level()),
                                                        "result.face": float(c.face())}, S2_SRC))
    elif tool == "vertices":
        for i, c in enumerate(cells(88102, 20, ["47e66f"]), 1):
            cell = s2.Cell(c)
            e = {"result.cell": c.to_token()}
            for k in range(4):
                ll = s2.LatLng.from_point(cell.get_vertex(k))
                e[f"result.boundary.{k}.lat.value"], e[f"result.boundary.{k}.lon.value"] = ll.lat().degrees, ll.lng().degrees
            out.append(vec(i, {"cell": written(c, i)}, e, S2_SRC, {"abs": 1e-11}))
    elif tool == "hierarchy":
        rng = random.Random(88103)
        for i, c in enumerate(cells(88104, 20, ["89c25a3"]), 1):
            inp, e = {"cell": written(c, i)}, {"result.cell": c.to_token(), "result.level": float(c.level())}
            if c.level() > 0:
                e["result.parent"] = c.parent().to_token()
                level = 6 if i == 1 else rng.randint(0, c.level())
                inp["level"] = level
                e["result.ancestor"] = c.parent(level).to_token()
            if c.level() < 30:
                for k, child in enumerate(c.children()):
                    e[f"result.children.{k}.cell"] = child.to_token()
            out.append(vec(i, inp, e, S2_SRC))
        out.append(refusal(len(out) + 1, {"cell": "89c25a3", "level": 20}, "/level"))
    elif tool == "area":
        for i, c in enumerate(cells(88105, 20, ["6b12ae3fc"]), 1):
            if c.level() == 30:
                c = c.parent(29)
            a, avg = s2.Cell(c).exact_area() * R * R, s2.Cell(c).average_area() * R * R
            # The indexing ceiling, 1e-8 relative: below a few square meters the spherical
            # excess of two independent implementations parts in the ninth digit.
            out.append(vec(i, {"cell": written(c, i)}, {"result.area.value": a, "result.average_area.value": avg, "result.ratio": a / avg}, S2_SRC,
                           {"rel": 1e-8, "abs": 1e-9}))
    elif tool in ("xyz-tms", "tile-quadkey"):
        rng = random.Random({"xyz-tms": 88106, "tile-quadkey": 88107}[tool])
        cases = [("14/4551/6176", "xyz")] if tool == "xyz-tms" else [("13/2275/3088", None)]
        while len(cases) < 20:
            z = rng.randint(0, 22)
            n = 2 ** z
            cases.append((f"{z}/{rng.randrange(n)}/{rng.randrange(n)}", rng.choice(["xyz", "tms", None])))
        for i, (t, conv) in enumerate(cases, 1):
            z, x, y_in = (int(v) for v in t.split("/"))
            n = 2 ** z
            y = n - 1 - y_in if conv == "tms" else y_in
            inp = {"tile": t}
            if conv:
                inp["convention"] = conv
            if tool == "xyz-tms":
                xyz, tms = f"{z}/{x}/{y}", f"{z}/{x}/{n - 1 - y}"
                e = {"result.converted": f"XYZ {xyz}" if conv == "tms" else f"TMS {tms}", "result.xyz": xyz, "result.tms": tms}
            else:
                key = "".join(str(((x >> b) & 1) + 2 * ((y >> b) & 1)) for b in range(z - 1, -1, -1))
                e = {"result.quadkey": key, "result.xyz": f"{z}/{x}/{y}", "result.zoom": float(z)}
            out.append(vec(i, inp, e, TILE_SRC))
        out.append(refusal(len(out) + 1, {"tile": "3/9/1"}, "/tile"))
    if tool in ("token", "vertices", "hierarchy", "area"):
        out.append(refusal(len(out) + 1, {"cell": "not a token"}, "/cell"))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for tool, prefix in [("token", "s2"), ("vertices", "s2"), ("hierarchy", "s2"), ("area", "s2"), ("xyz-tms", "tile"), ("tile-quadkey", "tile")]:
        (out / f"indexing.{prefix}.{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in generate(tool)))


if __name__ == "__main__":
    main()
