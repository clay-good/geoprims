#!/usr/bin/env python3
"""Golden vectors for the drone mission-planning tools (add-flight-and-drone-
planning-tools): drone.mission.sorties, drone.ops.wind-limit,
drone.ops.vlos-check, and drone.photogrammetry.gcp-plan.

Each is worked here apart from the core:

- Distances come from GeographicLib's own command-line tools (GeodSolve for
  geodesics, Planimeter for polygon areas), a separate implementation from the
  Rust port the core uses.
- Sorties walk the path the way the spec states it: a sortie ends at the last
  waypoint from which the trip home (distance / (transit speed - headwind))
  still lands within the battery time less the reserve.
- Wind at height is the power law of NREL/TP-5000-63696 section 5.3.2 with the
  Table 1 textbook exponents, and alpha = 1/7 over open terrain
  (NREL/SR-440-22223, page 3-3).
- Checkpoint counts are ASPRS Positional Accuracy Standards Edition 2, Annex C,
  Table C.1, typed in from the standard (Version 1.0, February 2023), and its
  C.3 worked case: 1,500 km2 with 500 km2 vegetated takes 40 + 30 = 70.

Run: python3 tools/vectors/gen_mission_planning.py
It writes only these four tools' files, and only ids not already present, so
published vectors stay frozen.
"""
import json
import math
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
KT = 1852.0 / 3600.0


def fx(x):
    # GeographicLib's tools read an exponent's "e" as a hemisphere letter.
    return f"{x:.12f}"


def geod_inverse(pairs):
    """Geodesic distances (m) for [(lat1, lon1, lat2, lon2)] via GeodSolve."""
    if not pairs:
        return []
    text = "\n".join(" ".join(fx(v) for v in p) for p in pairs) + "\n"
    out = subprocess.run(["GeodSolve", "-i", "-p", "9"], input=text, capture_output=True, text=True, check=True).stdout
    return [float(line.split()[2]) for line in out.strip().splitlines()]


def geod_direct(lat, lon, azi, s):
    out = subprocess.run(["GeodSolve", "-p", "12"], input=f"{fx(lat)} {fx(lon)} {fx(azi)} {fx(s)}\n", capture_output=True, text=True, check=True).stdout
    la, lo, _ = out.split()
    return float(la), float(lo)


def planimeter(ring):
    """Geodesic polygon area (m2) via Planimeter."""
    text = "\n".join(f"{fx(a)} {fx(b)}" for a, b in ring) + "\n"
    out = subprocess.run(["Planimeter", "-p", "6"], input=text, capture_output=True, text=True, check=True).stdout
    return abs(float(out.split()[2]))


# ------------------------------------------------------------------ sorties

def sorties(home, wps, flight_s, reserve, v_c, v_t=None, w=0.0):
    """[(first, last, flying_s, home_s)] 0-based, or ('out', index, distance)."""
    v_t = v_t or v_c
    f = flight_s * (1 - reserve)
    d = geod_inverse([(home[0], home[1], p[0], p[1]) for p in wps])
    legs = geod_inverse([(a[0], a[1], b[0], b[1]) for a, b in zip(wps, wps[1:])])
    back = lambda j: d[j] / (v_t - w)
    out, s = [], 0
    while True:
        t = d[s] / v_t
        if t + back(s) > f:
            return ("out", s, d[s])
        j = s
        while j + 1 < len(wps) and t + legs[j] / v_c + back(j + 1) <= f:
            t += legs[j] / v_c
            j += 1
        if j == s and j + 1 < len(wps):
            return ("out", j + 1, d[j + 1])
        out.append((s, j, t, back(j)))
        if j + 1 >= len(wps):
            return out
        s = j


def grid(lat0, lon0, lines, dlat, dlon, first=0.0009):
    """A serpentine: `lines` east-west lines dlat apart, dlon long."""
    pts = []
    for k in range(lines):
        la = lat0 + first + k * dlat
        ends = [lon0, lon0 + dlon]
        if k % 2:
            ends.reverse()
        pts += [(round(la, 7), round(ends[0], 7)), (round(la, 7), round(ends[1], 7))]
    return pts


def wp_json(pts):
    return [{"lat": round(a, 7), "lon": round(b, 7)} for a, b in pts]


def tol(paths, rel=1e-9, ab=1e-9):
    return {p: {"rel": rel, "abs": ab} for p in paths}


