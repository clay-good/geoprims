#!/usr/bin/env python3
"""Golden vectors for geometry.validity.make-valid, from GeographicLib's C++
tools (independent of the tool's geographiclib-rs): areas by Planimeter, and
the bow-tie's crossing on its mirror meridian by bisection with GeodSolve.
Requires GeodSolve and Planimeter."""
import json
import subprocess
import sys
from pathlib import Path

SRC = "Areas by GeographicLib's Planimeter and crossings by GeodSolve (C++), via tools/vectors/gen_validity.py"
SPEC = "add-navigation-and-geometry scenarios"
VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"


def area(pts):
    out = subprocess.run(["Planimeter", "-p", "9"], input="".join(f"{a} {b}\n" for a, b in pts), capture_output=True, text=True, check=True).stdout.split()
    return abs(float(out[2]))


def geod(args, line):
    return subprocess.run(["GeodSolve", "-p", "12", *args], input=line + "\n", capture_output=True, text=True, check=True).stdout.split()


def crossing_on_meridian(a, b, lon):
    az, s12 = map(float, (lambda o: (o[0], o[2]))(geod(["-i"], f"{a[0]} {a[1]} {b[0]} {b[1]}")))
    lo, hi = 0.0, s12
    for _ in range(60):
        mid = (lo + hi) / 2
        o = geod([], f"{a[0]} {a[1]} {az} {mid}")
        if (float(o[1]) - lon) * (b[1] - a[1]) < 0:
            lo = mid
        else:
            hi = mid
    o = geod([], f"{a[0]} {a[1]} {az} {(lo + hi) / 2}")
    return float(o[0]), float(o[1])


def vec(i, inp, exp, src=SRC, tol=None):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: (tol or {}).get(k, {"rel": 1e-9, "abs": 1e-9}) for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def P(pts, ring=None):
    return [dict({"lat": a, "lon": b}, **({"ring": ring} if ring else {})) for a, b in pts]


def main():
    out = []
    # The spec's bow-tie: two triangles, the crossing located on the mirror meridian.
    c = [(40.0, -105.0), (40.004, -104.995), (40.0, -104.995), (40.004, -105.0)]
    x = crossing_on_meridian(c[0], c[1], -104.9975)
    tri = area([c[0], x, c[3]]) + area([x, c[1], c[2]])
    out.append(vec(len(out) + 1, {"polygon": P(c)},
                   {"result.valid": "no", "result.parts": 2, "result.problem_count": 1, "result.problems.0.problem": "self-intersection: edges 1 and 3 cross or touch",
                    "result.problems.0.lat.value": x[0], "result.problems.0.lon.value": x[1], "result.area.value": tri / 1e6, "meta.warnings.1.code": "GEOMETRY_INVALID"},
                   SPEC, tol={"result.problems.0.lat.value": {"rel": 0, "abs": 1e-8}, "result.problems.0.lon.value": {"rel": 0, "abs": 1e-8}, "result.area.value": {"rel": 1e-6, "abs": 0}}))
    # A valid square with a hole: nothing to report, the area the difference.
    sq = [(40.0, -105.0), (40.0, -104.99), (40.008, -104.99), (40.008, -105.0)]
    hole = [(40.002, -104.998), (40.006, -104.998), (40.006, -104.994), (40.002, -104.994)]
    out.append(vec(len(out) + 1, {"polygon": P(sq) + P(hole, 1)}, {"result.valid": "yes", "result.problem_count": 0, "result.parts": 1, "result.area.value": (area(sq) - area(hole)) / 1e6},
                   tol={"result.area.value": {"rel": 1e-9, "abs": 0}}))
    # A repeated corner is reported and dropped.
    out.append(vec(len(out) + 1, {"polygon": P([sq[0], sq[1], sq[1], sq[2], sq[3]])}, {"result.valid": "no", "result.problems.0.problem": "repeated corner", "result.problems.0.corner": 3, "result.area.value": area(sq) / 1e6},
                   tol={"result.area.value": {"rel": 1e-9, "abs": 0}}))
    # A spike out and straight back is reported, and the repair drops it.
    spike = [sq[0], sq[1], (40.004, -104.99), (40.004, -104.98), (40.004, -104.99), sq[2], sq[3]]
    out.append(vec(len(out) + 1, {"polygon": P(spike)}, {"result.valid": "no", "result.problems.0.problem": "spike", "result.problems.0.corner": 4, "result.parts": 1, "result.area.value": area(sq) / 1e6},
                   tol={"result.area.value": {"rel": 1e-6, "abs": 0}}))
    # A hole outside the outline.
    far = [(40.1, -105.0), (40.101, -105.0), (40.101, -104.999)]
    out.append(vec(len(out) + 1, {"polygon": P(sq) + P(far, 1)}, {"result.valid": "no", "result.problems.0.problem": "hole outside the outline"}))
    # A clockwise outline is valid under Simple Features: noted, not counted.
    out.append(vec(len(out) + 1, {"polygon": P(sq[::-1])}, {"result.valid": "yes", "result.problem_count": 0, "result.problems.0.problem": "note: outline runs clockwise (RFC 7946 wants counterclockwise)"}))
    out.append(vec(len(out) + 1, {"polygon": P(sq[:2])}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geometry.validity.make-valid.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
