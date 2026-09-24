#!/usr/bin/env python3
"""More course-intersection vectors, and a parity fixture, from GeographicLib's
C++ IntersectTool (promotion of navigation.route.course-intersection).

Appends to core/vectors/navigation.route.course-intersection.jsonl (published
vectors are frozen: existing lines are left byte for byte) and writes
core/crates/gp-navigation/tests/data/course_intersection.json.

Two geodesics cross over and over as they wind round the Earth. The tool
reports one of the two nearest crossings (the pair about half the Earth
apart): the one behind fewer of the two positions, then the nearer. The
reference takes every crossing IntersectTool lists within one circumference
(-R), keeps the two nearest by |x| + |y|, and applies that rule. Pairs where
the two nearest are within 0.1% of each other, or where a crossing lies
within 1% of half the world away along either course (where ahead and
behind name the same place), are left out, since either is a fair answer. Rhumb vectors use gen_intersect.py's Mercator crossing
with RhumbSolve for the runs."""
import json
import random
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from gen_intersect import VER, direct, rhumb_cross, vec  # noqa: E402

HALF = 20_003_931.5  # half a meridian on WGS 84, m

VEC = Path("core/vectors/navigation.route.course-intersection.jsonl")
FIX = Path("core/crates/gp-navigation/tests/data/course_intersection.json")


def crossings(cases):
    lines = [" ".join(f"{v:.12f}" for v in c) for c in cases]
    out = subprocess.run(["IntersectTool", "-c", "-R", "40100000", "-p", "9"], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True).stdout.splitlines()
    res, cur = [], []
    for line in out:
        f = line.split()
        if f[0] == "nan":
            res.append(cur)
            cur = []
        else:
            cur.append((float(f[0]), float(f[1]), int(float(f[2]))))
    assert len(res) == len(cases)
    picked = []
    for r in res:
        near = sorted([x for x in r if x[2] == 0], key=lambda x: abs(x[0]) + abs(x[1]))[:2]
        if len(near) < 2:
            picked.append(None)
            continue
        l0, l1 = (abs(x[0]) + abs(x[1]) for x in near)
        key = lambda x: ((x[0] < 0) + (x[1] < 0), abs(x[0]) + abs(x[1]))
        a, b = sorted(near, key=key)
        # About half the world away along either course, ahead and behind
        # name the same place (runs are read within half a circumference
        # either way), so which crossing is "behind fewer" is not defined.
        if any(abs(abs(v) - HALF) < 0.01 * HALF for x in near for v in x[:2]):
            picked.append(None)
            continue
        if key(a)[0] == key(b)[0] and abs(l1 - l0) < 1e-3 * l1:
            picked.append(None)  # a near-tie: either crossing is a fair answer
            continue
        picked.append(a[:2])
    return picked


def random_cases(rng, n):
    out = []
    for _ in range(n):
        la1, lo1 = rng.uniform(-80, 80), rng.uniform(-180, 180)
        la2 = max(-85.0, min(85.0, la1 + rng.uniform(-25, 25)))
        lo2 = (lo1 + rng.uniform(-30, 30) + 540) % 360 - 180
        out.append([la1, lo1, rng.uniform(0, 360), la2, lo2, rng.uniform(0, 360)])
    return out


def main():
    rng = random.Random(42)
    cases = random_cases(rng, 620)
    ref = crossings(cases)
    keep = [(c, r) for c, r in zip(cases, ref) if r is not None][:500]
    FIX.parent.mkdir(parents=True, exist_ok=True)
    FIX.write_text(json.dumps({"source": f"IntersectTool -R, {VER}", "cases": [c for c, _ in keep], "runs": [r for _, r in keep]}, separators=(",", ":")) + "\n")

    lines = VEC.read_text().splitlines()
    kept = [l for l in lines if "gen_intersect_more.py" not in l]
    n = len(kept)
    new = []
    src = f"GeographicLib IntersectTool -R, the two nearest crossings, fewer behind then nearer (tools/vectors/gen_intersect_more.py)"
    for c, _ in keep:
        if len(new) == 10:
            break
        inp = {"lat1": round(c[0], 6), "lon1": round(c[1], 6), "course1": f"{round(c[2], 4)} deg", "lat2": round(c[3], 6), "lon2": round(c[4], 6), "course2": f"{round(c[5], 4)} deg"}
        # Recomputed on the rounded inputs the vector carries.
        r = crossings([[inp["lat1"], inp["lon1"], round(c[2], 4), inp["lat2"], inp["lon2"], round(c[5], 4)]])[0]
        if r is None:
            continue
        x, y = r
        lat, lon = direct(inp["lat1"], inp["lon1"], round(c[2], 4), x)
        new.append(vec(n + len(new) + 1, inp, {"result.distance1.value": x, "result.distance2.value": y, "result.lat.value": lat, "result.lon.value": lon}, src))
    # Kept off the antimeridian: gen_intersect.py's Mercator helper does not
    # wrap longitude, so across it the reference would take a far crossing.
    for a, b in [((0.0, 0.0, 30), (10.0, 20.0, 300)), ((-40.0, 150.0, 100), (-45.0, 165.0, 350)), ((65.0, -20.0, 60), (70.0, 10.0, 170)), ((5.0, 100.0, 225), (-5.0, 95.0, 315))]:
        lat, lon, s1, s2 = rhumb_cross(a, b)
        inp = {"lat1": a[0], "lon1": a[1], "course1": f"{a[2]} deg", "lat2": b[0], "lon2": b[1], "course2": f"{b[2]} deg", "method": "rhumb"}
        v = vec(n + len(new) + 1, inp, {"result.distance1.value": s1, "result.distance2.value": s2, "result.lat.value": lat, "result.lon.value": ((lon + 540) % 360) - 180}, "Mercator crossing with RhumbSolve runs (tools/vectors/gen_intersect.py helpers, via gen_intersect_more.py)", tol=1e-5)
        new.append(v)
    VEC.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print(len(keep), "fixture pairs;", n, "->", n + len(new), "vectors")


if __name__ == "__main__":
    main()
