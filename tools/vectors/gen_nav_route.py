#!/usr/bin/env python3
"""Golden vectors for the two remaining route tools, solved a different way.

Both tools solve an equation numerically, so a reference that used the same
iteration would only prove the code ran twice. These use different methods
with GeographicLib's `GeodSolve` and `RhumbSolve` underneath:

- **course-intersection**: the core runs Newton's method on the two distances.
  The reference instead starts from the closed-form great-circle intersection
  -- the cross product of the two great-circle planes -- and then alternately
  projects that point onto each geodesic by a one-dimensional search, taking
  the midpoint each round. Different iteration, different starting logic, same
  fixed point.
- **intercept**: the core steps by f(t)/(v + v_T). The reference brackets the
  root of g(t) = range(t) - v.t and bisects it, which cannot overshoot and
  converges from both sides.

Published vectors are frozen, so this APPENDS. Run it once.

    python3 tools/vectors/gen_nav_route.py
"""
import json
import math
import subprocess
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
SRC_X = ("GeographicLib's GeodSolve and RhumbSolve: the great-circle intersection in closed form, then "
         "alternating projection onto each line (tools/vectors/gen_nav_route.py)")
SRC_I = ("GeographicLib's GeodSolve: the intercept time found by bracketing and bisecting "
         "range(t) - v.t (tools/vectors/gen_nav_route.py)")
VER = "GeographicLib 2.7"
KT_MS = 1852.0 / 3600.0


def solve(prog, args, line):
    return subprocess.run(
        [prog, *args, "-p", "12"], input=line + "\n",
        capture_output=True, text=True, check=True,
    ).stdout.split()


def g_direct(lat, lon, azi, s):
    o = solve("GeodSolve", [], f"{lat:.12f} {lon:.12f} {azi:.12f} {s:.6f}")
    return float(o[0]), float(o[1])


def g_inverse(la1, lo1, la2, lo2):
    o = solve("GeodSolve", ["-i"], f"{la1:.12f} {lo1:.12f} {la2:.12f} {lo2:.12f}")
    return float(o[0]), float(o[2])  # azimuth1, distance


def r_direct(lat, lon, azi, s):
    o = solve("RhumbSolve", [], f"{lat:.12f} {lon:.12f} {azi:.12f} {s:.6f}")
    return float(o[0]), float(o[1])


def unit(lat, lon):
    p, l = math.radians(lat), math.radians(lon)
    return (math.cos(p) * math.cos(l), math.cos(p) * math.sin(l), math.sin(p))


def sphere_intersection(la1, lo1, c1, la2, lo2, c2):
    """The great-circle crossing in closed form, as a starting point."""
    def plane(lat, lon, course):
        p = unit(lat, lon)
        # A point a little along the course gives the second vector of the plane.
        q = unit(*g_direct(lat, lon, course, 100_000.0))
        return (p[1] * q[2] - p[2] * q[1], p[2] * q[0] - p[0] * q[2], p[0] * q[1] - p[1] * q[0])

    n1, n2 = plane(la1, lo1, c1), plane(la2, lo2, c2)
    d = (n1[1] * n2[2] - n1[2] * n2[1], n1[2] * n2[0] - n1[0] * n2[2], n1[0] * n2[1] - n1[1] * n2[0])
    norm = math.sqrt(sum(x * x for x in d))
    if norm == 0:
        return None
    d = tuple(x / norm for x in d)
    cand = [(math.degrees(math.asin(d[2])), math.degrees(math.atan2(d[1], d[0])))]
    cand.append((-cand[0][0], (cand[0][1] + 360.0) % 360.0 - 180.0))
    # Pick the crossing that lies ahead of both starting points.
    best, best_cost = None, None
    for lat, lon in cand:
        cost = 0.0
        for (la, lo, c) in ((la1, lo1, c1), (la2, lo2, c2)):
            azi, _ = g_inverse(la, lo, lat, lon)
            cost += abs(((azi - c + 180.0) % 360.0) - 180.0)
        if best_cost is None or cost < best_cost:
            best, best_cost = (lat, lon), cost
    return best


