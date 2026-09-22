#!/usr/bin/env python3
"""Golden vectors for course intersections and intercepts, from GeographicLib's
C++ tools (independent of the tool's geographiclib-rs): IntersectTool for the
geodesic crossing and GeodSolve for its position; for rhumbs, the crossing of
two straight lines in Mercator coordinates with the isometric latitude
psi = asinh(tan phi) - e atanh(e sin phi), and RhumbSolve for the runs; for
intercepts, bisection on range(t) - v t with GeodSolve. Requires GeographicLib."""
import json
import math
import subprocess
import sys
from pathlib import Path

VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
SRC = "GeographicLib IntersectTool, GeodSolve, and RhumbSolve (tools/vectors/gen_intersect.py)"
SPEC = "add-navigation-and-geometry route-geometry scenarios"
F = 1 / 298.257223563
E = math.sqrt(F * (2 - F))
KT = 1852 / 3600


def run(cmd, line):
    return [float(x) for x in subprocess.run(cmd, input=line + "\n", capture_output=True, text=True, check=True).stdout.split()]


def direct(lat, lon, az, s):
    return run(["GeodSolve", "-p", "12"], f"{lat} {lon} {az} {s}")[:2]


def inverse(a, b):
    o = run(["GeodSolve", "-i", "-p", "12"], f"{a[0]} {a[1]} {b[0]} {b[1]}")
    return o[2], o[0]  # distance, azi1


def vec(i, inp, exp, src=SRC, tol=1e-6):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": 1e-9 if "lat" in k or "lon" in k or "course" in k else tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": VER, "tolerance": t}


def psi(lat):
    p = math.radians(lat)
    return math.asinh(math.tan(p)) - E * math.atanh(E * math.sin(p))


def lat_of(y):
    p = math.atan(math.sinh(y))
    for _ in range(60):
        p = math.atan(math.sinh(y + E * math.atanh(E * math.sin(p))))
    return math.degrees(p)


def rhumb_cross(a, b):
    da = (math.sin(math.radians(a[2])), math.cos(math.radians(a[2])))
    db = (math.sin(math.radians(b[2])), math.cos(math.radians(b[2])))
    det = da[0] * -db[1] + da[1] * db[0]
    dx, dy = math.radians(b[1] - a[1]), psi(b[0]) - psi(a[0])
    u = (dx * -db[1] - dy * -db[0]) / det
    lat = lat_of(psi(a[0]) + u * da[1])
    lon = a[1] + math.degrees(u * da[0])
    s = []
    for p in (a, b):
        d, az = run(["RhumbSolve", "-i", "-p", "12"], f"{p[0]} {p[1]} {lat} {lon}")[1], run(["RhumbSolve", "-i", "-p", "12"], f"{p[0]} {p[1]} {lat} {lon}")[0]
        off = (az - p[2] + 540) % 360 - 180
        s.append(-d if abs(off) > 90 else d)
    return lat, lon, s[0], s[1]


def intersections():
    out = []
    for a, b in [((42.36, -71.0, 45), (43.66, -70.26, 135)), ((10, 0, 80), (-20, 40, 10)), ((51.5, -0.1, 270), (40.6, -73.8, 30)),
                 ((-33.9, 151.2, 120), (-36.8, 174.8, 200)), ((60, 20, 0), (60, 30, 315))]:
        x, y, _ = run(["IntersectTool", "-p", "12"], f"{a[0]} {a[1]} {a[2]} {b[0]} {b[1]} {b[2]}")
        lat, lon = direct(a[0], a[1], a[2], x)
        inp = {"lat1": a[0], "lon1": a[1], "course1": f"{a[2]} deg", "lat2": b[0], "lon2": b[1], "course2": f"{b[2]} deg"}
        out.append(vec(len(out) + 1, inp, {"result.distance1.value": x, "result.distance2.value": y, "result.lat.value": lat, "result.lon.value": lon}))
    for a, b in [((42.36, -71.0, 45), (43.66, -70.26, 135)), ((10, 0, 80), (20, 30, 190))]:
        lat, lon, s1, s2 = rhumb_cross(a, b)
        inp = {"lat1": a[0], "lon1": a[1], "course1": f"{a[2]} deg", "lat2": b[0], "lon2": b[1], "course2": f"{b[2]} deg", "method": "rhumb"}
        out.append(vec(len(out) + 1, inp, {"result.distance1.value": s1, "result.distance2.value": s2, "result.lat.value": lat, "result.lon.value": lon}, tol=1e-5))
    out.append(vec(len(out) + 1, {"lat1": 0, "lon1": 0, "course1": "90 deg", "lat2": 0, "lon2": 10, "course2": "90 deg"}, {"ok": False, "error.code": "NO_SOLUTION"}))
    return out


def intercepts():
    out = []
    for p, v, t0, tc, vt in [((40.0, -70.0), 20, (40.1665, -70.0), 90, 12), ((36.0, -5.0), 15, (36.3, -4.5), 200, 10),
                             ((0.0, 0.0), 30, (0.5, 0.5), 315, 25), ((55.0, 10.0), 8, (55.05, 10.05), 0, 0)]:
        vm, vtm = v * KT * 3600, vt * KT * 3600
        f = lambda t: inverse(p, direct(t0[0], t0[1], tc, vtm * t))[0] - vm * t
        lo, hi = 0.0, 0.01
        while f(hi) > 0:
            lo, hi = hi, hi * 2
        for _ in range(60):
            mid = (lo + hi) / 2
            if f(mid) > 0:
                lo = mid
            else:
                hi = mid
        t = (lo + hi) / 2
        meet = direct(t0[0], t0[1], tc, vtm * t)
        s, az = inverse(p, meet)
        inp = {"lat": p[0], "lon": p[1], "speed": f"{v} kt", "target_lat": t0[0], "target_lon": t0[1], "target_course": f"{tc} deg", "target_speed": f"{vt} kt"}
        out.append(vec(len(out) + 1, inp, {"result.course.value": az % 360, "result.time.value": t * 60, "result.distance.value": s,
                                           "result.meet_lat.value": meet[0], "result.meet_lon.value": meet[1]}, tol=1e-2))
        out[-1]["tolerance"].update({k: {"rel": 0, "abs": 1e-6} for k in ("result.course.value", "result.meet_lat.value", "result.meet_lon.value")})
        out[-1]["tolerance"]["result.time.value"] = {"rel": 0, "abs": 1e-5}
    out.append(vec(len(out) + 1, {"lat": 40.0, "lon": -70.0, "speed": "10 kt", "target_lat": 40.1665, "target_lon": -70.0, "target_course": "0 deg", "target_speed": "12 kt"},
                   {"ok": False, "error.code": "NO_SOLUTION"}, SPEC + " (unreachable: the target opens faster than the pursuer closes)"))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("navigation.route.course-intersection", intersections()), ("navigation.route.intercept", intercepts())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
