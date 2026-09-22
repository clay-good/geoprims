#!/usr/bin/env python3
"""Golden vectors for geometry.predicate.point-in-polygon, built with
GeographicLib's GeodSolve (C++, independent of the tool's geographiclib-rs):
points placed exactly on geodesic edges, a pentagram whose middle is wound
twice, and an edge between two corners at one latitude, whose geodesic bows
poleward so the parallel's midpoint lies outside by the bow, from Clairaut's
relation. Requires GeodSolve."""
import json
import math
import subprocess
import sys
from pathlib import Path

A, F = 6378137.0, 1 / 298.257223563
E2 = F * (2 - F)
SRC = "Points built with GeographicLib's GeodSolve (C++), classified by hand (tools/vectors/gen_predicate.py)"
SPEC = "add-navigation-and-geometry scenarios"
VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"


def geod(args, line):
    return subprocess.run(["GeodSolve", "-p", "12", *args], input=line + "\n", capture_output=True, text=True, check=True).stdout.split()


def direct(lat, lon, az, s):
    o = geod([], f"{lat} {lon} {az} {s}")
    return float(o[0]), float(o[1])


def inverse(a, b):
    o = geod(["-i"], f"{a[0]} {a[1]} {b[0]} {b[1]}")
    return float(o[0]), float(o[2])


def vec(i, inp, exp, src=SRC, tol=None):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": (tol or {}).get(k, 1e-9)} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def P(*ll, ring=None):
    return [dict({"lat": a, "lon": b}, **({"ring": ring} if ring else {})) for a, b in ll]


def results(*rows):
    e = {}
    for k, (w, nz, eo) in enumerate(rows):
        if nz != "on-boundary":  # on an edge the sweep is ±180°, so the number is not defined
            e[f"result.results.{k}.winding"] = w
        e[f"result.results.{k}.nonzero"] = nz
        e[f"result.results.{k}.even_odd"] = eo
    return e


def main():
    out = []
    field = P((40.0, -105.0), (40.0, -104.99), (40.006, -104.99), (40.006, -105.0))
    pond = P((40.002, -104.997), (40.004, -104.997), (40.004, -104.994), (40.002, -104.994), ring=1)
    # Inside, in the hole, outside, and on the west edge (a meridian, so exactly on the geodesic).
    out.append(vec(len(out) + 1, {"polygon": field + pond, "points": P((40.001, -104.998), (40.003, -104.995), (39.99, -104.995), (40.003, -105.0))},
                   dict(results((1, "inside", "inside"), (0, "outside", "outside"), (0, "outside", "outside"), (1, "on-boundary", "on-boundary")),
                        **{"result.inside_count": 2, "result.results.3.distance.value": 0.0}), tol={"result.results.3.distance.value": 1e-3}))
    # A point exactly on a slanted geodesic edge, placed halfway along it by GeodSolve.
    a, b = (40.0, -105.0), (40.004, -104.992)
    az, s12 = inverse(a, b)
    mid = direct(a[0], a[1], az, s12 / 2)
    tri = P(a, b, (40.008, -105.004))
    out.append(vec(len(out) + 1, {"polygon": tri, "points": P(mid)}, results((0, "on-boundary", "on-boundary"))
                   | {"result.results.0.distance.value": 0.0}, tol={"result.results.0.distance.value": 1e-3}))
    # The parallel at 40° between two corners: the geodesic edge bows north, so the
    # parallel's midpoint is outside by the bow, M·(phi_vertex - 40°) from Clairaut.
    e1, e2 = (40.0, -105.0), (40.0, -104.99)
    az1, _ = inverse(e1, e2)
    beta1 = math.atan((1 - F) * math.tan(math.radians(40.0)))
    bmax = math.acos(abs(math.sin(math.radians(az1))) * math.cos(beta1))
    phiv = math.atan(math.tan(bmax) / (1 - F))
    s = math.sin(math.radians(40.0))
    m40 = A * (1 - E2) / (1 - E2 * s * s) ** 1.5
    bow = m40 * (phiv - math.radians(40.0))
    out.append(vec(len(out) + 1, {"polygon": field, "points": P((40.0, -104.995))},
                   results((0, "outside", "outside")) | {"result.results.0.distance.value": bow}, tol={"result.results.0.distance.value": 1e-5}))
    # A pentagram: its middle is wound twice, so the winding rule says inside and even-odd outside.
    c = (40.0, -105.0)
    star = [direct(c[0], c[1], az, 1000) for az in [0, 144, 288, 72, 216]]
    tip = direct(c[0], c[1], 0, 800)
    out.append(vec(len(out) + 1, {"polygon": P(*star), "points": P(c, tip, (40.05, -105.0))},
                   results((2, "inside", "outside"), (1, "inside", "inside"), (0, "outside", "outside")), SPEC))
    # A ring around the North Pole holds the pole, and a box across the antimeridian holds 180°.
    out.append(vec(len(out) + 1, {"polygon": P((80, 0), (80, 90), (80, 180), (80, -90)), "points": P((90, 0), (85, 45), (70, 10))},
                   results((1, "inside", "inside"), (1, "inside", "inside"), (0, "outside", "outside"))))
    out.append(vec(len(out) + 1, {"polygon": P((-1, 179), (-1, -179), (1, -179), (1, 179)), "points": P((0, 180), (0, 179.5), (0, 178))},
                   results((1, "inside", "inside"), (1, "inside", "inside"), (0, "outside", "outside"))))
    # Holes given in the same direction as the outline still count as holes.
    same = P((40.002, -104.994), (40.004, -104.994), (40.004, -104.997), (40.002, -104.997), ring=1)
    out.append(vec(len(out) + 1, {"polygon": field + same, "points": P((40.003, -104.995))}, results((0, "outside", "outside"))))
    out.append(vec(len(out) + 1, {"polygon": P((0, 0), (0, 1)), "points": P((0, 0.5))}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "geometry.predicate.point-in-polygon.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