def sortie_vectors():
    src = "Sorties walked in Python from GeographicLib GeodSolve distances (tools/vectors/gen_mission_planning.py)"
    ver = "GeographicLib 2.x GeodSolve; drone/mission-planning spec"
    home = (40.0, -105.0)
    cases = [
        ("Hand-worked example: 1.2 km grid, 4 lines, 7 min battery, 20% reserve",
         grid(40.0, -105.0, 4, 0.0018, 0.01408), dict(flight_time="7 min", groundspeed="10 m/s", reserve=20), (420, 0.2, 10.0, None, 0.0)),
        ("One battery covers the whole path",
         grid(40.0, -105.0, 2, 0.0018, 0.005), dict(flight_time="20 min", groundspeed="10 m/s", reserve=20), (1200, 0.2, 10.0, None, 0.0)),
        ("A 2 km-long grid on 20 min batteries with a headwind home",
         grid(40.0, -105.0, 6, 0.0018, 0.0235), dict(flight_time="20 min", groundspeed="8 m/s", transit_speed="15 m/s", headwind="5 m/s", reserve=25), (1200, 0.25, 8.0, 15.0, 5.0)),
        ("Battery energy and power instead of a flight time",
         grid(40.0, -105.0, 4, 0.0018, 0.01408), dict(energy="50 Wh", usable=80, power="200 W", groundspeed="10 m/s", reserve=20), (50 * 3600 * 0.8 / 200, 0.2, 10.0, None, 0.0)),
        ("Metric speeds in km/h and a 30% reserve",
         grid(40.0, -105.0, 5, 0.0018, 0.01), dict(flight_time="9 min", groundspeed="36 km/h", reserve=30), (540, 0.3, 10.0, None, 0.0)),
        ("A single nearby waypoint",
         [(40.001, -105.0)], dict(flight_time="5 min", groundspeed="8 m/s"), (300, 0, 8.0, None, 0.0)),
        ("Zero reserve on a six-line grid",
         grid(40.0, -105.0, 6, 0.0012, 0.008), dict(flight_time="8 min", groundspeed="7 m/s", reserve=0), (480, 0, 7.0, None, 0.0)),
        ("A 50 percent reserve",
         grid(40.0, -105.0, 5, 0.001, 0.006), dict(flight_time="15 min", groundspeed="9 m/s", reserve=50), (900, 0.5, 9.0, None, 0.0)),
        ("Flight time in hours and speed in knots",
         grid(40.0, -105.0, 4, 0.0015, 0.009), dict(flight_time="0.25 h", groundspeed="20 kt", reserve=15), (900, 0.15, 20 * KT, None, 0.0)),
        ("Flight time in seconds and speed in kilometers per hour",
         grid(40.0, -105.0, 7, 0.001, 0.007), dict(flight_time="1200 s", groundspeed="25 km/h", reserve=20), (1200, 0.2, 25 / 3.6, None, 0.0)),
        ("Battery energy in kilowatt-hours",
         grid(40.0, -105.0, 4, 0.0013, 0.01), dict(energy="0.1 kWh", usable=70, power="300 W", groundspeed="11 m/s", reserve=10), (0.1 * 3_600_000 * 0.7 / 300, 0.1, 11.0, None, 0.0)),
        ("Battery energy in kilojoules",
         grid(40.0, -105.0, 3, 0.0017, 0.011), dict(energy="180 kJ", usable=85, power="250 W", groundspeed="8 m/s", reserve=20), (180_000 * 0.85 / 250, 0.2, 8.0, None, 0.0)),
        ("A strong but flyable headwind home",
         grid(40.0, -105.0, 5, 0.0014, 0.012), dict(flight_time="18 min", groundspeed="9 m/s", transit_speed="18 m/s", headwind="9 m/s", reserve=25), (1080, 0.25, 9.0, 18.0, 9.0)),
        ("Transit and headwind speeds in knots",
         grid(40.0, -105.0, 5, 0.0011, 0.009), dict(flight_time="14 min", groundspeed="8 m/s", transit_speed="24 kt", headwind="6 kt", reserve=20), (840, 0.2, 8.0, 24 * KT, 6 * KT)),
        ("A long eleven-line grid",
         grid(40.0, -105.0, 11, 0.001, 0.015), dict(flight_time="12 min", groundspeed="10 m/s", reserve=20), (720, 0.2, 10.0, None, 0.0)),
        ("A short route that ends near home",
         [(40.001, -105.0), (40.001, -104.996), (40.0, -105.0)], dict(flight_time="6 min", groundspeed="6 m/s", reserve=20), (360, 0.2, 6.0, None, 0.0)),
        ("A north-south out-and-back route",
         [(40.002, -105.0), (40.006, -105.0), (40.002, -105.0)], dict(flight_time="10 min", groundspeed="7 m/s", reserve=25), (600, 0.25, 7.0, None, 0.0)),
        ("A separate fast transit speed",
         grid(40.0, -105.0, 8, 0.001, 0.012), dict(flight_time="10 min", groundspeed="6 m/s", transit_speed="15 m/s", reserve=20), (600, 0.2, 6.0, 15.0, 0.0)),
        ("No usable-share deduction beyond reserve",
         grid(40.0, -105.0, 5, 0.0016, 0.008), dict(energy="75 Wh", usable=100, power="225 W", groundspeed="9 m/s", reserve=25), (75 * 3600 / 225, 0.25, 9.0, None, 0.0)),
        ("A low 5 percent reserve",
         grid(40.0, -105.0, 9, 0.0009, 0.01), dict(flight_time="11 min", groundspeed="12 m/s", reserve=5), (660, 0.05, 12.0, None, 0.0)),
        ("Survey and transit speeds in miles per hour",
         grid(40.0, -105.0, 6, 0.0013, 0.01), dict(flight_time="16 min", groundspeed="18 mph", transit_speed="28 mph", reserve=20), (960, 0.2, 18 * 0.44704, 28 * 0.44704, 0.0)),
    ]
    vecs = []
    for title, pts, extra, (fs, r, vc, vt, w) in cases:
        res = sorties(home, pts, fs, r, vc, vt, w)
        assert res[0] != "out", title
        inp = {"lat": home[0], "lon": home[1], "waypoints": wp_json(pts), **extra}
        exp = {"result.batteries": float(len(res)), "ok": True}
        for k, (s, j, t, b) in enumerate(res):
            exp[f"result.sorties.{k}.first_waypoint"] = float(s + 1)
            exp[f"result.sorties.{k}.last_waypoint"] = float(j + 1)
            exp[f"result.sorties.{k}.flying_time.value"] = t / 60
            exp[f"result.sorties.{k}.return_time.value"] = b / 60
        exp["result.total_time.value"] = sum(t + b for _, _, t, b in res) / 60
        num = [k for k, v in exp.items() if isinstance(v, float) and "time" in k]
        tl = {**tol([k for k in exp if isinstance(exp[k], float) and k not in num], 0, 1e-12), **tol(num, 1e-9, 1e-9)}
        vecs.append({"input": inp, "expect": exp, "source": f"{src}: {title}", "sourceVersion": ver, "tolerance": tl})
    # A waypoint too far to fly out to and back.
    far = [(40.0009, -105.0), (40.0009, -104.8)]
    res = sorties(home, far, 600, 0.2, 10.0)
    assert res[0] == "out" and res[1] == 1
    vecs.append({
        "input": {"lat": 40.0, "lon": -105.0, "waypoints": wp_json(far), "flight_time": "10 min", "groundspeed": "10 m/s", "reserve": 20},
        "expect": {"ok": False, "error.code": "NO_SOLUTION", "error.field": "/waypoints/1"},
        "source": f"{src}: waypoint 2 is {res[2]:.0f} m out, beyond the 2,400 m out-and-back of 8 min at 10 m/s",
        "sourceVersion": ver,
    })
    return vecs


