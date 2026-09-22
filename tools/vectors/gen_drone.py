#!/usr/bin/env python3
"""Golden vectors for drone.photogrammetry.* by direct evaluation of the
published formulas (Wolf, Dewitt & Wilkinson 2014, ch. 6; ASPRS Edition 2),
plus the add-drone-suite spec scenarios."""
import json
import math
import random
import sys
from pathlib import Path

SRC = "Photogrammetric formulas (Wolf, Dewitt & Wilkinson 2014, ch. 6) evaluated in Python (tools/vectors/gen_drone.py)"
SPEC = "add-drone-suite scenarios"
CAMS = [  # sensor w, h (mm), focal (mm), image w, h (px)
    (13.2, 8.8, 8.8, 5472, 3648),    # 1-inch 20 MP
    (17.3, 13.0, 12.29, 5280, 3956),  # Four Thirds 20 MP
    (6.17, 4.55, 4.5, 4000, 3000),   # 1/2.3-inch 12 MP
    (35.9, 24.0, 35.0, 8192, 5460),  # full frame 45 MP
    (23.5, 15.6, 16.0, 6000, 4000),  # APS-C 24 MP
]


# More camera and height pairs for the stable bar (20+ vectors per tool).
MORE = [(CAMS[0], 40), (CAMS[0], 120), (CAMS[1], 30), (CAMS[1], 150), (CAMS[2], 20), (CAMS[2], 121.92), (CAMS[3], 500),
        (CAMS[3], 60), (CAMS[4], 250), (CAMS[4], 15), ((7.6, 5.7, 6.72, 4000, 3000), 90), ((13.2, 8.8, 10.26, 5472, 3648), 75),
        ((36.0, 24.0, 21.0, 8256, 5504), 200), ((23.5, 15.6, 25.0, 6000, 4000), 110), ((6.4, 4.8, 4.3, 8000, 6000), 60)]
# Alshaibani et al. (2021), arXiv:2108.12811, equation 1: a 1-inch 20 MP camera (12.75 × 8.5 mm, 10.6 mm, 4608 × 3456 px).
PAPER = (12.75, 8.5, 10.6, 4608, None)  # the paper gives no image height
PAPER_SRC = "Alshaibani et al., Airplane Type Identification Based on Mask RCNN and Drone Images, arXiv:2108.12811, equation 1"
PAPER_VER = "arXiv v1 (2021)"