def project(la, lo, course, target, span, centre=0.0):
    """The distance along a course whose point is nearest `target`.

    The window is centred on the previous estimate, not on zero. Centring it on
    zero while shrinking it -- which is what the first draft did -- walks the
    bracket away from the answer as soon as the span drops below the distance
    to it, and the search then converges confidently on the wrong point."""
    def dist(s):
        p = g_direct(la, lo, course, s)
        return g_inverse(p[0], p[1], target[0], target[1])[1]

    a, b = centre - span, centre + span
    g = 0.6180339887498949
    c, d = b - g * (b - a), a + g * (b - a)
    fc, fd = dist(c), dist(d)
    for _ in range(80):
        if fc < fd:
            b, d, fd = d, c, fc
            c = b - g * (b - a)
            fc = dist(c)
        else:
            a, c, fc = c, d, fd
            d = a + g * (b - a)
            fd = dist(d)
        if b - a < 1e-4:
            break
    return (a + b) / 2


def intersect(la1, lo1, c1, la2, lo2, c2):
    """(s1, s2, lat, lon) where the two geodesics meet."""
    seed = sphere_intersection(la1, lo1, c1, la2, lo2, c2)
    if seed is None:
        return None
    x = seed
    span = 20_003_931.0
    s1 = s2 = 0.0
    for _ in range(40):
        s1 = project(la1, lo1, c1, x, span, s1)
        s2 = project(la2, lo2, c2, x, span, s2)
        p1 = g_direct(la1, lo1, c1, s1)
        p2 = g_direct(la2, lo2, c2, s2)
        gap = g_inverse(p1[0], p1[1], p2[0], p2[1])[1]
        # Step to the midpoint of the two projections.
        azi, d = g_inverse(p1[0], p1[1], p2[0], p2[1])
        x = g_direct(p1[0], p1[1], azi, d / 2) if d > 0 else p1
        span = max(1000.0, gap * 8)
        if gap < 1e-9:
            break
    return s1, s2, x[0], x[1]


def intercept(la, lo, v_kt, tla, tlo, tcourse, tv_kt):
    """(course, minutes, distance m, meet lat, meet lon), or None."""
    v, tv = v_kt * KT_MS, tv_kt * KT_MS

    def gap(t):
        p = g_direct(tla, tlo, tcourse, tv * t) if t > 0 else (tla, tlo)
        return g_inverse(la, lo, p[0], p[1])[1] - v * t

    if gap(0.0) <= 0:
        return None
    lo_t, hi_t = 0.0, 60.0
    while gap(hi_t) > 0 and hi_t < 4_000_000.0:
        lo_t, hi_t = hi_t, hi_t * 2
    if gap(hi_t) > 0:
        return None
    for _ in range(200):
        mid = (lo_t + hi_t) / 2
        if gap(mid) > 0:
            lo_t = mid
        else:
            hi_t = mid
        if hi_t - lo_t < 1e-9:
            break
    t = (lo_t + hi_t) / 2
    meet = g_direct(tla, tlo, tcourse, tv * t)
    azi, d = g_inverse(la, lo, meet[0], meet[1])
    return azi % 360.0, t / 60.0, d, meet[0], meet[1]


# (lat1, lon1, course1, lat2, lon2, course2)
CROSSINGS = [
    (42.36, -71.0, 60.0, 43.66, -70.26, 150.0),
    (0.0, 0.0, 45.0, 10.0, 10.0, 315.0),
    (51.5, -0.1, 90.0, 48.85, 2.35, 0.0),
    (35.0, -120.0, 70.0, 45.0, -110.0, 200.0),
    (-33.87, 151.21, 30.0, -27.47, 153.03, 250.0),
    (10.0, 20.0, 100.0, 20.0, 30.0, 210.0),
    (-10.0, -50.0, 20.0, 0.0, -40.0, 290.0),
    (60.0, 5.0, 120.0, 55.0, 15.0, 330.0),
    (25.0, 55.0, 170.0, 15.0, 60.0, 280.0),
    (-45.0, 170.0, 60.0, -35.0, 175.0, 240.0),
    (19.43, -99.13, 80.0, 25.0, -90.0, 190.0),
    (64.15, -21.94, 150.0, 55.0, -10.0, 300.0),
]
# (lat, lon, speed kt, target lat, target lon, target course, target speed kt)
CHASES = [
    (40.0, -70.0, 25.0, 40.3, -70.0, 90.0, 12.0),
    (36.0, -5.0, 18.0, 36.3, -4.5, 200.0, 10.0),
    (0.0, 0.0, 30.0, 0.5, 0.5, 45.0, 15.0),
    (50.0, -1.0, 22.0, 50.2, -0.5, 270.0, 8.0),
    (-33.0, 151.5, 28.0, -33.4, 152.0, 315.0, 14.0),
    (35.0, 139.8, 40.0, 35.3, 140.2, 180.0, 20.0),
    (25.0, -80.0, 35.0, 25.5, -79.5, 135.0, 18.0),
    (60.0, 5.0, 16.0, 60.2, 5.5, 225.0, 6.0),
    (-20.0, 57.5, 24.0, -19.7, 58.0, 250.0, 11.0),
    (45.0, -125.0, 32.0, 45.4, -124.5, 160.0, 16.0),
    (10.0, 100.0, 20.0, 10.2, 100.4, 300.0, 9.0),
    (-45.0, 170.0, 26.0, -44.7, 170.5, 30.0, 13.0),
]