# ------------------------------------------------------------------ wind limit

ALPHA = {"water": 0.09, "open": 1 / 7, "crops": 0.19, "hedges": 0.24, "suburbs": 0.31, "woodland": 0.43}


def wind_vectors():
    src = "The power law V = Vref (h / href)^alpha, NREL/TP-5000-63696 section 5.3.2 and Table 1 (textbook column), alpha = 1/7 over open terrain per NREL/SR-440-22223, evaluated in Python (tools/vectors/gen_mission_planning.py)"
    ver = "NREL/TP-5000-63696 (September 2015); NREL/SR-440-22223 (April 1997)"
    cases = [
        ({"wind_speed": "15 kt", "gust": "25 kt", "flying_height": "120 m", "wind_rating": "12 m/s", "airspeed": "15 m/s"}, 15 * KT, 25 * KT, 10, 120, "open", 12, 15),
        ({"wind_speed": "8 m/s", "flying_height": "10 m", "wind_rating": "12 m/s"}, 8, None, 10, 10, "open", 12, None),
        ({"wind_speed": "5 m/s", "gust": "9 m/s", "flying_height": "100 m", "terrain": "woodland", "wind_rating": "10 m/s"}, 5, 9, 10, 100, "woodland", 10, None),
        ({"wind_speed": "6 m/s", "flying_height": "60 m", "terrain": "water", "wind_rating": "12 m/s"}, 6, None, 10, 60, "water", 12, None),
        ({"wind_speed": "10 kt", "flying_height": "400 ft", "terrain": "suburbs", "airspeed": "16 m/s", "margin": 50}, 10 * KT, None, 10, 400 * 0.3048, "suburbs", 8, 16),
        ({"wind_speed": "4 m/s", "flying_height": "80 m", "report_height": "30 m", "terrain": "crops", "wind_rating": "10 m/s"}, 4, None, 30, 80, "crops", 10, None),
        ({"wind_speed": "4 m/s", "flying_height": "50 m", "terrain": "hedges", "wind_rating": "10 m/s"}, 4, None, 10, 50, "hedges", 10, None),
        ({"wind_speed": "12 mph", "flying_height": "200 ft", "report_height": "33 ft", "terrain": "water", "wind_rating": "20 mph"}, 12 * 0.44704, None, 33 * 0.3048, 200 * 0.3048, "water", 20 * 0.44704, None),
        ({"wind_speed": "0 m/s", "flying_height": "1000 m", "wind_rating": "5 m/s"}, 0, None, 10, 1000, "open", 5, None),
        ({"wind_speed": "7 m/s", "gust": "8 m/s", "flying_height": "5 m", "report_height": "10 m", "terrain": "crops", "wind_rating": "10 m/s"}, 7, 8, 10, 5, "crops", 10, None),
        ({"wind_speed": "10 kt", "gust": "12 kt", "flying_height": "30 m", "report_height": "30 m", "terrain": "hedges", "wind_rating": "6 m/s"}, 10 * KT, 12 * KT, 30, 30, "hedges", 6, None),
        ({"wind_speed": "9 m/s", "flying_height": "100 m", "terrain": "suburbs", "wind_rating": "10 m/s"}, 9, None, 10, 100, "suburbs", 10, None),
        ({"wind_speed": "2 m/s", "gust": "3 m/s", "flying_height": "15 m", "report_height": "50 m", "terrain": "woodland", "wind_rating": "8 m/s"}, 2, 3, 50, 15, "woodland", 8, None),
        ({"wind_speed": "15 kt", "flying_height": "1000 m", "terrain": "water", "wind_rating": "20 m/s"}, 15 * KT, None, 10, 1000, "water", 20, None),
        ({"wind_speed": "5 m/s", "flying_height": "20 m", "report_height": "40 m", "wind_rating": "10 m/s"}, 5, None, 40, 20, "open", 10, None),
        ({"wind_speed": "6 m/s", "flying_height": "120 m", "report_height": "20 m", "terrain": "crops", "airspeed": "20 m/s", "margin": 25}, 6, None, 20, 120, "crops", 15, 20),
        ({"wind_speed": "6 m/s", "gust": "7 m/s", "flying_height": "120 m", "report_height": "20 m", "terrain": "hedges", "airspeed": "16 m/s", "margin": 25}, 6, 7, 20, 120, "hedges", 12, 16),
        ({"wind_speed": "4 m/s", "flying_height": "50 m", "report_height": "5 m", "terrain": "suburbs", "wind_rating": "15 m/s"}, 4, None, 5, 50, "suburbs", 15, None),
        ({"wind_speed": "1 m/s", "flying_height": "500 m", "terrain": "woodland", "airspeed": "25 m/s", "margin": 40}, 1, None, 10, 500, "woodland", 15, 25),
        ({"wind_speed": "7.5 m/s", "gust": "7.5 m/s", "flying_height": "80 m", "terrain": "open", "wind_rating": "10 m/s"}, 7.5, 7.5, 10, 80, "open", 10, None),
    ]
    vecs = []
    for inp, v, g, href, h, terr, rating, air in cases:
        a = ALPHA[terr]
        f = (h / href) ** a
        exp = {"result.wind_at_height.value": v * f, "result.exponent": a, "result.rating.value": float(rating), "ok": True}
        peak = v * f
        if g is not None:
            exp["result.gust_at_height.value"] = g * f
            peak = g * f
        exp["result.margin_to_rating.value"] = rating - peak
        if air is not None:
            exp["result.groundspeed_into_wind.value"] = air - v * f
        exp["result.status"] = "over the limit" if v * f > rating else ("gusts over the limit" if peak > rating else "within the limit")
        if peak > rating:
            exp["meta.warnings.*.code"] = "WIND_EXCEEDS_RATING" if v * f > rating else "GUST_EXCEEDS_RATING"
        vecs.append({"input": inp, "expect": exp, "source": src, "sourceVersion": ver,
                     "tolerance": tol([k for k, x in exp.items() if isinstance(x, float)], 1e-12, 1e-12)})
    # NYSERDA prints this worked example to 0.1 mph. Its alpha is calculated
    # from measured speeds at 100 ft and 120 ft, then printed as 0.230 before
    # it is used to estimate the speed at 110 ft.
    mph = 0.44704
    vecs.append({
        "input": {"wind_speed": "11.7 mph", "flying_height": "110 ft", "report_height": "100 ft", "exponent": 0.230, "wind_rating": "20 mph"},
        "expect": {"result.wind_at_height.value": 12.0 * mph, "result.exponent": 0.230, "result.status": "within the limit", "ok": True},
        "source": "NYSERDA Customer Sited Wind Handbook, Final Report 10-08, section 6.2, page 33: 11.7 mph at 100 ft with alpha 0.230 gives 12.0 mph at 110 ft",
        "sourceVersion": "Final Report 10-08 (August 2010)",
        "tolerance": {"result.wind_at_height.value": {"rel": 0, "abs": 0.05 * mph}, "result.exponent": {"rel": 0, "abs": 1e-12}},
    })
    # A reader-supplied zero exponent makes the wind constant with height and
    # checks that equality with the rating remains within the limit.
    vecs.append({
        "input": {"wind_speed": "6 m/s", "gust": "6 m/s", "flying_height": "750 m", "report_height": "2 m", "exponent": 0, "wind_rating": "6 m/s"},
        "expect": {"result.wind_at_height.value": 6.0, "result.gust_at_height.value": 6.0, "result.exponent": 0.0, "result.rating.value": 6.0, "result.margin_to_rating.value": 0.0, "result.status": "within the limit", "ok": True},
        "source": f"{src}: alpha = 0 makes the power-law height factor exactly 1",
        "sourceVersion": ver,
        "tolerance": tol(["result.wind_at_height.value", "result.gust_at_height.value", "result.exponent", "result.rating.value", "result.margin_to_rating.value"], 0, 1e-12),
    })
    return vecs


