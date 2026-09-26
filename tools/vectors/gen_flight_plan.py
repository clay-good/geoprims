#!/usr/bin/env python3
"""Golden vectors for the flight-planning tools: the navigation log, the climb
plan, and the equal time point and point of no return.

The published examples are typed in from their sources with the tolerance of
their printed rounding:

- nav log: FAA-H-8083-25C (PHAK) chapter 16, Figure 16-26 (Chickasha to
  Guthrie, TC 031, TAS 115 kt, wind 360 at 10 kt, 7 E, deviation +2 applied as
  a correction, four checkpoints);
- climb plan: PHAK chapter 11, Figure 11-25 (the chart read at 6,000 ft and at
  10,000 ft and subtracted);
- ETP: CASA AC 91-15 v1.2 Annex B, Table 9 (906 NM, GS 250 kt on and 290 kt
  back, critical point 487 NM from Darwin).

Everything else is computed here a different way from the core: the wind
triangle by the law of cosines, GS = sqrt(TAS^2 - (W sin d)^2) - W cos d, not
the core's TAS cos(WCA) - W cos d; courses and distances by GeographicLib's
C++ GeodSolve. New files, written whole; published vectors are frozen, so run
this once and append by hand afterward.

    python3 tools/vectors/gen_flight_plan.py
"""
import json
import math
import subprocess
import sys
from pathlib import Path

VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
SRC = "Independent Python solution: wind triangle by the law of cosines, legs by GeographicLib GeodSolve (tools/vectors/gen_flight_plan.py)"
NM = 1852.0
PHAK = "FAA-H-8083-25C"


def inverse(la1, lo1, la2, lo2):
    line = f"{la1:.15f} {lo1:.15f} {la2:.15f} {lo2:.15f}"
    o = subprocess.run(["GeodSolve", "-i", "-p", "12"], input=line + "\n", capture_output=True, text=True, check=True).stdout.split()
    return float(o[0]) % 360.0, float(o[2]) / NM  # initial azimuth, NM


def triangle(tc, tas, wd, ws):
    """(WCA, TH, GS) by the law of cosines; None when the crosswind wins."""
    d = math.radians(wd - tc)
    cross = ws * math.sin(d)
    if abs(cross) > tas:
        return None
    wca = math.degrees(math.asin(cross / tas))
    gs = math.sqrt(tas * tas - cross * cross) - ws * math.cos(d)
    return wca, (tc + wca) % 360.0, gs


