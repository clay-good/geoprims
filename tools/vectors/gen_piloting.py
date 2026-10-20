#!/usr/bin/env python3
"""Golden vectors for navigation.piloting.*: Bowditch 2024's worked examples
first, then random cases from the piloting formulas written out here (vector
triangle, Table 18 triangle, Table 15 formula, the TVMDC chain), with pyproj
for positions. Prints JSON lines; writes nothing.

    python3 tools/vectors/gen_piloting.py set-and-drift > core/vectors/navigation.piloting.set-and-drift.jsonl
    (likewise dead-reckoning, running-fix, two-bearing-fix, distance-by-vertical-angle, compass-conversion)

Needs pyproj (a venv is fine)."""
import json
import math
import random
import sys

import pyproj

VER = f"Bowditch 2024; Python math; pyproj {pyproj.__version__}"
SRC = "Piloting formulas of Bowditch 2024 written out in Python (tools/vectors/gen_piloting.py)"
GEOD = pyproj.Geod(ellps="WGS84")
F = 1 / 298.257223563


def az(x):
    return x % 360


def en(a, s):
    r = math.radians(a)
    return s * math.sin(r), s * math.cos(r)


def polar(e, n):
    return az(math.degrees(math.atan2(e, n))), math.hypot(e, n)


def vec(i, inp, exp, tol, src=SRC):
    return {"id": f"v{i:03d}", "input": inp, "expect": {"ok": True, **exp}, "source": src, "sourceVersion": VER, "tolerance": tol}


def refusal(i, inp, code):
    return {"id": f"v{i:03d}", "input": inp, "expect": {"ok": False, "error.code": code}, "source": SRC, "sourceVersion": VER, "tolerance": {}}


def set_drift(rng):
    b = "Bowditch 2024 Vol. I §1008"
    out = [
        vec(1, {"problem": "track", "course": "80 deg", "speed": "10 kt", "set": "140 deg", "drift": "2 kt"}, {"result.track.value": 89, "result.speed_made_good.value": 11.2}, {"result.track.value": {"abs": 0.5}, "result.speed_made_good.value": {"abs": 0.1}}, f"{b} example 1"),
        vec(2, {"problem": "course-to-steer", "course": "95 deg", "speed": "12 kt", "set": "170 deg", "drift": "2.5 kt"}, {"result.course_to_steer.value": 83.5, "result.speed_made_good.value": 12.4}, {"result.course_to_steer.value": {"abs": 0.5}, "result.speed_made_good.value": {"abs": 0.1}}, f"{b} example 2"),
        vec(3, {"problem": "course-and-speed", "course": "265 deg", "speed": "15 kt", "set": "185 deg", "drift": "3 kt"}, {"result.course_to_steer.value": 276, "result.speed_to_use.value": 14.8}, {"result.course_to_steer.value": {"abs": 1.0}, "result.speed_to_use.value": {"abs": 0.1}}, f"{b} example 3 (plotted; the course printed to the degree)"),
    ]
    i = 4
    while i <= 20:
        p = rng.choice(["track", "course-to-steer", "course-and-speed"])
        c, v, s, d = round(rng.uniform(0, 360), 1), round(rng.uniform(4, 25), 1), round(rng.uniform(0, 360), 1), round(rng.uniform(0, 3.5), 1)
        ce, cn = en(s, d)
        tol = {"result.track.value": {"abs": 1e-9}, "result.course_to_steer.value": {"abs": 1e-9}, "result.speed_made_good.value": {"abs": 1e-9}, "result.speed_to_use.value": {"abs": 1e-9}}
        if p == "track":
            se, sn = en(c, v)
            t, g = polar(se + ce, sn + cn)
            exp = {"result.track.value": t, "result.speed_made_good.value": g}
        elif p == "course-to-steer":
            # Steer h so that the water vector plus the current lies along c.
            rel = math.radians(s - c)
            x = d * math.sin(rel) / v
            if abs(x) > 1:
                continue
            wca = -math.degrees(math.asin(x))
            g = v * math.cos(math.radians(wca)) + d * math.cos(rel)
            if g <= 0:
                continue
            exp = {"result.course_to_steer.value": az(c + wca), "result.speed_made_good.value": g}
        else:
            ge, gn = en(c, v)
            h, w = polar(ge - ce, gn - cn)
            exp = {"result.course_to_steer.value": h, "result.speed_to_use.value": w}
        out.append(vec(i, {"problem": p, "course": f"{c} deg", "speed": f"{v} kt", "set": f"{s} deg", "drift": f"{d} kt"}, exp, {k: tol[k.replace('result.', 'result.')] for k in exp}))
        i += 1
    out.append(refusal(21, {"problem": "course-to-steer", "course": "0 deg", "speed": "3 kt", "set": "180 deg", "drift": "5 kt"}, "NO_SOLUTION"))
    return out