def vec(i, inp, exp, src=SRC, ver="4th edition (2014)"):
    e = dict(exp)
    e.setdefault("ok", True)
    tol = {k: {"rel": 1e-12, "abs": 1e-12} for k, v in e.items() if isinstance(v, float)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def cam_inp(c):
    out = {"sensor_width": f"{c[0]} mm", "sensor_height": f"{c[1]} mm", "focal_length": f"{c[2]} mm", "image_width": c[3], "image_height": c[4]}
    return {k: v for k, v in out.items() if v is not None}


def gsd():
    out = []
    for i, (c, h) in enumerate(zip(CAMS, [100, 80, 50, 300, 120]), 1):
        g = c[0] * h / (c[2] * c[3]) * 100  # cm
        out.append(vec(i, dict(cam_inp(c), height=f"{h} m"),
                       {"result.gsd.value": g, "result.footprint_across.value": c[0] * h / c[2], "result.gsd_along.value": c[1] * h / (c[2] * c[4]) * 100}))
    out.append(vec(6, dict(cam_inp((13.2, 8.8, 24.0, 5472, 3648)), height="100 m"), {"meta.warnings.0.code": "EQUIVALENT_FOCAL_LENGTH"}, SPEC, "2026"))
    for c, h in MORE:
        g = c[0] * h / (c[2] * c[3]) * 100
        out.append(vec(len(out) + 1, dict(cam_inp(c), height=f"{h} m"),
                       {"result.gsd.value": g, "result.footprint_across.value": c[0] * h / c[2], "result.gsd_along.value": c[1] * h / (c[2] * c[4]) * 100}))
    out.append(vec(len(out) + 1, dict(cam_inp(PAPER), height="120 m"), {"result.gsd.value": 3.13}, PAPER_SRC, PAPER_VER))
    out[-1]["tolerance"] = {"result.gsd.value": {"abs": 0.005}}
    return out


def alt():
    out = []
    for i, (c, g) in enumerate(zip(CAMS, [2.0, 1.5, 3.0, 0.8, 2.5]), 1):
        inp = cam_inp(c)
        inp["target_gsd"] = f"{g} cm"
        out.append(vec(i, inp, {"result.height.value": g / 100 * c[2] * c[3] / c[0]}))
    for c, h in MORE:
        g = round(c[0] * h / (c[2] * c[3]) * 100, 2)
        inp = cam_inp(c)
        inp["target_gsd"] = f"{g} cm"
        out.append(vec(len(out) + 1, inp, {"result.height.value": g / 100 * c[2] * c[3] / c[0], "result.footprint_across.value": g / 100 * c[3]}))
    # The published example inverted: 3.13 cm/px (rounded from 3.1324) needs about 120 m.
    inp = cam_inp(PAPER)
    inp["target_gsd"] = "3.13 cm"
    out.append(vec(len(out) + 1, inp, {"result.height.value": 120.0}, PAPER_SRC, PAPER_VER))
    out[-1]["tolerance"] = {"result.height.value": {"abs": 0.2}}
    return out


def trigger():
    out = []
    cases = [(CAMS[0], 100, 10, 75, 65), (CAMS[1], 80, 8, 80, 70), (CAMS[2], 60, 5, 85, 70), (CAMS[3], 200, 15, 70, 60), (CAMS[4], 120, 12, 75, 60)]
    for i, (c, h, v, fo, so) in enumerate(cases, 1):
        across, along = c[0] * h / c[2], c[1] * h / c[2]
        d = along * (1 - fo / 100)
        inp = dict(cam_inp(c), height=f"{h} m", groundspeed=f"{v} m/s", front_overlap=fo, side_overlap=so)
        del inp["image_height"]
        out.append(vec(i, inp, {"result.trigger_distance.value": d, "result.trigger_interval.value": d / v, "result.line_spacing.value": across * (1 - so / 100)}))
    return out


def blur():
    cases = [(10, 0.001, 2.741), (15, 1 / 2000, 1.0), (5, 1 / 500, 3.0), (20, 1 / 4000, 0.8), (8, 1 / 800, 2.0)]
    out = []
    for i, (v, t, g) in enumerate(cases, 1):
        out.append(vec(i, {"groundspeed": f"{v} m/s", "exposure": f"{t} s", "gsd": f"{g} cm"},
                       {"result.blur": v * t / (g / 100), "result.max_exposure.value": 0.5 * (g / 100) / v}))
    return out


def asprs():
    cases = [((None, None, 1.0), 2.0, 30), ((1.0, 1.0, None), 1.0, 40), ((2.0, 1.5, 3.0), 1.2, 35), ((0.5, 0.5, 0.5), 0.3, 30), ((None, None, 5.0), 0.0, 100)]
    out = []
    for i, ((x, y, z), cp, n) in enumerate(cases, 1):
        inp = {"checkpoint_rmse": f"{cp} cm", "checkpoints": n}
        exp = {}
        if x is not None:
            inp["rmse_x"], inp["rmse_y"] = f"{x} cm", f"{y} cm"
            exp["result.horizontal.value"] = math.hypot(math.sqrt(x * x + cp * cp), math.sqrt(y * y + cp * cp))
        if z is not None:
            inp["rmse_z"] = f"{z} cm"
            exp["result.vertical.value"] = math.sqrt(z * z + cp * cp)
        out.append(vec(i, inp, exp, "ASPRS Positional Accuracy Standards, Edition 2 (quadrature of fit and checkpoint RMSE)", "Edition 2, Version 2.0"))
    out.append(vec(6, {"rmse_z": "1 cm", "checkpoint_rmse": "1 cm", "checkpoints": 20}, {"meta.warnings.1.code": "INSUFFICIENT_CHECKPOINTS"}, SPEC, "2026"))
    return out


# ---------------------------------------------------------------- slice 2

PW_SRC = "Momentum theory (Leishman 2006, ch. 2) and battery arithmetic evaluated in Python (tools/vectors/gen_drone.py)"
PW_VER = "2nd edition (2006)"
OPS_SRC = "data/regulations.json values (14 CFR 107.51, Regulation (EU) 2019/945 and 2019/947, EASA VLOS guidance) applied in Python (tools/vectors/gen_drone.py)"
OPS_VER = "Rules as of 2026-09-18"
G0, R_AIR, T0, P0 = 9.80665, 287.05287, 288.15, 101325.0


def fvec(i, inp, exp, src, ver, rel=1e-9):
    e = dict(exp)
    e.setdefault("ok", True)
    tol = {k: {"rel": rel, "abs": 1e-9} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def rho_at(h_m, t_k=None):
    t = T0 - 0.0065 * h_m
    p = P0 * (t / T0) ** (G0 / (R_AIR * 0.0065))
    return p / (R_AIR * (t_k if t_k else t))


def battery_vectors():
    cases = [(5870, 15.4), (5000, 14.8), (2200, 11.1), (10000, 22.2), (3850, 15.4), (1500, 7.4)]
    out = [fvec(i, {"capacity": f"{c} mAh", "voltage": f"{v} V"}, {"result.energy.value": c / 1000 * v}, PW_SRC, PW_VER)
           for i, (c, v) in enumerate(cases, 1)]
    for c, v in [(4500, 11.55), (7000, 44.4), (22000, 22.2), (1300, 14.8), (16000, 51.8)]:
        out.append(fvec(len(out) + 1, {"capacity": f"{c} mAh", "voltage": f"{v} V"}, {"result.energy.value": c / 1000 * v}, PW_SRC, PW_VER))
    # Nominal cell voltages: LiPo 3.7 V, Li-ion 3.6 V.
    for c, n, chem, per in [(5870, 4, "lipo", 3.7), (3000, 6, "li-ion", 3.6), (10000, 12, "lipo", 3.7)]:
        out.append(fvec(len(out) + 1, {"capacity": f"{c} mAh", "cells": n, "chemistry": chem},
                        {"result.voltage.value": n * per, "result.energy.value": c / 1000 * n * per}, PW_SRC, PW_VER))
    # Usable energy between the depth-of-discharge limit and the reserve.
    for c, v, dod, res in [(5000, 22.2, 80, 20), (8000, 14.8, 90, 0), (12000, 44.4, 100, 30)]:
        out.append(fvec(len(out) + 1, {"capacity": f"{c} mAh", "voltage": f"{v} V", "depth_of_discharge": dod, "reserve": res},
                        {"result.usable_energy.value": c / 1000 * v * (dod - res) / 100}, PW_SRC, PW_VER))
    # Current and C-rate for a steady draw: I = P / V, C = I / Ah.
    for c, v, pw in [(5000, 22.2, 400), (2200, 11.1, 150)]:
        i = pw / v
        out.append(fvec(len(out) + 1, {"capacity": f"{c} mAh", "voltage": f"{v} V", "power": f"{pw} W"},
                        {"result.current": i, "result.c_rate": i / (c / 1000)}, PW_SRC, PW_VER))
    # FAA PackSafe, Batteries Carried by Airline Passengers, Q3: a 12-volt battery rated to 8 Ah is rated at 96 Wh.
    out.append(fvec(len(out) + 1, {"capacity": "8 Ah", "voltage": "12 V"}, {"result.energy.value": 96.0},
                    "FAA PackSafe, Batteries Carried by Airline Passengers (Q3 worked example)", "December 2024"))
    return out


def hover_vectors():
    out = []
    cases = [(1.4, 4, 9.4, None), (0.9, 4, 7.0, None), (6.5, 6, 22.0, None), (2.5, 4, 13.0, 1500), (25.0, 8, 30.0, 500)]
    for i, (m, n, d_in, alt_ft) in enumerate(cases, 1):
        d = d_in * 0.0254
        a = n * math.pi * d * d / 4
        rho = rho_at((alt_ft or 0) * 0.3048)
        ideal = (m * G0) ** 1.5 / math.sqrt(2 * rho * a)
        inp = {"mass": f"{m} kg", "rotors": n, "rotor_diameter": f"{d_in} in"}
        if alt_ft is not None:
            inp["altitude"] = f"{alt_ft} ft"
        out.append(fvec(i, inp, {"result.ideal_power.value": ideal, "result.electrical_power.value": ideal / 0.51,
                                  "result.disk_area.value": a}, PW_SRC, PW_VER))
    return out


def endurance_vectors():
    cases = [(90.4, 80, 150.6, 0), (100, 100, 100, 20), (77, 90, 220, 15), (274, 85, 900, 25), (45, 100, 60, 0)]
    out = []
    for i, (e, u, p, r) in enumerate(cases, 1):
        t = e * u / 100 * (1 - r / 100) / p * 60
        out.append(fvec(i, {"energy": f"{e} Wh", "usable": u, "power": f"{p} W", "reserve": r}, {"result.hover_time.value": t}, PW_SRC, PW_VER))
    return out


def payload_vectors():
    out = []
    cases = [(1.4, 4, 9.4, 72, 20), (0.9, 4, 7.0, 45, 25), (6.5, 6, 22.0, 400, 25), (2.5, 4, 13.0, 90, 30), (4.0, 4, 15.0, 160, 20)]
    for i, (m, n, d_in, e, t) in enumerate(cases, 1):
        d = d_in * 0.0254
        a = n * math.pi * d * d / 4
        budget = e * 60 / t
        # ISA sea-level density exactly as defined, not 1.225
        mmax = (budget * 0.51 * math.sqrt(2 * (P0 / (R_AIR * T0)) * a)) ** (2 / 3) / G0
        exp = {"result.max_payload.value": mmax - m} if mmax > m else {"ok": False, "error.code": "NO_SOLUTION"}
        out.append(fvec(i, {"mass": f"{m} kg", "rotors": n, "rotor_diameter": f"{d_in} in", "usable_energy": f"{e} Wh", "target_time": f"{t} min"},
                        exp, PW_SRC, PW_VER))
    return out


def rth_vectors():
    out = []
    cases = [(1.5, 15, 10, 180, 40, 10), (2.0, 12, 0, 150, 50, 5), (0.8, 18, -5, 300, 30, 10), (3.0, 20, 8, 250, 80, 15), (1.0, 10, 6, 120, 25, 0)]
    for i, (dk, v, w, p, e, r) in enumerate(cases, 1):
        t = dk * 1000 / (v - w)
        need = p * t / 3600
        out.append(fvec(i, {"distance": f"{dk} km", "airspeed": f"{v} m/s", "headwind": f"{w} m/s", "power": f"{p} W",
                            "remaining_energy": f"{e} Wh", "reserve_energy": f"{r} Wh"},
                        {"result.return_energy.value": need, "result.margin.value": e - r - need}, PW_SRC, PW_VER))
    return out


def altitude_vectors():
    cases = [({}, 400), ({"structure_height": "300 ft", "structure_distance": "200 ft"}, 700), ({"structure_height": "300 ft", "structure_distance": "401 ft"}, 400),
             ({"structure_height": "120 ft", "structure_distance": "400 ft"}, 520), ({"structure_height": "90 m", "structure_distance": "100 m"}, 90 / 0.3048 + 400)]
    return [fvec(i, inp, {"result.max_agl.value": v}, OPS_SRC, OPS_VER) for i, (inp, v) in enumerate(cases, 1)]


def speed_vectors():
    lim = 87 * 1852 / 1609.344
    cases = [(90, 15), (60, 20), (100, 0), (80, -10), (95, 6)]
    return [fvec(i, {"airspeed": f"{a} mph", "tailwind": f"{w} mph"}, {"result.margin.value": lim - (a + w),
                                                                         "result.status": "over the limit" if a + w > lim else "within the limit"}, OPS_SRC, OPS_VER)
            for i, (a, w) in enumerate(cases, 1)]


def ke_vectors():
    cases = [(0.9, 19), (0.249, 16), (2.0, 20), (0.5, 12), (25.0, 23)]
    return [fvec(i, {"mass": f"{m} kg", "speed": f"{v} m/s"}, {"result.energy.value": 0.5 * m * v * v,
                                                                "result.energy_ft_lbf.value": 0.5 * m * v * v / (0.3048 * 4.4482216152605)}, OPS_SRC, OPS_VER)
            for i, (m, v) in enumerate(cases, 1)]


def easa_vectors():
    cases = [("2 kg", "none", "A3"), ("0.2 kg", "none", "A1 and A3"), ("0.8 kg", "c1", "A1 and A3"), ("3.5 kg", "c2", "A2 and A3"), ("12 kg", "c3", "A3"), ("0.24 kg", "c0", "A1 and A3")]
    return [fvec(i, {"mass": m, "class_mark": c}, {"result.available": a}, OPS_SRC, OPS_VER) for i, (m, c, a) in enumerate(cases, 1)]


def vlos_vectors():
    cases = [(0.35, "multirotor", None, 327 * 0.35 + 20), (0.9, "multirotor", 5, 327 * 0.9 + 20), (2.0, "fixed-wing", None, 490 * 2 + 30),
             (1.0, "multirotor", 1, 300.0), (3.0, "fixed-wing", 3, 900.0)]
    out = []
    for i, (cd, kind, gv, want) in enumerate(cases, 1):
        inp = {"characteristic_dimension": f"{cd} m", "aircraft_type": kind}
        if gv:
            inp["ground_visibility"] = f"{gv} km"
        out.append(fvec(i, inp, {"result.vlos.value": want}, OPS_SRC, OPS_VER))
    rnd = random.Random(8)
    for _ in range(6):
        cd, kind, gv = round(rnd.uniform(0.2, 5.0), 2), rnd.choice(["multirotor", "fixed-wing"]), round(rnd.uniform(0.5, 5.0), 1)
        alos = (327 * cd + 20) if kind == "multirotor" else (490 * cd + 30)
        out.append(fvec(len(out) + 1, {"characteristic_dimension": f"{cd} m", "aircraft_type": kind, "ground_visibility": f"{gv} km"},
                        {"result.alos.value": alos, "result.dlos.value": 300 * gv, "result.vlos.value": min(alos, 300 * gv)}, OPS_SRC, OPS_VER))
    # Luftfahrt-Bundesamt, Guidance for Dimensioning of Flight Geography, Contingency Volume and Ground Risk Buffer, section 7.1
    # (ground visibility 5 km or more). Its 3 m rotary-wing row prints 1000 m where the formula gives 1001 m, so it is left out.
    for cd, kind, want in [(1, "multirotor", 347), (1, "fixed-wing", 520), (2, "multirotor", 674), (2, "fixed-wing", 1010),
                           (3, "fixed-wing", 1500), (3.5, "multirotor", 1164.5), (4, "multirotor", 1328), (4.53, "multirotor", 1500)]:
        out.append(fvec(len(out) + 1, {"characteristic_dimension": f"{cd} m", "aircraft_type": kind, "ground_visibility": "5 km"},
                        {"result.vlos.value": float(want)}, LBA, LBA_VER))
    # Checked against a planned farthest point.
    for far, status in [(300, "within"), (400, "beyond")]:
        out.append(fvec(len(out) + 1, {"characteristic_dimension": "1 m", "aircraft_type": "multirotor", "farthest_distance": f"{far} m"},
                        {"result.margin.value": 347.0 - far}, OPS_SRC, OPS_VER))
    # Visibility is taken as at most 5 km (LBA section 7: GVmax = 5 km), so VLOS never exceeds 1,500 m.
    out.append(fvec(len(out) + 1, {"characteristic_dimension": "10 m", "aircraft_type": "multirotor"},
                    {"result.vlos.value": 1500.0, "meta.warnings.0.code": "NOMINAL_VALUE_USED"}, LBA, LBA_VER))
    out.append(fvec(len(out) + 1, {"characteristic_dimension": "10 m", "aircraft_type": "fixed-wing", "ground_visibility": "8 km"},
                    {"result.vlos.value": 1500.0, "meta.warnings.0.code": "INPUT_NORMALIZED"}, LBA, LBA_VER))
    return out


LBA = "Luftfahrt-Bundesamt, Guidance for Dimensioning of Flight Geography, Contingency Volume and Ground Risk Buffer, section 7.1 (maximum VLOS distance table)"
LBA_VER = "LBA guidance (English), retrieved 2026-09-19"


MIS_SRC = "Flight-line arithmetic (Wolf, Dewitt & Wilkinson 2014, ch. 18) evaluated in Python (tools/vectors/gen_drone.py)"


def rect(lat0, lon0, dlat, dlon):
    return [{"lat": lat0, "lon": lon0}, {"lat": lat0, "lon": lon0 + dlon}, {"lat": lat0 + dlat, "lon": lon0 + dlon}, {"lat": lat0 + dlat, "lon": lon0}]


def meters_per_deg_lat(lat):
    # WGS 84 meridional radius of curvature × π/180
    a, f = 6378137.0, 1 / 298.257223563
    e2 = f * (2 - f)
    s = math.sin(math.radians(lat))
    return a * (1 - e2) / (1 - e2 * s * s) ** 1.5 * math.pi / 180


def grid_vectors(tool):
    out = []
    cases = [(40.0, -105.0, 0.00135, 0.00705, 52.5), (35.0, -100.0, 0.009, 0.002, 40.0), (51.0, 0.1, 0.004, 0.012, 60.0),
             (-33.0, 151.0, 0.0022, 0.0022, 25.0), (60.0, 25.0, 0.02, 0.005, 100.0)]
    for i, (la, lo, dla, dlo, sp) in enumerate(cases, 1):
        mid = la + dla / 2
        w_ns = dla * meters_per_deg_lat(mid)
        a, f = 6378137.0, 1 / 298.257223563
        e2 = f * (2 - f)
        n_rad = a / math.sqrt(1 - e2 * math.sin(math.radians(mid)) ** 2)
        w_ew = dlo * math.pi / 180 * n_rad * math.cos(math.radians(mid))
        lines = math.ceil(min(w_ns, w_ew) / sp - 1e-6)
        out.append(vec(i, {"area": rect(la, lo, dla, dlo), "line_spacing": f"{sp} m", "photo_spacing": "30 m"}, {"result.lines": float(lines)}, MIS_SRC))
    return out


def corridor_vectors():
    cases = [(120, 52.5, 3), (100, 50, 2), (300, 60, 5), (30, 40, 1), (250, 45, 6)]
    return [vec(i, {"centerline": [{"lat": 40.0, "lon": -105.0}, {"lat": 40.01, "lon": -104.99}], "width": f"{w} m", "line_spacing": f"{s} m"},
                {"result.line_count": float(n)}, MIS_SRC) for i, (w, s, n) in enumerate(cases, 1)]


def orbit_vectors():
    cases = [(50, 80, 60), (30, 40, 0), (100, 120, 100), (20, 25, 10), (75, 60, 0)]
    return [vec(i, {"lat": 40.0, "lon": -105.0, "radius": f"{r} m", "height": f"{h} m", "target_height": f"{t} m", "photos": 24},
                {"result.gimbal_pitch.value": -math.degrees(math.atan((h - t) / r))}, MIS_SRC) for i, (r, h, t) in enumerate(cases, 1)]


OBL_SRC = "Tilted-photo geometry (Wolf, Dewitt & Wilkinson 2014, ch. 10) in closed angle form, evaluated in Python (tools/vectors/gen_drone.py)"


def oblique_vectors():
    """Ground ahead of nadir for image row y (mm from center, up = far): H·tan(θ + atan(y/f));
    across at (x, y): H·x / (√(f² + y²)·cos(θ + atan(y/f)))."""
    out = []
    cases = [(CAMS[0], 100, 45), (CAMS[0], 100, 0), (CAMS[0], 60, 30), (CAMS[1], 80, 20), (CAMS[2], 50, 50),
             (CAMS[3], 150, 35), (CAMS[4], 120, 10), (CAMS[0], 45, 55), (CAMS[1], 200, 40), (CAMS[4], 30, 60)]
    for c, h, th in cases:
        sw, sh, f, iw, ih = c
        p, t = sw / iw, math.radians(th)
        ahead = lambda y: h * math.tan(t + math.atan(y / f))
        right = lambda x, y: h * x / (math.hypot(f, y) * math.cos(t + math.atan(y / f)))
        e = {
            "result.gsd_center.value": 100 * (right(p / 2, 0) - right(-p / 2, 0)),
            "result.gsd_center_along.value": 100 * (ahead(p / 2) - ahead(-p / 2)),
            "result.gsd_near.value": 100 * (ahead(-sh / 2 + p) - ahead(-sh / 2)),
            "result.gsd_far.value": 100 * (ahead(sh / 2) - ahead(sh / 2 - p)),
            "result.gsd_nadir.value": 100 * p * h / f,
            "result.near_distance.value": ahead(-sh / 2),
            "result.far_distance.value": ahead(sh / 2),
            "result.footprint.0.ahead.value": ahead(-sh / 2), "result.footprint.0.right.value": right(-sw / 2, -sh / 2),
            "result.footprint.2.ahead.value": ahead(sh / 2), "result.footprint.2.right.value": right(sw / 2, sh / 2),
        }
        out.append(fvec(len(out) + 1, dict(cam_inp(c), height=f"{h} m", pitch=f"{th} deg"), e, OBL_SRC, "4th edition (2014)"))
    # The spec scenario: at 45° and 100 m the center GSD is coarser than nadir (checked in the Rust tests too).
    c = CAMS[0]
    out.append(fvec(len(out) + 1, dict(cam_inp(c), height="100 m", pitch="75 deg"), {"meta.warnings.0.code": "BEYOND_HORIZON", "result.horizon": "the far edge reaches the horizon"}, SPEC, "2026"))
    sq = {k: v for k, v in cam_inp(c).items() if k != "sensor_height"}
    t = math.radians(45)
    out.append(fvec(len(out) + 1, dict(sq, height="100 m", pitch="45 deg"), {"result.near_distance.value": 100 * math.tan(t - math.atan(c[0] * c[4] / c[3] / 2 / c[2]))}, OBL_SRC, "4th edition (2014)"))
    out.append(fvec(len(out) + 1, dict(cam_inp(c), height="100 m", pitch="90 deg"), {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    nop = {k: v for k, v in sq.items() if k != "image_height"}
    out.append(fvec(len(out) + 1, dict(nop, height="100 m", pitch="45 deg"), {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    return out


TER_SRC = "Flight-planning geometry (Wolf, Dewitt & Wilkinson 2014, ch. 18): footprint and fixed photo spacing, evaluated in Python (tools/vectors/gen_drone.py)"


def terrain_vectors():
    """Footprint L(h) = sensor·h/f; the plan fixes spacing B = (1 − o)·L(H); over ground t the overlap is 1 − B/L(H − t)."""
    out = []
    c = CAMS[0]
    cases = [(100, 40, 75, 65, None), (100, 0, 75, 65, None), (120, 30, 80, 70, 70), (60, 10, 85, 70, None), (150, 90, 75, 60, 60),
             (100, -20, 75, 65, None), (80, 50, 90, 80, 75), (200, 25, 70, None, 65), (45, 5, 80, 60, 50), (100, 40, 75, 65, 70),
             (130, 60, 85, 75, 80), (90, 30, 60, 50, None)]
    for h, t, fo, so, tgt in cases:
        lf, ls = c[1] * h / c[2], c[0] * h / c[2]  # along (short side) and across footprints, mm/mm × m
        bf, bs = (1 - fo / 100) * lf, (1 - (so or 0) / 100) * ls
        lf2, ls2 = c[1] * (h - t) / c[2], c[0] * (h - t) / c[2]
        inp = {"height": f"{h} m", "highest_terrain": f"{t} m", "front_overlap": fo}
        e = {"result.effective_height.value": float(h - t), "result.front_overlap_worst": 100 * (1 - bf / lf2)}
        if so is not None:
            inp["side_overlap"] = so
            e["result.side_overlap_worst"] = 100 * (1 - bs / ls2)
        if tgt is not None:
            inp["target_overlap"] = tgt
            planned = [fo] + ([so] if so is not None else [])
            if t > 0 and all(tgt < o for o in planned):
                # Bisect for the height where the tighter direction's overlap at the terrain equals the target.
                def ok(H):
                    return all(1 - (1 - o / 100) * H / (H - t) >= tgt / 100 for o in planned)
                lo, hi = t + 1e-9, 1e7
                for _ in range(200):
                    mid = (lo + hi) / 2
                    lo, hi = (lo, mid) if ok(mid) else (mid, hi)
                e["result.min_height.value"] = hi
        worst = min([e["result.front_overlap_worst"]] + ([e["result.side_overlap_worst"]] if so is not None else []))
        if worst < (tgt if tgt is not None else min([fo] + ([so] if so is not None else []))) - 1e-9:
            e["meta.warnings.1.code"] = "OVERLAP_BELOW_TARGET"
        out.append(fvec(len(out) + 1, inp, e, TER_SRC, "4th edition (2014)"))
    out.append(fvec(len(out) + 1, {"height": "100 m", "highest_terrain": "40 m", "front_overlap": 75, "side_overlap": 65,
                                   "sensor_width": "13.2 mm", "focal_length": "8.8 mm", "image_width": 5472},
                    {"result.gsd_worst.value": 13.2 / 5472 * 60 / 8.8 * 100, "result.gsd_takeoff.value": 13.2 / 5472 * 100 / 8.8 * 100,
                     "result.front_overlap_worst": 100 * (1 - 0.25 * 100 / 60)}, SPEC, "2026"))
    out.append(fvec(len(out) + 1, {"height": "100 m", "highest_terrain": "100 m", "front_overlap": 75}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    return out


FAC_SRC = "Coverage with the standoff as the object distance (Wolf, Dewitt & Wilkinson 2014, ch. 6), on the equator where the geodesic is closed form, evaluated in Python (tools/vectors/gen_drone.py)"


def facade_vectors():
    """A facade on the equator running east: its length is a·Δλ, and a station
    D to the right (south) sits at latitude −D / (a(1 − e²)) to well under 1e-12."""
    a, f = 6378137.0, 1 / 298.257223563
    e2 = f * (2 - f)
    out = []
    cases = [(CAMS[0], 50, 30, 25, 0, 75, 60), (CAMS[0], 120, 15, 40, 2, 80, 70), (CAMS[1], 30, 20, 10, 0, 70, 60),
             (CAMS[2], 80, 10, 30, 5, 75, 65), (CAMS[3], 200, 40, 60, 0, 80, 80), (CAMS[4], 60, 25, 20, 3, 60, 50),
             (CAMS[0], 10, 30, 8, 0, 75, 60), (CAMS[1], 150, 50, 90, 10, 85, 75), (CAMS[2], 40, 8, 12, 1, 70, 70),
             (CAMS[4], 90, 35, 45, 0, 75, 60)]

    def stations(span, foot, o):
        if span <= foot:
            return [span / 2]
        n = math.ceil((span - foot) / (foot * (1 - o)) - 1e-9) + 1
        return [foot / 2 + k * (span - foot) / (n - 1) for k in range(n)]

    for c, length, d, top, bottom, oh, ov in cases:
        sw, sh, fl, iw, ih = c
        w, h = sw * d / fl, sh * d / fl
        cols, rows = stations(length, w, oh / 100), stations(top - bottom, h, ov / 100)
        inp = {"facade": [{"lat": 0, "lon": 0}, {"lat": 0, "lon": math.degrees(length / a)}], "standoff": f"{d} m", "top_height": f"{top} m",
               "sensor_width": f"{sw} mm", "focal_length": f"{fl} mm", "image_width": iw, "sensor_height": f"{sh} mm",
               "horizontal_overlap": oh, "vertical_overlap": ov}
        if bottom:
            inp["bottom_height"] = f"{bottom} m"
        e = {"result.gsd.value": 100 * sw / iw * d / fl, "result.footprint_width.value": w, "result.footprint_height.value": h,
             "result.passes": len(rows), "result.photos_per_pass": len(cols), "result.photos": len(rows) * len(cols),
             "result.photo_spacing.value": cols[1] - cols[0] if len(cols) > 1 else 0.0,
             "result.pass_spacing.value": rows[1] - rows[0] if len(rows) > 1 else 0.0,
             "result.waypoints.0.lat.value": -math.degrees(d / (a * (1 - e2))),
             "result.waypoints.0.lon.value": math.degrees(cols[0] / a),
             "result.waypoints.0.height.value": bottom + rows[0], "result.waypoints.0.heading.value": 0.0}
        v = fvec(len(out) + 1, inp, e, FAC_SRC, "4th edition (2014)")
        v["tolerance"]["result.facade_length.value"] = {"rel": 1e-9, "abs": 1e-6}
        v["expect"]["result.facade_length.value"] = float(length)
        out.append(v)
    # The spec scenario: 30 m standoff, 1-inch camera, GSD = 13.2 / 5472 × 30 / 8.8 mm.
    out.append(fvec(len(out) + 1, {"facade": [{"lat": 40.4406, "lon": -80.002}, {"lat": 40.4406, "lon": -80.001411}], "standoff": "30 m", "top_height": "25 m",
                                   "sensor_width": "13.2 mm", "focal_length": "8.8 mm", "image_width": 5472, "sensor_height": "8.8 mm"},
                    {"result.gsd.value": 100 * 13.2 / 5472 * 30 / 8.8}, SPEC, "2026"))
    out.append(fvec(len(out) + 1, {"facade": [{"lat": 0, "lon": 0}, {"lat": 0, "lon": 0.001}], "standoff": "30 m", "top_height": "0 m",
                                   "sensor_width": "13.2 mm", "focal_length": "8.8 mm", "image_width": 5472, "sensor_height": "8.8 mm"},
                    {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    out.append(fvec(len(out) + 1, {"facade": [{"lat": 0, "lon": 0}, {"lat": 0, "lon": 0.5}], "standoff": "1 m", "top_height": "300 m",
                                   "sensor_width": "13.2 mm", "focal_length": "8.8 mm", "image_width": 5472, "sensor_height": "8.8 mm"},
                    {"ok": False, "error.code": "LIMIT_EXCEEDED"}, SPEC, "2026"))
    out.append(fvec(len(out) + 1, {"facade": [{"lat": 0, "lon": 0}, {"lat": 0, "lon": 0.001}], "standoff": "30 m", "top_height": "25 m",
                                   "sensor_width": "13.2 mm", "focal_length": "8.8 mm", "image_width": 5472},
                    {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    return out


GEO_SRC = "Geodesic distance on the equator and a meridian in closed form, and Steiner's formula for the fence area, evaluated in Python (tools/vectors/gen_drone.py)"


def geofence_vectors():
    """A rectangle whose south edge is the equator and west edge the prime meridian:
    a waypoint x m south of the equator edge or x m west of the meridian edge
    (at the rectangle's mid-height) is x m from the area, so it is outside a
    d m fence by exactly x − d."""
    a, f = 6378137.0, 1 / 298.257223563
    m0 = a * (1 - f * (2 - f))
    dlat, dlon = (lambda m: math.degrees(m / m0)), (lambda m: math.degrees(m / a))
    out = []
    cases = [(200, 150, 50, 30, [62, 40, 10]), (500, 300, 100, 60, [150, 80, 20, 101]), (100, 100, 25, None, [30, 20]),
             (1000, 400, 200, 150, [260, 180, 120]), (300, 300, 50, 45, [48, 55])]
    for w, h, d, warn, south in cases:
        area = [{"lat": 0, "lon": 0}, {"lat": 0, "lon": dlon(w)}, {"lat": dlat(h), "lon": dlon(w)}, {"lat": dlat(h), "lon": 0}]
        wps = [{"lat": -dlat(x), "lon": dlon(w / 2)} for x in south] + [{"lat": dlat(h / 2), "lon": -dlon(south[0])}]
        dists = south + [south[0]]
        inp = {"area": area, "distance": f"{d} m", "waypoints": wps}
        if warn:
            inp["warning_distance"] = f"{warn} m"
        e = {"result.outside_count": sum(1 for x in dists if x > d),
             "result.area_enclosed.value": (w * h + 2 * (w + h) * d + math.pi * d * d) / 1e6}
        flagged = [(i + 1, x) for i, x in enumerate(dists) if x > d or (warn and x > warn)]
        for k, (idx, x) in enumerate(flagged):
            e[f"result.flagged.{k}.waypoint"] = idx
            e[f"result.flagged.{k}.beyond.value"] = float(x - d if x > d else x - warn)
        if warn:
            e["result.warning_count"] = sum(1 for x in dists if warn < x <= d)
        v = fvec(len(out) + 1, inp, e, GEO_SRC, "2026", rel=1e-6)
        v["tolerance"]["result.area_enclosed.value"] = {"rel": 2e-3, "abs": 0}
        for k in v["tolerance"]:
            if k.endswith("beyond.value"):
                v["tolerance"][k] = {"rel": 0, "abs": 1e-3}
        v["expect"]["result.max_deviation.value"] = 0.0
        v["tolerance"]["result.max_deviation.value"] = {"rel": 0, "abs": max(0.001 * d, 0.5)}
        out.append(v)
    # A route: a line's fence is a stadium, 2dL + πd², and a point's a circle.
    L, d = 2000, 100
    out.append(fvec(len(out) + 1, {"area": [{"lat": 0, "lon": 0}, {"lat": 0, "lon": dlon(L)}], "distance": f"{d} m",
                                   "waypoints": [{"lat": dlat(130), "lon": dlon(L / 2)}]},
                    {"result.outside_count": 1, "result.flagged.0.beyond.value": 30.0, "result.area_enclosed.value": (2 * d * L + math.pi * d * d) / 1e6}, GEO_SRC, "2026", rel=2e-3))
    out[-1]["tolerance"]["result.flagged.0.beyond.value"] = {"rel": 0, "abs": 1e-3}
    out.append(fvec(len(out) + 1, {"area": [{"lat": 0, "lon": 0}], "distance": "500 m"}, {"result.outside_count": 0, "result.area_enclosed.value": math.pi * 0.25}, GEO_SRC, "2026", rel=1e-3))
    # The spec scenario: a waypoint 12 m outside is flagged with its index and distance.
    out.append(fvec(len(out) + 1, {"area": [{"lat": 0, "lon": 0}, {"lat": 0, "lon": dlon(300)}, {"lat": dlat(200), "lon": dlon(300)}, {"lat": dlat(200), "lon": 0}],
                                   "distance": "50 m", "waypoints": [{"lat": dlat(100), "lon": dlon(150)}, {"lat": -dlat(62), "lon": dlon(150)}]},
                    {"meta.warnings.1.code": "WAYPOINT_OUTSIDE_GEOFENCE", "result.flagged.0.waypoint": 2, "result.flagged.0.beyond.value": 12.0}, SPEC, "2026"))
    out[-1]["tolerance"]["result.flagged.0.beyond.value"] = {"rel": 0, "abs": 1e-3}
    out.append(fvec(len(out) + 1, {"area": [{"lat": 0, "lon": 0}], "distance": "50 m", "warning_distance": "60 m"}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    files = {"drone.photogrammetry.gsd": gsd(), "drone.photogrammetry.altitude-for-gsd": alt(), "drone.photogrammetry.trigger": trigger(),
             "drone.photogrammetry.motion-blur": blur(), "drone.photogrammetry.asprs-accuracy": asprs(),
             "drone.power.battery-energy": battery_vectors(), "drone.power.hover-power": hover_vectors(),
             "drone.power.endurance": endurance_vectors(), "drone.power.max-payload": payload_vectors(),
             "drone.power.rth-budget": rth_vectors(), "drone.ops.part107-altitude": altitude_vectors(),
             "drone.ops.speed-check": speed_vectors(), "drone.ops.kinetic-energy": ke_vectors(),
             "drone.ops.easa-subcategory": easa_vectors(), "drone.sensors.vlos": vlos_vectors(),
             "drone.mission.survey-grid": grid_vectors("grid"), "drone.photogrammetry.image-count": grid_vectors("count"),
             "drone.mission.corridor": corridor_vectors(), "drone.mission.orbit": orbit_vectors(),
             "drone.photogrammetry.oblique-gsd": oblique_vectors(),
             "drone.photogrammetry.terrain-overlap": terrain_vectors(),
             "drone.mission.facade": facade_vectors(),
             "drone.mission.geofence": geofence_vectors()}
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
