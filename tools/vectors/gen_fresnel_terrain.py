#!/usr/bin/env python3
"""Golden vectors for raster.terrain.fresnel-terrain, worked independently of
the core with the polar equation of the sight line (as in gen_sightline.py,
copied here because that script writes its own file when imported):

    1 / r(phi) = [sin(theta - phi) / r_o + sin(phi) / r_t] / sin(theta),

on a sphere of effective radius Re = R / (1 - k), k = 0.25 unless given, and
the first Fresnel radius sqrt(lambda d1 d2 / d) (ITU-R P.530). A point passes
when its clearance is at least 0.6 of that radius.

Usage: python3 tools/vectors/gen_fresnel_terrain.py [out_dir]
"""
import json
import math
import random
import sys
from pathlib import Path

C = 299_792_458.0
SRC = "The polar equation of the sight line in Python, independent of the core's Cartesian form (tools/vectors/gen_fresnel_terrain.py)"
VER = "ITU-R P.530-19"


def solve(profile, ho, ht, freq_hz, k=0.25, radius=6_371_000.0):
    re = radius / (1 - k)
    s0 = profile[0][0]
    pts = [(s - s0, e) for s, e in profile]
    total = pts[-1][0]
    theta = total / re
    ro, rt = re + pts[0][1] + ho, re + pts[-1][1] + ht
    rows = []
    for s, e in pts[1:-1]:
        phi = s / re
        r = math.sin(theta) / (math.sin(theta - phi) / ro + math.sin(phi) / rt)
        clear = r - re - e
        f1 = math.sqrt(C / freq_hz * s * (total - s) / total)
        rows.append({"s": s, "clear": clear, "f1": f1, "ratio": clear / f1})
    least = min(rows, key=lambda w: w["clear"])
    worst = min(rows, key=lambda w: w["ratio"])
    short = sum(w["ratio"] < 0.6 for w in rows)
    return {"fresnel_clear": "yes" if short == 0 else "no", "worst": worst, "short": float(short),
            "visible": "yes" if all(w["clear"] >= 0 for w in rows) else "no", "least": least}


def main():
    rng = random.Random(58)
    cases = [
        ([(0, 210), (3, 188), (6, 236), (9, 181), (12, 205)], 38, 32, 5.8, None),
        ([(0, 300), (4, 340), (8, 395), (12, 330), (16, 310)], 40, 45, 11.0, None),
        ([(0, 50), (10, 40), (20, 30), (30, 45), (40, 60)], 30, 30, 2.4, 0.13),
        ([(0, 0), (5, 0), (10, 0), (15, 0), (20, 0)], 15, 15, 5.8, 0.0),
    ]
    while len(cases) < 22:
        n = rng.randint(3, 9)
        length = rng.choice([2, 5, 10, 20, 40])
        dists = sorted({0, length, *(round(rng.uniform(0.2, length - 0.2), 3) for _ in range(n - 2))})
        ground = [round(rng.uniform(100, 400), 1) for _ in dists]
        cases.append((list(zip(dists, ground)), rng.choice([10, 20, 30, 50]), rng.choice([10, 20, 30, 50]),
                      rng.choice([0.9, 2.4, 5.8, 11.0, 18.0, 23.0]), rng.choice([None, None, 0.13, 0.0])))
    out = []
    for prof, ho, ht, ghz, k in cases:
        r = solve([(d * 1000.0, float(e)) for d, e in prof], float(ho), float(ht), ghz * 1e9, **({} if k is None else {"k": k}))
        inp = {"points": [{"distance": f"{d} km", "elevation": f"{e} m"} for d, e in prof],
               "observer_height": f"{ho} m", "target_height": f"{ht} m", "frequency": f"{ghz} GHz"}
        if k is not None:
            inp["k"] = k
        exp = {"ok": True, "result.fresnel_clear": r["fresnel_clear"], "result.fresnel_worst_percent": 100 * r["worst"]["ratio"],
               "result.fresnel_worst_at.value": r["worst"]["s"] / 1000, "result.fresnel_short": r["short"],
               "result.visible": r["visible"], "result.clearance.value": r["least"]["clear"]}
        tol = {"result.fresnel_worst_percent": {"abs": 1e-6}, "result.fresnel_worst_at.value": {"abs": 1e-9},
               "result.fresnel_short": {"abs": 0}, "result.clearance.value": {"abs": 1e-6}}
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    base = {"points": [{"distance": "0 km", "elevation": "100 m"}, {"distance": "5 km", "elevation": "120 m"}, {"distance": "10 km", "elevation": "100 m"}],
            "observer_height": "20 m", "target_height": "20 m"}
    for inp, code, field in [(base, "INVALID_INPUT", "/frequency"), (base | {"frequency": "0 GHz"}, "OUT_OF_DOMAIN", "/frequency")]:
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": {"ok": False, "error.code": code, "error.field": field},
                    "source": "add-spatial-indexing-and-raster scenarios", "sourceVersion": "2026", "tolerance": {}})
    (Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors") / "raster.terrain.fresnel-terrain.jsonl").write_text(
        "".join(json.dumps(v, separators=(",", ":")) + "\n" for v in out))
    print(len(out))


if __name__ == "__main__":
    main()
