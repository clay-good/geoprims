#!/usr/bin/env python3
"""Golden vectors for navigation.route.cross-track, computed with Karney's own
Python geographiclib (pip install geographiclib): the closest point on the
geodesic A→B is where the course to P is perpendicular to the line, found by
bisection after a coarse scan; independent of the core's gnomonic method.

Also fly-by turns, time-speed-distance, and CPA, evaluated here from the
spec's formulas. Writes core/vectors/navigation.route.*.jsonl.
"""
import json
import math
import random
from pathlib import Path

from geographiclib.geodesic import Geodesic

G = Geodesic.WGS84
VECTORS = Path(__file__).resolve().parents[2] / "core/vectors"
OUT = VECTORS / "navigation.route.cross-track.jsonl"
KT = 1852 / 3600
G0 = 9.80665
SRC = "Karney's geographiclib (Python): perpendicularity bisection along the geodesic"
VER = "geographiclib 2.1"


def closest(a, b, p):
    line = G.InverseLine(a[0], a[1], b[0], b[1])
    L = line.s13

    def ahead(s):
        x = line.Position(s)
        to_p = G.Inverse(x["lat2"], x["lon2"], p[0], p[1])["azi1"]
        return math.cos(math.radians(to_p - x["azi2"]))

    # Coarse scan for the sign change, then bisection.
    grid = [L * (k / 400 * 2 - 0.5) for k in range(401)]
    for s0, s1 in zip(grid, grid[1:]):
        if ahead(s0) > 0 >= ahead(s1):
            lo, hi = s0, s1
            break
    else:
        raise ValueError("no foot in range")
    for _ in range(80):
        mid = (lo + hi) / 2
        if ahead(mid) > 0:
            lo = mid
        else:
            hi = mid
    s = (lo + hi) / 2
    x = line.Position(s)
    xt = G.Inverse(x["lat2"], x["lon2"], p[0], p[1])["s12"]
    # Right of course when P is clockwise from the line direction.
    to_p = G.Inverse(x["lat2"], x["lon2"], p[0], p[1])["azi1"]
    side = math.sin(math.radians(to_p - x["azi2"]))
    return s, (xt if side > 0 else -xt), x["lat2"], x["lon2"], L


def main():
    rnd = random.Random(99)
    cases = [((40.6413, -73.7781), (51.47, -0.4543), (44.0, -60.0))]
    for _ in range(21):
        a = (rnd.uniform(-70, 70), rnd.uniform(-180, 180))
        d = G.Direct(a[0], a[1], rnd.uniform(0, 360), rnd.uniform(50e3, 2000e3))
        b = (d["lat2"], d["lon2"])
        f = G.Direct(a[0], a[1], d["azi1"], d["s12"] * rnd.uniform(-0.3, 1.3))
        pp = G.Direct(f["lat2"], f["lon2"], f["azi2"] + rnd.choice([90, -90]), rnd.uniform(0, 300e3))
        cases.append((a, b, (pp["lat2"], pp["lon2"])))
    out = []
    for i, (a, b, p) in enumerate(cases, 1):
        s, xt, flat, flon, L = closest(a, b, p)
        exp = {"result.cross_track.value": xt / 1000, "result.along_track.value": s / 1000,
               "result.foot_lat.value": flat, "result.foot_lon.value": flon, "result.segment.value": L / 1000,
               "result.within": "yes" if 0 <= s <= L else "no", "ok": True}
        tol = {k: {"abs": 1e-6} for k in ("result.cross_track.value", "result.along_track.value", "result.segment.value")}
        tol.update({k: {"abs": 1e-9} for k in ("result.foot_lat.value", "result.foot_lon.value")})
        out.append({"id": f"v{i:03d}", "input": {"lat1": a[0], "lon1": a[1], "lat2": b[0], "lon2": b[1], "lat": p[0], "lon": p[1]},
                    "expect": exp, "source": SRC, "sourceVersion": VER, "tolerance": tol})
    # Ed Williams' Aviation Formulary, "Cross track error": LAX to JFK with the aircraft at N34:30 W116:30 gives
    # 7.4512 nm right of course and 99.588 nm along it. The formulary is spherical with a nautical mile per minute
    # of arc (R = 6,366.7 km), 0.07% smaller than the mean radius the tool's spherical method uses.
    av = ("Ed Williams, Aviation Formulary, Cross track error worked example (LAX to JFK)", "version 1.47")
    out.append({"id": f"v{len(out) + 1:03d}",
                "input": {"lat1": 33 + 57 / 60, "lon1": -(118 + 24 / 60), "lat2": 40 + 38 / 60, "lon2": -(73 + 47 / 60),
                          "lat": 34.5, "lon": -116.5, "method": "spherical",
                          "options": {"outputUnits": {"cross_track": "NM", "along_track": "NM"}}},
                "expect": {"result.cross_track.value": 7.4512, "result.along_track.value": 99.588, "ok": True},
                "source": av[0], "sourceVersion": av[1],
                "tolerance": {"result.cross_track.value": {"abs": 0.01}, "result.along_track.value": {"abs": 0.08}}})
    OUT.write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))
    print(len(out), "vectors ->", OUT.name)
    formulas(rnd)


