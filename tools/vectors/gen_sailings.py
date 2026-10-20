#!/usr/bin/env python3
"""Golden vectors for navigation.sailing.*: Bowditch 2024 Volume I chapter 13's
worked examples (full precision where the book rounds midway), then random
cases from the sailing formulas written out here, with pyproj.Geod for the
WGS 84 geodesic beside the great circle. Prints JSON lines; writes nothing.

    python3 tools/vectors/gen_sailings.py plane > core/vectors/navigation.sailing.plane.jsonl
    (likewise traverse, parallel, mid-latitude, mercator, great-circle)

Needs pyproj (a venv is fine)."""
import json
import math
import random
import sys

import pyproj

VER = f"Bowditch 2024; Python math; pyproj {pyproj.__version__}"
SRC = "Sailing formulas of Bowditch 2024 ch. 13 written out in Python (tools/vectors/gen_sailings.py)"
BOW = "Bowditch 2024 Vol. I ch. 13"
GEOD = pyproj.Geod(ellps="WGS84")
F = 1 / 298.257223563


def mp(lat):
    e = math.sqrt(F * (2 - F))
    s = math.sin(math.radians(lat))
    return 10800 / math.pi * (math.atanh(s) - e * math.atanh(e * s))


def wrap(x):
    return (x + 180) % 360 - 180


def az(x):
    return x % 360


def vec(id_, inp, exp, tol, src=SRC):
    return {"id": id_, "input": inp, "expect": {"ok": True, **exp}, "source": src, "sourceVersion": VER, "tolerance": tol}


def nm(v):
    return f"{v} NM"


def dg(v):
    return f"{v} deg"


