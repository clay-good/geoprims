#!/usr/bin/env python3
"""Intercepts against an independent solver (promotion of
navigation.route.intercept).

The target runs a geodesic from its position; the pursuer flies a geodesic
to where they meet; the intercept is the first time t at which the range from
the pursuer's start to the target equals the pursuer's run, v t. The tool
steps f(t) = range - v t forward by f / (v + v_target). This finds the same
first root differently: f on a fixed grid of 0.01 h out to the tool's horizon,
the first grid interval where f changes sign, then bisection, all with
GeographicLib's Python package (not the Rust port the tool uses).
Cases where f comes within 50 m of zero before its first crossing (a
near-graze, where "first" depends on resolution) are skipped.

Writes core/crates/gp-navigation/tests/data/intercept.json (200 cases, some
unreachable) and appends 10 vectors to
core/vectors/navigation.route.intercept.jsonl (existing lines are left byte
for byte). Requires geographiclib."""
import json
import math
import random
from pathlib import Path

from geographiclib.geodesic import Geodesic

G = Geodesic.WGS84
KT = 1852.0  # m per hour per knot
FIX = Path("core/crates/gp-navigation/tests/data/intercept.json")
VEC = Path("core/vectors/navigation.route.intercept.jsonl")
DT = 0.01


def solve(p, v_kt, t0, tc, vt_kt):
    v, vt = v_kt * KT, vt_kt * KT

    def target(t):
        d = G.Direct(t0[0], t0[1], tc, vt * t)
        return d["lat2"], d["lon2"]

    def f(t):
        la, lo = target(t)
        return G.Inverse(p[0], p[1], la, lo)["s12"] - v * t

    horizon = min(2.0e7 / vt, 4.0e7 / v) if vt > 0 else 4.0e7 / v
    t, ft = 0.0, f(0.0)
    if ft <= 1e-3:
        return 0.0, "meets"
    while t < horizon:
        t2 = min(t + DT, horizon)
        f2 = f(t2)
        if f2 <= 0:
            lo, hi = t, t2
            for _ in range(80):
                m = (lo + hi) / 2
                if f(m) > 0:
                    lo = m
                else:
                    hi = m
            return (lo + hi) / 2, "meets"
        if f2 < 50.0:
            return None, "graze"
        t, ft = t2, f2
    return None, "unreachable"


def meet(p, t0, tc, vt_kt, t):
    d = G.Direct(t0[0], t0[1], tc, vt_kt * KT * t)
    inv = G.Inverse(p[0], p[1], d["lat2"], d["lon2"])
    return d["lat2"], d["lon2"], inv["s12"], inv["azi1"] % 360


def main():
    rng = random.Random(7)
    cases = []
    while len(cases) < 200:
        p = (rng.uniform(-70, 70), rng.uniform(-180, 180))
        brg, rng_nm = rng.uniform(0, 360), rng.uniform(1, 150)
        d = G.Direct(p[0], p[1], brg, rng_nm * 1852)
        t0 = (round(d["lat2"], 6), round(d["lon2"], 6))
        p = (round(p[0], 6), round(p[1], 6))
        v = round(rng.uniform(5, 40), 1)
        vt = round(rng.uniform(0, 30), 1)
        tc = round(rng.uniform(0, 360), 2)
        t, kind = solve(p, v, t0, tc, vt)
        if kind == "graze":
            continue
        row = {"lat": p[0], "lon": p[1], "speed": v, "target_lat": t0[0], "target_lon": t0[1], "target_course": tc, "target_speed": vt}
        if t is None:
            row["unreachable"] = True
        else:
            la, lo, s, az = meet(p, t0, tc, vt, t)
            row.update({"time_h": t, "meet_lat": la, "meet_lon": lo, "distance": s, "course": az})
        cases.append(row)
    FIX.parent.mkdir(parents=True, exist_ok=True)
    FIX.write_text(json.dumps({"source": f"geographiclib {__import__('geographiclib').__version__} (Python), 0.01 h grid then bisection", "cases": cases}, separators=(",", ":")) + "\n")

    lines = VEC.read_text().splitlines()
    kept = [l for l in lines if "gen_intercept_more.py" not in l]
    n, new = len(kept), []
    src = "First root of range - v t on a 0.01 h grid then bisection, GeographicLib Python (tools/vectors/gen_intercept_more.py)"
    for c in [c for c in cases if not c.get("unreachable")][:8] + [c for c in cases if c.get("unreachable")][:2]:
        inp = {"lat": c["lat"], "lon": c["lon"], "speed": f"{c['speed']} kt", "target_lat": c["target_lat"], "target_lon": c["target_lon"], "target_course": f"{c['target_course']} deg", "target_speed": f"{c['target_speed']} kt"}
        if c.get("unreachable"):
            exp, tol = {"ok": False, "error.code": "NO_SOLUTION"}, {}
        else:
            exp = {"result.time.value": c["time_h"] * 60, "result.distance.value": c["distance"], "result.course.value": c["course"], "result.meet_lat.value": c["meet_lat"], "result.meet_lon.value": c["meet_lon"], "ok": True}
            # The tool stops at a millimeter of range; at these speeds that is
            # well under 1e-5 min, and under a centimeter of run.
            tol = {"result.time.value": {"rel": 0, "abs": 1e-5}, "result.distance.value": {"rel": 0, "abs": 1e-2},
                   "result.course.value": {"rel": 0, "abs": 1e-6}, "result.meet_lat.value": {"rel": 0, "abs": 1e-7}, "result.meet_lon.value": {"rel": 0, "abs": 1e-7}}
        new.append({"id": f"v{n + len(new) + 1:03d}", "input": inp, "expect": exp, "source": src, "sourceVersion": "geographiclib 2 (Python)", "tolerance": tol})
    VEC.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print(len(cases), "cases,", sum(1 for c in cases if c.get("unreachable")), "unreachable;", n, "->", n + len(new), "vectors")


if __name__ == "__main__":
    main()
