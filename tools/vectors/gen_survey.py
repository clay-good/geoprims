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
        "survey.land.legacy-units": legacy_units(), "survey.land.deed-plot": deed_plot(), "survey.land.plss-parse": plss(),
        "survey.land.basis-rotation": rotation(), "survey.land.deed-parse": deed_parse(),
    }
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
