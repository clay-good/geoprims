#!/usr/bin/env python3
"""Golden vectors for survey.* by direct evaluation of the textbook formulas
(Ghilani & Wolf, Elementary Surveying, 15th ed.; Stem 1990, NOAA Manual NOS
NGS 5), plus the add-survey-suite spec scenarios."""
import json
import math
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
                   {"meta.warnings.1.code": "PERFECT_CLOSURE", "result.precision": "perfect"}, SPEC, "2026"))
    return out


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
                   {"result.acres.value": 660 * 660 / 43560, "result.acres.unit": "acUS", "meta.warnings.1.code": "LEGACY_UNIT"}, SPEC, "2026", rel=1e-11))
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
    return out


def station(v, per=100, dec=2):
    whole, rest = divmod(round(v * 10**dec), per * 10**dec)
    return f"{whole}+{rest / 10**dec:0{3 + dec}.{dec}f}"


def vertical():
    cases = [(2, -3, 600, 1000, 100.0), (-4, 2, 400, 2500, 350.0), (1.5, -1.5, 300, 12000, 20.5), (-2, -0.5, 500, 800, 60.0), (3, 1, 200, 1500, 75.25)]
    out = []
    for i, (g1, g2, l, s, y) in enumerate(cases, 1):
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
    return out


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
    out.append(vec(6, {"grid_scale": 0.99991, "elevation": "1500 ft"}, {"meta.warnings.1.code": "ORTHOMETRIC_AS_ELLIPSOIDAL"}, SPEC, "2026"))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    files = {
        "survey.cogo.inverse": inverse(), "survey.cogo.forward": forward(), "survey.cogo.traverse-closure": traverse(),
        "survey.cogo.area-by-coordinates": area(), "survey.curves.circular-curve": circular(), "survey.curves.vertical-curve": vertical(),
        "survey.earthwork.average-end-area": aea(), "survey.earthwork.prismoidal": prismoidal(), "survey.earthwork.shrink-swell": swell(),
        "survey.reduction.combined-factor": combined(),
    }
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
