#!/usr/bin/env python3
"""Golden vectors for survey.* by direct evaluation of the textbook formulas
(Ghilani & Wolf, Elementary Surveying, 15th ed.; Stem 1990, NOAA Manual NOS
NGS 5), plus the add-survey-suite spec scenarios."""
import json
import math
import random
import sys
from pathlib import Path

SRC = "Surveying formulas (Ghilani & Wolf 2018) evaluated in Python (tools/vectors/gen_survey.py)"
VER = "15th edition (2018)"
SPEC = "add-survey-suite scenarios"
FT = 0.3048
YD3_FT3 = 27.0


def vec(i, inp, exp, src=SRC, ver=VER, rel=1e-12):
    e = dict(exp)
    e.setdefault("ok", True)
    tol = {k: {"rel": rel, "abs": 1e-9} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def dms(angle):
    s = round(angle * 3600)
    return f"{s // 3600}°{s % 3600 // 60:02d}'{s % 60:02d}\""


def bearing(az):
    az %= 360
    if az <= 90:
        return f"N {dms(az)} E"
    if az <= 180:
        return f"S {dms(180 - az)} E"
    if az <= 270:
        return f"S {dms(az - 180)} W"
    return f"N {dms(360 - az)} W"


def inverse():
    cases = [(1000, 1000, 1100, 1100), (5000, 5000, 4800, 4900), (0, 0, -120.5, 300.25), (250, 800, 900, 100), (10, 10, 10, 20)]
    out = []
    for i, (n1, e1, n2, e2) in enumerate(cases, 1):
        dn, de = n2 - n1, e2 - e1
        az = math.degrees(math.atan2(de, dn)) % 360
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        out.append(vec(i, {"northing1": n1, "easting1": e1, "northing2": n2, "easting2": e2},
                       {"result.distance.value": math.hypot(dn, de), "result.azimuth.value": float(az), "result.bearing": bearing(az)}, src, ver))
    out.append(vec(6, {"northing1": "0 ft", "easting1": "0 ftUS", "northing2": 1, "easting2": 1}, {"ok": False, "error.code": "UNIT_MISMATCH"}, SPEC, "2026"))
    return out


def forward():
    cases = [(1000, 1000, "N 45°00'00\" E", 45.0, 141.421356237), (5000, 5000, "S 44°30'00\" W", 224.5, 300.0),
             (0, 0, "S44-30-00W", 224.5, 100.0), (100, 200, "123.25", 123.25, 55.5), (0, 0, "N 10°15'30\" W", 360 - (10 + 15 / 60 + 30 / 3600), 1000.0)]
    out = []
    for i, (n, e, d, az, dist) in enumerate(cases, 1):
        a = math.radians(az)
        out.append(vec(i, {"northing": n, "easting": e, "direction": d, "distance": dist},
                       {"result.northing.value": n + dist * math.cos(a), "result.easting.value": e + dist * math.sin(a)}, rel=1e-11))
    return out


def traverse_case(courses, method="compass"):
    lat = [d * math.cos(math.radians(a)) for a, d in courses]
    dep = [d * math.sin(math.radians(a)) for a, d in courses]
    sl, sd, total = sum(lat), sum(dep), sum(d for _, d in courses)
    mis = math.hypot(sl, sd)
    exp = {"result.sum_latitudes.value": sl, "result.sum_departures.value": sd, "result.misclosure.value": mis,
           "result.total_length.value": float(total), "result.precision_ratio": total / mis}
    # Adjusted second point.
    if method == "compass":
        cl, cd = -sl * courses[0][1] / total, -sd * courses[0][1] / total
    else:
        cl = -sl * abs(lat[0]) / sum(abs(x) for x in lat)
        cd = -sd * abs(dep[0]) / sum(abs(x) for x in dep)
    exp["result.adjusted.1.northing.value"] = 5000 + lat[0] + cl
    exp["result.adjusted.1.easting.value"] = 5000 + dep[0] + cd
    return exp


def traverse():
    loops = [
        [(0, 300.00), (90, 400.02), (180, 299.95), (270.01, 400.00)],
        [(10.5, 500.0), (100.25, 350.0), (190.6, 499.8), (280.2, 350.3)],
        [(30, 200.0), (150, 200.05), (270.02, 199.9)],
        [(45, 1000.0), (135, 1000.0), (225, 999.7), (315.01, 1000.2)],
        [(0, 100.0), (72, 100.01), (144, 99.98), (216, 100.02), (288.01, 100.0)],
    ]
    out = []
    for i, loop in enumerate(loops, 1):
        method = "transit" if i == 4 else "compass"
        inp = {"courses": [{"direction": str(a), "distance": d} for a, d in loop], "adjustment": method}
        exp = traverse_case(loop, method)
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        if i == 1:
            exp["result.precision"] = "1:11,525"
        out.append(vec(i, inp, exp, src, ver, rel=1e-9))
    out.append(vec(6, {"courses": [{"direction": "0", "distance": 100}, {"direction": "180", "distance": 100}]},
                   {"meta.warnings.0.code": "PERFECT_CLOSURE", "result.precision": "perfect"}, SPEC, "2026"))
    for k, loop in enumerate(MORE_LOOPS):
        method = ["compass", "transit"][k % 2]
        out.append(vec(len(out) + 1, {"courses": [{"direction": str(a), "distance": d} for a, d in loop], "adjustment": method},
                       traverse_case(loop, method), SRC, VER, rel=1e-9))
    # University of Memphis CIVL 1112, Surveying - Traverse Calculations: two published closures and the compass-rule balanced table.
    memphis = [("S 6-15 W", 189.53), ("S 29-38 E", 175.18), ("N 81-18 W", 197.78), ("N 12-24 W", 142.39), ("N 42-59 E", 234.58)]
    exp = {"result.precision": "1:5,175", "result.misclosure.value": 0.182, "result.sum_latitudes.value": -0.079,
           "result.sum_departures.value": -0.163, "result.total_length.value": 939.46}
    # Balanced latitudes and departures accumulated from (0, 0): B, C, D, E.
    for n, (lat, dep) in enumerate([(-188.388, -20.601), (-340.641, 66.047), (-310.708, -129.423), (-171.628, -159.974)], 1):
        exp[f"result.adjusted.{n}.northing.value"] = lat
        exp[f"result.adjusted.{n}.easting.value"] = dep
    out.append(vec(len(out) + 1, {"courses": [{"direction": b, "distance": d} for b, d in memphis], "adjustment": "compass",
                                  "start_northing": 0, "start_easting": 0}, exp, MEMPHIS, MEMPHIS_VER))
    out[-1]["tolerance"] = {k: {"abs": 0.0015} for k in exp if isinstance(exp[k], float)}
    group = [("S 77-10 E", 651.2), ("S 38-43 W", 826.7), ("N 64-09 W", 491.0), ("N 29-16 E", 660.5)]
    out.append(vec(len(out) + 1, {"courses": [{"direction": b, "distance": d} for b, d in group]},
                   {"result.precision": "1:2,083", "result.misclosure.value": 1.262, "result.sum_latitudes.value": 0.601,
                    "result.sum_departures.value": -1.110}, MEMPHIS, MEMPHIS_VER))
    out[-1]["tolerance"] = {k: {"abs": 0.0005} for k in out[-1]["expect"] if isinstance(out[-1]["expect"][k], float)}
    return out


MORE_LOOPS = [
    [(12.25, 250.0), (95.5, 310.2), (170.75, 260.4), (281.0, 290.1)],
    [(0.0, 1500.0), (120.0, 1500.3), (240.01, 1499.8)],
    [(33.3, 88.8), (123.3, 77.7), (213.31, 88.79), (303.3, 77.72)],
    [(5.0, 2640.0), (95.0, 5280.1), (185.0, 2639.7), (275.02, 5280.0)],
    [(60.0, 45.5), (140.0, 60.2), (230.0, 70.1), (320.0, 50.3), (355.0, 20.0)],
    [(18.0, 400.0), (90.0, 250.0), (162.0, 400.2), (234.0, 250.1), (306.0, 330.0)],
    [(270.0, 800.0), (0.0, 600.0), (90.0, 800.1), (180.01, 599.9)],
    [(44.9, 150.0), (134.9, 150.02), (224.9, 149.98), (314.91, 150.0)],
    [(200.0, 333.3), (320.0, 333.4), (80.0, 333.2)],
    [(1.0, 999.99), (89.0, 500.0), (181.0, 1000.03), (269.0, 499.97)],
    [(15.0, 120.0), (75.0, 95.0), (130.0, 140.0), (200.0, 180.0), (260.0, 90.0), (320.0, 110.0)],
    [(350.0, 60.0), (80.0, 60.01), (170.0, 59.99), (260.02, 60.0)],
    [(100.0, 1234.56), (220.0, 1234.5), (340.01, 1234.6)],
    [(7.5, 700.0), (97.5, 350.0), (187.5, 700.05), (277.5, 349.9)],
]
MEMPHIS = "University of Memphis CIVL 1112, Surveying - Traverse Calculations (latitudes and departures example, group example 1)"
MEMPHIS_VER = "course notes, retrieved 2026-09-19"


def area():
    polys = [
        [(0, 0), (0, 100), (50, 100), (50, 0)],
        [(1000, 1000), (1200, 1100), (1150, 1400), (900, 1300)],
        [(0, 0), (300, 0), (0, 400)],
        [(5000, 5000), (5000, 5660), (4340, 5660), (4340, 5000)],
        [(10, 10), (20, 50), (60, 70), (80, 30), (40, 0)],
    ]
    out = []
    for i, p in enumerate(polys, 1):
        twice = sum(p[k][1] * p[(k + 1) % len(p)][0] - p[(k + 1) % len(p)][1] * p[k][0] for k in range(len(p)))
        a = abs(twice) / 2
        per = sum(math.hypot(p[(k + 1) % len(p)][0] - p[k][0], p[(k + 1) % len(p)][1] - p[k][1]) for k in range(len(p)))
        exp = {"result.area.value": float(a), "result.acres.value": a / 43560, "result.hectares.value": a * FT * FT / 10000,
               "result.perimeter.value": float(per), "result.orientation": "counterclockwise" if twice > 0 else "clockwise"}
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        out.append(vec(i, {"points": [{"northing": n, "easting": e} for n, e in p]}, exp, src, ver, rel=1e-11))
    # US survey feet report US survey acres.
    out.append(vec(6, {"points": [{"northing": f"{n} ftUS", "easting": f"{e} ftUS"} for n, e in polys[3]]},
                   {"result.acres.value": 660 * 660 / 43560, "result.acres.unit": "acUS", "meta.warnings.0.code": "LEGACY_UNIT"}, SPEC, "2026", rel=1e-11))
    more = [
        [(0, 0), (0, 208.71), (208.71, 208.71), (208.71, 0)],
        [(100, 100), (340, 180), (520, 90), (610, 400), (300, 520), (80, 330)],
        [(0, 0), (1320, 0), (1320, 1320), (0, 1320)],
        [(5000.25, 3000.5), (5100.75, 3150.25), (4980.5, 3300.0), (4870.0, 3120.75)],
        [(0, 0), (10, 0), (10, 10), (5, 3), (0, 10)],
        [(250000, 1500000), (250400, 1500900), (249700, 1501300), (249300, 1500500)],
        [(0, 0), (2640, 0), (2640, 1320), (1320, 1320), (1320, 2640), (0, 2640)],
        [(12.5, 7.25), (40.0, 3.5), (61.75, 22.0), (55.5, 48.25), (30.0, 60.0), (8.0, 41.5)],
        [(0, 0), (-500, 300), (-200, 900), (400, 700), (600, 100)],
        [(1000, 1000), (1000, 1001), (1001, 1001), (1001, 1000)],
        [(0, 0), (3000, 250), (5200, 2900), (2600, 5400), (-300, 3100)],
        [(33, 44), (133, 44), (133, 144)],
        [(7000, 7000), (7600, 7050), (7650, 7700), (7040, 7640)],
    ]
    for p in more:
        twice = sum(p[k][1] * p[(k + 1) % len(p)][0] - p[(k + 1) % len(p)][1] * p[k][0] for k in range(len(p)))
        a = abs(twice) / 2
        per = sum(math.hypot(p[(k + 1) % len(p)][0] - p[k][0], p[(k + 1) % len(p)][1] - p[k][1]) for k in range(len(p)))
        exp = {"result.area.value": float(a), "result.acres.value": a / 43560, "result.hectares.value": a * FT * FT / 10000,
               "result.perimeter.value": float(per), "result.orientation": "counterclockwise" if twice > 0 else "clockwise"}
        out.append(vec(len(out) + 1, {"points": [{"northing": n, "easting": e} for n, e in p]}, exp, SRC, VER, rel=1e-11))
    # Wikipedia's shoelace example: (1, 6), (3, 1), (7, 2), (4, 4), (8, 5) as (x, y) = (easting, northing) encloses 16.5.
    out.append(vec(len(out) + 1, {"points": [{"easting": x, "northing": y} for x, y in [(1, 6), (3, 1), (7, 2), (4, 4), (8, 5)]]},
                   {"result.area.value": 16.5}, WIKI_SHOELACE, "retrieved 2026-09-19", rel=1e-12))
    return out


def curve_elems(r, d):
    t = math.radians(d) / 2
    return {"tangent": r * math.tan(t), "length": r * 2 * t, "chord": 2 * r * math.sin(t),
            "external": r * (1 / math.cos(t) - 1), "middle_ordinate": r * (1 - math.cos(t))}


def circular():
    out = []
    e = curve_elems(500, 30)
    out.append(vec(1, {"radius": "500 ft", "delta": "30 deg"}, {f"result.{k}.value": v for k, v in e.items()}, SPEC, "2026"))
    # Any two elements: solve from a pair and check the rest.
    pairs = [(800, 42.5, ("tangent", "length")), (1200, 18.25, ("chord", "middle_ordinate")), (300, 75.0, ("external", "radius")), (650, 55.0, ("delta", "length"))]
    for i, (r, d, (a, b)) in enumerate(pairs, 2):
        e = curve_elems(r, d)
        vals = dict(e, radius=r, delta=d)
        inp = {k: (f"{vals[k]} deg" if k == "delta" else vals[k]) for k in (a, b)}
        exp = {f"result.{k}.value": float(v) for k, v in dict(e, radius=r).items()}
        exp["result.delta.value"] = float(d)
        out.append(vec(i, inp, exp, rel=1e-9))
    e = curve_elems(18000 / math.pi / 4, 20)
    out.append(vec(6, {"degree": "4 deg", "delta": "20 deg", "pi_station": "50+00"},
                   {"result.pc_station": station(5000 - e["tangent"]), "result.pt_station": station(5000 - e["tangent"] + e["length"])},
                   "Arc definition D = 18000/(πR) with stationing", VER))
    out.append(vec(7, {"radius": 500, "delta": "30 deg", "tangent": 100}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    more = [(2500, 12.0, ("tangent", "external")), (95.5, 110.0, ("chord", "length")), (4000, 3.5, ("middle_ordinate", "delta")),
            (1432.39, 64.0, ("radius", "middle_ordinate")), (750, 150.0, ("external", "delta")), (210, 88.0, ("length", "radius")),
            (5729.58, 1.0, ("tangent", "delta"))]
    for r, d, (a, b) in more:
        e = curve_elems(r, d)
        vals = dict(e, radius=r, delta=d)
        inp = {k: (f"{vals[k]} deg" if k == "delta" else vals[k]) for k in (a, b)}
        exp = {f"result.{k}.value": float(v) for k, v in dict(e, radius=r).items()}
        exp["result.delta.value"] = float(d)
        out.append(vec(len(out) + 1, inp, exp, rel=1e-9))
    # Chord definition: R = 50 / sin(D/2).
    r = 50 / math.sin(math.radians(12) / 2)
    e = curve_elems(r, 38.5)
    out.append(vec(len(out) + 1, {"degree_chord": "12 deg", "delta": "38.5 deg"}, {f"result.{k}.value": v for k, v in dict(e, radius=r).items()}))
    out.append(vec(len(out) + 1, {"degree_chord": "12 deg", "radius": 400}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    # FM 5-233 chapter 3: PI 18+00, I = 45 deg, D = 15 deg (chord definition), first stake 16+50 after an 8.67 ft subchord, so PC 16+41.33;
    # I = 75 deg, D = 15 deg: T 293.11 and E 99.50 (arc, from its 5,730 ft tables) and T 293.94 and E 99.79 (chord);
    # I = 42 deg 15', D = 5 deg 37': L = 752.23 ft.
    fm = [
        ({"delta": "45 deg", "degree_chord": "15 deg", "pi_station": "18+00"}, {"result.pc_station": "16+41.33"}, {}),
        ({"delta": "75 deg", "degree": "15 deg"}, {"result.tangent.value": 293.11, "result.external.value": 99.50}, {"abs": 0.02}),
        ({"delta": "75 deg", "degree_chord": "15 deg"}, {"result.tangent.value": 293.94, "result.external.value": 99.79}, {"abs": 0.015}),
        ({"delta": "42°15'", "degree": "5°37'"}, {"result.length.value": 752.23}, {"abs": 0.005}),
    ]
    for inp, exp, tol in fm:
        out.append(vec(len(out) + 1, inp, exp, FM5233, FM5233_VER))
        out[-1]["tolerance"] = {k: tol for k in exp if not isinstance(exp[k], str)}
    return out


def station(v, per=100, dec=2):
    whole, rest = divmod(round(v * 10**dec), per * 10**dec)
    return f"{whole}+{rest / 10**dec:0{3 + dec}.{dec}f}"


def vertical():
    cases = [(2, -3, 600, 1000, 100.0), (-4, 2, 400, 2500, 350.0), (1.5, -1.5, 300, 12000, 20.5), (-2, -0.5, 500, 800, 60.0), (3, 1, 200, 1500, 75.25)]
    out = []
    for i, (g1, g2, l, s, y) in enumerate(cases + MORE_VERTICAL, 1):
        a1, a2 = g1 / 100, g2 / 100
        pvc_s, y_pvc = s - l / 2, y - a1 * l / 2
        exp = {"result.pvc_station": station(pvc_s), "result.pvt_station": station(pvc_s + l), "result.pvc_elevation.value": y_pvc,
               "result.pvt_elevation.value": y + a2 * l / 2, "result.k": l / abs(g2 - g1)}
        x = -a1 * l / (a2 - a1)
        if 0 < x < l:
            exp["result.turning_station"] = station(pvc_s + x)
            exp["result.turning_elevation.value"] = y_pvc + a1 * x + (a2 - a1) * x * x / (2 * l)
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        out.append(vec(i, {"g1": g1, "g2": g2, "length": f"{l} ft", "pvi_station": station(s), "pvi_elevation": f"{y} ft"}, exp, src, ver, rel=1e-11))
    # Indiana DOT Design Manual chapter 44, example 44-3.1 (figure 44-3G). Its printed low-point elevation, 580.33,
    # has an arithmetic slip (500 x 1.75^2 / 800 is 1.914, not 1.545), so only the correct values are pinned.
    out.append(vec(len(out) + 1, {"g1": -1.75, "g2": 2.25, "length": "500 ft", "pvi_station": "13+80", "pvi_elevation": "577.50 ft"},
                   {"result.pvc_station": "11+30.00", "result.pvc_elevation.value": 581.875, "result.pvt_station": "16+30.00",
                    "result.pvt_elevation.value": 583.125, "result.turning_station": "13+48.75", "result.k": 125.0, "result.curve_type": "low"},
                   INDOT, INDOT_VER, rel=1e-12))
    return out


MORE_VERTICAL = [(-3.5, 1.25, 800, 3450, 912.4), (0.8, -2.2, 450, 6800, 1520.75), (-1.0, -3.0, 350, 910, 44.0), (4.5, 0.5, 1000, 22000, 5280.0),
                 (-0.6, 0.9, 250, 125, 10.0), (2.75, -2.75, 900, 15050, 734.6), (-5.0, 3.0, 640, 4000, 250.5), (1.2, 3.4, 300, 700, 88.8),
                 (-2.4, 0.3, 720, 9990, 1001.1), (6.0, -4.0, 1200, 30000, 3000.0), (-0.25, 0.75, 150, 555, 12.34), (3.3, -0.7, 520, 1234, 456.7),
                 (-1.8, 2.6, 660, 7777, 640.0), (0.4, -0.4, 200, 300, 5.0), (-3.0, -0.2, 480, 16000, 2020.2)]
INDOT = "Indiana Department of Transportation, Indiana Design Manual chapter 44, example 44-3.1 (figure 44-3G)"
INDOT_VER = "Chapter 44 (current), retrieved 2026-09-19"
FM5233 = "Headquarters, Department of the Army, FM 5-233 Construction Surveying, chapter 3 (simple curves)"
FM5233_VER = "FM 5-233 (1985)"
WIKI_SHOELACE = "Wikipedia, Shoelace formula (worked example)"


def aea():
    cases = [(120, 180, 100), (0, 250, 50), (340.5, 290.25, 100), (15, 15, 25), (800, 1200, 100)]
    return [vec(i, {"area1": f"{a} ft2", "area2": f"{b} ft2", "length": f"{l} ft"},
                {"result.volume_ft3.value": l * (a + b) / 2, "result.volume.value": l * (a + b) / 2 / YD3_FT3}, SPEC if i == 1 else SRC, "2026" if i == 1 else VER, rel=1e-11)
            for i, (a, b, l) in enumerate(cases, 1)]


def prismoidal():
    cases = [(120, 180, 148, 100), (0, 300, 75, 60), (200, 400, 290, 100), (50, 80, 64, 30), (1000, 1500, 1240, 100)]
    out = []
    for i, (a, b, m, l) in enumerate(cases, 1):
        v = l / 6 * (a + 4 * m + b)
        out.append(vec(i, {"area1": f"{a} ft2", "area2": f"{b} ft2", "area_middle": f"{m} ft2", "length": f"{l} ft"},
                       {"result.volume_ft3.value": v, "result.volume.value": v / YD3_FT3, "result.difference.value": (l * (a + b) / 2 - v) / YD3_FT3},
                       SPEC if i == 1 else SRC, "2026" if i == 1 else VER, rel=1e-11))
    out.append(vec(6, {"area1": "120 ft2", "area2": "180 ft2", "area_middle": "150 ft2", "length": "100 ft"},
                   {"meta.warnings.1.code": "PRISMOIDAL_MIDDLE_AREA_AVERAGED"}, SPEC, "2026"))
    return out


def swell():
    cases = [(1000, 25, None, 12, 105), (500, 30, 10, 10, 65), (2400, 15, 8, 14, 198), (120, 40, None, 12, 14), (960, 25, 5, 12, 100)]
    out = []
    for i, (b, s, sh, cap, loads) in enumerate(cases, 1):
        inp = {"bank_volume": f"{b} yd3", "swell": s, "truck_capacity": f"{cap} yd3"}
        exp = {"result.loose_volume.value": b * (1 + s / 100), "result.loads": float(math.ceil(b * (1 + s / 100) / cap - 1e-9))}
        assert exp["result.loads"] == loads, (i, exp)
        if sh is not None:
            inp["shrink"] = sh
            exp["result.compacted_volume.value"] = b * (1 - sh / 100)
        out.append(vec(i, inp, exp, SPEC if i == 1 else SRC, "2026" if i == 1 else VER, rel=1e-11))
    return out


def combined():
    r = 6_372_000.0
    cases = [(0.99991, 1500, 1000), (1.0000234, 5280, 2500.5), (0.9999, 0, 100), (0.99995, -50, 3000), (1.00004, 12000, 750)]
    out = []
    src = "NOAA Manual NOS NGS 5 (Stem 1990), section 4, evaluated in Python"
    for i, (k, h, g) in enumerate(cases, 1):
        ef = r / (r + h * FT)
        out.append(vec(i, {"grid_scale": k, "ellipsoid_height": f"{h} ft", "ground_distance": f"{g} ft"},
                       {"result.elevation_factor": ef, "result.combined_factor": k * ef, "result.grid_distance.value": g * k * ef}, src, "1990", rel=1e-12))
    out.append(vec(6, {"grid_scale": 0.99991, "elevation": "1500 ft"}, {"meta.warnings.0.code": "ORTHOMETRIC_AS_ELLIPSOIDAL"}, SPEC, "2026"))
    more = [(0.9999, 0, 10), (1.0001, 9000, 5000), (0.99996, 3200.5, 1320), (1.00002, 750, 66), (0.999925, 14000, 8000),
            (1.0000076, 420, 300.25), (0.99987, 2100, 12345.678), (1.00003, -200, 45.6)]
    for k, h, g in more:
        ef = r / (r + h * FT)
        out.append(vec(len(out) + 1, {"grid_scale": k, "ellipsoid_height": f"{h} ft", "ground_distance": f"{g} ft"},
                       {"result.elevation_factor": ef, "result.combined_factor": k * ef, "result.grid_distance.value": g * k * ef}, src, "1990"))
    # Orthometric height plus geoid height (h = H + N), in meters.
    for k, h_m, n_m, g in [(0.99995, 250.0, -30.5, 1000.0), (1.00001, 1609.3, -18.2, 2500.0), (0.99992, 12.0, 34.1, 400.0)]:
        ef = r / (r + h_m + n_m)
        out.append(vec(len(out) + 1, {"grid_scale": k, "elevation": f"{h_m} m", "geoid_height": f"{n_m} m", "ground_distance": f"{g} m"},
                       {"result.elevation_factor": ef, "result.combined_factor": k * ef, "result.grid_distance.value": g * k * ef}, src, "1990"))
    # NOAA Manual NOS NGS 5 section 4.4 worked example (Wisconsin south): mean grid scale 1.0000450, H 865 ft,
    # N -100 ft, R 20,906,000 ft; elevation factor 0.9999634, combined factor 1.0000084, and the step 5 grid lengths.
    for ground, grid in [(4805.468, 4805.508), (3963.694, 3963.727), (4966.083, 4966.125), (3501.223, 3501.252), (4466.935, 4466.973)]:
        out.append(vec(len(out) + 1, {"grid_scale": 1.0000450, "elevation": "865 ft", "geoid_height": "-100 ft", "radius": "20906000 ft",
                                      "ground_distance": f"{ground} ft"},
                       {"result.elevation_factor": 0.9999634, "result.combined_factor": 1.0000084, "result.grid_distance.value": grid},
                       "NOAA Manual NOS NGS 5, State Plane Coordinate System of 1983 (Stem), section 4.4 example, steps 4 and 5", "1990"))
        out[-1]["tolerance"] = {"result.elevation_factor": {"abs": 5e-8}, "result.combined_factor": {"abs": 5e-8},
                                "result.grid_distance.value": {"abs": 0.0005}}
    return out


# ---------------------------------------------------------------- land descriptions

LAND_SRC = "Independent Python arithmetic on the BLM Manual unit definitions and latitudes and departures (tools/vectors/gen_survey.py)"
LAND_VER = "BLM Manual of Surveying Instructions (2009)"


def slope():
    R = 6371000.0
    out = []
    # The spec scenario: 500 m at 85°00'00".
    cases = [("500 m", "85°00'00\"", 500.0), ("250 ft", "92°30'00\"", 250.0), ("1,234.567 m", "88°15'30\"", 1234.567), ("80 m", "90", 80.0), ("300.25 ft", "45°", 300.25)]
    for i, (sd, z, v) in enumerate(cases, 1):
        zz = z.replace("\"", "").replace("'", ":").replace("°", ":").rstrip(":").split(":")
        zd = sum(float(x) / 60 ** k for k, x in enumerate(zz))
        exp = {"result.horizontal_distance.value": v * math.sin(math.radians(zd)), "result.vertical_difference.value": v * math.cos(math.radians(zd))}
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        out.append(vec(i, {"slope_distance": sd, "zenith": z}, exp, src, ver))
    # The two-face scenario: FL 85°00'10", FR 274°59'40" -> mean 85°00'15", index error -5".
    fl, fr = 85 + 10 / 3600, 274 + 59 / 60 + 40 / 3600
    mean, idx = (fl + 360 - fr) / 2, (fl + fr - 360) / 2
    out.append(vec(6, {"slope_distance": "500 m", "zenith": "85°00'10\"", "zenith_face_right": "274°59'40\""},
                   {"result.mean_zenith.value": mean, "result.index_error.value": idx * 3600, "result.mean_zenith_dms": "85°00'15.0\"",
                    "result.horizontal_distance.value": 500 * math.sin(math.radians(mean))}, SPEC, "2026", rel=1e-11))
    # Elevation difference with instrument and target heights, then with curvature and refraction.
    hd, vd = 800 * math.sin(math.radians(89.5)), 800 * math.cos(math.radians(89.5))
    out.append(vec(7, {"slope_distance": "800 m", "zenith": "89°30'", "instrument_height": "1.55 m", "target_height": "1.8 m"},
                   {"result.elevation_difference.value": 1.55 + vd - 1.8}))
    cr = (1 - 0.13) * hd * hd / (2 * R)
    out.append(vec(8, {"slope_distance": "800 m", "zenith": "89°30'", "instrument_height": "1.55 m", "target_height": "1.8 m", "curvature_beyond": "150 m"},
                   {"result.elevation_difference.value": 1.55 + vd - 1.8 + cr, "result.curvature_refraction.value": cr}))
    out.append(vec(9, {"slope_distance": "100 m", "vertical_angle": "5°"}, {"result.vertical_difference.value": 100 * math.cos(math.radians(85))}))
    out.append(vec(10, {"slope_distance": "100 m", "zenith": "274°59'40\""}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    return out


def curvature():
    R = 6371000.0
    out = []
    # The spec scenario: 1 km with k = 0.14 is 0.0675 m; with k = 0.13, 0.0683 m.
    for i, (d, dm, k) in enumerate([("1 km", 1000.0, 0.14), ("1 km", 1000.0, 0.13), ("150 m", 150.0, 0.13), ("5 km", 5000.0, 0.0), ("2 km", 2000.0, 0.2)], 1):
        c = (1 - k) * dm * dm / (2 * R)
        exp = {"result.coefficient": (1 - k) * 1e6 / (2 * R)}
        if d.endswith("km"):
            exp["result.correction.value"] = c / 1000
        else:
            exp["result.correction.value"] = c
        src, ver = (SPEC, "2026") if i <= 2 else (SRC, VER)
        out.append(vec(i, {"distance": d, "refraction": k}, exp, src, ver))
    out.append(vec(6, {"distance": "500 ft"}, {"result.correction.value": (1 - 0.13) * (500 * FT) ** 2 / (2 * R) / FT}))
    return out


def stadia():
    out = []
    cases = [("1.234 m", 88.5, 1.5, 1.6), ("0.875 m", 91.25, 1.45, 2.1), ("3.5 ft", 90.0, 5.0, 5.0), ("2.1 m", 80.0, None, None), ("1.0 m", 95.5, 1.55, 0.9)]
    for i, (iv, z, hi, rod) in enumerate(cases, 1):
        v = float(iv.split()[0])
        zr = math.radians(z)
        inp = {"interval": iv, "zenith": f"{z} deg"}
        exp = {"result.horizontal_distance.value": 100 * v * math.sin(zr) ** 2, "result.vertical_difference.value": 100 * v * math.sin(zr) * math.cos(zr)}
        if hi is not None:
            u = iv.split()[1]
            inp.update({"instrument_height": f"{hi} {u}", "rod_reading": f"{rod} {u}"})
            exp["result.elevation_difference.value"] = hi + 100 * v * math.sin(zr) * math.cos(zr) - rod
        out.append(vec(i, inp, exp))
    zr = math.radians(87)
    out.append(vec(6, {"interval": "1.5 m", "vertical_angle": "3 deg", "stadia_constant": 100, "additive_constant": "0.3 m"},
                   {"result.horizontal_distance.value": 150 * math.sin(zr) ** 2 + 0.3 * math.sin(zr), "result.vertical_difference.value": 150 * math.sin(zr) * math.cos(zr) + 0.3 * math.cos(zr)}))
    return out


def inaccessible():
    R = 6371000.0
    out = []
    cot = lambda z: 1 / math.tan(math.radians(z))
    # One station: height = D (cot Z_top - cot Z_base).
    for i, (zt, zb, d) in enumerate([(70.0, 92.0, 120.0), (80.0, 90.5, 250.0), (60.0, 95.0, 45.5), (85.0, 89.0, 400.0)], 1):
        out.append(vec(i, {"zenith_top": f"{zt} deg", "zenith_base": f"{zb} deg", "distance": f"{d} m"},
                       {"result.height.value": d * (cot(zt) - cot(zb)), "result.top_above_instrument.value": d * cot(zt)}))
    # Two stations in line, built from a real tower: 42 m of top above the instrument, near station 90 m away, baseline 35 m.
    h, dn, b = 42.0, 90.0, 35.0
    zn, zf = math.degrees(math.atan2(dn, h)), math.degrees(math.atan2(dn + b, h))
    out.append(vec(5, {"zenith_top": f"{zn!r} deg", "baseline": f"{b} m", "zenith_top_far": f"{zf!r} deg"},
                   {"result.distance_used.value": dn, "result.top_above_instrument.value": h}, rel=1e-10))
    # The tower scenario beyond the threshold: curvature and refraction lift each sight's height, and cancel in the height.
    d = 600.0
    cr = (1 - 0.13) * d * d / (2 * R)
    out.append(vec(6, {"zenith_top": "85 deg", "zenith_base": "90.5 deg", "distance": "600 m", "curvature_beyond": "150 m"},
                   {"result.height.value": d * (cot(85.0) - cot(90.5)), "result.top_above_instrument.value": d * cot(85.0) + cr, "result.curvature_refraction.value": cr}, SPEC, "2026"))
    return out


def offset_shot():
    out = []

    def at(n, e, az, along, across):
        t = math.radians(az)
        return n + along * math.cos(t) + across * math.cos(t + math.pi / 2), e + along * math.sin(t) + across * math.sin(t + math.pi / 2)

    # The tree-center scenario: 150 ft to the side, 0.75 ft radius, the center turned to at N 30°17' E.
    c = 30 + 17 / 60
    n2, e2 = at(5000, 5000, c, 150.75, 0)
    out.append(vec(1, {"northing": "5000 ft", "easting": "5000 ft", "direction": "N 30°00'00\" E", "distance": "150 ft", "center_direction": "N 30°17'00\" E", "radius": "0.75 ft"},
                   {"result.northing.value": n2, "result.easting.value": e2, "result.distance.value": 150.75}, SPEC, "2026"))
    # Distance offsets: right, left, out, and in.
    for i, (az, d, r, o) in enumerate([(30.0, 150.0, 2.5, 0.0), (210.0, 80.0, -1.2, 0.0), (90.0, 45.0, 0.0, 1.0), (315.0, 200.0, 3.0, -0.5)], 2):
        n2, e2 = at(1000, 2000, az, d + o, r)
        inp = {"northing": "1000 m", "easting": "2000 m", "direction": f"{az}", "distance": f"{d} m"}
        if r:
            inp["offset_right"] = f"{r} m"
        if o:
            inp["offset_out"] = f"{o} m"
        out.append(vec(i, inp, {"result.northing.value": n2, "result.easting.value": e2, "result.distance.value": math.hypot(d + o, r)}))
    out.append(vec(6, {"northing": "0 m", "easting": "0 m", "direction": "N 10 E", "distance": "10 m", "center_direction": "N 11 E"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def grade():
    out = []
    # The spec scenario's reading: 3H:1V is about 18.43° and 33.3%.
    for i, (g, gr) in enumerate([("3H:1V", 1 / 3), ("1V:3H", 1 / 3), ("5%", 0.05), ("2.5°", math.tan(math.radians(2.5))), ("50‰", 0.05), ("0.125", 0.125)], 1):
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        out.append(vec(i, {"grade": g}, {"result.percent": gr * 100, "result.degrees.value": math.degrees(math.atan(gr)), "result.per_mille": gr * 1000}, src, ver))
    out.append(vec(7, {"grade": "3:1", "convention": "V:H"}, {"result.percent": 300.0}))
    out.append(vec(8, {"grade": "3:1"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/convention"}, SPEC, "2026"))
    out.append(vec(9, {"rise": "5 ft", "run": "100 ft"}, {"result.percent": 5.0, "result.slope_length.value": math.hypot(5, 100)}))
    out.append(vec(10, {"grade": "2%", "run": "250 m"}, {"result.rise.value": 5.0, "result.slope_length.value": math.hypot(5, 250)}))
    return out


def level_run():
    """Level notes reduced by hand-style arithmetic: HI = elev + BS, elev = HI - FS."""
    out = []

    def reduce(start, book):
        elev, hi, sbs, sfs, elevs = start, None, 0.0, 0.0, []
        for st, bs, fs, d in book:
            if fs is not None:
                elev = hi - fs
                sfs += fs
            elevs.append(elev)
            hi = elev + bs if bs is not None else None
            if bs is not None:
                sbs += bs
        return elevs, sbs, sfs

    def book_input(book, u):
        rows = []
        for st, bs, fs, d in book:
            r = {"station": st}
            if bs is not None:
                r["backsight"] = f"{bs} {u}"
            if fs is not None:
                r["foresight"] = f"{fs} {u}"
            if d is not None:
                r["distance"] = f"{d} {u}"
            rows.append(r)
        return rows

    loop = [("BM 1", 4.52, None, None), ("TP 1", 6.13, 3.97, 300), ("TP 2", 2.84, 5.26, 280), ("BM 1", None, 4.28, 310)]
    e, sbs, sfs = reduce(100.0, loop)
    mis = e[-1] - 100.0
    cum = [0, 300, 580, 890]
    adj = [e[i] - mis * cum[i] / 890 for i in range(4)]
    out.append(vec(1, {"start_elevation": "100 ft", "shots": book_input(loop, "ft")},
                   {"result.sum_backsights.value": sbs, "result.sum_foresights.value": sfs, "result.misclosure.value": mis,
                    "result.stations.1.elevation.value": e[1], "result.stations.2.adjusted.value": adj[2], "result.stations.3.adjusted.value": 100.0}))
    km = 890 * FT / 1000
    out.append(vec(2, {"start_elevation": "100 ft", "shots": book_input(loop, "ft"), "allowable_constant": "0.05 ft"},
                   {"result.allowable.value": 0.05 * math.sqrt(km), "result.closure": "within the allowable"}))
    run = [("BM A", 1.234, None, None), ("TP 1", 2.110, 0.987, 120), ("TP 2", 0.542, 3.004, 95), ("BM B", None, 1.876, 150)]
    e, sbs, sfs = reduce(250.0, run)
    mis = e[-1] - 247.975
    out.append(vec(3, {"start_elevation": "250 m", "shots": book_input(run, "m"), "end_elevation": "247.975 m"},
                   {"result.misclosure.value": mis, "result.stations.3.elevation.value": e[-1], "result.stations.3.adjusted.value": 247.975,
                    "result.stations.1.adjusted.value": e[1] - mis * 120 / 365}))
    # No distances: the misclosure goes by setups.
    nd = [(a, b, c, None) for a, b, c, _ in run]
    e, _, _ = reduce(250.0, nd)
    mis = e[-1] - 247.975
    out.append(vec(4, {"start_elevation": "250 m", "shots": book_input(nd, "m"), "end_elevation": "247.975 m"},
                   {"result.stations.1.adjusted.value": e[1] - mis * 1 / 3, "result.stations.2.adjusted.value": e[2] - mis * 2 / 3}))
    # An open run: elevations and the check, no closure.
    open_ = run[:3] + [("TP 3", None, 1.2, 80)]
    e, sbs, sfs = reduce(250.0, open_)
    out.append(vec(5, {"start_elevation": "250 m", "shots": book_input(open_, "m")},
                   {"result.stations.3.elevation.value": e[-1], "result.sum_backsights.value": sbs, "result.sum_foresights.value": sfs}))
    out.append(vec(6, {"start_elevation": "100 ft", "shots": [{"station": "BM 1", "foresight": "1 ft"}, {"station": "TP 1", "foresight": "2 ft"}]},
                   {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def intersection():
    out = []
    # The two-solution scenario: circles of 300 ft and 250 ft about points 400 ft apart on an east-west line.
    a, b, r1, r2 = (1000.0, 1000.0), (1000.0, 1400.0), 300.0, 250.0
    x = (r1 * r1 - r2 * r2 + 400 * 400) / 800
    h = math.sqrt(r1 * r1 - x * x)
    # Facing east from the first point, north is on the left.
    out.append(vec(1, {"northing1": "1000 ft", "easting1": "1000 ft", "distance1": "300 ft", "northing2": "1000 ft", "easting2": "1400 ft", "distance2": "250 ft"},
                   {"result.count": 2.0, "result.solutions.0.label": "left of the baseline", "result.solutions.0.northing.value": 1000 + h, "result.solutions.0.easting.value": 1000 + x,
                    "result.solutions.1.label": "right of the baseline", "result.solutions.1.northing.value": 1000 - h}, SPEC, "2026"))
    # Bearing-bearing: from (0,0) on N 45° E and from (0,100) on N 45° W meet at (50, 50).
    out.append(vec(2, {"northing1": "0 m", "easting1": "0 m", "direction1": "N 45°00'00\" E", "northing2": "0 m", "easting2": "100 m", "direction2": "N 45°00'00\" W"},
                   {"result.count": 1.0, "result.solutions.0.northing.value": 50.0, "result.solutions.0.easting.value": 50.0}))
    # Bearing-distance: due north from (0,0), a 50 m circle about (40, 30) meets it at N = 0 and N = 80.
    out.append(vec(3, {"northing1": "0 m", "easting1": "0 m", "direction1": "0", "northing2": "40 m", "easting2": "30 m", "distance2": "50 m"},
                   {"result.count": 2.0, "result.solutions.0.northing.value": 0.0, "result.solutions.1.northing.value": 80.0}))
    # Tangent circles meet once.
    out.append(vec(4, {"northing1": "0 ft", "easting1": "0 ft", "distance1": "60 ft", "northing2": "0 ft", "easting2": "100 ft", "distance2": "40 ft"},
                   {"result.count": 1.0, "result.solutions.0.easting.value": 60.0}))
    out.append(vec(5, {"northing1": "0 ft", "easting1": "0 ft", "direction1": "N 10 E", "northing2": "0 ft", "easting2": "100 ft", "direction2": "N 10 E"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(6, {"northing1": "0 ft", "easting1": "0 ft", "distance1": "10 ft", "northing2": "0 ft", "easting2": "100 ft", "distance2": "10 ft"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def resection():
    out = []
    az = lambda p, q: math.degrees(math.atan2(q[1] - p[1], q[0] - p[0])) % 360
    A, B, C = (1000.0, 1000.0), (1500.0, 1400.0), (1100.0, 1800.0)
    CTRL = [{"northing": f"{p[0]:g} ft", "easting": f"{p[1]:g} ft"} for p in (A, B, C)]
    # Stations built first, then the angles they would turn: outside, inside, and far off the triangle.
    for i, P in enumerate([(500.0, 1350.0), (1200.0, 1400.0), (300.0, 600.0), (2200.0, 1300.0)], 1):
        ab, bc = (az(P, B) - az(P, A)) % 360, (az(P, C) - az(P, B)) % 360
        out.append(vec(i, {"control": CTRL, "angle_ab": f"{ab!r} deg", "angle_bc": f"{bc!r} deg"},
                       {"result.northing.value": P[0], "result.easting.value": P[1]}, rel=1e-8))
    # The danger-circle scenario: a station just off the circle through A, B, and C.
    ax, ay, bx, by, cx, cy = *A, *B, *C
    d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    ox = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d
    oy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d
    R = math.hypot(ax - ox, ay - oy)
    P = (ox - 1.01 * R, oy)
    ab, bc = (az(P, B) - az(P, A)) % 360, (az(P, C) - az(P, B)) % 360
    out.append(vec(5, {"control": CTRL, "angle_ab": f"{ab!r} deg", "angle_bc": f"{bc!r} deg"},
                   {"meta.warnings.*.code": "RESECTION_UNSTABLE"}, SPEC, "2026"))
    return out


def angular_closure():
    out = []

    def secs(d, m, s):
        return d * 3600 + m * 60 + s

    # The spec scenario: five interior angles summing to 540°00'25" miss by +25".
    five = [(108, 0, 5), (101, 59, 55), (115, 0, 10), (95, 0, 0), (120, 0, 15)]
    rows = [{"angle": f"{d}°{m:02d}'{s:02d}\""} for d, m, s in five]
    mis = sum(secs(*a) for a in five) - 540 * 3600
    out.append(vec(1, {"angles": rows, "allowable_k": 10}, {"result.misclosure.value": float(mis), "result.allowable.value": 10 * math.sqrt(5),
                                                             "result.closure": "beyond the allowable", "result.correction.value": -mis / 5}, SPEC, "2026"))
    four = [(90, 0, 10), (89, 59, 40), (90, 0, 20), (89, 59, 55)]
    mis = sum(secs(*a) for a in four) - 360 * 3600
    out.append(vec(2, {"angles": [{"angle": f"{d}-{m:02d}-{s:02d}"} for d, m, s in four], "allowable_k": 15},
                   {"result.misclosure.value": float(mis), "result.closure": "within the allowable"}))
    ext = [(270, 0, 5), (270, 0, 0), (269, 59, 50), (270, 0, 0)]
    mis = sum(secs(*a) for a in ext) - 6 * 180 * 3600
    out.append(vec(3, {"angles": [{"angle": f"{d}°{m:02d}'{s:02d}\""} for d, m, s in ext], "kind": "exterior"}, {"result.misclosure.value": float(mis)}))
    tri = [(60, 0, 0), (60, 0, 0), (60, 0, 3)]
    out.append(vec(4, {"angles": [{"angle": f"{d}°{m:02d}'{s:02d}\""} for d, m, s in tri]}, {"result.misclosure.value": 3.0, "result.correction.value": -1.0,
                                                                                           "result.adjusted.2.angle": "60°00'02.0\""}))
    out.append(vec(5, {"angles": [{"angle": "90"}, {"angle": "90"}, {"angle": "90"}, {"angle": "90"}, {"angle": "180"}]}, {"result.misclosure.value": 0.0}))
    out.append(vec(6, {"angles": [{"angle": "90"}, {"angle": "abc"}, {"angle": "90"}]}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def slope_stake():
    """Catch points solved in closed form, segment by segment, against the tool's iteration."""
    out = []

    def solve(G, w, pts, cut_s, fill_s):
        gw = next(ya + (yb - ya) * (w - xa) / (xb - xa) for (xa, ya), (xb, yb) in zip(pts, pts[1:]) if xa <= w <= xb)
        cut = gw > G
        s = cut_s if cut else fill_s
        sign = 1 if cut else -1
        for (xa, ya), (xb, yb) in zip(pts, pts[1:]):
            if xb <= w:
                continue
            m = (yb - ya) / (xb - xa)
            # ya + m (x - xa) = G + sign (x - w) / s
            x = (G - sign * w / s - ya + m * xa) / (m - sign / s)
            if max(xa, w) - 1e-12 <= x <= xb + 1e-12:
                return x, abs(ya + m * (x - xa) - G), cut
        raise ValueError("no catch")

    def pts_in(pts):
        return [{"offset": f"{x} ft", "elevation": f"{y} ft"} for x, y in pts]

    cases = [(100.0, 16.0, [(0.0, 102.0), (60.0, 108.0)], "right"),  # the scenario: cut on ground rising 10%
             (100.0, 16.0, [(0.0, 98.0), (80.0, 90.0)], "left"),     # fill on falling ground
             (250.0, 12.0, [(0.0, 251.0), (20.0, 252.5), (50.0, 260.0)], "right"),  # a break in the ground
             (50.0, 20.0, [(0.0, 49.0), (30.0, 47.3), (90.0, 40.0)], "right")]
    for i, (G, w, pts, side) in enumerate(cases, 1):
        x, depth, cut = solve(G, w, pts, 2.0, 3.0)
        stake = f"{'C' if cut else 'F'} {depth:.1f} / {x:.1f} {'L' if side == 'left' else 'R'}"
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        out.append(vec(i, {"grade_elevation": f"{G} ft", "half_width": f"{w} ft", "ground": pts_in(pts), "cut_slope": 2, "fill_slope": 3, "side": side},
                       {"result.catch_offset.value": x, "result.cut_fill.value": depth, "result.stake": stake}, src, ver, rel=1e-6))
    out.append(vec(5, {"grade_elevation": "100 ft", "half_width": "16 ft", "ground": pts_in([(0.0, 102.0), (30.0, 140.0)]), "cut_slope": 2},
                   {"ok": False, "error.code": "DID_NOT_CONVERGE"}))
    out.append(vec(6, {"grade_elevation": "100 ft", "half_width": "16 ft", "ground": pts_in([(0.0, 98.0), (60.0, 90.0)]), "cut_slope": 2},
                   {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/fill_slope"}))
    return out


def section_area():
    """Cut and fill by dense numerical integration: a different method from the tool's exact splitting."""
    out = []

    def at(pts, x):
        for (xa, ya), (xb, yb) in zip(pts, pts[1:]):
            if x <= xb:
                return ya + (yb - ya) * (x - xa) / (xb - xa)
        return pts[-1][1]

    def integrate(g, t, n=400000):
        lo, hi = max(g[0][0], t[0][0]), min(g[-1][0], t[-1][0])
        h = (hi - lo) / n
        cut = fill = 0.0
        for k in range(n):
            x = lo + (k + 0.5) * h
            d = at(g, x) - at(t, x)
            if d > 0:
                cut += d * h
            else:
                fill -= d * h
        return cut, fill

    def pts_in(pts, u="ft"):
        return [{"offset": f"{x} {u}", "elevation": f"{y} {u}"} for x, y in pts]

    template = [(-40.0, 94.0), (-28.0, 100.0), (28.0, 100.0), (40.0, 94.0)]
    cases = [
        ([(-40.0, 106.0), (40.0, 96.0)], [(-40.0, 100.0), (40.0, 100.0)]),  # the mixed-section scenario
        ([(-50.0, 103.0), (0.0, 104.0), (50.0, 101.0)], template),
        ([(-45.0, 96.0), (-10.0, 97.5), (20.0, 99.0), (45.0, 101.0)], template),
        ([(-40.0, 90.0), (40.0, 91.0)], template),
    ]
    for i, (g, t) in enumerate(cases, 1):
        cut, fill = integrate(g, t)
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        exp = {"result.cut_area.value": cut, "result.fill_area.value": fill}
        if i == 1:
            exp["result.grade_points.0.offset.value"] = 8.0
        v = vec(i, {"ground": pts_in(g), "template": pts_in(t)}, exp, src, ver)
        for k in ("result.cut_area.value", "result.fill_area.value"):
            v["tolerance"][k] = {"rel": 1e-6, "abs": 1e-6}
        out.append(v)
    out.append(vec(5, {"ground": pts_in([(-10.0, 5.0), (10.0, 5.0)], "m"), "template": pts_in([(-10.0, 5.0), (10.0, 5.0)], "m")}, {"result.cut_area.value": 0.0, "result.fill_area.value": 0.0}))
    out.append(vec(6, {"ground": pts_in([(0.0, 1.0), (10.0, 1.0)]), "template": pts_in([(20.0, 1.0), (30.0, 1.0)])}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def borrow_pit():
    out = []

    def run(ex, fg, a):
        h = [[e - fg for e in r] for r in ex]
        R, C = len(h), len(h[0])
        net = 0.0
        for i in range(R):
            for j in range(C):
                edges = sum([i == 0, i == R - 1, j == 0, j == C - 1])
                w = 4 if edges == 0 else 2 if edges == 1 else 1
                net += w * h[i][j]
        net *= a * a / 4
        cut = fill = 0.0
        for i in range(R - 1):
            for j in range(C - 1):
                hs = [h[i][j], h[i][j + 1], h[i + 1][j + 1], h[i + 1][j]]
                p, n = sum(x for x in hs if x > 0), -sum(x for x in hs if x < 0)
                if n == 0:
                    cut += a * a * p / 4
                elif p == 0:
                    fill += a * a * n / 4
                else:
                    cut += a * a * p * p / (4 * (p + n))
                    fill += a * a * n * n / (4 * (p + n))
        return net, cut, fill

    def rows(g):
        return [{"elevations": ", ".join(str(v) for v in r)} for r in g]

    # The corner-weights scenario: a 3 x 3-node grid.
    g1 = [[102.0, 101.5, 101.0], [101.2, 100.6, 100.2], [100.4, 99.8, 99.2]]
    grids = [(g1, 100.0, 25.0), ([[5.0, 6.0], [7.0, 8.0]], 4.0, 10.0), ([[98.0, 99.0, 100.5, 101.0], [98.5, 99.5, 100.2, 101.5], [99.0, 100.0, 100.8, 102.0]], 100.0, 20.0),
             ([[50.0, 50.0], [50.0, 50.0]], 51.0, 5.0)]
    for i, (g, fg, a) in enumerate(grids, 1):
        net, cut, fill = run(g, fg, a)
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        exp = {"result.net_volume.value": net, "result.cut_volume.value": cut, "result.fill_volume.value": fill}
        if i == 1:
            exp["result.net_cubic_yards.value"] = net / 27
        out.append(vec(i, {"existing": rows(g), "cell_size": f"{a} ft", "finished_grade": f"{fg} ft"}, exp, src, ver, rel=1e-11))
    pr = [[100.5, 100.3, 100.1], [100.4, 100.2, 100.0], [100.3, 100.1, 99.9]]
    h = [[e - p for e, p in zip(re, rp)] for re, rp in zip(g1, pr)]
    net = 25.0 ** 2 / 4 * sum((1 if (i in (0, 2) and j in (0, 2)) else 4 if (i, j) == (1, 1) else 2) * h[i][j] for i in range(3) for j in range(3))
    out.append(vec(5, {"existing": rows(g1), "proposed": rows(pr), "cell_size": "25 ft"}, {"result.net_volume.value": net}, rel=1e-11))
    out.append(vec(6, {"existing": [{"elevations": "1, 2, 3"}, {"elevations": "1, 2"}], "cell_size": "10 ft", "finished_grade": "0 ft"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def curve_layout():
    """Layout checked from the curve's center (rotation about it), not by deflections and chords."""
    out = []

    def fmt(v, metric=False):
        per, dec = (1000, 3) if metric else (100, 2)
        tot = round(v * 10 ** dec)
        whole = tot // (per * 10 ** dec)
        rest = (tot - whole * per * 10 ** dec) / 10 ** dec
        return f"{whole}+{rest:0{7 if metric else 5}.{dec}f}"

    def layout(R, delta, pi, step, right=True, pin=None, pie=None, az=None):
        d = math.radians(delta)
        T, L = R * math.tan(d / 2), R * d
        pc, pt = pi - T, pi - T + L
        sts = [pc]
        s = (pc // step) * step + step
        while s < pt - 1e-9:
            if s > pc + 1e-9:
                sts.append(s)
            s += step
        sts.append(pt)
        pts = None
        if pin is not None:
            a = math.radians(az)
            n0, e0 = pin - T * math.cos(a), pie - T * math.sin(a)
            side = 1 if right else -1
            cn, ce = n0 + R * math.cos(a + side * math.pi / 2), e0 + R * math.sin(a + side * math.pi / 2)
            start = math.atan2(e0 - ce, n0 - cn)
            pts = [(cn + R * math.cos(start + side * (st - pc) / R), ce + R * math.sin(start + side * (st - pc) / R)) for st in sts]
        return pc, pt, T, L, sts, pts

    # The layout scenario: R = 500 ft, Δ = 30°, PI at 12+34.56, every 50 ft; the last deflection is Δ/2.
    pc, pt, T, L, sts, _ = layout(500, 30, 1234.56, 50)
    out.append(vec(1, {"radius": "500 ft", "delta": "30°00'00\"", "pi_station": "12+34.56", "interval": "50 ft"},
                   {"result.pc_station": fmt(pc), "result.pt_station": fmt(pt), "result.tangent.value": T, "result.length.value": L,
                    f"result.rows.{len(sts) - 1}.deflection": "15°00'00.0\"", "result.rows.1.station": fmt(sts[1]),
                    "result.rows.1.chord_from_pc.value": 2 * 500 * math.sin((sts[1] - pc) / 1000)}, SPEC, "2026"))
    # With coordinates, right and left.
    for i, right in enumerate([True, False], 2):
        pc, pt, T, L, sts, pts = layout(800, 40, 2500.0, 100, right, 5000.0, 5000.0, 45.0)
        k = len(sts) - 1
        out.append(vec(i, {"radius": "800 ft", "delta": "40 deg", "pi_station": "25+00", "interval": "100 ft", "turn": "right" if right else "left",
                           "pi_northing": "5000 ft", "pi_easting": "5000 ft", "back_azimuth": "N 45°00'00\" E"},
                       {"result.rows.2.northing.value": pts[2][0], "result.rows.2.easting.value": pts[2][1],
                        f"result.rows.{k}.northing.value": pts[k][0], f"result.rows.{k}.easting.value": pts[k][1]}, rel=1e-10))
    # Metric stationing.
    pc, pt, T, L, sts, _ = layout(300, 25, 1234.567, 20)
    out.append(vec(4, {"radius": "300 m", "delta": "25 deg", "pi_station": "1+234.567", "interval": "20 m"},
                   {"result.pc_station": fmt(pc, True), "result.pt_station": fmt(pt, True), "result.rows.1.station": fmt(sts[1], True)}))
    pc, pt, T, L, sts, _ = layout(1000, 12.5, 5000, 25)
    out.append(vec(5, {"radius": "1000 ft", "delta": "12°30'", "pi_station": "50+00", "interval": "25 ft"}, {"result.length.value": L, "result.tangent.value": T}))
    out.append(vec(6, {"radius": "500 ft", "delta": "30 deg", "pi_station": "12+34.56", "interval": "50 ft", "pi_northing": "0 ft"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def spiral():
    """Spiral elements from the Fresnel integrals by Simpson's rule: independent of the tool's series."""
    out = []

    def fresnel(l, R, Ls, n=20000):
        h = l / n
        sx = sy = 0.0
        for k in range(n + 1):
            s = k * h
            w = 1 if k in (0, n) else (4 if k % 2 else 2)
            t = s * s / (2 * R * Ls)
            sx += w * math.cos(t)
            sy += w * math.sin(t)
        return sx * h / 3, sy * h / 3

    def fmt(v):
        tot = round(v * 100)
        whole = tot // 10000
        return f"{whole}+{(tot - whole * 10000) / 100:05.2f}"

    for i, (Ls, R, dd, pi) in enumerate([(200.0, 1000.0, 40.0, 5000.0), (150.0, 800.0, 30.0, 2345.67), (300.0, 1500.0, 55.0, 10000.0), (100.0, 2000.0, 12.0, 750.0)], 1):
        th = Ls / (2 * R)
        X, Y = fresnel(Ls, R, Ls)
        p, k = Y - R * (1 - math.cos(th)), X - R * math.sin(th)
        d = math.radians(dd)
        Ts = (R + p) * math.tan(d / 2) + k
        arc = R * (d - 2 * th)
        ts = pi - Ts
        exp = {"result.x.value": X, "result.y.value": Y, "result.p.value": p, "result.k.value": k, "result.total_tangent.value": Ts,
               "result.arc_length.value": arc, "result.ts_station": fmt(ts), "result.st_station": fmt(ts + 2 * Ls + arc)}
        src, ver = (SPEC, "2026") if i == 1 else (SRC, VER)
        v = vec(i, {"spiral_length": f"{Ls} ft", "radius": f"{R} ft", "delta": f"{dd} deg", "pi_station": fmt(pi)}, exp, src, ver)
        # The spec asks for 0.001 of the unit; the series is held to a millionth.
        for key in ("result.x.value", "result.y.value", "result.p.value", "result.k.value", "result.total_tangent.value"):
            v["tolerance"][key] = {"rel": 0.0, "abs": 1e-6}
        out.append(v)
    out.append(vec(5, {"spiral_length": "200 ft", "radius": "100 ft", "delta": "40 deg", "pi_station": "50+00"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(6, {"spiral_length": "200 ft", "radius": "1000 ft", "delta": "10 deg", "pi_station": "50+00"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def sight_distance():
    out = []

    def crest(S, A, h1, h2):
        c = (math.sqrt(h1) + math.sqrt(h2)) ** 2
        L = A * S * S / (200 * c)
        return (L, "S < L: the sight line lies within the curve") if L >= S else (2 * S - 200 * c / A, "S > L: the sight line extends past the curve")

    def sag(S, A, H, beta):
        c = H + S * math.tan(math.radians(beta))
        L = A * S * S / (200 * c)
        return (L, "S < L: the sight line lies within the curve") if L >= S else (max(0.0, 2 * S - 200 * c / A), "S > L: the sight line extends past the curve")

    # The crest scenario: S = 400 ft, A = 5%, h1 3.5 ft, h2 2.0 ft: about 368.3 ft, S > L.
    L, case = crest(400, 5, 3.5, 2.0)
    out.append(vec(1, {"curve": "crest", "sight_distance": "400 ft", "grade_change": 5, "eye_height": "3.5 ft", "object_height": "2.0 ft"},
                   {"result.min_length.value": L, "result.case": case, "result.k": L / 5}, SPEC, "2026"))
    L, case = crest(650, 7.5, 3.5, 2.0)
    out.append(vec(2, {"curve": "crest", "sight_distance": "650 ft", "grade_change": 7.5, "eye_height": "3.5 ft", "object_height": "2.0 ft"}, {"result.min_length.value": L, "result.case": case}))
    L, case = sag(500, 6, 2.0, 1.0)
    out.append(vec(3, {"curve": "sag", "sight_distance": "500 ft", "grade_change": 6, "headlight_height": "2 ft", "divergence": "1 deg"}, {"result.min_length.value": L, "result.case": case}))
    L, case = sag(150, 2, 0.6, 1.0)
    out.append(vec(4, {"curve": "sag", "sight_distance": "150 m", "grade_change": 2, "headlight_height": "0.6 m", "divergence": "1 deg"}, {"result.min_length.value": L, "result.case": case}))
    out.append(vec(5, {"curve": "crest", "sight_distance": "400 ft", "grade_change": 5, "eye_height": "3.5 ft", "object_height": "2.0 ft", "design_k": 44,
                       "design_k_source": "A design manual's crest K table"}, {"result.design_length.value": 220.0, "result.design_source": "A design manual's crest K table"}))
    out.append(vec(6, {"curve": "crest", "sight_distance": "400 ft", "grade_change": 5}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/eye_height"}))
    return out


def profile_grades():
    out = []

    def run(pts, limit=None):
        grades = [(b[1] - a[1]) / (b[0] - a[0]) * 100 for a, b in zip(pts, pts[1:])]
        climb = sum(max(0.0, b[1] - a[1]) for a, b in zip(pts, pts[1:]))
        desc = sum(max(0.0, a[1] - b[1]) for a, b in zip(pts, pts[1:]))
        mx = max(grades, key=abs)
        avg = (pts[-1][1] - pts[0][1]) / (pts[-1][0] - pts[0][0]) * 100
        return grades, climb, desc, mx, avg, (sum(abs(g) > limit for g in grades) if limit is not None else None)

    def pin(pts, u="ft"):
        return [{"distance": f"{d} {u}", "elevation": f"{e} {u}"} for d, e in pts]

    # The threshold scenario: an 11% segment against an 8% limit is flagged.
    p1 = [(0.0, 400.0), (100.0, 406.0), (200.0, 417.0), (300.0, 414.0)]
    g, c, d, mx, avg, fl = run(p1, 8)
    out.append(vec(1, {"points": pin(p1), "limit": 8}, {"result.segments.1.grade": g[1], "result.segments.1.over_limit": "yes", "result.segments.0.over_limit": "no",
                                                        "result.flagged": float(fl), "result.max_grade": mx, "result.climb.value": c, "result.descent.value": d}, SPEC, "2026"))
    for i, (pts, lim, u) in enumerate([([(0.0, 10.0), (50.0, 12.5), (80.0, 9.0), (200.0, 15.0)], 10, "m"), ([(0.0, 100.0), (25.0, 99.0), (75.0, 96.5)], None, "ft"),
                                        ([(0.0, 0.0), (10.0, 1.2), (20.0, 2.0), (30.0, 1.0), (40.0, 3.5)], 12, "m")], 2):
        g, c, d, mx, avg, fl = run(pts, lim)
        exp = {"result.max_grade": mx, "result.average_grade": avg, "result.climb.value": c, "result.descent.value": d}
        if fl is not None:
            exp["result.flagged"] = float(fl)
        inp = {"points": pin(pts, u)}
        if lim is not None:
            inp["limit"] = lim
        out.append(vec(i, inp, exp))
    out.append(vec(5, {"points": pin([(0.0, 1.0), (10.0, 1.5)])}, {"result.average_grade": 5.0}))
    out.append(vec(6, {"points": pin([(0.0, 1.0), (0.0, 2.0)])}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def stockpile():
    """One interior apex over a convex base forces the TIN (a fan), so volume = base area x apex height / 3 exactly."""
    out = []

    def xyz(pts, u="ft"):
        return [{"easting": f"{x} {u}", "northing": f"{y} {u}", "elevation": f"{z} {u}"} for x, y, z in pts]

    def area(poly):
        return abs(sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(poly, poly[1:] + poly[:1]))) / 2

    cases = [([(0, 0, 100), (50, 0, 100), (50, 50, 100), (0, 50, 100)], (25, 25, 110)),
             ([(0, 0, 100), (50, 0, 100), (50, 50, 100), (0, 50, 100)], (10, 35, 107.5)),
             ([(0, 0, 20), (60, 0, 20), (30, 40, 20)], (30, 15, 26)),
             ([(0, 0, 10), (40, 0, 12), (40, 30, 14), (0, 30, 12)], (20, 15, 20))]
    for i, (base, apex) in enumerate(cases, 1):
        # The base plane (these bases are planar): z = a + b x + c y, fitted exactly.
        (x0, y0, z0), (x1, y1, z1), (x2, y2, z2) = base[0], base[1], base[2]
        det = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0)
        b = ((z1 - z0) * (y2 - y0) - (z2 - z0) * (y1 - y0)) / det
        c = ((x1 - x0) * (z2 - z0) - (x2 - x0) * (z1 - z0)) / det
        a = z0 - b * x0 - c * y0
        h = apex[2] - (a + b * apex[0] + c * apex[1])
        A = area([(p[0], p[1]) for p in base])
        out.append(vec(i, {"base": xyz(base), "surface": xyz([apex])}, {"result.volume.value": A * h / 3, "result.base_area.value": float(A), "result.cubic_yards.value": A * h / 3 / 27}, rel=1e-10))
    out.append(vec(5, {"base": xyz([(0, 0, 0), (10, 0, 0), (20, 0, 0)]), "surface": xyz([(5, 1, 3)])}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(6, {"base": xyz([(0, 0, 0), (10, 0, 0), (10, 10, 0)]), "surface": xyz([(0, 0, 5)])}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def solid_volume():
    out = []
    pi = math.pi
    out.append(vec(1, {"shape": "cone", "radius": "20 ft", "height": "12 ft"}, {"result.volume.value": pi * 400 * 12 / 3, "result.cubic_yards.value": pi * 400 * 12 / 3 / 27}))
    out.append(vec(2, {"shape": "frustum", "radius": "10 m", "top_radius": "4 m", "height": "6 m"}, {"result.volume.value": pi * 6 * (100 + 40 + 16) / 3}))
    out.append(vec(3, {"shape": "prism", "base_area": "150 ft2", "height": "40 ft"}, {"result.volume.value": 6000.0}))
    h = 15 * math.tan(math.radians(34))
    out.append(vec(4, {"shape": "cone", "radius": "15 ft", "repose_angle": "34 deg", "repose_source": "measured on site"}, {"result.height_used.value": h, "result.volume.value": pi * 225 * h / 3, "result.repose_source": "measured on site"}))
    out.append(vec(5, {"shape": "cone", "radius": "15 ft", "repose_angle": "34 deg"}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/repose_source"}))
    out.append(vec(6, {"shape": "frustum", "radius": "10 m", "height": "6 m"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def station_offset():
    """Checked by rotating the point into the line's frame with a rotation matrix."""
    out = []

    def fmt(v):
        tot = round(v * 100)
        whole = tot // 10000
        return f"{whole}+{(tot - whole * 10000) / 100:05.2f}"

    def frame(sn, se, az, pn, pe):
        a = math.radians(az)
        # Rotate by -az: north along the line, east to its right.
        dn, de = pn - sn, pe - se
        return dn * math.cos(a) + de * math.sin(a), -dn * math.sin(a) + de * math.cos(a)

    for i, (sn, se, s0, az, pn, pe) in enumerate([(5000, 5000, 1000, 60, 5300, 5400), (0, 0, 0, 0, 50, -20), (1000, 2000, 500, 225, 900, 1950), (200, 300, 1250, 135, 150, 400)], 1):
        along, off = frame(sn, se, az, pn, pe)
        side = "right" if off > 0 else "left" if off < 0 else "on the line"
        out.append(vec(i, {"start_northing": f"{sn} ft", "start_easting": f"{se} ft", "start_station": fmt(s0), "direction": f"{az}", "northing": f"{pn} ft", "easting": f"{pe} ft"},
                       {"result.station": fmt(s0 + along), "result.offset.value": off, "result.side": side,
                        "result.foot_northing.value": sn + along * math.cos(math.radians(az)), "result.foot_easting.value": se + along * math.sin(math.radians(az))}, rel=1e-11))
    # And back: the point at 13+50, 12.5 ft left of a line N 60 E from 10+00.
    a = math.radians(60)
    along, off = 350, -12.5
    pn, pe = 5000 + along * math.cos(a) - off * math.sin(a), 5000 + along * math.sin(a) + off * math.cos(a)
    out.append(vec(5, {"start_northing": "5000 ft", "start_easting": "5000 ft", "start_station": "10+00", "direction": "60", "station": "13+50", "offset": "-12.5 ft"},
                   {"result.northing.value": pn, "result.easting.value": pe, "result.side": "left"}, rel=1e-11))
    out.append(vec(6, {"start_northing": "0 ft", "start_easting": "0 ft", "direction": "0"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def lvec(i, inp, exp, src=LAND_SRC, ver=LAND_VER):
    e = dict(exp)
    e.setdefault("ok", True)
    tol = {k: {"rel": 1e-12, "abs": 1e-9} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def legacy_units():
    cases = [("12 chains 34 links", None, 12 * 66 + 34 * 0.66), ("40 rods", None, 660.0), ("1 furlong", None, 660.0),
             ("1,000 varas", "texas", 1000 * 100 / 36), ("10 arpents", "louisiana", 1919.94), ("80 chains", None, 5280.0),
             ("5 varas", "california", 5 * 33.372 / 12)]
    return [lvec(i, {"length": t, **({"jurisdiction": j} if j else {})}, {"result.us_survey_feet.value": float(v)}) for i, (t, j, v) in enumerate(cases, 1)]


def deed_plot():
    import math
    out = []
    cases = [[("N 0 E", 500), ("N 90 E", 425), ("S 0 E", 500), ("S 89°56'36\" W", 425)],
             [("N 45 E", 300), ("S 45 E", 300), ("S 45 W", 300.5), ("N 45 W", 300)],
             [("N 10 E", 250), ("S 80 E", 180), ("S 10 W", 250), ("S 80 W", 180.1)],
             [("N 30 E", 120), ("S 60 E", 200), ("S 30 W", 120), ("N 60 W", 199.9)],
             [("N 0 E", 1000), ("S 89 E", 1000), ("S 1 E", 1000), ("N 88°59'00\" W", 1000)]]
    rnd = random.Random(19)
    for _ in range(12):  # random quadrilaterals to hexagons, closing roughly as surveyed parcels do
        k = rnd.randint(4, 6)
        legs = []
        for j in range(k):
            az = (360 / k * j + rnd.uniform(-20, 20)) % 360
            q = int(az // 90)
            ang = [az, 180 - az, az - 180, 360 - az][q]
            d, m, sec = int(ang), int(ang * 60 % 60), int(ang * 3600 % 60)
            legs.append((f"{'NSSN'[q]} {d}°{m:02d}'{sec:02d}\" {'EEWW'[q]}", round(rnd.uniform(80, 900), 2)))
        cases.append(legs)
    for i, legs in enumerate(cases, 1):
        n = e = 0.0
        pts = [(0.0, 0.0)]
        total = 0.0
        for b, d in legs:
            ns, rest = b[0], b[1:-1].strip()
            ew = b[-1]
            parts = rest.replace("°", " ").replace("'", " ").replace('"', " ").split()
            ang = sum(float(x) / 60 ** k for k, x in enumerate(parts))
            az = {("N", "E"): ang, ("S", "E"): 180 - ang, ("S", "W"): 180 + ang, ("N", "W"): 360 - ang}[(ns, ew)]
            n += d * math.cos(math.radians(az))
            e += d * math.sin(math.radians(az))
            pts.append((n, e))
            total += d
        twice = sum(pts[k][1] * pts[(k + 1) % len(pts)][0] - pts[(k + 1) % len(pts)][1] * pts[k][0] for k in range(len(pts)))
        rows = [{"direction": b, "distance": f"{d} ftUS"} for b, d in legs]
        out.append(lvec(i, {"calls": rows}, {"result.misclosure.value": math.hypot(n, e), "result.area.value": abs(twice) / 2,
                                             "result.total_length.value": float(total)}, ver="Ghilani and Wolf, 15th ed. (2018)"))
    # University of Memphis CIVL 1112 (traverse and area by DMD): the five courses in international feet, compass-balanced,
    # close 0.182 ft (1:5,175, shown to two figures) and enclose 36,320 ft2 = 0.834 acre.
    memphis = [("S 6-15 W", 189.53), ("S 29-38 E", 175.18), ("N 81-18 W", 197.78), ("N 12-24 W", 142.39), ("N 42-59 E", 234.58)]
    v = lvec(len(out) + 1, {"calls": [{"direction": b, "distance": f"{d} ft"} for b, d in memphis], "adjustment": "compass"},
             {"result.misclosure.value": 0.182, "result.misclosure.unit": "ft", "result.precision": "1:5,200",
              "result.area.value": 36320.0, "result.area.unit": "ft2", "result.acres.value": 0.834}, MEMPHIS, MEMPHIS_VER)
    v["tolerance"] = {"result.misclosure.value": {"abs": 0.0005}, "result.area.value": {"abs": 0.5}, "result.acres.value": {"abs": 0.0005}}
    out.append(v)
    # Calls in meters report in meters and international acres; mixed US survey and international feet are refused.
    sq = [("N 0 E", 100), ("N 90 E", 100), ("S 0 E", 100), ("S 90 W", 100.1)]
    out.append(lvec(len(out) + 1, {"calls": [{"direction": b, "distance": f"{d} m"} for b, d in sq]},
                    {"result.misclosure.value": 0.1, "result.misclosure.unit": "m", "result.area.unit": "m2", "result.acres.unit": "ac"}))
    mixed = [{"direction": b, "distance": f"{d} {'ftUS' if k == 2 else 'ft'}"} for k, (b, d) in enumerate(sq)]
    out.append(lvec(len(out) + 1, {"calls": mixed}, {"ok": False, "error.code": "UNIT_MISMATCH", "error.field": "/calls/2/distance"}))
    return out


def plss():
    cases = [("NE¼ SW¼ Sec 12, T3N R4W, 6th PM", 40.0), ("N1/2 NE1/4 Section 5 T12S R3E Willamette Meridian", 80.0),
             ("SW¼ Sec 36 T1N R1E Mount Diablo", 160.0), ("NW¼ NE¼ SE¼ Sec 8 T20N R5W Salt Lake", 10.0), ("E½ Sec 1 T4S R68W 6th PM", 320.0)]
    return [lvec(i, {"description": d}, {"result.nominal_area.value": a}) for i, (d, a) in enumerate(cases, 1)]


def rotation():
    cases = [("N 10°00'00\" E", "N 10°02'30\" E", 2.5 / 60), ("S 45 E", "S 44 E", 1.0), ("N 89 W", "S 89 W", -2.0),
             ("N 0 E", "N 0°00'30\" W", -0.5 / 60), ("S 30 W", "S 30°15' W", 0.25)]
    return [lvec(i, {"record_bearing": a, "new_bearing": b}, {"result.rotation.value": float(r)}) for i, (a, b, r) in enumerate(cases, 1)]


def deed_parse():
    cases = [("Beginning; thence N 45°30'15\" E 200.00 feet; thence S 44-29-45E 100 feet", 2, "N 45°30'15\" E"),
             ("Beginning; thence North 10 degrees East 5 chains 20 links; thence due south 343.2 feet", 2, "N 10°00'00\" E"),
             ("Beginning; thence S0-15-00E 150 ft; thence along the creek to a stone", 2, "S 0°15'00\" E"),
             ("Beginning; thence N 1 W 10 feet; thence N 2 W 10 feet; thence N 3 W 10 feet", 3, "N 1°00'00\" W"),
             ("Beginning at a stake; thence S 89°59'59\" W 1,320.00 feet to a pipe", 1, "S 89°59'59\" W")]
    return [lvec(i, {"text": t}, {"result.count": float(n), "result.calls.0.direction": d}, src="Hand-parsed call forms (Brown's Boundary Control, ch. 5)", ver="7th edition (2014)")
            for i, (t, n, d) in enumerate(cases, 1)]


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    files = {
        "survey.cogo.inverse": inverse(), "survey.cogo.forward": forward(), "survey.cogo.traverse-closure": traverse(),
        "survey.cogo.area-by-coordinates": area(), "survey.curves.circular-curve": circular(), "survey.curves.vertical-curve": vertical(),
        "survey.earthwork.average-end-area": aea(), "survey.earthwork.prismoidal": prismoidal(), "survey.earthwork.shrink-swell": swell(),
        "survey.reduction.combined-factor": combined(),
        "survey.reduction.slope": slope(), "survey.reduction.curvature-refraction": curvature(),
        "survey.reduction.stadia": stadia(), "survey.reduction.inaccessible-height": inaccessible(),
        "survey.cogo.offset-shot": offset_shot(), "survey.earthwork.grade": grade(),
        "survey.reduction.level-run": level_run(),
        "survey.cogo.intersection": intersection(), "survey.cogo.resection": resection(),
        "survey.cogo.angular-closure": angular_closure(), "survey.cogo.station-offset": station_offset(), "survey.earthwork.slope-stake": slope_stake(),
        "survey.earthwork.section-area": section_area(), "survey.earthwork.borrow-pit": borrow_pit(),
        "survey.curves.curve-layout": curve_layout(), "survey.curves.spiral": spiral(),
        "survey.curves.sight-distance": sight_distance(), "survey.earthwork.profile-grades": profile_grades(),
        "survey.earthwork.stockpile": stockpile(), "survey.earthwork.solid-volume": solid_volume(),
        "survey.land.legacy-units": legacy_units(), "survey.land.deed-plot": deed_plot(), "survey.land.plss-parse": plss(),
        "survey.land.basis-rotation": rotation(), "survey.land.deed-parse": deed_parse(),
    }
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