def write(tool, rows):
    (VECTORS / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in rows))
    print(len(rows), "vectors ->", tool)


def vec(i, inp, exp, tol, src):
    e = dict(exp)
    e["ok"] = True
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": tol}


def formulas(rnd):
    turn_src = "Coordinated turn: R = V^2/(g tan bank), lead = R tan(dpsi/2), evaluated in Python"
    rows = []
    cases = [(360, 90, 120, 25), (90, 45, 120, None)] + [(rnd.uniform(0, 360), rnd.uniform(0, 360), rnd.uniform(60, 480), rnd.choice([None, rnd.uniform(10, 35)])) for _ in range(20)]
    for i, (a, b, kt, bank) in enumerate(cases, 1):
        v = kt * KT
        phi = bank if bank is not None else math.degrees(math.atan(v * math.radians(3) / G0))
        r = v * v / (G0 * math.tan(math.radians(phi)))
        d = (b - a + 180) % 360 - 180
        lead = r * math.tan(math.radians(abs(d) / 2))
        inp = {"inbound": f"{a!r} deg", "outbound": f"{b!r} deg", "speed": f"{kt!r} kt"}
        if bank is not None:
            inp["bank"] = f"{bank!r} deg"
        rows.append(vec(i, inp, {"result.radius.value": r, "result.lead_distance.value": lead, "result.arc_length.value": r * math.radians(abs(d)),
                                 "result.direction": "right" if d >= 0 else "left"},
                        {k: {"rel": 1e-12, "abs": 1e-9} for k in ("result.radius.value", "result.lead_distance.value", "result.arc_length.value")}, turn_src))
    write("navigation.route.fly-by", rows)

    tsd_src = "distance = speed x time, evaluated in Python"
    rows = []
    for i in range(1, 23):
        nm, kt = rnd.uniform(5, 3000), rnd.uniform(40, 550)
        h = nm / kt
        total = round(h * 60)
        ete = f"{total // 60} h {total % 60:02d} min" if total >= 60 else f"{total} min"
        which = i % 3
        if which == 0:
            inp, exp = {"distance": f"{nm!r} NM", "speed": f"{kt!r} kt"}, {"result.time.value": h, "result.ete": ete}
        elif which == 1:
            inp, exp = {"distance": f"{nm!r} NM", "time": f"{h!r} h"}, {"result.speed.value": kt}
        else:
            inp, exp = {"speed": f"{kt!r} kt", "time": f"{h!r} h"}, {"result.distance.value": nm}
        rows.append(vec(i, inp, exp, {k: {"rel": 1e-12, "abs": 1e-9} for k, v in exp.items() if isinstance(v, float)}, tsd_src))
    # Bowditch's printed Table 11 (NGA Pub. 9, Volume II, 2024 edition): published
    # cells across the table, each to the tenth of a mile it is printed to. The cell
    # at 38 minutes and 10.5 knots misprints 6.8 for 6.65 and is left out; the
    # differential in tests/tsd_parity.rs pins it instead.
    bow = ("Bowditch, The American Practical Navigator (NGA Pub. 9), Volume II, Table 11, Speed, Time, and Distance", "2024 edition")
    for minutes, knots, miles in [(60, 12.0, 12.0), (30, 20.0, 10.0), (45, 8.5, 6.4), (12, 32.0, 6.4), (7, 40.0, 4.7), (23, 15.5, 5.9), (50, 0.5, 0.4)]:
        rows.append({"id": f"v{len(rows) + 1:03d}", "input": {"speed": f"{knots} kt", "time": f"{minutes} min", "options": {"outputUnits": {"distance": "NM"}}},
                     "expect": {"result.distance.value": miles, "ok": True}, "source": bow[0], "sourceVersion": bow[1],
                     # Half a tenth, plus slack for a cell that lands exactly on the half.
                     "tolerance": {"result.distance.value": {"abs": 0.051}}})
    write("navigation.route.time-speed-distance", rows)

    cpa_src = "Relative motion in a flat plane: t = -(r.v)/|v|^2, evaluated in Python"
    rows = []
    cases = [(90, 10, 1000, 1200, 180, 10)] + [(rnd.uniform(0, 360), rnd.uniform(1, 60), rnd.uniform(-2e4, 2e4), rnd.uniform(-2e4, 2e4), rnd.uniform(0, 360), rnd.uniform(1, 60)) for _ in range(21)]
    for i, (ac, asp, bx, by, bc, bsp) in enumerate(cases, 1):
        va = (asp * math.sin(math.radians(ac)), asp * math.cos(math.radians(ac)))
        vb = (bsp * math.sin(math.radians(bc)), bsp * math.cos(math.radians(bc)))
        vx, vy = vb[0] - va[0], vb[1] - va[1]
        t = max(0.0, -(bx * vx + by * vy) / (vx * vx + vy * vy))
        rx, ry = bx + vx * t, by + vy * t
        inp = {"a_course": f"{ac!r} deg", "a_speed": f"{asp!r} m/s", "b_east": f"{bx!r} m", "b_north": f"{by!r} m", "b_course": f"{bc!r} deg", "b_speed": f"{bsp!r} m/s"}
        exp = {"result.time.value": t, "result.separation.value": math.hypot(rx, ry)}
        rows.append(vec(i, inp, exp, {k: {"rel": 1e-9, "abs": 1e-9} for k in exp}, cpa_src))
    # In 3D, with heights and climb rates: the same minimum over (east, north, up).
    cpa3_src = "Relative motion in a local east-north-up frame: t = -(r.v)/|v|^2, evaluated in Python"
    for ac, asp, bx, by, bz, bc, bsp, vza, vzb in [(360, 128.6111, 0, 37040, 304.8, 180, 128.6111, 0, 0),
                                                  (360, 100, 0, 10000, -1000, 180, 100, -10, 0),
                                                  (45, 60, 5000, 8000, 600, 250, 55, 2.54, -5.08),
                                                  (270, 80, -12000, 3000, 0, 90, 20, 0, 7.5)]:
        va = (asp * math.sin(math.radians(ac)), asp * math.cos(math.radians(ac)), vza)
        vb = (bsp * math.sin(math.radians(bc)), bsp * math.cos(math.radians(bc)), vzb)
        v = [vb[k] - va[k] for k in range(3)]
        r0 = (bx, by, bz)
        t = max(0.0, -sum(r0[k] * v[k] for k in range(3)) / sum(x * x for x in v))
        r = [r0[k] + v[k] * t for k in range(3)]
        inp = {"a_course": f"{ac} deg", "a_speed": f"{asp} m/s", "b_east": f"{bx} m", "b_north": f"{by} m", "b_course": f"{bc} deg",
               "b_speed": f"{bsp} m/s", "b_up": f"{bz} m", "a_vertical_speed": f"{vza} m/s", "b_vertical_speed": f"{vzb} m/s"}
        exp = {"result.time.value": t, "result.separation.value": math.sqrt(sum(x * x for x in r)),
               "result.horizontal_separation.value": math.hypot(r[0], r[1]), "result.vertical_separation.value": r[2] / 0.3048}
        rows.append(vec(len(rows) + 1, inp, exp, {k: {"rel": 1e-9, "abs": 1e-6} for k in exp}, cpa3_src))
    write("navigation.route.cpa", rows)

    legs_src = "Karney's geographiclib (Python) geodesic inverse per leg"
    rows = []
    for i in range(1, 23):
        n = rnd.randint(2, 6)
        wps = [(rnd.uniform(-60, 60), rnd.uniform(-180, 180))]
        for _ in range(n - 1):
            d = G.Direct(wps[-1][0], wps[-1][1], rnd.uniform(0, 360), rnd.uniform(20e3, 800e3))
            wps.append((d["lat2"], d["lon2"]))
        exp, tol, cum = {}, {}, 0.0
        for k in range(n - 1):
            inv = G.Inverse(wps[k][0], wps[k][1], wps[k + 1][0], wps[k + 1][1])
            cum += inv["s12"]
            exp[f"result.legs.{k}.distance.value"] = inv["s12"] / 1852
            exp[f"result.legs.{k}.true_course.value"] = inv["azi1"] % 360
            tol[f"result.legs.{k}.distance.value"] = {"abs": 1e-9}
            tol[f"result.legs.{k}.true_course.value"] = {"abs": 1e-9}
        exp["result.total_distance.value"] = cum / 1852
        tol["result.total_distance.value"] = {"abs": 1e-9}
        rows.append(vec(i, {"waypoints": [{"lat": a, "lon": b} for a, b in wps]}, exp, tol, legs_src))
    write("navigation.route.legs", rows)

    wp_src = "Karney's geographiclib (Python): points along the geodesic by the direct problem"
    rows = []
    for i in range(1, 23):
        a = (rnd.uniform(-70, 70), rnd.uniform(-180, 180))
        d = G.Direct(a[0], a[1], rnd.uniform(0, 360), rnd.uniform(1e4, 1.5e7))
        n = rnd.randint(1, 40)
        line = G.InverseLine(a[0], a[1], d["lat2"], d["lon2"])
        exp, tol = {"result.count": n + 1, "result.length.value": line.s13 / 1000}, {"result.count": {"abs": 0}, "result.length.value": {"abs": 1e-9}}
        for k in sorted({0, n // 2, n}):
            q = line.Position(line.s13 * k / n)
            exp[f"result.points.{k}.lat.value"] = q["lat2"]
            exp[f"result.points.{k}.lon.value"] = q["lon2"]
            tol[f"result.points.{k}.lat.value"] = {"abs": 1e-9}
            tol[f"result.points.{k}.lon.value"] = {"abs": 1e-9}
        rows.append(vec(i, {"lat1": a[0], "lon1": a[1], "lat2": d["lat2"], "lon2": d["lon2"], "intervals": n}, exp, tol, wp_src))
    write("navigation.geodesic.waypoints", rows)

    cp_src = "Karney's geographiclib (Python): per-leg perpendicularity bisection, clamped to each leg"
    rows = []
    i = 0
    while len(rows) < 22:
        n = rnd.randint(2, 5)
        wps = [(rnd.uniform(-60, 60), rnd.uniform(-180, 180))]
        for _ in range(n):
            d = G.Direct(wps[-1][0], wps[-1][1], rnd.uniform(0, 360), rnd.uniform(50e3, 500e3))
            wps.append((d["lat2"], d["lon2"]))
        k = rnd.randrange(n)
        mid = G.InverseLine(wps[k][0], wps[k][1], wps[k + 1][0], wps[k + 1][1]).Position(rnd.uniform(0.1, 0.9) * G.Inverse(wps[k][0], wps[k][1], wps[k + 1][0], wps[k + 1][1])["s12"])
        pp = G.Direct(mid["lat2"], mid["lon2"], mid["azi2"] + rnd.choice([90, -90]), rnd.uniform(0, 40e3))
        p = (pp["lat2"], pp["lon2"])
        best, before = None, 0.0
        for j in range(n):
            a, b = wps[j], wps[j + 1]
            L = G.Inverse(a[0], a[1], b[0], b[1])["s12"]
            try:
                s_, xt, flat, flon, _ = closest(a, b, p)
            except ValueError:
                s_ = -1.0
            if s_ < 0 or s_ > L:
                c, along = (a, 0.0) if s_ < 0 else (b, L)
                dd = G.Inverse(c[0], c[1], p[0], p[1])["s12"]
            else:
                c, along, dd = (flat, flon), s_, abs(xt)
            if best is None or dd < best[0]:
                best = (dd, j + 1, before + along)
            before += L
        i += 1
        rows.append(vec(i, {"route": [{"lat": x, "lon": y} for x, y in wps], "lat": p[0], "lon": p[1]},
                        {"result.leg": best[1], "result.along_route.value": best[2] / 1000, "result.route_length.value": before / 1000},
                        {"result.leg": {"abs": 0}, "result.along_route.value": {"abs": 1e-6}, "result.route_length.value": {"abs": 1e-9}}, cp_src))
    write("navigation.route.closest-point", rows)


if __name__ == "__main__":
    main()
