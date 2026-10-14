#!/usr/bin/env python3
"""Golden vectors for geodesy.grid-ref.grid-zone-lookup and mgrs-precision,
from the `mgrs` Python package (NGA's GeoTrans MGRS code, run from a
virtualenv with `pip install mgrs packaging`), not from the core."""
import json
import math
import random
import sys
from pathlib import Path

import mgrs

M = mgrs.MGRS()
SRC = ("mgrs (Python bindings to NGA GeoTrans MGRS) (tools/vectors/gen_mgrs_views.py)", "GeoTrans 3.x via mgrs")
SPEC = ("add-geodesy-suite scenarios", "2026")


def words(size):
    if size >= 1000:
        return f"{size // 1000:g} km"
    if size >= 1:
        return f"{size:g} m"
    return f"{size * 100:g} cm" if size >= 0.01 else f"{size * 1000:g} mm"


def vec(i, inp, exp, src=SRC):
    e = dict(exp)
    e.setdefault("ok", True)
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src[0], "sourceVersion": src[1],
            "tolerance": {k: {"abs": 1e-9} for k, v in e.items() if isinstance(v, float)}}


def split(ref):
    """(grid zone, square id) of a GeoTrans 100 km reference: the last two letters are the square."""
    return ref[:-2], ref[-2:]


def zone_vectors():
    rng = random.Random(99301)
    pts = [(78.2232, 15.6267), (40.446111, -79.982222), (-89.5, 10.0), (88.0, -45.0), (60.5, 4.5), (72.0, 20.0), (-33.8688, 151.2093), (0.0, 0.0), (83.9, -170.0)]
    while len(pts) < 20:
        pts.append((round(math.degrees(math.asin(rng.uniform(-1, 1))), 5), round(rng.uniform(-180, 180), 5)))
    out = []
    for i, (lat, lon) in enumerate(pts, 1):
        ref = M.toMGRS(lat, lon, MGRSPrecision=0)
        gzd, sq = split(ref)
        out.append(vec(i, {"lat": lat, "lon": lon}, {"result.square_100km": ref, "result.grid_zone": gzd, "result.square_id": sq}))
    out.append(vec(len(out) + 1, {"lat": 91, "lon": 0}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    return out


def precision_vectors():
    rng = random.Random(99302)
    cases = [("17T NE 86309 77770", None)]
    while len(cases) < 20:
        lat, lon = round(rng.uniform(-79, 83), 5), round(rng.uniform(-180, 180), 5)
        p = rng.randint(0, 5)
        ref = M.toMGRS(lat, lon, MGRSPrecision=p)
        cases.append((ref, (lat, lon, p)))
    out = []
    for i, (text, src) in enumerate(cases, 1):
        compact = text.replace(" ", "")
        if src is None:
            lat, lon = M.toLatLon(compact)
            src = (lat, lon, (len(compact) - 5) // 2)
        lat, lon, p = src
        exp = {"result.precision": words(10 ** (5 - p)), "result.digits": float(p), "result.square_size.value": float(10 ** (5 - p))}
        # Each coarser reference is the GeoTrans encoding at that precision: MGRS truncates.
        for k, q in enumerate(range(p - 1, -1, -1)):
            exp[f"result.coarser.{k}.reference"] = compact[:len(compact) - 2 * p] + compact[len(compact) - 2 * p:][:q] + compact[len(compact) - p:][:q]
            exp[f"result.coarser.{k}.square"] = words(10 ** (5 - q))
            if src is not None and i > 1:
                assert exp[f"result.coarser.{k}.reference"] == M.toMGRS(lat, lon, MGRSPrecision=q), (text, q)
        out.append(vec(i, {"mgrs": text}, exp))
    for bad in ["17TNE863097777", "99XAA", "not a reference"]:
        out.append(vec(len(out) + 1, {"mgrs": bad}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/mgrs"}, SPEC))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, make in [("grid-zone-lookup", zone_vectors), ("mgrs-precision", precision_vectors)]:
        (out / f"geodesy.grid-ref.{name}.jsonl").write_text("".join(json.dumps(v, separators=(",", ":")) + "\n" for v in make()))


if __name__ == "__main__":
    main()