def mp(lat):
    e = math.sqrt(F * (2 - F))
    s = math.sin(math.radians(lat))
    return 10800 / math.pi * (math.atanh(s) - e * math.atanh(e * s))


def dead_reckoning(rng):
    out = []
    i = 1
    while i <= 20:
        lat, lon = round(rng.uniform(-60, 60), 4), round(rng.uniform(-180, 180), 4)
        c, d = round(rng.uniform(0, 360), 1), round(rng.uniform(1, 99), 2)
        lat2 = lat + d * math.cos(math.radians(c)) / 60
        dlo = (mp(lat2) - mp(lat)) * math.tan(math.radians(c)) if abs(lat2 - lat) > 1e-12 else d * math.sin(math.radians(c)) / math.cos(math.radians(lat))
        lon2 = (lon + dlo / 60 + 180) % 360 - 180
        inp = {"lat": lat, "lon": lon, "course": f"{c} deg"}
        if i % 2:
            inp["distance"] = f"{d} NM"
        else:
            inp["speed"], inp["time"] = f"{d * 2} kt", "0.5 h"
        out.append(vec(i, inp, {"result.lat.value": lat2, "result.lon.value": lon2, "result.distance.value": d}, {"result.lat.value": {"abs": 1e-10}, "result.lon.value": {"abs": 1e-9}, "result.distance.value": {"abs": 1e-9}}))
        i += 1
    out.append(refusal(21, {"lat": 30, "lon": -40, "course": "90 deg", "distance": "700 NM"}, "LIMIT_EXCEEDED"))
    return out


def two_b(a, b, run):
    d2 = run * math.sin(math.radians(a)) / math.sin(math.radians(b - a))
    return d2, d2 * math.sin(math.radians(b))


def running_fix(rng):
    out = [vec(1, {"course": "50 deg", "bearing1": "24 deg", "bearing2": "359 deg", "speed": "15 kt", "time": "10 min"},
               {"result.distance.value": 2.6, "result.distance_abeam.value": 2.0, "result.run.value": 2.5},
               {"result.distance.value": {"abs": 0.05}, "result.distance_abeam.value": {"abs": 0.05}, "result.run.value": {"abs": 1e-9}}, "Bowditch 2024 Vol. I §1121 (Table 18 factors 1.04 and 0.81)")]
    i = 2
    while i <= 20:
        c = round(rng.uniform(0, 360), 1)
        side = rng.choice([1, -1])
        a = rng.uniform(10, 60)
        b = a + rng.uniform(10, 80)
        if b >= 170:
            continue
        b1, b2 = az(c + side * a), az(c + side * b)
        run = round(rng.uniform(0.5, 8), 2)
        d2, ab = two_b(a, b, run)
        olat, olon = round(rng.uniform(-60, 60), 4), round(rng.uniform(-180, 180), 4)
        flon, flat, _ = GEOD.fwd(olon, olat, b2 + 180, d2 * 1852)
        out.append(vec(i, {"course": f"{c} deg", "bearing1": f"{round(b1, 6)} deg", "bearing2": f"{round(b2, 6)} deg", "run": f"{run} NM", "object_lat": olat, "object_lon": olon},
                       {"result.lat.value": flat, "result.lon.value": flon},
                       {"result.lat.value": {"abs": 1e-7}, "result.lon.value": {"abs": 1e-7}}))
        i += 1
    out.append(refusal(21, {"course": "50 deg", "bearing1": "359 deg", "bearing2": "24 deg", "run": "2 NM"}, "INVALID_INPUT"))
    return out


def two_bearing(rng):
    out = [vec(1, {"preset": "doubling", "first": "30 deg", "run": "4 NM"}, {"result.distance_second.value": 4.0, "result.distance_abeam.value": 3.46}, {"result.distance_second.value": {"abs": 1e-9}, "result.distance_abeam.value": {"abs": 0.01}}, "add-marine-navigation scenario: doubling the angle on the bow")]
    for i in range(2, 21):
        a = round(rng.uniform(10, 70), 1)
        b = round(a + rng.uniform(5, 90), 1)
        run = round(rng.uniform(0.5, 10), 2)
        d2, ab = two_b(a, b, run)
        out.append(vec(i, {"preset": "two-bearings", "first": f"{a} deg", "second": f"{b} deg", "run": f"{run} NM"}, {"result.distance_second.value": d2, "result.distance_abeam.value": ab}, {"result.distance_second.value": {"abs": 1e-9}, "result.distance_abeam.value": {"abs": 1e-9}}))
    out.append(refusal(21, {"preset": "two-bearings", "first": "60 deg", "second": "30 deg", "run": "2 NM"}, "INVALID_INPUT"))
    return out