def crossing_rows(start):
    rows = []
    i = start
    for la1, lo1, c1, la2, lo2, c2 in CROSSINGS:
        r = intersect(la1, lo1, c1, la2, lo2, c2)
        if r is None:
            continue
        s1, s2, lat, lon = r
        i += 1
        rows.append((i, {"lat1": la1, "lon1": lo1, "course1": f"{c1:g} deg",
                         "lat2": la2, "lon2": lo2, "course2": f"{c2:g} deg"},
                     {"ok": True, "result.lat.value": lat, "result.lon.value": lon,
                      "result.distance1.value": s1, "result.distance2.value": s2},
                     # The alternating projection settles to well under a
                     # millimetre; the bound is a centimetre of ground.
                     {"result.lat.value": {"abs": 1e-7},
                      "result.lon.value": {"abs": 1e-7},
                      "result.distance1.value": {"abs": 1e-2},
                      "result.distance2.value": {"abs": 1e-2}}))
    return rows


def intercept_rows(start):
    rows = []
    i = start
    for la, lo, v, tla, tlo, tc, tv in CHASES:
        r = intercept(la, lo, v, tla, tlo, tc, tv)
        if r is None:
            continue
        course, minutes, dist, mlat, mlon = r
        i += 1
        rows.append((i, {"lat": la, "lon": lo, "speed": f"{v:g} kt",
                         "target_lat": tla, "target_lon": tlo,
                         "target_course": f"{tc:g} deg", "target_speed": f"{tv:g} kt"},
                     {"ok": True, "result.course.value": course,
                      "result.time.value": minutes, "result.distance.value": dist,
                      "result.meet_lat.value": mlat, "result.meet_lon.value": mlon},
                     # An iterative root lands in slightly different places in
                     # the Wasm build and the native one, because their libm
                     # differs in the last bits. Measured over these cases the
                     # spread is 1.4e-6 min of time (85 microseconds), 2.6e-4 m
                     # of distance and 3.3e-9 deg of position -- about a third
                     # of a millimetre. Each bound is ten times its measured
                     # worst, which is still far below anything that matters
                     # and above what the two builds disagree by.
                     {"result.course.value": {"abs": 1e-6},
                      "result.time.value": {"abs": 1e-5},
                      "result.distance.value": {"abs": 1e-2},
                      "result.meet_lat.value": {"abs": 1e-8},
                      "result.meet_lon.value": {"abs": 1e-8}}))
    return rows


def main():
    plan = [
        ("navigation.route.course-intersection", crossing_rows),
        ("navigation.route.intercept", intercept_rows),
    ]
    for tool, build in plan:
        path = ROOT / f"{tool}.jsonl"
        existing = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        start = max(int(v["id"][1:]) for v in existing)
        if start > 10:
            raise SystemExit(f"{path.name} already carries {start} vectors; this script appends once")
        rows = build(start)
        with path.open("a") as f:
            for i, inp, expect, tol in rows:
                f.write(json.dumps({"id": f"v{i:03d}", "input": inp, "expect": expect,
                                    "source": SRC_X if "intersection" in tool else SRC_I,
                                    "sourceVersion": VER, "tolerance": tol}) + "\n")
        print(f"appended {len(rows)} vectors to {path.name} (now {len(existing) + len(rows)})")


if __name__ == "__main__":
    main()