def vec(i, inp, exp, src=SRC, ver=VER, tol=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {}
    for k, v in e.items():
        if isinstance(v, (int, float)) and not isinstance(v, bool):
            t[k] = {"rel": 0, "abs": tol.get(k, 1e-9) if isinstance(tol, dict) else tol}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


# ---------------------------------------------------------------- nav log

def log_expect(legs, tas, wind, var=None, burn=None):
    """Expected per-leg values for [(tc, nm)] legs in one wind."""
    e, cum_t, cum_d = {}, 0.0, 0.0
    for i, (tc, nm) in enumerate(legs):
        wca, th, gs = triangle(tc, tas, *wind)
        t = nm / gs * 60.0
        cum_t += t
        cum_d += nm
        p = f"result.legs.{i}."
        e[p + "true_course.value"] = tc
        e[p + "distance.value"] = nm
        e[p + "wind_correction_angle.value"] = wca
        e[p + "true_heading.value"] = th
        e[p + "groundspeed.value"] = gs
        e[p + "time.value"] = t
        if var is not None:
            e[p + "magnetic_heading.value"] = (th - var) % 360.0
        if burn is not None:
            e[p + "fuel.value"] = t / 60.0 * burn
        e[p + "cumulative_distance.value"] = cum_d
    e["result.total_time.value"] = cum_t
    e["result.total_distance.value"] = cum_d
    return e


def nav_log():
    out = []
    # PHAK Figure 16-26: the checkpoint legs of the Chickasha-Guthrie flight.
    dists = [11, 10, 10.5, 13, 8.5]
    phak_in = {
        "legs": [{"course": "031 deg", "distance": f"{d} NM"} for d in dists],
        "tas": "115 kt", "wind_direction": "360 deg", "wind_speed": "10 kt",
        "variation": "7 deg", "deviation_card": [{"heading": "000 deg", "deviation": "-2 deg"}],
        "fuel_burn": "8 gal/h",
    }
    exp, tol = {}, {}
    for i, (t, cum) in enumerate(zip([6, 6, 6, 7, 5], [11, 21, 31.5, 44.5, 53])):
        p = f"result.legs.{i}."
        exp.update({p + "wind_correction_angle.value": -3, p + "true_heading.value": 28, p + "magnetic_heading.value": 21,
                    p + "compass_heading.value": 23, p + "groundspeed.value": 106, p + "time.value": t, p + "cumulative_distance.value": cum})
        tol.update({p + "wind_correction_angle.value": 0.5, p + "true_heading.value": 0.5, p + "magnetic_heading.value": 0.5,
                    p + "compass_heading.value": 0.5, p + "groundspeed.value": 0.5, p + "time.value": 0.5, p + "cumulative_distance.value": 1e-9})
    exp.update({"result.total_time.value": 30, "result.total_distance.value": 53})
    tol.update({"result.total_time.value": 0.5, "result.total_distance.value": 1e-9})
    out.append(vec(1, phak_in, exp,
                   src=f"{PHAK} chapter 16, Figure 16-26: WCA 3° L, TH 28°, MH 21°, CH 23° (7° E variation, +2° deviation added), GS 106 kt, checkpoint times 6, 6, 6, 7, 5 min, 30 min in 53 NM",
                   ver=PHAK, tol=tol))
    # The spec scenario: TC 090, TAS 120, wind 030 at 20.
    out.append(vec(2, {"legs": [{"course": "90 deg", "distance": "100 NM"}], "tas": "120 kt", "wind_direction": "30 deg", "wind_speed": "20 kt"},
                   log_expect([(90.0, 100.0)], 120.0, (30.0, 20.0))))
    # Waypoints: Chickasha, Wiley Post, Guthrie (the primary example).
    wps = [("KCHK", 35.0967, -97.9678), ("KPWA", 35.5342, -97.6471), ("KGOK", 35.8498, -97.4156)]
    legs = [inverse(a[1], a[2], b[1], b[2]) for a, b in zip(wps, wps[1:])]
    e = log_expect(legs, 115.0, (0.0, 10.0), var=7.0, burn=8.0)
    e["result.total_fuel.value"] = e["result.total_time.value"] / 60.0 * 8.0
    out.append(vec(3, {"waypoints": [{"name": n, "lat": la, "lon": lo} for n, la, lo in wps], "tas": "115 kt",
                       "wind_direction": "360 deg", "wind_speed": "10 kt", "variation": "7 deg", "fuel_burn": "8 gal/h"}, e))
    # Across the antimeridian: the short way, eastbound.
    legs = [inverse(50.0, 170.0, 50.0, -170.0)]
    out.append(vec(4, {"waypoints": [{"lat": 50, "lon": 170}, {"lat": 50, "lon": -170}], "tas": "150 kt", "wind_direction": "270 deg", "wind_speed": "40 kt"},
                   log_expect(legs, 150.0, (270.0, 40.0))))
    # A wind stronger than the airspeed: the leg is reported, the totals are not.
    out.append(vec(5, {"legs": [{"course": "90 deg", "distance": "50 NM"}], "tas": "30 kt", "wind_direction": "0 deg", "wind_speed": "40 kt"},
                   {"result.total_distance.value": 50.0, "result.legs.0.cumulative_distance.value": 50.0, "meta.warnings.*.code": "WIND_EXCEEDS_TAS"}))
    # Each leg's own wind wins over the route's.
    legs = [(45.0, 40.0), (135.0, 25.0)]
    e = {}
    cum = 0.0
    for i, ((tc, nm), w) in enumerate(zip(legs, [(0.0, 15.0), (200.0, 25.0)])):
        wca, th, gs = triangle(tc, 100.0, *w)
        cum += nm / gs * 60.0
        e[f"result.legs.{i}.true_heading.value"] = th
        e[f"result.legs.{i}.groundspeed.value"] = gs
    e["result.total_time.value"] = cum
    out.append(vec(6, {"legs": [{"course": "45 deg", "distance": "40 NM"}, {"course": "135 deg", "distance": "25 NM", "wind_direction": "200 deg", "wind_speed": "25 kt"}],
                       "tas": "100 kt", "wind_direction": "0 deg", "wind_speed": "15 kt"}, e))
    out.append(vec(7, {"waypoints": [{"lat": 35, "lon": -98}, {"lat": 36, "lon": -97}], "legs": [{"course": "90 deg", "distance": "10 NM"}], "tas": "100 kt"},
                   {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(8, {"legs": [{"course": "90 deg", "distance": "10 NM"}], "tas": "100 kt", "date": "2026-09-25"},
                   {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


# ---------------------------------------------------------------- climb plan

def climb_plan():
    out = []
    out.append(vec(1, {"field_elevation": "6000 ft", "cruise_altitude": "10000 ft",
                       "poh_table": [{"time": "6 min", "fuel": "3.5 gal", "distance": "9 NM"}, {"time": "10.5 min", "fuel": "6 gal", "distance": "15 NM"}]},
                   {"result.fuel.value": 2.5, "result.top_of_climb.value": 6.0, "result.time.value": 4.5, "result.height.value": 4000.0},
                   src=f"{PHAK} chapter 11, Figure 11-25: 3.5 gal, 6 min, 9 NM at 6,000 ft and 6 gal, 10.5 min, 15 NM at 10,000 ft, subtracted: 2.5 gal and 6 NM (the text prints 4 minutes; its own readings subtract to 4.5)",
                   ver=PHAK))

    def rate(field, cruise, fpm, tas, burn=None, course=None, wind=None, taxi=None):
        inp = {"field_elevation": f"{field} ft", "cruise_altitude": f"{cruise} ft", "climb_rate": f"{fpm} fpm", "climb_tas": f"{tas} kt"}
        gs = float(tas)
        if course is not None:
            inp["course"] = f"{course} deg"
        if wind is not None:
            inp["wind_direction"], inp["wind_speed"] = f"{wind[0]} deg", f"{wind[1]} kt"
            gs = triangle(float(course), float(tas), float(wind[0]), float(wind[1]))[2]
        t = (cruise - field) / fpm
        e = {"result.time.value": t, "result.top_of_climb.value": gs * t / 60.0, "result.groundspeed.value": gs, "result.height.value": float(cruise - field)}
        if burn is not None:
            inp["climb_burn"] = f"{burn} gal/h"
            e["result.fuel.value"] = burn * t / 60.0
        if taxi is not None:
            inp["taxi_fuel"] = f"{taxi} gal"
            e["result.total_fuel.value"] = taxi + (burn * t / 60.0 if burn else 0.0)
        return inp, e

    for inp, e in [rate(1085, 5500, 500, 90, burn=11, course=31, wind=(360, 10)), rate(0, 3000, 600, 80),
                   rate(500, 4500, 700, 85, burn=9, course=360, wind=(360, 20), taxi=1.1)]:
        out.append(vec(len(out) + 1, inp, e))
    out.append(vec(len(out) + 1, {"field_elevation": "5500 ft", "cruise_altitude": "5500 ft", "climb_rate": "500 fpm", "climb_tas": "90 kt", "climb_burn": "11 gal/h"},
                   {"result.time.value": 0.0, "result.top_of_climb.value": 0.0, "result.fuel.value": 0.0,
                    "result.note": "No climb is needed: the field is at the cruise altitude."},
                   src="add-flight-and-drone-planning-tools climb scenario: field at cruise altitude gives zero time, fuel, and distance with a note"))
    out.append(vec(len(out) + 1, {"field_elevation": "6000 ft", "cruise_altitude": "4000 ft", "climb_rate": "500 fpm", "climb_tas": "90 kt"},
                   {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(len(out) + 1, {"field_elevation": "0 ft", "cruise_altitude": "4000 ft", "climb_rate": "500 fpm",
                                  "poh_table": [{"time": "0 min", "fuel": "0 gal", "distance": "0 NM"}, {"time": "7 min", "fuel": "2 gal", "distance": "13 NM"}]},
                   {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


# ---------------------------------------------------------------- ETP and PNR

def etp_pnr():
    out = []
    casa = "CASA AC 91-15 v1.2 Annex B, Table 9: route 906 NM, GS 250 kt on to Cairns and 290 kt back to Darwin, critical point 487 NM from Darwin and 419 NM from Cairns"
    out.append(vec(1, {"distance": "906 NM", "groundspeed_out": "250 kt", "groundspeed_back": "290 kt"},
                   {"result.etp_distance.value": 487, "result.etp_to_destination.value": 419},
                   src=casa, ver="AC 91-15 v1.2", tol=0.5))

    def solve(d, go, gb, e_h=None):
        share = gb / (go + gb)
        x = {"result.etp_distance.value": d * share, "result.etp_to_destination.value": d - d * share,
             "result.etp_time.value": d * share / go * 60.0, "result.groundspeed_out.value": go, "result.groundspeed_back.value": gb}
        if e_h is not None:
            x.update({"result.safe_endurance.value": e_h * 60.0, "result.pnr_time.value": e_h * share * 60.0, "result.pnr_distance.value": e_h * share * go})
        return x

    tri = lambda c, tas, w: triangle(c, tas, *w)[2]
    out.append(vec(2, {"distance": "906 NM", "tas": "270 kt", "course": "090 deg", "wind_direction": "090 deg", "wind_speed": "20 kt", "safe_endurance": "4 h"},
                   solve(906.0, tri(90.0, 270.0, (90.0, 20.0)), tri(270.0, 270.0, (90.0, 20.0)), 4.0)))
    out.append(vec(3, {"distance": "300 NM", "tas": "150 kt"}, solve(300.0, 150.0, 150.0), tol=0))
    out.append(vec(4, {"distance": "200 NM", "tas": "120 kt", "course": "360 deg", "wind_direction": "360 deg", "wind_speed": "20 kt",
                       "usable_fuel": "50 gal", "reserve_fuel": "10 gal", "fuel_burn": "10 gal/h"},
                   solve(200.0, tri(0.0, 120.0, (0.0, 20.0)), tri(180.0, 120.0, (0.0, 20.0)), 4.0)))
    out.append(vec(5, {"distance": "150 NM", "tas": "120 kt", "course": "090 deg", "wind_direction": "000 deg", "wind_speed": "30 kt"},
                   solve(150.0, tri(90.0, 120.0, (0.0, 30.0)), tri(270.0, 120.0, (0.0, 30.0)))))
    out.append(vec(6, {"distance": "100 NM", "tas": "40 kt", "course": "090 deg", "wind_direction": "090 deg", "wind_speed": "45 kt"},
                   {"ok": False, "error.code": "NO_SOLUTION"}))
    out.append(vec(7, {"distance": "100 NM", "groundspeed_out": "100 kt"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parents[2] / "core/vectors")
    for tool, cases in [("aviation.flight-plan.nav-log", nav_log()), ("aviation.performance.climb-plan", climb_plan()),
                        ("aviation.performance.etp-pnr", etp_pnr())]:
        (dest / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in cases))


if __name__ == "__main__":
    main()