def plane(rng):
    out = [vec("v001", {"course": dg(5), "distance": nm(188)}, {"result.lat_difference.value": 187.3, "result.departure.value": 16.4},
               {"result.lat_difference.value": {"abs": 0.05}, "result.departure.value": {"abs": 0.05}}, f"{BOW} §1302")]
    for i in range(19):
        c, d = round(rng.uniform(0, 360), 2), round(rng.uniform(1, 600), 1)
        if i % 2:
            out.append(vec(f"v{i + 2:03d}", {"course": dg(c), "distance": nm(d)},
                           {"result.lat_difference.value": d * math.cos(math.radians(c)), "result.departure.value": d * math.sin(math.radians(c))},
                           {"result.lat_difference.value": {"abs": 1e-9}, "result.departure.value": {"abs": 1e-9}}))
        else:
            l, p = round(rng.uniform(-400, 400), 1), round(rng.uniform(-400, 400), 1)
            out.append(vec(f"v{i + 2:03d}", {"lat_difference": nm(l), "departure": nm(p)},
                           {"result.course.value": az(math.degrees(math.atan2(p, l))), "result.distance.value": math.hypot(l, p)},
                           {"result.course.value": {"abs": 1e-9}, "result.distance.value": {"abs": 1e-9}}))
    out.append({"id": "v021", "input": {"departure": nm(5)}, "expect": {"ok": False, "error.code": "INVALID_INPUT"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    return out


def traverse(rng):
    legs = [(158, 15.5), (135, 33.7), (259, 16.1), (293, 39.0), (169, 40.4)]
    out = [vec("v001", {"legs": [{"course": dg(c), "distance": nm(d)} for c, d in legs]},
               {"result.course.value": 192.3, "result.distance.value": 67.3},
               {"result.course.value": {"abs": 0.05}, "result.distance.value": {"abs": 0.1}}, f"{BOW} §1303 (table values rounded to 0.1 NM per leg)")]
    for i in range(19):
        legs = [(round(rng.uniform(0, 360), 1), round(rng.uniform(1, 80), 1)) for _ in range(rng.randint(1, 8))]
        l = sum(d * math.cos(math.radians(c)) for c, d in legs)
        p = sum(d * math.sin(math.radians(c)) for c, d in legs)
        out.append(vec(f"v{i + 2:03d}", {"legs": [{"course": dg(c), "distance": nm(d)} for c, d in legs]},
                       {"result.course.value": az(math.degrees(math.atan2(p, l))), "result.distance.value": math.hypot(l, p), "result.lat_difference.value": l, "result.departure.value": p},
                       {"result.course.value": {"abs": 1e-9}, "result.distance.value": {"abs": 1e-9}, "result.lat_difference.value": {"abs": 1e-9}, "result.departure.value": {"abs": 1e-9}}))
    return out


def parallel(rng):
    out = [vec("v001", {"latitude": "49°30'N", "dlo": dg(3.5)}, {"result.departure.value": 136.4}, {"result.departure.value": {"abs": 0.05}}, f"{BOW} §1304")]
    for i in range(19):
        lat = round(rng.uniform(-80, 80), 3)
        if i % 2:
            dlo = round(rng.uniform(-20, 20), 3)
            out.append(vec(f"v{i + 2:03d}", {"latitude": dg(lat), "dlo": dg(dlo)}, {"result.departure.value": dlo * 60 * math.cos(math.radians(lat))}, {"result.departure.value": {"abs": 1e-9}}))
        else:
            p = round(rng.uniform(-900, 900), 1)
            out.append(vec(f"v{i + 2:03d}", {"latitude": dg(lat), "departure": nm(p)}, {"result.dlo.value": p / math.cos(math.radians(lat)) / 60}, {"result.dlo.value": {"abs": 1e-11}}))
    out.append({"id": "v021", "input": {"latitude": dg(90), "departure": nm(10)}, "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    return out


def midlat(rng):
    out = [vec("v001", {"lat1": "15°17.0'N", "lon1": "151°37.0'E", "course": dg(70), "distance": nm(1253)},
               {"result.lat2.value": 22 + 25.6 / 60, "result.lon2.value": 172 + 21.2 / 60},
               {"result.lat2.value": {"abs": 0.05 / 60}, "result.lon2.value": {"abs": 0.05 / 60}},
               f"{BOW} §1305 (printed 172°21.1'E from DLo rounded to 1,244.1'; 172°21.2'E at full precision)")]
    for i in range(19):
        lat1, lon1 = round(rng.uniform(-60, 60), 4), round(rng.uniform(-180, 180), 4)
        if i % 2:
            lat2 = max(min(lat1 + rng.uniform(-8, 8), 70), -70)
            if lat1 * lat2 < 0:
                lat2 = -lat2
            lat2, lon2 = round(lat2, 4), round(wrap(lon1 + rng.uniform(-15, 15)), 4)
            l = (lat2 - lat1) * 60
            lm = (lat1 + lat2) / 2
            p = wrap(lon2 - lon1) * 60 * math.cos(math.radians(lm))
            out.append(vec(f"v{i + 2:03d}", {"lat1": lat1, "lon1": lon1, "lat2": lat2, "lon2": lon2},
                           {"result.course.value": az(math.degrees(math.atan2(p, l))), "result.distance.value": math.hypot(l, p)},
                           {"result.course.value": {"abs": 1e-9}, "result.distance.value": {"abs": 1e-9}}))
        else:
            c, d = round(rng.uniform(0, 360), 2), round(rng.uniform(10, 500), 1)
            l, p = d * math.cos(math.radians(c)), d * math.sin(math.radians(c))
            lat2 = lat1 + l / 60
            lm = (lat1 + lat2) / 2
            lon2 = wrap(lon1 + p / math.cos(math.radians(lm)) / 60)
            out.append(vec(f"v{i + 2:03d}", {"lat1": lat1, "lon1": lon1, "course": dg(c), "distance": nm(d)},
                           {"result.lat2.value": lat2, "result.lon2.value": lon2},
                           {"result.lat2.value": {"abs": 1e-11}, "result.lon2.value": {"abs": 1e-10}}))
    return out


def mercator(rng):
    out = [vec("v001", {"lat1": "32°14.7'N", "lon1": "66°28.9'W", "lat2": "36°58.7'N", "lon2": "75°42.2'W"},
               {"result.m1": 2033.4, "result.m2": 2377.1, "result.meridional_difference": 343.7, "result.course.value": 301.8, "result.distance.value": 538.2},
               {"result.m1": {"abs": 0.06}, "result.m2": {"abs": 0.06}, "result.meridional_difference": {"abs": 0.05}, "result.course.value": {"abs": 0.05}, "result.distance.value": {"abs": 0.05}},
               f"{BOW} §1306 (printed 538.7 NM uses 58.18521°; atan(553.3/343.7) = 58.1526° gives 538.2 NM)")]
    for i in range(19):
        lat1, lon1 = round(rng.uniform(-70, 70), 4), round(rng.uniform(-180, 180), 4)
        if i % 2:
            lat2, lon2 = round(max(min(lat1 + rng.uniform(-20, 20), 75), -75), 4), round(wrap(lon1 + rng.uniform(-60, 60)), 4)
            m = mp(lat2) - mp(lat1)
            dlo = wrap(lon2 - lon1) * 60
            c = math.degrees(math.atan2(dlo, m))
            l = (lat2 - lat1) * 60
            d = abs(l / math.cos(math.radians(c)))
            out.append(vec(f"v{i + 2:03d}", {"lat1": lat1, "lon1": lon1, "lat2": lat2, "lon2": lon2},
                           {"result.course.value": az(c), "result.distance.value": d, "result.m1": mp(lat1), "result.m2": mp(lat2)},
                           {"result.course.value": {"abs": 1e-9}, "result.distance.value": {"rel": 1e-10, "abs": 1e-9}, "result.m1": {"abs": 1e-9}, "result.m2": {"abs": 1e-9}}))
        else:
            c, d = rng.choice([round(rng.uniform(0, 360), 2), round(rng.uniform(10, 80), 2)]), round(rng.uniform(50, 1500), 1)
            l = d * math.cos(math.radians(c))
            lat2 = lat1 + l / 60
            if abs(lat2) >= 85:
                c, lat2 = 45.0, lat1 + d * math.cos(math.radians(45)) / 60
                l = d * math.cos(math.radians(c))
            m = mp(lat2) - mp(lat1)
            lon2 = wrap(lon1 + m * math.tan(math.radians(c)) / 60)
            out.append(vec(f"v{i + 2:03d}", {"lat1": lat1, "lon1": lon1, "course": dg(c), "distance": nm(d)},
                           {"result.lat2.value": lat2, "result.lon2.value": lon2},
                           {"result.lat2.value": {"abs": 1e-11}, "result.lon2.value": {"abs": 1e-9}}))
    out.append({"id": "v021", "input": {"lat1": 90, "lon1": 0, "lat2": 10, "lon2": 10}, "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    return out


def gc(rng):
    out = [
        vec("v001", {"lat1": "22 S", "lon1": "116 E", "lat2": "20 S", "lon2": "31 E"},
            {"result.distance.value": 4693.5, "result.course.value": 253.0, "result.final_course.value": 289.4},
            {"result.distance.value": {"abs": 0.05}, "result.course.value": {"abs": 0.05}, "result.final_course.value": {"abs": 0.05}}, f"{BOW} §1307 examples 1 and 3"),
        vec("v002", {"lat1": "28 N", "lon1": "122 W", "lat2": "24 S", "lon2": "151 E"},
            {"result.distance.value": 5913.1, "result.course.value": 247.3},
            {"result.distance.value": {"abs": 0.15}, "result.course.value": {"abs": 0.05}}, f"{BOW} §1307 example 2 (D rounded to 98.55° in print)"),
    ]
    for i in range(18):
        lat1, lon1 = round(rng.uniform(-70, 70), 4), round(rng.uniform(-180, 180), 4)
        lat2, lon2 = round(rng.uniform(-70, 70), 4), round(rng.uniform(-180, 180), 4)
        a, b = math.radians(lat1), math.radians(lat2)
        dl = math.radians(wrap(lon2 - lon1))
        cd = math.sin(a) * math.sin(b) + math.cos(a) * math.cos(b) * math.cos(dl)
        d = math.degrees(math.acos(max(-1, min(1, cd)))) * 60
        c = math.degrees(math.atan2(math.sin(dl) * math.cos(b), math.cos(a) * math.sin(b) - math.sin(a) * math.cos(b) * math.cos(dl)))
        gaz, _, gs = GEOD.inv(lon1, lat1, lon2, lat2)
        out.append(vec(f"v{i + 3:03d}", {"lat1": lat1, "lon1": lon1, "lat2": lat2, "lon2": lon2},
                       {"result.distance.value": d, "result.course.value": az(c), "result.geodesic_distance.value": gs / 1852, "result.geodesic_course.value": az(gaz)},
                       {"result.distance.value": {"rel": 1e-10, "abs": 1e-8}, "result.course.value": {"abs": 1e-8}, "result.geodesic_distance.value": {"abs": 1e-6}, "result.geodesic_course.value": {"abs": 1e-8}}))
    out.append({"id": "v021", "input": {"lat1": 10, "lon1": 20, "lat2": -10, "lon2": -160}, "expect": {"ok": False, "error.code": "DEGENERATE_GEOMETRY"}, "source": SRC, "sourceVersion": VER, "tolerance": {}})
    return out


if __name__ == "__main__":
    which = sys.argv[1]
    rng = random.Random(f"sailing-{which}")
    fn = {"plane": plane, "traverse": traverse, "parallel": parallel, "mid-latitude": midlat, "mercator": mercator, "great-circle": gc}[which]
    for v in fn(rng):
        print(json.dumps(v, ensure_ascii=False, separators=(",", ":")))
