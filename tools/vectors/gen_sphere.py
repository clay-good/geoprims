#!/usr/bin/env python3
"""Golden vectors for the spherical great-circle tools: the sphere worked by
hand in Python (haversine for the central angle, the standard course and
destination formulas, and slerp for the intermediate point) on R1 =
6,371,008.771 m, and the ellipsoidal comparisons from GeographicLib's
GeodSolve (C++). Requires GeodSolve."""
import json
import math
import subprocess
import sys
from pathlib import Path

VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
SRC = "Spherical formulas in Python and GeographicLib GeodSolve (tools/vectors/gen_sphere.py)"
SPEC = "add-navigation-and-geometry geodesic scenarios"
R1 = 6371008.771


def geod(args, line):
    # Fixed-point only: GeodSolve reads the "e" of 2.5e-15 as a hemisphere letter.
    line = " ".join(f"{float(x):.15f}" for x in line.split())
    return [float(x) for x in subprocess.run(["GeodSolve", "-p", "12", *args], input=line + "\n", capture_output=True, text=True, check=True).stdout.split()]


def vec(i, inp, exp, src=SRC, tol=1e-6):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": 1e-9 if ("lat" in k or "lon" in k or "course" in k) else tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def central(a, b):
    p1, p2, dl = math.radians(a[0]), math.radians(b[0]), math.radians(b[1] - a[1])
    h = math.sin((p2 - p1) / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 2 * math.asin(math.sqrt(h))


def course(a, b):
    p1, p2, dl = math.radians(a[0]), math.radians(b[0]), math.radians(b[1] - a[1])
    return math.degrees(math.atan2(math.sin(dl) * math.cos(p2), math.cos(p1) * math.sin(p2) - math.sin(p1) * math.cos(p2) * math.cos(dl))) % 360


def wrap(x):
    return (x + 180) % 360 - 180


PAIRS = [((40.6413, -73.7781), (51.47, -0.4543)), ((-33.9, 151.2), (1.35, 103.99)), ((0, 0), (0, 90)), ((64.1, -21.9), (61.2, -149.9)), ((10, 20), (10.0001, 20.0001))]


def inverse():
    out = []
    for a, b in PAIRS:
        s = central(a, b)
        s12, az1 = geod(["-i"], f"{a[0]} {a[1]} {b[0]} {b[1]}")[2], geod(["-i"], f"{a[0]} {a[1]} {b[0]} {b[1]}")[0]
        c1 = course(a, b)
        c2 = (course(b, a) + 180) % 360
        inp = {"lat1": a[0], "lon1": a[1], "lat2": b[0], "lon2": b[1]}
        out.append(vec(len(out) + 1, inp, {"result.distance.value": R1 * s / 1000, "result.initial_course.value": c1, "result.final_course.value": c2,
                                           "result.ellipsoidal_distance.value": s12 / 1000, "result.difference.value": s12 - R1 * s, "result.course_difference.value": wrap(c1 - az1)}))
    out[0]["source"] = SPEC + " (JFK to Heathrow: 5,540,019 m on R1, about 14,890 m shorter than the geodesic)"
    return out


def direct():
    out = []
    for (lat, lon), c, km in [((40, -74), 45, 1000), ((-33.9, 151.2), 290, 7000), ((0, 0), 90, 5000), ((70, 10), 0, 3000), ((51.47, -0.4543), 250, 0)]:
        d = km * 1000 / R1
        p1, th = math.radians(lat), math.radians(c)
        p2 = math.asin(math.sin(p1) * math.cos(d) + math.cos(p1) * math.sin(d) * math.cos(th))
        l2 = lon + math.degrees(math.atan2(math.sin(th) * math.sin(d) * math.cos(p1), math.cos(d) - math.sin(p1) * math.sin(p2)))
        la2, lo2 = math.degrees(p2), wrap(l2)
        ela, elo = geod([], f"{lat} {lon} {c} {km * 1000}")[:2]
        off = geod(["-i"], f"{la2} {lo2} {ela} {elo}")[2]
        fc = (course((la2, lo2), (lat, lon)) + 180) % 360 if km else c
        exp = {"result.lat2.value": la2, "result.lon2.value": lo2, "result.offset.value": off}
        if km:
            exp["result.final_course.value"] = fc
        out.append(vec(len(out) + 1, {"lat1": lat, "lon1": lon, "course": f"{c} deg", "distance": f"{km} km"}, exp, tol=1e-5))
    return out


def intermediate():
    out = []
    for (a, b), f in zip(PAIRS[:4] + [PAIRS[0]], [0.25, 0.5, 0.75, 0.1, 0.0]):
        d = central(a, b)
        A, B = math.sin((1 - f) * d) / math.sin(d), math.sin(f * d) / math.sin(d)
        v1 = [math.cos(math.radians(a[0])) * math.cos(math.radians(a[1])), math.cos(math.radians(a[0])) * math.sin(math.radians(a[1])), math.sin(math.radians(a[0]))]
        v2 = [math.cos(math.radians(b[0])) * math.cos(math.radians(b[1])), math.cos(math.radians(b[0])) * math.sin(math.radians(b[1])), math.sin(math.radians(b[0]))]
        v = [A * x + B * y for x, y in zip(v1, v2)]
        la, lo = math.degrees(math.atan2(v[2], math.hypot(v[0], v[1]))), math.degrees(math.atan2(v[1], v[0]))
        az1, _, s12 = geod(["-i"], f"{a[0]} {a[1]} {b[0]} {b[1]}")[:3]
        ela, elo = geod([], f"{a[0]} {a[1]} {az1} {f * s12}")[:2]
        off = geod(["-i"], f"{la} {lo} {ela} {elo}")[2]
        out.append(vec(len(out) + 1, {"lat1": a[0], "lon1": a[1], "lat2": b[0], "lon2": b[1], "fraction": f},
                       {"result.lat.value": la, "result.lon.value": lo, "result.ellipsoidal_lat.value": ela, "result.ellipsoidal_lon.value": elo, "result.offset.value": off}, tol=1e-5))
    out.append(vec(len(out) + 1, {"lat1": 0, "lon1": 0, "lat2": 0, "lon2": 180, "fraction": 0.5}, {"ok": False, "error.code": "NO_SOLUTION"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("navigation.geodesic.spherical-inverse", inverse()), ("navigation.geodesic.spherical-direct", direct()), ("navigation.geodesic.intermediate-point", intermediate())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