# ------------------------------------------------------------------ VLOS

def vlos_vectors():
    src = "Geodesic distances by GeographicLib GeodSolve (tools/vectors/gen_mission_planning.py); 14 CFR 107.31 sets no distance, so the range is the reader's"
    ver = "GeographicLib 2.x GeodSolve"
    vecs = []
    pilot = (40.0, -105.0)
    grids = [
        (pilot, grid(40.0, -105.0, 2, 0.0018, 0.01408), 500.0),
        ((51.5, -0.12), [(51.505, -0.12), (51.505, -0.1), (51.51, -0.1), (51.51, -0.12)], 1000.0),
        ((-33.9, 151.2), [(-33.895, 151.2), (-33.895, 151.21), (-33.89, 151.21)], 800.0),
        ((0.0, 179.999), [(0.001, -179.998), (0.002, 179.997)], 400.0),
    ]
    for p, pts, rng in grids:
        d = geod_inverse([(p[0], p[1], a, b) for a, b in pts])
        far = max(range(len(d)), key=lambda i: d[i])
        beyond = [i for i, x in enumerate(d) if x > rng]
        exp = {"result.farthest_distance.value": d[far], "result.farthest_waypoint": float(far + 1), "result.beyond_count": float(len(beyond)), "ok": True}
        for k, i in enumerate(beyond):
            exp[f"result.beyond.{k}.waypoint"] = float(i + 1)
        if beyond:
            exp["meta.warnings.*.code"] = "BEYOND_VISUAL_RANGE"
        for i, x in enumerate(d):
            exp[f"result.waypoints.{i}.distance.value"] = x
        vecs.append({"input": {"lat": p[0], "lon": p[1], "waypoints": wp_json(pts), "visual_range": f"{rng:g} m"}, "expect": exp,
                     "source": src, "sourceVersion": ver,
                     "tolerance": {**tol([k for k in exp if "distance" in k], 1e-9, 1e-6), **tol([k for k in exp if isinstance(exp[k], float) and "distance" not in k], 0, 1e-12)}})
    # A waypoint placed exactly on the ring by the direct problem counts as inside.
    on = geod_direct(40.0, -105.0, 37.0, 500.0)
    out = geod_direct(40.0, -105.0, 37.0, 500.01)
    vecs.append({"input": {"lat": 40.0, "lon": -105.0, "waypoints": [{"lat": on[0], "lon": on[1]}, {"lat": out[0], "lon": out[1]}], "visual_range": "500 m"},
                 "expect": {"result.beyond_count": 1.0, "result.beyond.0.waypoint": 2.0, "result.waypoints.0.beyond": "no", "ok": True},
                 "source": f"{src}: waypoint 1 put on the 500 m ring and waypoint 2 1 cm past it with GeodSolve's direct problem",
                 "sourceVersion": ver, "tolerance": tol(["result.beyond_count", "result.beyond.0.waypoint"], 0, 1e-12)})

    def mission(p, pts, range_m, range_text, extra=None, source=src, version=ver):
        d = geod_inverse([(p[0], p[1], a, b) for a, b in pts])
        far = max(range(len(d)), key=lambda i: d[i])
        beyond = [i for i, x in enumerate(d) if x > range_m + 1e-6]
        exp = {"result.farthest_distance.value": d[far], "result.farthest_waypoint": float(far + 1),
               "result.beyond_count": float(len(beyond)), "result.visual_range_used.value": range_m, "ok": True}
        for k, i in enumerate(beyond):
            exp[f"result.beyond.{k}.waypoint"] = float(i + 1)
        for i, x in enumerate(d):
            exp[f"result.waypoints.{i}.distance.value"] = x
        if beyond:
            exp["meta.warnings.*.code"] = "BEYOND_VISUAL_RANGE"
        inp = {"lat": p[0], "lon": p[1], "waypoints": wp_json(pts)}
        if range_text:
            inp["visual_range"] = range_text
        if extra:
            inp.update(extra)
        vecs.append({"input": inp, "expect": exp, "source": source, "sourceVersion": version,
                     "tolerance": {**tol([k for k in exp if "distance" in k or "range_used" in k], 1e-9, 1e-6),
                                   **tol([k for k in exp if isinstance(exp[k], float) and "distance" not in k and "range_used" not in k], 0, 1e-12)}})

    extra_missions = [
        ((0.0, 0.0), [(0.0, 0.0)], 1.0, "1 m"),
        ((0.0, 0.0), [(0.0, 0.005), (0.005, 0.0)], 1000.0, "1 km"),
        ((89.9, 45.0), [(89.901, 45.0), (89.9, 46.0)], 250.0, "250 m"),
        ((-20.0, 179.999), [(-20.0, -179.999), (-19.995, 179.999)], 1000.0, "1 km"),
        ((35.0, -100.0), [(35.001, -100.0), (35.002, -100.0)], 304.8, "1000 ft"),
        ((51.5, -0.12), [(51.51, -0.12), (51.5, -0.10)], 1609.344, "1 mi"),
        ((-33.9, 151.2), [(-33.9, 151.201), (-33.9, 151.202)], 200.0, "0.2 km"),
        ((40.0, -105.0), [(40.0001, -105.0), (40.05, -105.0)], 50_000.0, "50 km"),
        ((10.0, 20.0), [(10.001, 20.001), (9.999, 19.999), (10.002, 20.0)], 500.0, "500 m"),
    ]
    for p, pts, range_m, range_text in extra_missions:
        mission(p, pts, range_m, range_text)

    lba = "Luftfahrt-Bundesamt VLOS guidance section 7.1, composed with GeographicLib GeodSolve mission distances"
    dimension_cases = [
        ("1 m", "multirotor", None, 347.0),
        ("1 m", "fixed-wing", None, 520.0),
        ("2 m", "multirotor", "1 km", 300.0),
        ("2 m", "fixed-wing", "5 km", 1010.0),
    ]
    pts = [(40.001, -105.0), (40.005, -105.0)]
    for cd, kind, visibility, range_m in dimension_cases:
        extra = {"characteristic_dimension": cd, "aircraft_type": kind}
        if visibility:
            extra["ground_visibility"] = visibility
        mission(pilot, pts, range_m, None, extra, lba, "LBA guidance (English), retrieved 2026-09-19")

    failures = [
        ({"lat": 40.0, "lon": -105.0, "waypoints": [{"lat": 40.001, "lon": -105.0}]}, "INVALID_INPUT", "/visual_range"),
        ({"lat": 40.0, "lon": -105.0, "waypoints": [{"lat": 40.001, "lon": -105.0}], "visual_range": "500 m", "characteristic_dimension": "1 m"}, "INVALID_INPUT", "/characteristic_dimension"),
        ({"lat": 91.0, "lon": 0.0, "waypoints": [{"lat": 0.0, "lon": 0.0}], "visual_range": "500 m"}, "OUT_OF_DOMAIN", "/lat"),
        ({"lat": 0.0, "lon": 0.0, "waypoints": [{"lat": -91.0, "lon": 0.0}], "visual_range": "500 m"}, "OUT_OF_DOMAIN", "/waypoints/0/lat"),
        ({"lat": 0.0, "lon": 0.0, "waypoints": [{"lat": 0.0, "lon": 0.0}], "visual_range": "0 m"}, "OUT_OF_DOMAIN", "/visual_range"),
        ({"lat": 0.0, "lon": 0.0, "waypoints": [{"lat": 0.0, "lon": 0.0}], "visual_range": "50001 m"}, "OUT_OF_DOMAIN", "/visual_range"),
        ({"lat": 0.0, "lon": 0.0, "waypoints": [{"lat": 0.0, "lon": 0.0}], "characteristic_dimension": "0 m"}, "INVALID_INPUT", "/characteristic_dimension"),
    ]
    for inp, code, field in failures:
        vecs.append({"input": inp, "expect": {"ok": False, "error.code": code, "error.field": field},
                     "source": src, "sourceVersion": ver, "tolerance": {}})
    return vecs