def vertical(rng):
    out = []
    table = [("0°30'", 100, 1.8), ("0°00'", 25, 5.8), ("0°10'", 50, 2.6), ("0°05'", 100, 7.1), ("1°00'", 140, 1.3), ("0°15'", 70, 2.5)]
    for i, (a, hh, want) in enumerate(table, 1):
        out.append(vec(i, {"angle": a, "object_height": f"{hh + 20} ft", "eye_height": "20 ft"}, {"result.distance.value": want}, {"result.distance.value": {"abs": 0.05}}, "Bowditch 2024 Vol. II Table 15"))
    for i in range(len(table) + 1, 21):
        a = round(rng.uniform(0.02, 3), 4)
        h = round(rng.uniform(5, 60), 1)
        H = round(h + rng.uniform(20, 900), 1)
        t = math.tan(math.radians(a)) / 0.0002419
        d = math.sqrt(t * t + (H - h) / 0.7349) - t
        out.append(vec(i, {"angle": f"{a} deg", "object_height": f"{H} ft", "eye_height": f"{h} ft"}, {"result.distance.value": d, "result.horizon.value": 1.17 * math.sqrt(h)}, {"result.distance.value": {"abs": 1e-9}, "result.horizon.value": {"abs": 1e-12}}))
    out.append(refusal(21, {"angle": "0°30'", "object_height": "10 ft", "eye_height": "20 ft"}, "NO_SOLUTION"))
    return out


def compass(rng):
    b = "Bowditch 2024 Vol. II §402"
    out = [
        vec(1, {"heading": "215 deg", "variation": "7 W", "deviation": "1.5 W", "bearing": "306.5 deg", "bearing_reference": "compass"},
            {"result.magnetic_heading.value": 222, "result.compass_heading.value": 223.5, "result.compass_error": "8.5° W", "result.magnetic_bearing.value": 305, "result.true_bearing.value": 298, "result.relative_bearing.value": 83},
            {"result.magnetic_heading.value": {"abs": 1e-9}, "result.compass_heading.value": {"abs": 1e-9}, "result.magnetic_bearing.value": {"abs": 1e-9}, "result.true_bearing.value": {"abs": 1e-9}, "result.relative_bearing.value": {"abs": 1e-9}}, f"{b} example"),
        vec(2, {"heading": "105 deg", "variation": "15 E", "deviation": "5 W"}, {"result.magnetic_heading.value": 90, "result.compass_heading.value": 95, "result.compass_error": "10.0° E"},
            {"result.magnetic_heading.value": {"abs": 1e-9}, "result.compass_heading.value": {"abs": 1e-9}}, f"{b} problem 1, row 1"),
        vec(3, {"heading": "150 deg", "heading_reference": "compass", "variation": "19 E", "deviation": "1 E"}, {"result.magnetic_heading.value": 151, "result.true_heading.value": 170, "result.compass_error": "20.0° E"},
            {"result.magnetic_heading.value": {"abs": 1e-9}, "result.true_heading.value": {"abs": 1e-9}}, f"{b} problem 2"),
        vec(4, {"heading": "67 deg", "heading_reference": "compass", "variation": "12 W", "deviation": "5 E"}, {"result.true_heading.value": 60, "result.magnetic_heading.value": 72},
            {"result.true_heading.value": {"abs": 1e-9}, "result.magnetic_heading.value": {"abs": 1e-9}}, f"{b} problem 1, row 3"),
    ]
    for i in range(5, 21):
        h = round(rng.uniform(0, 360), 1)
        var = round(rng.uniform(-25, 25), 1)
        dev = round(rng.uniform(-6, 6), 1)
        ref = rng.choice(["true", "magnetic", "compass"])
        if ref == "true":
            th = h
        elif ref == "magnetic":
            th = az(h + var)
        else:
            th = az(h + dev + var)
        mh, ch = az(th - var), az(th - var - dev)
        out.append(vec(i, {"heading": f"{h} deg", "heading_reference": ref, "variation": var, "deviation": dev},
                       {"result.true_heading.value": th, "result.magnetic_heading.value": mh, "result.compass_heading.value": ch},
                       {"result.true_heading.value": {"abs": 1e-9}, "result.magnetic_heading.value": {"abs": 1e-9}, "result.compass_heading.value": {"abs": 1e-9}}))
    out.append(refusal(21, {"heading": "10 deg", "variation": "seven"}, "INVALID_INPUT"))
    return out


if __name__ == "__main__":
    which = sys.argv[1]
    rng = random.Random(f"piloting-{which}")
    fn = {"set-and-drift": set_drift, "dead-reckoning": dead_reckoning, "running-fix": running_fix, "two-bearing-fix": two_bearing, "distance-by-vertical-angle": vertical, "compass-conversion": compass}[which]
    for v in fn(rng):
        print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
