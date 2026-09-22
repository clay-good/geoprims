#!/usr/bin/env python3
"""Golden vectors for navigation.vector.distance-3d and look-angles, from
GeographicLib's own command-line tools: CartConvert -l for the local
east-north-up frame, GeodSolve -i for the ground distance, and GeoidEval
(egm96-15, cubic) for the geoid height. Numbers go in fixed-point because the
GeographicLib parsers read an exponent's "e" as a hemisphere."""
import json
import math
import subprocess
import sys
from pathlib import Path

SRC = "GeographicLib CartConvert -l, GeodSolve -i, and GeoidEval (egm96-15) (tools/vectors/gen_vector3d.py)"
VER = "GeographicLib 2.x"
SPEC = "add-navigation-and-geometry scenarios"
GEOID_DIR = Path(__file__).resolve().parents[2] / "assets/data/egm96-15/2009-08-29"


def run(cmd, line):
    return [float(x) for x in subprocess.run(cmd, input=line + "\n", capture_output=True, text=True, check=True).stdout.split()]


def fx(v):
    return f"{v:.12f}"


def enu(p, q):
    return run(["CartConvert", "-p", "9", "-l", fx(p[0]), fx(p[1]), fx(p[2])], f"{fx(q[0])} {fx(q[1])} {fx(q[2])}")


def ground(p, q):
    return run(["GeodSolve", "-i", "-p", "9"], f"{fx(p[0])} {fx(p[1])} {fx(q[0])} {fx(q[1])}")[2]


def geoid(lat, lon):
    return run(["GeoidEval", "-n", "egm96-15", "-d", str(GEOID_DIR)], f"{fx(lat)} {fx(lon)}")[0]


def vec(i, inp, exp, src=SRC, ver=VER, tol=None):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: tol or {"abs": 1e-6} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def aer(e, n, u):
    return (math.degrees(math.atan2(e, n)) % 360, math.degrees(math.atan2(u, math.hypot(e, n))), math.sqrt(e * e + n * n + u * u))


def distance_vectors():
    out = []
    # The spec scenario: 2,000 m due north along the geodesic from 40° N, 105° W.
    north = run(["GeodSolve", "-p", "12"], "40 -105 0 2000")
    cases = [((40.0, -105.0, 250.0), (north[0], -105.0, 370.0)),
             ((51.4700, -0.4543, 25.0), (51.5074, -0.1278, 310.0)),
             ((-33.8688, 151.2093, 5.0), (-33.9399, 151.1753, 11000.0)),
             ((64.1466, -21.9426, 0.0), (65.6835, -18.1262, 2500.0)),
             ((0.0, 179.9, 100.0), (0.1, -179.9, 35786000.0))]
    for i, (p, q) in enumerate(cases, 1):
        e, n, u = enu(p, q)
        az, el, s = aer(e, n, u)
        g = ground(p, q)
        inp = {"lat1": p[0], "lon1": p[1], "height1": f"{p[2]} m", "lat2": q[0], "lon2": q[1], "height2": f"{q[2]} m"}
        exp = {"result.slant_range.value": s, "result.elevation_angle.value": el, "result.ground_distance.value": g,
               "result.height_difference.value": q[2] - p[2], "result.flat_earth_range.value": math.hypot(g, q[2] - p[2])}
        tol = {"abs": 1e-6} if s < 1e6 else {"abs": 1e-5}
        if i == 1:
            exp["result.azimuth.value"] = 0.0
        out.append(vec(i, inp, exp, SPEC if i == 1 else SRC, "2026" if i == 1 else VER, tol))
    # Sea-level heights through EGM96: h = H + N at each point.
    p, q = (39.7392, -104.9903, 1609.0), (39.9, -105.2, 2500.0)
    n1, n2 = geoid(p[0], p[1]), geoid(q[0], q[1])
    e, n, u = enu((p[0], p[1], p[2] + n1), (q[0], q[1], q[2] + n2))
    out.append(vec(len(out) + 1, {"lat1": p[0], "lon1": p[1], "height1": f"{p[2]} m", "reference1": "msl", "lat2": q[0], "lon2": q[1],
                                  "height2": f"{q[2]} m", "reference2": "msl", "geoid": "egm96"},
                   {"result.slant_range.value": aer(e, n, u)[2], "result.height_difference.value": q[2] + n2 - p[2] - n1}, tol={"abs": 1e-4}))
    # Mixed: only the second point is above sea level.
    e, n, u = enu(p, (q[0], q[1], q[2] + n2))
    out.append(vec(len(out) + 1, {"lat1": p[0], "lon1": p[1], "height1": f"{p[2]} m", "lat2": q[0], "lon2": q[1],
                                  "height2": f"{q[2]} m", "reference2": "msl", "geoid": "egm96"},
                   {"result.slant_range.value": aer(e, n, u)[2]}, tol={"abs": 1e-4}))
    # Mixed references with no geoid are refused.
    out.append(vec(len(out) + 1, {"lat1": 40, "lon1": -105, "height1": "250 m", "lat2": 40.01, "lon2": -105, "height2": "370 m", "reference2": "msl"},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/geoid"}, SPEC, "2026"))
    return out


def look_vectors():
    out = []
    R = 6371000.0
    cases = [((40.0, -105.0, 10.0), (40.0, -103.24, 3000.0), None),
             ((40.0, -105.0, 10.0), (40.0, -100.0, 3000.0), None),   # 426 km out: below the horizon
             ((40.0, -105.0, 1500.0), (40.2, -104.7, 1400.0), 0.13),
             ((34.0522, -118.2437, 100.0), (33.9416, -118.4085, 10.0), 0.25),
             ((0.0, 0.0, 0.0), (0.0, 0.0, 35786000.0), None)]
    for i, (p, q, k) in enumerate(cases, 1):
        e, n, u = enu(p, q)
        az, el, s = aer(e, n, u)
        kk = k or 0.0
        re = R / (1 - kk)
        horizon = -math.degrees(math.acos(re / (re + max(p[2], 0.0))))
        inp = {"observer_lat": p[0], "observer_lon": p[1], "observer_height": f"{p[2]} m",
               "target_lat": q[0], "target_lon": q[1], "target_height": f"{q[2]} m"}
        exp = {"result.elevation.value": el, "result.slant_range.value": s, "result.horizon_elevation.value": horizon}
        if e or n:
            exp["result.azimuth.value"] = az
        if k is not None:
            inp["k"] = k
            exp["result.apparent_elevation.value"] = el + math.degrees(kk * ground(p, q) / R / 2)
        if i == 2:
            exp["meta.warnings.0.code"] = "BELOW_HORIZON"
        out.append(vec(i, inp, exp, tol={"abs": 1e-6} if s < 1e6 else {"abs": 1e-5}))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for tool, vs in {"navigation.vector.distance-3d": distance_vectors(), "navigation.vector.look-angles": look_vectors()}.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