# ------------------------------------------------------------------ GCP plan

def table_c1(km2):
    """ASPRS Edition 2, Table C.1, typed from the standard's rows."""
    rows = [(500, 30), (750, 35), (1000, 40), (1250, 45), (1500, 50), (1750, 55), (2000, 60), (2250, 65), (2500, 70)]
    a = round(km2)
    for top, n in rows:
        if a <= top:
            return n
    return None


def square(lat0, lon0, side_m):
    """A square about side_m on a side, centered at (lat0, lon0)."""
    dlat = side_m / 111_000 / 2
    dlon = side_m / (111_000 * math.cos(math.radians(lat0))) / 2
    return [(round(a, 7), round(b, 7)) for a, b in [(lat0 - dlat, lon0 - dlon), (lat0 - dlat, lon0 + dlon), (lat0 + dlat, lon0 + dlon), (lat0 + dlat, lon0 - dlon)]]


def gcp_vectors():
    src = "ASPRS Positional Accuracy Standards, Edition 2, Annex C Table C.1 rows and section 7.9, with areas by GeographicLib Planimeter (tools/vectors/gen_mission_planning.py)"
    ver = "Edition 2, Version 1.0 (February 2023)"
    vecs = []
    for side_km, veg, note in [(0.4, 0, "a 16 ha field"), (20, 0, "400 km2"), (24.5, 0, "about 600 km2"), (33.2, 0, "about 1,100 km2"), (49, 0, "about 2,400 km2"),
                               (38.6, 100 / 3, "the C.3 worked case: about 1,490 km2, a third vegetated, 40 + 30 = 70")]:
        ring = square(35.0, -100.0, side_km * 1000)
        a = planimeter(ring) / 1e6
        open_n, veg_n = table_c1(a * (1 - veg / 100)), (table_c1(a * veg / 100) if veg else None)
        # Keep each case well inside its table row, clear of the plane-versus-ellipsoid difference.
        for part in [a * (1 - veg / 100)] + ([a * veg / 100] if veg else []):
            assert part <= 500 - 1 or min(abs(part - t) for t in range(500, 2501, 250)) > 2, (note, part)
        exp = {"result.checkpoints": float(open_n + (veg_n or 0)), "result.checkpoints_open": float(open_n), "ok": True}
        if veg_n:
            exp["result.checkpoints_vegetated"] = float(veg_n)
        exp["result.area_size.value"] = a
        inp = {"area": [{"lat": round(x, 7), "lon": round(y, 7)} for x, y in ring], "horizontal_class": "10 cm"}
        if veg:
            inp["vegetated"] = round(veg, 4)
        exp["result.gcp_rmse_h.value"] = 5.0
        exp["result.gcp_rmse_v.value"] = 10.0
        vecs.append({"input": inp, "expect": exp, "source": f"{src}: {note}", "sourceVersion": ver,
                     "tolerance": {**tol([k for k in exp if isinstance(exp[k], float) and "area" not in k], 0, 1e-9), "result.area_size.value": {"rel": 1e-6, "abs": 1e-9}}})
    # Beyond the table's last row.
    ring = square(35.0, -100.0, 52_000)
    vecs.append({"input": {"area": [{"lat": round(x, 7), "lon": round(y, 7)} for x, y in ring]}, "expect": {"ok": False, "error.code": "OUT_OF_DOMAIN"},
                 "source": f"{src}: Table C.1 ends at 2,500 km2", "sourceVersion": ver})

    # Cover every table band, both accuracy-class branches, and split land cover.
    more = [
        (32.0, -110.0, 5, 0, 5, 10, "a 25 km2 site with horizontal and vertical classes"),
        (45.0, -93.0, 10, 0, None, 12, "a 100 km2 elevation-only site"),
        (-33.0, 18.0, 15, 0, 7.5, None, "a 225 km2 planimetric site"),
        (52.0, 0.0, 26, 0, 10, 20, "about 675 km2"),
        (10.0, 120.0, 30, 0, 8, 15, "about 900 km2"),
        (-20.0, 130.0, 34, 0, 12, 24, "about 1,150 km2"),
        (60.0, 10.0, 37, 0, 15, 30, "about 1,370 km2"),
        (0.0, -75.0, 40, 0, 20, 40, "about 1,600 km2"),
        (25.0, 55.0, 43, 0, 25, 50, "about 1,850 km2"),
        (-35.0, 149.0, 46, 0, 30, 60, "about 2,120 km2"),
        (42.0, -72.0, 48, 0, 40, 80, "about 2,300 km2"),
        (35.0, -100.0, 30, 30, 10, 20, "a 30 percent vegetated site"),
        (35.0, -100.0, 40, 50, 10, 20, "a site split evenly between open ground and vegetation"),
    ]
    for lat, lon, side_km, veg, h_cm, v_cm, note in more:
        ring = square(lat, lon, side_km * 1000)
        a = planimeter(ring) / 1e6
        open_n = table_c1(a * (1 - veg / 100))
        veg_n = table_c1(a * veg / 100) if veg else None
        assert open_n is not None and (veg_n is not None if veg else True)
        inp = {"area": [{"lat": x, "lon": y} for x, y in ring]}
        if veg:
            inp["vegetated"] = veg
        if h_cm is not None:
            inp["horizontal_class"] = f"{h_cm} cm"
        if v_cm is not None:
            inp["vertical_class"] = f"{v_cm} cm"
        exp = {
            "result.checkpoints": float(open_n + (veg_n or 0)),
            "result.checkpoints_open": float(open_n),
            "result.area_size.value": a,
            "ok": True,
        }
        if veg_n is not None:
            exp["result.checkpoints_vegetated"] = float(veg_n)
        if h_cm is not None:
            exp["result.gcp_rmse_h.value"] = h_cm / 2
            exp["result.checkpoint_rmse_h.value"] = h_cm / 2
            exp["result.gcp_rmse_v.value"] = v_cm / 2 if v_cm is not None else h_cm
        elif v_cm is not None:
            exp["result.gcp_rmse_v.value"] = v_cm / 2
        if v_cm is not None:
            exp["result.checkpoint_rmse_v.value"] = v_cm / 2
        vecs.append({
            "input": inp,
            "expect": exp,
            "source": f"{src}: {note}",
            "sourceVersion": ver,
            "tolerance": {
                **tol([k for k in exp if isinstance(exp[k], float) and "area" not in k], 0, 1e-9),
                "result.area_size.value": {"rel": 1e-6, "abs": 1e-9},
            },
        })
    return vecs


def write(tool, vecs):
    path = ROOT / "core/vectors" / f"{tool}.jsonl"
    have = path.read_text().splitlines() if path.exists() else []
    ids = {json.loads(line)["id"] for line in have}
    seen = {json.dumps(json.loads(line)["input"], sort_keys=True) for line in have}
    n = len(have)
    for v in vecs:
        if json.dumps(v["input"], sort_keys=True) in seen:
            continue
        n += 1
        vid = f"v{n:03d}"
        assert vid not in ids
        have.append(json.dumps({"id": vid, **v}, ensure_ascii=False, separators=(",", ":")))
    path.write_text("\n".join(have) + "\n")


if __name__ == "__main__":
    write("drone.mission.sorties", sortie_vectors())
    write("drone.ops.wind-limit", wind_vectors())
    write("drone.ops.vlos-check", vlos_vectors())
    write("drone.photogrammetry.gcp-plan", gcp_vectors())
