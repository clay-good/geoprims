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
    # Keep the five published vector identities stable, then add Han and Han's
    # printed example and a sweep over the formula's practical input range.
    old_cases = [
        (10, 0.001, 2.741, 0.5),
        (15, 1 / 2000, 1.0, 0.5),
        (5, 1 / 500, 3.0, 0.5),
        (20, 1 / 4000, 0.8, 0.5),
        (8, 1 / 800, 2.0, 0.5),
    ]
    cases = [
        (10, 1 / 500, 2.0, 0.5),
        (1, 1 / 100, 10.0, 0.25),
        (2, 1 / 250, 5.0, 0.25),
        (3, 1 / 320, 4.0, 0.4),
        (4, 1 / 400, 3.5, 0.4),
        (6, 1 / 640, 2.5, 0.5),
        (7.5, 1 / 800, 1.8, 0.5),
        (9, 1 / 1000, 1.5, 0.75),
        (10, 1 / 1250, 1.2, 0.75),
        (12, 1 / 1600, 1.0, 1.0),
        (14, 1 / 2000, 0.9, 1.0),
        (16, 1 / 2500, 0.8, 1.25),
        (18, 1 / 3200, 0.7, 1.25),
        (20, 1 / 4000, 0.6, 1.5),
        (22, 1 / 5000, 0.5, 1.5),
        (25, 1 / 6400, 0.4, 2.0),
        (30, 1 / 8000, 0.3, 2.0),
    ]
    out = []
    for i, (v, t, g, limit) in enumerate(old_cases, 1):
        out.append(vec(i, {"groundspeed": f"{v} m/s", "exposure": f"{t} s", "gsd": f"{g} cm"},
                       {"result.blur": v * t / (g / 100), "result.max_exposure.value": limit * (g / 100) / v}))
    for i, (v, t, g, limit) in enumerate(cases, len(out) + 1):
        inp = {"groundspeed": f"{v} m/s", "exposure": f"{t} s", "gsd": f"{g} cm"}
        if limit != 0.5:
            inp["blur_limit"] = limit
        source = "Han and Han, Drones 2024 8(12), section 2.1.1, equation (1), evaluated in Python (tools/vectors/gen_drone.py)"
        out.append(vec(i, inp,
                       {"result.blur": v * t / (g / 100), "result.max_exposure.value": limit * (g / 100) / v},
                       source, "Volume 8, issue 12, article 736 (2024)"))
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
    coax_src = "Momentum theory with the coaxial induced-power interference factor (Leishman and Syal 2008, 1.2818), evaluated in Python with the ISA sea-level density 101325 / (287.05287 x 288.15)"
    rho0 = P0 / (R_AIR * T0)
    for n, fm, eta, kappa in [(8, 0.6, 0.85, 1.2818), (8, 0.6, 0.85, 1.414), (8, 0.6, 0.85, 1.0), (8, 0.6, 0.85, None), (6, 0.5, 0.8, 1.2818)]:
        d = 12 * 0.0254
        a = n * math.pi * d * d / 4
        ideal = (2.5 * G0) ** 1.5 / math.sqrt(2 * rho0 * a)
        electrical = ideal / (fm * eta) if kappa is None or kappa == 1 else ideal * (kappa + 1 / fm - 1) / eta
        inp = {"mass": "2.5 kg", "rotors": n, "rotor_diameter": "12 in"}
        if kappa is not None:
            inp["coaxial"] = "yes"
            if kappa != 1.2818:
                inp["interference"] = kappa
        if fm != 0.6:
            inp["figure_of_merit"] = fm
        if eta != 0.85:
            inp["efficiency"] = eta
        source = coax_src
        if kappa == 1.414:
            source += "; 1.414 is the coplanar limit"
        elif kappa == 1.0:
            source = "A factor of 1 is no interference: the same as separate rotors"
        elif kappa is None:
            source += "; not coaxial"
        out.append(fvec(len(out) + 1, inp, {"result.electrical_power.value": electrical, **({"result.interference_factor": kappa} if kappa == 1.2818 and fm == 0.6 else {})}, source, "2026"))
    out.append(fvec(len(out) + 1, {"mass": "2.5 kg", "rotors": 7, "rotor_diameter": "12 in", "coaxial": "yes"},
                    {"ok": False, "error.code": "INVALID_INPUT"}, "add-drone-suite: coaxial rotors come in pairs", "2026"))
    out.append(fvec(len(out) + 1, {"mass": "2.5 kg", "rotors": 8, "rotor_diameter": "12 in", "interference": 1.3},
                    {"ok": False, "error.code": "INVALID_INPUT"}, "add-drone-suite: an interference factor without coaxial pairs", "2026"))
    # Leishman's open textbook works a 45,000 lb tiltrotor with two 38 ft
    # rotors at sea level: 3,940.4 kW ideal and 5,253.8 kW at FM = 0.75,
    # before the separate transmission loss. Preserve its printed precision.
    out.append(fvec(len(out) + 1, {"mass": "45000 lb", "rotors": 2, "rotor_diameter": "38 ft", "figure_of_merit": 0.75, "efficiency": 1},
                    {"result.ideal_power.value": 3_940_400.0, "result.electrical_power.value": 5_253_800.0},
                    "Leishman, Introduction to Aerospace Flight Vehicles, chapter 67, hover-power worked example", "2026 webbook"))
    out[-1]["tolerance"] = {"result.ideal_power.value": {"rel": 0.0005, "abs": 0}, "result.electrical_power.value": {"rel": 0.0005, "abs": 0}}
    # Broaden the differential matrix across density, losses, avionics, units,
    # rotor counts, and scale while retaining the same published equations.
    more = [
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "density_altitude": "8000 ft"}, 1.4, 4, 9.4 * 0.0254, rho_at(8000 * 0.3048), 0.6, 0.85, 0),
        ({"mass": "3 kg", "rotors": 6, "rotor_diameter": "15 in", "altitude": "5000 ft", "temperature": "30 degC"}, 3, 6, 15 * 0.0254, rho_at(5000 * 0.3048, 303.15), 0.6, 0.85, 0),
        ({"mass": "2 kg", "rotors": 4, "rotor_diameter": "0.3 m", "avionics_power": "25 W"}, 2, 4, 0.3, rho0, 0.6, 0.85, 25),
        ({"mass": "4 kg", "rotors": 8, "rotor_diameter": "18 in", "figure_of_merit": 0.4}, 4, 8, 18 * 0.0254, rho0, 0.4, 0.85, 0),
        ({"mass": "4 kg", "rotors": 8, "rotor_diameter": "18 in", "efficiency": 0.5}, 4, 8, 18 * 0.0254, rho0, 0.6, 0.5, 0),
        ({"mass": "500 g", "rotors": 4, "rotor_diameter": "5 in"}, 0.5, 4, 5 * 0.0254, rho0, 0.6, 0.85, 0),
        ({"mass": "50 kg", "rotors": 12, "rotor_diameter": "36 in"}, 50, 12, 36 * 0.0254, rho0, 0.6, 0.85, 0),
    ]
    for inp, m, n, d, rho, fm, eta, avionics in more:
        area = n * math.pi * d * d / 4
        ideal = (m * G0) ** 1.5 / math.sqrt(2 * rho * area)
        out.append(fvec(len(out) + 1, inp, {"result.ideal_power.value": ideal, "result.electrical_power.value": ideal / (fm * eta) + avionics,
                                            "result.disk_area.value": area, "result.air_density.value": rho}, PW_SRC, PW_VER))
    return out


def endurance_vectors():
    cases = [(90.4, 80, 150.6, 0), (100, 100, 100, 20), (77, 90, 220, 15), (274, 85, 900, 25), (45, 100, 60, 0)]
    out = []
    for i, (e, u, p, r) in enumerate(cases, 1):
        t = e * u / 100 * (1 - r / 100) / p * 60
        out.append(fvec(i, {"energy": f"{e} Wh", "usable": u, "power": f"{p} W", "reserve": r}, {"result.hover_time.value": t}, PW_SRC, PW_VER))
    # Peukert, only when an exponent is entered: energy × (pack energy / rated time / power)^(k − 1).
    for e, p, k, rated_h in [(100, 150, 1.05, None), (90.4, 150.6, 1.1, 1), (274, 900, 1.03, 0.5), (45, 30, 1.08, 2)]:
        f = (e / (rated_h or 1) / p) ** (k - 1)
        inp = {"energy": f"{e} Wh", "power": f"{p} W", "peukert": k}
        if rated_h:
            inp["rated_time"] = f"{rated_h} h"
        out.append(fvec(len(out) + 1, inp, {"result.hover_time.value": e * f / p * 60, "result.peukert_factor": f}, PW_SRC, PW_VER))
    return out


def calibrate_vectors():
    out = []
    rho0 = P0 / (R_AIR * T0)
    # mass, rotors, rotor in, Wh used, min, avionics W, efficiency, altitude ft
    cases = [(1.4, 4, 9.4, 40, 15, 0, None, None), (0.9, 4, 7.0, 20, 12, 5, None, None), (6.5, 6, 22.0, 150, 12, 20, 0.9, None),
             (2.5, 4, 13.0, 60, 15, 10, None, 5000), (25.0, 8, 30.0, 700, 10, 30, 0.8, None)]
    for i, (m, n, d_in, e, t, av, eta, alt_ft) in enumerate(cases, 1):
        d = d_in * 0.0254
        a = n * math.pi * d * d / 4
        rho = rho_at(alt_ft * 0.3048) if alt_ft else rho0
        ideal = (m * G0) ** 1.5 / math.sqrt(2 * rho * a)
        measured = e * 60 / t
        fe = ideal / (measured - av)
        inp = {"mass": f"{m} kg", "rotors": n, "rotor_diameter": f"{d_in} in", "energy_used": f"{e} Wh", "hover_time": f"{t} min"}
        if av:
            inp["avionics_power"] = f"{av} W"
        if eta:
            inp["efficiency"] = eta
        if alt_ft:
            inp["altitude"] = f"{alt_ft} ft"
        out.append(fvec(i, inp, {"result.fm_eta": fe, "result.figure_of_merit": fe / (eta or 0.85),
                                  "result.measured_power.value": measured}, PW_SRC, PW_VER))
    # A hover that would beat an ideal rotor is refused.
    out.append(fvec(len(out) + 1, {"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "10 Wh", "hover_time": "15 min"},
                    {"ok": False, "error.code": "OUT_OF_DOMAIN"}, PW_SRC, PW_VER))

    # Independently published hover-power example, inverted. Embry-Riddle
    # prints 5,253.8 kW at FM = 0.75 before transmission loss. One hour at
    # that power is 5,253.8 kWh, so calibration should recover the printed FM.
    published = fvec(len(out) + 1,
                     {"mass": "45000 lb", "rotors": 2, "rotor_diameter": "38 ft", "energy_used": "5253.8 kWh", "hover_time": "1 h", "efficiency": 1},
                     {"result.fm_eta": 0.75, "result.figure_of_merit": 0.75, "result.measured_power.value": 5253800.0},
                     "Leishman, Introduction to Aerospace Flight Vehicles, chapter 67, hover-power worked example inverted from its printed power",
                     "2026 webbook")
    published["tolerance"] = {
        "result.fm_eta": {"rel": 0.0005},
        "result.figure_of_merit": {"rel": 0.0005},
        "result.measured_power.value": {"abs": 1e-6},
    }
    out.append(published)

    def success(inp, mass_kg, rotors, diameter_m, energy_wh, time_s, avionics=0, efficiency=0.85, rho=rho0):
        area = rotors * math.pi * diameter_m ** 2 / 4
        ideal = (mass_kg * G0) ** 1.5 / math.sqrt(2 * rho * area)
        measured = energy_wh * 3600 / time_s
        fm_eta = ideal / (measured - avionics)
        out.append(fvec(len(out) + 1, inp,
                        {"result.fm_eta": fm_eta, "result.figure_of_merit": fm_eta / efficiency,
                         "result.measured_power.value": measured, "result.ideal_power.value": ideal}, PW_SRC, PW_VER))

    success({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "144000 J", "hover_time": "900 s"},
            1.4, 4, 9.4 * 0.0254, 40, 900)
    success({"mass": "0.9 kg", "rotors": 4, "rotor_diameter": "7 in", "energy_used": "20 Wh", "hover_time": "12 min", "avionics_power": "5 W", "efficiency": 0.5},
            0.9, 4, 7 * 0.0254, 20, 720, 5, 0.5)
    success({"mass": "6.5 kg", "rotors": 6, "rotor_diameter": "22 in", "energy_used": "150 Wh", "hover_time": "12 min", "avionics_power": "20 W", "efficiency": 1},
            6.5, 6, 22 * 0.0254, 150, 720, 20, 1)
    success({"mass": "2 kg", "rotors": 4, "rotor_diameter": "12 in", "energy_used": "70 Wh", "hover_time": "15 min", "altitude": "-1000 m"},
            2, 4, 12 * 0.0254, 70, 900, rho=rho_at(-1000))
    success({"mass": "2 kg", "rotors": 4, "rotor_diameter": "12 in", "energy_used": "70 Wh", "hover_time": "10 min", "altitude": "11000 m"},
            2, 4, 12 * 0.0254, 70, 600, rho=rho_at(11000))
    success({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "40 Wh", "hover_time": "12 min", "density_altitude": "8000 ft"},
            1.4, 4, 9.4 * 0.0254, 40, 720, rho=rho_at(8000 * 0.3048))
    hot_rho = rho_at(5000 * 0.3048, 303.15)
    success({"mass": "3 kg", "rotors": 6, "rotor_diameter": "15 in", "energy_used": "90 Wh", "hover_time": "12 min", "altitude": "5000 ft", "temperature": "30 degC"},
            3, 6, 15 * 0.0254, 90, 720, rho=hot_rho)
    success({"mass": "8 kg", "rotors": 1, "rotor_diameter": "36 in", "energy_used": "250 Wh", "hover_time": "12 min"},
            8, 1, 36 * 0.0254, 250, 720)
    success({"mass": "2 kg", "rotors": 32, "rotor_diameter": "5 in", "energy_used": "80 Wh", "hover_time": "20 min"},
            2, 32, 5 * 0.0254, 80, 1200)
    success({"mass": "1400 g", "rotors": 4, "rotor_diameter": "23.876 cm", "energy_used": "144 kJ", "hover_time": "900 s", "avionics_power": "25 W"},
            1.4, 4, 0.23876, 40, 900, 25)

    failures = [
        ({"mass": "1.4 kg", "rotors": 4.5, "rotor_diameter": "9.4 in", "energy_used": "40 Wh", "hover_time": "15 min"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "0 Wh", "hover_time": "15 min"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "40 Wh", "hover_time": "0 min"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "40 Wh", "hover_time": "15 min", "avionics_power": "-1 W"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "40 Wh", "hover_time": "15 min", "avionics_power": "160 W"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "40 Wh", "hover_time": "15 min", "altitude": "5000 ft", "density_altitude": "8000 ft"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "40 Wh", "hover_time": "15 min", "altitude": "11001 m"}, "OUT_OF_DOMAIN"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "energy_used": "40 Wh", "hover_time": "15 min", "temperature": "-273.15 degC"}, "INVALID_INPUT"),
    ]
    for inp, code in failures:
        out.append(fvec(len(out) + 1, inp, {"ok": False, "error.code": code}, PW_SRC, PW_VER))
    return out


def payload_vectors():
    out = []
    rho0 = P0 / (R_AIR * T0)
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

    def success(inp, mass_kg, rotors, diameter_m, energy_wh, target_s, avionics_w=0,
                figure_of_merit=0.6, efficiency=0.85, rho=rho0, source=PW_SRC, version=PW_VER):
        area = rotors * math.pi * diameter_m ** 2 / 4
        lift_budget = energy_wh * 3600 / target_s - avionics_w
        max_mass = (lift_budget * figure_of_merit * efficiency * math.sqrt(2 * rho * area)) ** (2 / 3) / G0
        base_power = (mass_kg * G0) ** 1.5 / math.sqrt(2 * rho * area) / (figure_of_merit * efficiency) + avionics_w
        vector = fvec(len(out) + 1, inp,
                      {"result.max_payload.value": max_mass - mass_kg, "result.max_mass.value": max_mass,
                       "result.base_hover_time.value": energy_wh * 60 / base_power}, source, version)
        out.append(vector)
        return vector

    # Invert Van Mulders et al.'s measured payload-hover fit at 0.25 kg.
    coefficient = 12.88
    fm, eta = 0.6, 0.85
    area = 1 / (2 * rho0 * (coefficient * fm * eta) ** 2)
    diameter = math.sqrt(area / math.pi)  # four rotors: A = pi D^2
    payload_power = 10.6 + coefficient * (0.25 * G0) ** 1.5
    published = success(
        {"mass": "0.2 kg", "rotors": 4, "rotor_diameter": f"{diameter:.17g} m", "usable_energy": "20 Wh",
         "target_time": f"{20 * 60 / payload_power:.17g} min", "avionics_power": "10.6 W"},
        0.2, 4, diameter, 20, 20 * 3600 / payload_power, 10.6,
        source="Van Mulders et al., Sensors 23(4), 2291 (2023), section 2.4.1 equation 6 and figure 4a payload-hover fit",
        version="Sensors 23(4), article 2291 (2023)")
    published["tolerance"] = {k: {"rel": 0.001} for k in published["expect"] if k != "ok"}

    success({"mass": "1400 g", "rotors": 4, "rotor_diameter": "23.876 cm", "usable_energy": "259200 J", "target_time": "1200 s"},
            1.4, 4, 0.23876, 72, 1200)
    success({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "72 Wh", "target_time": "18 min", "avionics_power": "25 W"},
            1.4, 4, 9.4 * 0.0254, 72, 18 * 60, 25)
    success({"mass": "2 kg", "rotors": 4, "rotor_diameter": "12 in", "usable_energy": "100 Wh", "target_time": "20 min", "altitude": "-1000 m"},
            2, 4, 12 * 0.0254, 100, 20 * 60, rho=rho_at(-1000))
    success({"mass": "2 kg", "rotors": 4, "rotor_diameter": "12 in", "usable_energy": "180 Wh", "target_time": "20 min", "altitude": "11000 m"},
            2, 4, 12 * 0.0254, 180, 20 * 60, rho=rho_at(11000))
    success({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "90 Wh", "target_time": "20 min", "density_altitude": "8000 ft"},
            1.4, 4, 9.4 * 0.0254, 90, 20 * 60, rho=rho_at(8000 * 0.3048))
    success({"mass": "3 kg", "rotors": 6, "rotor_diameter": "15 in", "usable_energy": "220 Wh", "target_time": "20 min", "altitude": "5000 ft", "temperature": "30 degC"},
            3, 6, 15 * 0.0254, 220, 20 * 60, rho=rho_at(5000 * 0.3048, 303.15))
    success({"mass": "6 kg", "rotors": 8, "rotor_diameter": "20 in", "usable_energy": "600 Wh", "target_time": "20 min", "figure_of_merit": 0.4, "efficiency": 0.5},
            6, 8, 20 * 0.0254, 600, 20 * 60, figure_of_merit=0.4, efficiency=0.5)
    success({"mass": "2 kg", "rotors": 32, "rotor_diameter": "5 in", "usable_energy": "100 Wh", "target_time": "30 min", "figure_of_merit": 0.8, "efficiency": 1},
            2, 32, 5 * 0.0254, 100, 30 * 60, figure_of_merit=0.8, efficiency=1)
    success({"mass": "25 kg", "rotors": 8, "rotor_diameter": "30 in", "usable_energy": "1800 Wh", "target_time": "20 min", "avionics_power": "30 W"},
            25, 8, 30 * 0.0254, 1800, 20 * 60, 30)
    success({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "144 Wh", "target_time": "20 min"},
            1.4, 4, 9.4 * 0.0254, 144, 20 * 60)

    failures = [
        ({"mass": "1.4 kg", "rotors": 4.5, "rotor_diameter": "9.4 in", "usable_energy": "72 Wh", "target_time": "20 min"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "0 Wh", "target_time": "20 min"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "72 Wh", "target_time": "0 min"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "72 Wh", "target_time": "20 min", "avionics_power": "-1 W"}, "INVALID_INPUT"),
        ({"mass": "0 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "72 Wh", "target_time": "20 min"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "72 Wh", "target_time": "20 min", "altitude": "5000 ft", "density_altitude": "8000 ft"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "72 Wh", "target_time": "20 min", "altitude": "11001 m"}, "OUT_OF_DOMAIN"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "72 Wh", "target_time": "20 min", "temperature": "-273.15 degC"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "usable_energy": "20 Wh", "target_time": "20 min"}, "NO_SOLUTION"),
    ]
    for inp, code in failures:
        out.append(fvec(len(out) + 1, inp, {"ok": False, "error.code": code}, PW_SRC, PW_VER))
    return out


def payload_impact_vectors():
    out = []
    # mass, rotors, rotor in, payload kg, usable Wh, payload W, avionics W, altitude ft
    cases = [(1.4, 4, 9.4, 0.3, 72, 8, 0, None), (0.9, 4, 7.0, 0.1, 45, 0, 0, None), (6.5, 6, 22.0, 1.5, 400, 20, 15, None),
             (2.5, 4, 13.0, 0.5, 90, 0, 10, 5000), (4.0, 4, 15.0, 0.0, 160, 12, 0, None), (1.4, 4, 9.4, 0.3, 72, 8, 0, 8000)]
    rho0 = P0 / (R_AIR * T0)
    for i, (m, n, d_in, pm, e, pp, av, alt_ft) in enumerate(cases, 1):
        d = d_in * 0.0254
        a = n * math.pi * d * d / 4
        rho = rho_at(alt_ft * 0.3048) if alt_ft else rho0
        lift = lambda mm: (mm * G0) ** 1.5 / math.sqrt(2 * rho * a) / 0.51
        p0, p1 = lift(m) + av, lift(m + pm) + av + pp
        t0, t1 = e * 60 / p0, e * 60 / p1
        inp = {"mass": f"{m} kg", "rotors": n, "rotor_diameter": f"{d_in} in", "payload_mass": f"{pm} kg", "usable_energy": f"{e} Wh"}
        if pp:
            inp["payload_power"] = f"{pp} W"
        if av:
            inp["avionics_power"] = f"{av} W"
        if alt_ft:
            inp["altitude"] = f"{alt_ft} ft"
        out.append(fvec(i, inp, {"result.hover_time_with.value": t1, "result.hover_time_without.value": t0,
                                  "result.power_increase.value": p1 - p0}, PW_SRC, PW_VER))

    # Van Mulders et al. fit measured payload-hover data to
    # P = 10.6 W + 12.88(mg)^1.5. Choose the equivalent disk area at the
    # default FM × efficiency so the tool expresses the same fitted model.
    coefficient = 12.88
    fm, eta = 0.6, 0.85
    area = 1 / (2 * rho0 * (coefficient * fm * eta) ** 2)
    diameter = math.sqrt(area / math.pi)  # four rotors: A = πD²
    p0 = 10.6 + coefficient * (0.2 * G0) ** 1.5
    p1 = 10.6 + coefficient * (0.25 * G0) ** 1.5
    published = fvec(len(out) + 1,
                     {"mass": "0.2 kg", "rotors": 4, "rotor_diameter": f"{diameter:.17g} m", "payload_mass": "0.05 kg",
                      "usable_energy": "20 Wh", "avionics_power": "10.6 W"},
                     {"result.hover_time_with.value": 20 * 60 / p1, "result.hover_time_without.value": 20 * 60 / p0,
                      "result.time_lost.value": 20 * 60 / p0 - 20 * 60 / p1, "result.power_with.value": p1,
                      "result.power_increase.value": p1 - p0, "result.power_increase_percent": (p1 - p0) / p0 * 100},
                     "Van Mulders et al., Sensors 23(4), 2291 (2023), section 2.4.1 equation 6 and figure 4a payload-hover fit",
                     "Sensors 23(4), article 2291 (2023)")
    published["tolerance"] = {k: {"rel": 0.001} for k in published["expect"] if k != "ok"}
    out.append(published)

    def success(inp, mass_kg, rotors, diameter_m, payload_kg, energy_wh, payload_w=0, avionics_w=0,
                figure_of_merit=0.6, efficiency=0.85, rho=rho0):
        disk_area = rotors * math.pi * diameter_m ** 2 / 4
        lift = lambda mass: (mass * G0) ** 1.5 / math.sqrt(2 * rho * disk_area) / (figure_of_merit * efficiency)
        power_without = lift(mass_kg) + avionics_w
        power_with = lift(mass_kg + payload_kg) + avionics_w + payload_w
        time_without = energy_wh * 60 / power_without
        time_with = energy_wh * 60 / power_with
        out.append(fvec(len(out) + 1, inp,
                        {"result.hover_time_with.value": time_with, "result.hover_time_without.value": time_without,
                         "result.time_lost.value": time_without - time_with, "result.power_with.value": power_with,
                         "result.power_increase.value": power_with - power_without,
                         "result.power_increase_percent": (power_with - power_without) / power_without * 100}, PW_SRC, PW_VER))

    success({"mass": "1400 g", "rotors": 4, "rotor_diameter": "23.876 cm", "payload_mass": "300 g", "usable_energy": "259200 J", "payload_power": "8 W"},
            1.4, 4, 0.23876, 0.3, 72, 8)
    success({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0 kg", "usable_energy": "72 Wh", "payload_power": "20 W"},
            1.4, 4, 9.4 * 0.0254, 0, 72, 20)
    success({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0 kg", "usable_energy": "72 Wh"},
            1.4, 4, 9.4 * 0.0254, 0, 72)
    success({"mass": "2 kg", "rotors": 4, "rotor_diameter": "12 in", "payload_mass": "0.4 kg", "usable_energy": "100 Wh", "altitude": "-1000 m"},
            2, 4, 12 * 0.0254, 0.4, 100, rho=rho_at(-1000))
    success({"mass": "2 kg", "rotors": 4, "rotor_diameter": "12 in", "payload_mass": "0.4 kg", "usable_energy": "100 Wh", "altitude": "11000 m"},
            2, 4, 12 * 0.0254, 0.4, 100, rho=rho_at(11000))
    success({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0.3 kg", "usable_energy": "72 Wh", "density_altitude": "8000 ft"},
            1.4, 4, 9.4 * 0.0254, 0.3, 72, rho=rho_at(8000 * 0.3048))
    success({"mass": "3 kg", "rotors": 6, "rotor_diameter": "15 in", "payload_mass": "0.5 kg", "usable_energy": "150 Wh", "altitude": "5000 ft", "temperature": "30 degC"},
            3, 6, 15 * 0.0254, 0.5, 150, rho=rho_at(5000 * 0.3048, 303.15))
    success({"mass": "6 kg", "rotors": 8, "rotor_diameter": "20 in", "payload_mass": "1 kg", "usable_energy": "300 Wh", "figure_of_merit": 0.4, "efficiency": 0.5},
            6, 8, 20 * 0.0254, 1, 300, figure_of_merit=0.4, efficiency=0.5)
    success({"mass": "2 kg", "rotors": 32, "rotor_diameter": "5 in", "payload_mass": "0.2 kg", "usable_energy": "80 Wh", "figure_of_merit": 0.8, "efficiency": 1},
            2, 32, 5 * 0.0254, 0.2, 80, figure_of_merit=0.8, efficiency=1)
    success({"mass": "25 kg", "rotors": 8, "rotor_diameter": "30 in", "payload_mass": "5 kg", "usable_energy": "1000 Wh", "payload_power": "100 W", "avionics_power": "30 W"},
            25, 8, 30 * 0.0254, 5, 1000, 100, 30)

    failures = [
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "-0.1 kg", "usable_energy": "72 Wh"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0.3 kg", "usable_energy": "72 Wh", "payload_power": "-1 W"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0.3 kg", "usable_energy": "72 Wh", "avionics_power": "-1 W"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0.3 kg", "usable_energy": "0 Wh"}, "INVALID_INPUT"),
        ({"mass": "0 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0.3 kg", "usable_energy": "72 Wh"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4.5, "rotor_diameter": "9.4 in", "payload_mass": "0.3 kg", "usable_energy": "72 Wh"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0.3 kg", "usable_energy": "72 Wh", "altitude": "5000 ft", "density_altitude": "8000 ft"}, "INVALID_INPUT"),
        ({"mass": "1.4 kg", "rotors": 4, "rotor_diameter": "9.4 in", "payload_mass": "0.3 kg", "usable_energy": "72 Wh", "altitude": "11001 m"}, "OUT_OF_DOMAIN"),
    ]
    for inp, code in failures:
        out.append(fvec(len(out) + 1, inp, {"ok": False, "error.code": code}, PW_SRC, PW_VER))
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

    def success(inp, distance_m, airspeed_mps, headwind_mps, power_w, energy_wh, reserve_wh=0,
                source=PW_SRC, version=PW_VER):
        return_speed = airspeed_mps - headwind_mps
        return_time = distance_m / return_speed
        return_wh = power_w * return_time / 3600
        available_j = max(0, energy_wh - reserve_wh) * 3600
        round_trip = available_j / (power_w * (1 / (airspeed_mps + headwind_mps) + 1 / return_speed))
        vector = fvec(len(out) + 1, inp,
                      {"result.return_groundspeed.value": return_speed, "result.return_time.value": return_time / 60,
                       "result.return_energy.value": return_wh, "result.margin.value": energy_wh - reserve_wh - return_wh,
                       "result.max_round_trip_distance.value": round_trip / 1000}, source, version)
        out.append(vector)
        return vector

    mph = 0.44704
    published = success(
        {"distance": "10 mi", "airspeed": "134.4 mph", "headwind": "30 mph", "power": "200 W",
         "remaining_energy": "100 Wh", "reserve_energy": "20 Wh"},
        10 * 1609.344, 134.4 * mph, 30 * mph, 200, 100, 20,
        "FAA Fostering Aviation Activities, Air Speed Indicator, pp. 23-25: 134.4 mph - 30 mph = 104.4 mph groundspeed",
        "Junior High Level aviation curriculum guide (1995)")
    published["tolerance"] = {"result.return_groundspeed.value": {"abs": 0.000001},
                              "result.return_time.value": {"rel": 1e-9}, "result.return_energy.value": {"rel": 1e-9},
                              "result.margin.value": {"rel": 1e-9}, "result.max_round_trip_distance.value": {"rel": 1e-9}}

    success({"distance": "1500 m", "airspeed": "54 km/h", "headwind": "36 km/h", "power": "0.18 kW",
             "remaining_energy": "144 kJ", "reserve_energy": "36 kJ"},
            1500, 15, 10, 180, 40, 10)
    success({"distance": "2 km", "airspeed": "12 m/s", "power": "150 W", "remaining_energy": "50 Wh", "reserve_energy": "5 Wh"},
            2000, 12, 0, 150, 50, 5)
    success({"distance": "800 m", "airspeed": "18 m/s", "headwind": "-5 m/s", "power": "300 W", "remaining_energy": "30 Wh", "reserve_energy": "10 Wh"},
            800, 18, -5, 300, 30, 10)
    success({"distance": "3 km", "airspeed": "20 m/s", "headwind": "19.5 m/s", "power": "250 W", "remaining_energy": "500 Wh", "reserve_energy": "15 Wh"},
            3000, 20, 19.5, 250, 500, 15)
    success({"distance": "1 km", "airspeed": "10 m/s", "headwind": "-9.5 m/s", "power": "120 W", "remaining_energy": "25 Wh"},
            1000, 10, -9.5, 120, 25)
    success({"distance": "500 m", "airspeed": "15 m/s", "headwind": "3 m/s", "power": "200 W", "remaining_energy": "10 Wh", "reserve_energy": "20 Wh"},
            500, 15, 3, 200, 10, 20)
    success({"distance": "1 m", "airspeed": "1 m/s", "headwind": "0 m/s", "power": "1 W", "remaining_energy": "1 Wh"},
            1, 1, 0, 1, 1)
    success({"distance": "25 km", "airspeed": "30 m/s", "headwind": "5 m/s", "power": "2 kW", "remaining_energy": "2 kWh", "reserve_energy": "0.5 kWh"},
            25000, 30, 5, 2000, 2000, 500)
    success({"distance": "6.2137119224 mi", "airspeed": "44.7387258411 mph", "headwind": "11.1846814603 mph", "power": "500 W",
             "remaining_energy": "250 Wh", "reserve_energy": "50 Wh"},
            10000, 20, 5, 500, 250, 50)
    success({"distance": "100 km", "airspeed": "50 m/s", "headwind": "-10 m/s", "power": "5 kW", "remaining_energy": "5 kWh", "reserve_energy": "1 kWh"},
            100000, 50, -10, 5000, 5000, 1000)

    failures = [
        ({"distance": "0 m", "airspeed": "15 m/s", "power": "180 W", "remaining_energy": "40 Wh"}, "INVALID_INPUT"),
        ({"distance": "1 km", "airspeed": "0 m/s", "power": "180 W", "remaining_energy": "40 Wh"}, "INVALID_INPUT"),
        ({"distance": "1 km", "airspeed": "15 m/s", "headwind": "15 m/s", "power": "180 W", "remaining_energy": "40 Wh"}, "NO_SOLUTION"),
        ({"distance": "1 km", "airspeed": "15 m/s", "headwind": "-15 m/s", "power": "180 W", "remaining_energy": "40 Wh"}, "NO_SOLUTION"),
        ({"distance": "1 km", "airspeed": "15 m/s", "power": "0 W", "remaining_energy": "40 Wh"}, "INVALID_INPUT"),
        ({"distance": "1 km", "airspeed": "15 m/s", "power": "180 W", "remaining_energy": "0 Wh"}, "INVALID_INPUT"),
        ({"distance": "1 km", "airspeed": "15 m/s", "power": "180 W", "remaining_energy": "40 Wh", "reserve_energy": "-1 Wh"}, "INVALID_INPUT"),
        ({"distance": "1 km", "airspeed": "15 m/s", "power": "-1 W", "remaining_energy": "40 Wh"}, "INVALID_INPUT"),
        ({"distance": "-1 km", "airspeed": "15 m/s", "power": "180 W", "remaining_energy": "40 Wh"}, "INVALID_INPUT"),
    ]
    for inp, code in failures:
        out.append(fvec(len(out) + 1, inp, {"ok": False, "error.code": code}, PW_SRC, PW_VER))
    return out


def altitude_vectors():
    cases = [({}, 400), ({"structure_height": "300 ft", "structure_distance": "200 ft"}, 700), ({"structure_height": "300 ft", "structure_distance": "401 ft"}, 400),
             ({"structure_height": "120 ft", "structure_distance": "400 ft"}, 520), ({"structure_height": "90 m", "structure_distance": "100 m"}, 90 / 0.3048 + 400)]
    return [fvec(i, inp, {"result.max_agl.value": v}, OPS_SRC, OPS_VER) for i, (inp, v) in enumerate(cases, 1)]


def speed_vectors():
    lim = 87 * 1852 / 1609.344
    cases = [(90, 15), (60, 20), (100, 0), (80, -10), (95, 6)]
    out = [fvec(i, {"airspeed": f"{a} mph", "tailwind": f"{w} mph"}, {"result.margin.value": lim - (a + w),
                                                                        "result.status": "over the limit" if a + w > lim else "within the limit"}, OPS_SRC, OPS_VER)
           for i, (a, w) in enumerate(cases, 1)]
    # In other units, at the limit itself, and with no wind given. The margin is in mph.
    mph = {"mph": 1.0, "kt": 1852 / 1609.344, "m/s": 3600 / 1609.344, "km/h": 1000 / 1609.344}
    more = [(87, "kt", None, None), (87.01, "kt", None, None), (86, "kt", 1, "kt"), (86, "kt", 2, "kt"), (40, "m/s", 0, "m/s"),
            (38, "m/s", 1.5, "m/s"), (150, "km/h", 12, "km/h"), (160, "km/h", 5, "km/h"), (20, "mph", None, None), (0, "mph", 30, "mph"),
            (70, "mph", 30, "mph"), (70, "mph", 31, "mph"), (45, "kt", -20, "kt"), (100.1, "mph", 0, "mph"), (100.2, "mph", 0, "mph")]
    for a, ua, w, uw in more:
        gs = a * mph[ua] + (w * mph[uw] if w is not None else 0)
        inp = {"airspeed": f"{a} {ua}"}
        if w is not None:
            inp["tailwind"] = f"{w} {uw}"
        out.append(fvec(len(out) + 1, inp, {"result.margin.value": lim - gs, "result.groundspeed.value": gs,
                                            "result.status": "over the limit" if gs > lim else "within the limit"}, OPS_SRC, OPS_VER))
    return out


def ke_vectors():
    cases = [(0.9, 19), (0.249, 16), (2.0, 20), (0.5, 12), (25.0, 23)]
    out = [fvec(i, {"mass": f"{m} kg", "speed": f"{v} m/s"}, {"result.energy.value": 0.5 * m * v * v,
                                                               "result.energy_ft_lbf.value": 0.5 * m * v * v / (0.3048 * 4.4482216152605)}, OPS_SRC, OPS_VER)
           for i, (m, v) in enumerate(cases, 1)]
    # Other units, and energies either side of the two thresholds: 80 J
    # (Regulation (EU) 2019/945, Annex Part 2) and 11 ft-lb (14 CFR 107.120(a)(1)).
    kg = {"kg": 1.0, "g": 0.001, "lb": 0.45359237}
    mps = {"m/s": 1.0, "km/h": 1 / 3.6, "mph": 0.44704, "kt": 1852 / 3600}
    more = [(900, "g", 13.3, "m/s"), (900, "g", 13.4, "m/s"), (1.0, "kg", 12.6, "m/s"), (1.0, "kg", 12.7, "m/s"), (0.3, "kg", 9.9, "m/s"), (0.3, "kg", 10.0, "m/s"),
            (250, "g", 19, "m/s"), (899, "g", 19, "m/s"), (4.0, "kg", 5.4, "m/s"), (4.0, "kg", 6.4, "m/s"), (2.2, "lb", 30, "mph"), (55, "lb", 87, "kt"),
            (1.5, "kg", 36, "km/h"), (0.5, "lb", 15, "mph"), (10, "kg", 0, "m/s")]
    for m, um, v, uv in more:
        ke = 0.5 * (m * kg[um]) * (v * mps[uv]) ** 2
        out.append(fvec(len(out) + 1, {"mass": f"{m} {um}", "speed": f"{v} {uv}"},
                        {"result.energy.value": ke, "result.energy_ft_lbf.value": ke / (0.3048 * 4.4482216152605)}, OPS_SRC, OPS_VER))
    return out


def easa_vectors():
    cases = [("2 kg", "none", "A3"), ("0.2 kg", "none", "A1 and A3"), ("0.8 kg", "c1", "A1 and A3"), ("3.5 kg", "c2", "A2 and A3"), ("12 kg", "c3", "A3"), ("0.24 kg", "c0", "A1 and A3")]
    out = [fvec(i, {"mass": m, "class_mark": c}, {"result.available": a}, OPS_SRC, OPS_VER) for i, (m, c, a) in enumerate(cases, 1)]
    # Every row of EASA's class label table (open category page, read
    # 2026-10-10), the 250 g and 25 kg lines for drones without a label
    # (Regulation (EU) 2019/947, Article 20), and what the open category excludes.
    easa = "EASA, Open category: low risk civil drones, class label table (read 2026-10-10), with Regulation (EU) 2019/947 Article 20 and Article 4(1)(b)"
    more = [("200 g", "c0", "A1 and A3"), ("0.5 lb", "c0", "A1 and A3"), ("899 g", "c1", "A1 and A3"), ("1.2 kg", "c2", "A2 and A3"), ("3.9 kg", "c2", "A2 and A3"),
            ("5 kg", "c3", "A3"), ("24 kg", "c3", "A3"), ("8 kg", "c4", "A3"), ("249 g", "none", "A1 and A3"), ("250 g", "none", "A3"), ("24.9 kg", "none", "A3")]
    for m, c, a in more:
        out.append(fvec(len(out) + 1, {"mass": m, "class_mark": c}, {"result.available": a}, easa, "2026-10-10"))
    for inp, code, field in [({"mass": "25 kg", "class_mark": "none"}, "OUT_OF_DOMAIN", "/mass"), ({"mass": "10 kg", "class_mark": "c5"}, "OUT_OF_DOMAIN", "/class_mark"),
                             ({"mass": "10 kg", "class_mark": "c6"}, "OUT_OF_DOMAIN", "/class_mark")]:
        out.append(fvec(len(out) + 1, inp, {"ok": False, "error.code": code, "error.field": field}, easa, "2026-10-10"))
    return out


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
    # Keep the map-placement vector at v015. GeodSolve is independent of the
    # Rust GeographicLib port used by the core.
    map_expect = {
        "ok": True,
        "result.footprint.0.lat.value": 40.0006368337481,
        "result.footprint.0.lon.value": -104.99960964823252,
        "result.footprint.1.lat.value": 39.99936316486702,
        "result.footprint.1.lon.value": -104.99960965548499,
        "result.footprint.2.lat.value": 39.99808944312047,
        "result.footprint.2.lon.value": -104.99648696463524,
        "result.footprint.3.lat.value": 40.001910449759414,
        "result.footprint.3.lon.value": -104.99648676881883,
    }
    out.append({
        "id": f"v{len(out) + 1:03d}",
        "input": {"height": "100 m", "pitch": "45 deg", "sensor_width": "13.2 mm", "focal_length": "8.8 mm", "image_width": 5472,
                  "image_height": 3648, "lat": 40.0, "lon": -105.0, "heading": "90 deg"},
        "expect": map_expect,
        "source": "The 45° scenario's corners (ahead and right on flat ground, as in the earlier vectors) placed from 40° N, 105° W looking east (90° true) by GeodSolve's direct problem on WGS 84: azimuth = heading + atan2(right, ahead), distance = hypot(ahead, right)",
        "sourceVersion": "GeodSolve: GeographicLib version 2.7",
        "tolerance": {k: {"abs": 1e-9} for k in map_expect if k != "ok"},
    })

    # Lassiter's published MATLAB program fixes this Canon camera and prints
    # 2.1, 3.6, and 7.5 cm for the near, median, and far rows at 100 m and 45°.
    # Its 14.9 mm height rounds to 3,465 rows at a 0.0043 mm pixel pitch, so
    # use the represented 14.8995 mm height here to compare the same edges.
    lassiter = {"height": "100 m", "pitch": "45 deg", "sensor_width": "22.3 mm", "sensor_height": "14.8995 mm",
                "focal_length": "24 mm", "image_width": 22.3 / 0.0043, "image_height": 3465}
    out.append(fvec(len(out) + 1, lassiter,
                    {"result.gsd_near.value": 2.1, "result.gsd_center_along.value": 3.6, "result.gsd_far.value": 7.5},
                    "Andrew Lassiter, uas-oblique-photo, uas_oblique_sim.m: printed min, median, and max GSD for its fixed Canon camera",
                    "commit 6c93edd08609a0a0430d92b744995d074bd6a270"))
    out[-1]["tolerance"] = {k: {"rel": 0, "abs": 0.05} for k in out[-1]["expect"] if k != "ok"}

    # Extend the closed-form sweep without changing any published vector id.
    for c, h, th in [(CAMS[0], 25, 5), (CAMS[1], 121.92, 55), (CAMS[2], 90, 55), (CAMS[3], 304.8, 20)]:
        sw, sh, f, iw, ih = c
        p, t = sw / iw, math.radians(th)
        ahead = lambda y: h * math.tan(t + math.atan(y / f))
        right = lambda x, y: h * x / (math.hypot(f, y) * math.cos(t + math.atan(y / f)))
        e = {"result.gsd_center.value": 100 * (right(p / 2, 0) - right(-p / 2, 0)),
             "result.gsd_center_along.value": 100 * (ahead(p / 2) - ahead(-p / 2)),
             "result.gsd_near.value": 100 * (ahead(-sh / 2 + p) - ahead(-sh / 2)),
             "result.gsd_far.value": 100 * (ahead(sh / 2) - ahead(sh / 2 - p)),
             "result.near_distance.value": ahead(-sh / 2), "result.far_distance.value": ahead(sh / 2)}
        out.append(fvec(len(out) + 1, dict(cam_inp(c), height=f"{h} m", pitch=f"{th} deg"), e, OBL_SRC, "4th edition (2014)"))
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
    # Found by the fuzzer: an area pasted 15 times over took 8 s and enclosed 300 times its size.
    sq = [{"lat": 0, "lon": 0}, {"lat": 0, "lon": dlon(300)}, {"lat": dlat(200), "lon": dlon(300)}, {"lat": dlat(200), "lon": 0}]
    out.append(fvec(len(out) + 1, {"area": sq * 15, "distance": "50 m"}, {"ok": False, "error.code": "INVALID_INPUT"}, SPEC, "2026"))
    # The scenario vector pinned the warning's place in the list, second after
    # EXPERIMENTAL_TOOL. A stable tool carries no such warning, so the same
    # case is held again by the code alone, wherever it sits.
    old = out[7]
    new = json.loads(json.dumps(old))
    new["id"] = f"v{len(out) + 1:03d}"
    del new["expect"]["meta.warnings.1.code"]
    new["expect"]["meta.warnings.*.code"] = "WAYPOINT_OUTSIDE_GEOFENCE"
    old["supersededBy"] = new["id"]
    old["reason"] = "Promotion to stable removed EXPERIMENTAL_TOOL from the tool's warnings, which moved WAYPOINT_OUTSIDE_GEOFENCE from index 1 to index 0; this vector pinned the old index. The answer is unchanged."
    out.append(new)
    return out


EXP_SRC = "OGC KML 2.3 section 9.13 altitude modes and RFC 7946, applied by hand (tools/vectors/gen_drone.py)"


def export_vectors():
    wps = [{"lat": 40.4406, "lon": -80.002, "height": "80 m", "heading": 90, "gimbal_pitch": -90, "action": "photo"},
           {"lat": 40.4406, "lon": -80.0005, "height": "80 m"}, {"lat": 40.4412, "lon": -80.0005, "height": "95 m"}]
    out = []
    modes = {"agl": "relativeToGround", "msl": "absolute", "takeoff": "absolute", "hae": "absolute"}
    labels = {"agl": "AGL", "msl": "MSL", "takeoff": "above takeoff", "hae": "HAE"}
    for ref in ["agl", "msl", "takeoff", "hae"]:
        for fmt, ext in [("kml", "kml"), ("geojson", "geojson"), ("csv", "csv")]:
            inp = {"waypoints": wps, "height_reference": ref, "format": fmt, "name": "North field"}
            if ref == "takeoff":
                inp["takeoff_elevation"] = "312 m"
            if ref == "hae":
                inp["geoid_height"] = "-33.9 m"
            mode = modes[ref] if fmt == "kml" else (f"{labels[ref]} in each waypoint's properties" if fmt == "geojson" else f"{labels[ref]} in the reference column")
            out.append(fvec(len(out) + 1, inp, {"result.altitude_mode": mode, "result.filename": f"north-field.{ext}", "result.waypoint_count": 3}, EXP_SRC, "2026"))
    # Without the takeoff elevation or the geoid height, KML cannot say where the heights are.
    for ref, field in [("takeoff", "takeoff_elevation"), ("hae", "geoid_height")]:
        out.append(fvec(len(out) + 1, {"waypoints": wps, "height_reference": ref}, {"ok": False, "error.code": "INVALID_INPUT", "error.field": f"/{field}"}, SPEC, "2026"))
    # Terrain following: blocked until the surface-model notice is acknowledged; then the margin rides on every waypoint.
    out.append(fvec(len(out) + 1, {"waypoints": wps, "height_reference": "terrain"},
                    {"ok": False, "error.code": "INVALID_INPUT", "error.field": "/surface_model_acknowledged"}, SPEC + " (terrain following requires acknowledgment)", "2026"))
    for margin, expect in [(None, 15.0), ("25 m", 25.0)]:
        inp = {"waypoints": wps, "height_reference": "terrain", "surface_model_acknowledged": "yes", "format": "kml", "name": "North field"}
        if margin:
            inp["clearance_margin"] = margin
        out.append(fvec(len(out) + 1, inp, {"result.altitude_mode": "absolute", "result.clearance_margin.value": expect, "result.waypoint_count": 3}, SPEC, "2026"))
    out.append(fvec(len(out) + 1, {"waypoints": wps, "height_reference": "terrain", "surface_model_acknowledged": "yes", "format": "csv"},
                    {"result.altitude_mode": "terrain-following MSL in the reference column", "result.clearance_margin.value": 15.0}, SPEC, "2026"))
    # Each file's registered media type: KML's from OGC KML 2.3, GeoJSON's from
    # RFC 7946 section 12, CSV's from RFC 4180 section 3.
    for fmt, media in [("kml", "application/vnd.google-earth.kml+xml"), ("geojson", "application/geo+json"), ("csv", "text/csv")]:
        out.append(fvec(len(out) + 1, {"waypoints": wps, "height_reference": "msl", "format": fmt},
                        {"result.media_type": media, "result.waypoint_count": 3}, "OGC KML 2.3, RFC 7946 section 12, and RFC 4180 section 3: the media type each registers", "2026"))
    return out


FULL_FRAME_DIAGONAL = math.hypot(36.0, 24.0)
ASPRS_C1 = "ASPRS Positional Accuracy Standards, Edition 2, Annex C Table C.1 rows and section 7.12 (tools/vectors/gen_drone.py)"
ASPRS_VER = "Edition 2, Version 1.0 (February 2023)"
SENSORS = [c[:3] for c in CAMS] + [(7.6, 5.7, 6.72), (13.2, 8.8, 10.26), (36.0, 24.0, 21.0), (23.5, 15.6, 25.0), (6.4, 4.8, 4.3), (12.75, 8.5, 10.6)]
HEIGHTS = [(80, "m"), (90, "m"), (120, "m"), (400, "ft"), (60, "m"), (35, "m"), (250, "ft"), (150, "m"), (30, "m"), (110, "m"), (200, "ft")]
FT_M = 0.3048


def refusal(i, inp, code, field=None):
    expected = {"ok": False, "error.code": code}
    if field:
        expected["error.field"] = field
    return vec(i, inp, expected, SPEC, "2026")


def sensor_inp(sensor, height=None):
    inp = {"sensor_width": f"{sensor[0]} mm", "sensor_height": f"{sensor[1]} mm", "focal_length": f"{sensor[2]} mm"}
    if height:
        inp["height"] = f"{height[0]} {height[1]}"
    return inp


def meters(height):
    return height[0] * (FT_M if height[1] == "ft" else 1.0)


def footprint_vectors():
    rng = random.Random(55101)
    out = []

    def case(i, sensor, height, equivalent=False):
        sw, sh, f = sensor
        h = meters(height)
        inp = sensor_inp(sensor, height)
        if equivalent:
            crop = FULL_FRAME_DIAGONAL / math.hypot(sw, sh)
            inp["focal_length"] = f"{f * crop} mm"
            inp["focal_length_type"] = "equivalent-35mm"
            f = (f * crop) / crop
        across, along = sw * h / f, sh * h / f
        v = vec(i, inp, {"result.footprint_across.value": across, "result.footprint_along.value": along,
                         "result.footprint_area.value": across * along, "result.focal_length_used.value": float(f)})
        for key in v["tolerance"]:
            v["tolerance"][key] = {"rel": 1e-11, "abs": 1e-12}
        return v

    out.append(case(1, SENSORS[0], (80, "m")))
    while len(out) < 20:
        sensor, height = SENSORS[len(out) % len(SENSORS)], rng.choice(HEIGHTS)
        out.append(case(len(out) + 1, sensor, height, equivalent=len(out) % 5 == 4))
    base = sensor_inp(SENSORS[0], (80, "m"))
    out.append(refusal(len(out) + 1, {**base, "height": "0 m"}, "INVALID_INPUT", "/height"))
    out.append(refusal(len(out) + 1, {**base, "sensor_width": "0 mm"}, "INVALID_INPUT", "/sensor_width"))
    return out


def crop_vectors():
    out = []

    def case(i, sensor, give):
        sw, sh, f = sensor
        diag = math.hypot(sw, sh)
        crop = FULL_FRAME_DIAGONAL / diag
        inp = {"sensor_width": f"{sw} mm", "sensor_height": f"{sh} mm"}
        exp = {"result.crop_factor": crop, "result.sensor_diagonal.value": diag}
        if give == "physical":
            inp["focal_length"] = f"{f} mm"
            exp.update({"result.equivalent_focal_length.value": f * crop, "result.focal_length.value": float(f)})
        elif give == "equivalent":
            inp["equivalent_focal_length"] = "24 mm"
            exp.update({"result.equivalent_focal_length.value": 24.0, "result.focal_length.value": 24 / crop})
        v = vec(i, inp, exp)
        for key in v["tolerance"]:
            v["tolerance"][key] = {"rel": 1e-11, "abs": 1e-12}
        return v

    out.append(case(1, SENSORS[0], "physical"))
    # A full frame is its own reference: crop factor 1 exactly.
    out.append(vec(2, {"sensor_width": "36 mm", "sensor_height": "24 mm", "focal_length": "35 mm"},
                   {"result.crop_factor": 1.0, "result.equivalent_focal_length.value": 35.0}))
    more = SENSORS + [(4.8, 3.6, 3.6), (8.8, 6.6, 7.0), (43.8, 32.9, 40.0), (5.6, 4.2, 4.0), (22.3, 14.9, 18.0), (17.3, 13.0, 25.0), (6.3, 4.7, 5.0)]
    while len(out) < 20:
        out.append(case(len(out) + 1, more[len(out) % len(more)], ["physical", "equivalent", "none"][len(out) % 3]))
    out.append(refusal(len(out) + 1, {"sensor_width": "13.2 mm", "sensor_height": "0 mm"}, "INVALID_INPUT", "/sensor_width"))
    out.append(refusal(len(out) + 1, {"sensor_width": "13.2 mm", "sensor_height": "8.8 mm", "focal_length": "8.8 mm", "equivalent_focal_length": "24 mm"},
                       "INVALID_INPUT", "/equivalent_focal_length"))
    return out


def spacing_vectors(mode):
    rng = random.Random({"trigger-distance": 55102, "line-spacing": 55103, "trigger-interval": 55104}[mode])
    out = []

    def case(i, sensor, height, overlap=None, portrait=False, speed=None, min_interval=None):
        sw, sh, f = sensor
        h = meters(height)
        across, along = sw * h / f, sh * h / f
        if portrait:
            across, along = along, across
        inp = sensor_inp(sensor, height)
        if portrait:
            inp["orientation"] = "portrait"
        if mode == "line-spacing":
            side = 60 if overlap is None else overlap
            if overlap is not None:
                inp["side_overlap"] = overlap
            exp = {"result.line_spacing.value": across * (1 - side / 100), "result.footprint_across.value": across}
        else:
            front = 75 if overlap is None else overlap
            if overlap is not None:
                inp["front_overlap"] = overlap
            dist = along * (1 - front / 100)
            exp = {"result.trigger_distance.value": dist}
            if mode == "trigger-distance":
                exp["result.footprint_along.value"] = along
            else:
                inp["groundspeed"] = f"{speed} m/s"
                exp["result.trigger_interval.value"] = dist / speed
                if min_interval:
                    inp["min_interval"] = f"{min_interval} s"
                    exp["result.max_groundspeed.value"] = dist / min_interval
        v = vec(i, inp, exp)
        for key in v["tolerance"]:
            v["tolerance"][key] = {"rel": 1e-11, "abs": 1e-12}
        return v

    if mode == "trigger-distance":
        out.append(case(1, SENSORS[0], (90, "m"), 80))
    elif mode == "line-spacing":
        out.append(case(1, SENSORS[0], (90, "m"), 70))
    else:
        out.append(case(1, SENSORS[0], (90, "m"), 80, speed=12, min_interval=2))
    while len(out) < 20:
        sensor, height = SENSORS[len(out) % len(SENSORS)], rng.choice(HEIGHTS)
        overlap = None if len(out) % 6 == 5 else rng.choice([50, 60, 65, 70, 75, 80, 85, 90])
        speed = rng.choice([3, 5, 8, 10, 12, 15, 18, 22])
        out.append(case(len(out) + 1, sensor, height, overlap, portrait=len(out) % 4 == 3, speed=speed,
                        min_interval=rng.choice([None, 0.7, 1, 2, 2.5])))
    base = sensor_inp(SENSORS[0], (90, "m"))
    if mode == "trigger-interval":
        base["groundspeed"] = "10 m/s"
        out.append(refusal(len(out) + 1, {**base, "groundspeed": "0 m/s"}, "INVALID_INPUT", "/groundspeed"))
        out.append(refusal(len(out) + 1, {**base, "min_interval": "0 s"}, "INVALID_INPUT", "/min_interval"))
    out.append(refusal(len(out) + 1, {**base, "height": "-5 m"}, "INVALID_INPUT", "/height"))
    out.append(refusal(len(out) + 1, {**base, "focal_length": "0 mm"}, "INVALID_INPUT", "/sensor_width"))
    return out


def shutter_vectors():
    rng = random.Random(55105)
    out = []

    def case(i, speed, gsd_cm, limit=None, speed_unit="m/s"):
        v_ms = speed * {"m/s": 1.0, "km/h": 1 / 3.6, "mph": 0.44704}[speed_unit]
        used = 0.5 if limit is None else limit
        inp = {"groundspeed": f"{speed} {speed_unit}", "gsd": f"{gsd_cm} cm"}
        if limit is not None:
            inp["blur_limit"] = limit
        v = vec(i, inp, {"result.max_exposure.value": used * (gsd_cm / 100) / v_ms, "result.blur_limit": float(used)})
        for key in v["tolerance"]:
            v["tolerance"][key] = {"rel": 1e-11, "abs": 1e-15}
        return v

    out.append(case(1, 12, 2.5))
    out[0]["expect"]["result.max_exposure_fraction"] = "1/960 s"
    # Han and Han's case in the parent's vectors: 10 m/s over 2 cm at half a pixel is 1/1,000 s.
    out.append(case(2, 10, 2.0))
    while len(out) < 20:
        out.append(case(len(out) + 1, rng.choice([2, 4, 5, 8, 10, 15, 18, 22, 36, 54]), rng.choice([0.5, 0.8, 1.0, 1.5, 2.0, 2.74, 3.0, 5.0, 10.0]),
                        rng.choice([None, 0.25, 0.5, 1.0, 1.5]), ["m/s", "km/h", "mph"][len(out) % 3]))
    out.append(refusal(len(out) + 1, {"groundspeed": "0 m/s", "gsd": "2 cm"}, "NO_SOLUTION", "/groundspeed"))
    out.append(refusal(len(out) + 1, {"groundspeed": "10 m/s", "gsd": "0 cm"}, "INVALID_INPUT", "/exposure"))
    return out


def checkpoint_vectors():
    out = []

    def table_c1(km2):
        # Table C.1 as printed: project area (km²) against total checkpoints.
        rows = [(500, 30), (750, 35), (1000, 40), (1250, 45), (1500, 50), (1750, 55), (2000, 60), (2250, 65), (2500, 70)]
        area = math.floor(km2 + 0.5)
        return next(n for limit, n in rows if area <= limit)

    def case(i, area, unit_name, vegetated=None, h_cm=None, v_cm=None):
        km2 = area * {"km2": 1.0, "ha": 0.01, "acre": 0.0040468564224, "mi2": 2.589988110336}[unit_name]
        inp = {"area_size": f"{area} {unit_name}"}
        share = (vegetated or 0) / 100
        open_n = table_c1(km2 * (1 - share))
        exp = {"result.checkpoints_open": float(open_n)}
        total = open_n
        if vegetated:
            inp["vegetated"] = vegetated
            veg_n = table_c1(km2 * share)
            exp["result.checkpoints_vegetated"] = float(veg_n)
            total += veg_n
        exp["result.checkpoints"] = float(total)
        if h_cm:
            inp["horizontal_class"] = f"{h_cm} cm"
            exp["result.checkpoint_rmse_h.value"] = h_cm / 2
        if v_cm:
            inp["vertical_class"] = f"{v_cm} cm"
            exp["result.checkpoint_rmse_v.value"] = v_cm / 2
        v = vec(i, inp, exp, ASPRS_C1, ASPRS_VER)
        for key in v["tolerance"]:
            v["tolerance"][key] = {"rel": 1e-12, "abs": 1e-9}
        return v

    out.append(case(1, 12, "km2", 25, 5, 10))
    # Every row of Table C.1, at its upper edge.
    for area in [500, 750, 1000, 1250, 1500, 1750, 2000, 2250, 2500]:
        out.append(case(len(out) + 1, area, "km2"))
    # Just past each of the first edges, and small projects in the units a drone job uses.
    out.append(case(len(out) + 1, 501, "km2"))
    out.append(case(len(out) + 1, 751, "km2", None, 10))
    out.append(case(len(out) + 1, 40, "ha", None, 2.5, 5))
    out.append(case(len(out) + 1, 160, "acre", 50, None, 15))
    out.append(case(len(out) + 1, 3, "mi2", 10, 7.5))
    out.append(case(len(out) + 1, 2400, "km2", 50, 30, 30))
    out.append(case(len(out) + 1, 3000, "km2", 40))
    out.append(case(len(out) + 1, 1800, "km2", 30, 20, 40))
    out.append(case(len(out) + 1, 0.5, "km2", 100, 3, 6))
    out.append(case(len(out) + 1, 900, "km2", 5))
    out.append(refusal(len(out) + 1, {"area_size": "2600 km2"}, "OUT_OF_DOMAIN", "/area"))
    out.append(refusal(len(out) + 1, {"area_size": "0 km2"}, "INVALID_INPUT", "/area_size"))
    out.append(refusal(len(out) + 1, {"area_size": "10 km2", "horizontal_class": "0 cm"}, "INVALID_INPUT", "/horizontal_class"))
    return out


POWER_SPEC = "add-drone-suite power requirements, evaluated in Python (tools/vectors/gen_drone.py)"


def pvec(i, inp, exp, rel=1e-11):
    v = vec(i, inp, exp, POWER_SPEC, "2026")
    for key in v["tolerance"]:
        v["tolerance"][key] = {"rel": rel, "abs": 1e-12}
    return v


def cold(t_c):
    """The labeled rule of thumb: 0% at 20 °C, 20% at 0 °C, capped at 50%."""
    return 0.0 if t_c >= 20 else min((20 - t_c) * 0.01, 0.5)


def usable_energy_vectors():
    rng = random.Random(55201)
    out = []

    def case(i, wh, usable=None, reserve=None, derating=None, temp_c=None):
        inp = {"energy": f"{wh} Wh"}
        d = 0.0
        if usable is not None:
            inp["usable"] = usable
        if reserve is not None:
            inp["reserve"] = reserve
        if derating is not None:
            inp["derating"] = derating
            d = derating / 100
        elif temp_c is not None:
            inp["battery_temperature"] = f"{temp_c} degC"
            d = cold(temp_c)
        avail = wh * (100 if usable is None else usable) / 100 * (1 - d)
        r = (reserve or 0) / 100
        return pvec(i, inp, {"result.flyable_energy.value": avail * (1 - r), "result.usable_energy.value": avail,
                             "result.reserve_energy.value": avail * r, "result.derating_applied": d * 100})

    out.append(case(1, 99, 85, 25))
    out.append(case(2, 90.4))
    while len(out) < 20:
        wh = rng.choice([43.6, 77, 90.4, 99, 129.96, 263.2, 274, 588, 1000])
        usable = rng.choice([None, 80, 85, 90, 95, 100])
        reserve = rng.choice([None, 10, 15, 20, 25, 30])
        which = len(out) % 3
        out.append(case(len(out) + 1, wh, usable, reserve, derating=rng.choice([5, 10, 12.5, 30]) if which == 1 else None,
                        temp_c=rng.choice([-25, -10, -5, 0, 5, 10, 15, 20, 30]) if which == 2 else None))
    out.append(refusal(len(out) + 1, {"energy": "0 Wh"}, "INVALID_INPUT", "/energy"))
    out.append(refusal(len(out) + 1, {"energy": "-5 Wh", "usable": 80}, "INVALID_INPUT", "/energy"))
    return out


def derating_vectors():
    out = []
    temps = [(5, "degC"), (20, "degC"), (0, "degC"), (-10, "degC"), (-30, "degC"), (-45, "degC"), (35, "degC"), (19, "degC"),
             (12.5, "degC"), (-0.5, "degC"), (32, "degF"), (68, "degF"), (50, "degF"), (14, "degF"), (-4, "degF"),
             (273.15, "K"), (283.15, "K"), (263.15, "K"), (293.15, "K"), (300, "K")]
    for i, (t, u) in enumerate(temps, 1):
        t_c = t if u == "degC" else (t - 32) / 1.8 if u == "degF" else t - 273.15
        d = cold(t_c)
        inp = {"battery_temperature": f"{t} {u}"}
        exp = {"result.derating_applied": d * 100, "result.share_remaining": (1 - d) * 100}
        if i % 2:
            inp["energy"] = "99 Wh"
            exp["result.energy_remaining.value"] = 99 * (1 - d)
        v = pvec(i, inp, exp, rel=1e-9)
        for key in v["tolerance"]:
            v["tolerance"][key]["abs"] = 1e-9
        out.append(v)
    out.append(refusal(len(out) + 1, {"battery_temperature": "5 degC", "energy": "0 Wh"}, "INVALID_INPUT", "/energy"))
    return out


def range_vectors():
    rng = random.Random(55202)
    out = []

    def case(i, wh, power, speed, usable=None, reserve=None, derating=None, wind=None):
        inp = {"energy": f"{wh} Wh", "power": f"{power} W"}
        gs = float(speed)
        if wind is None:
            inp["groundspeed"] = f"{speed} m/s"
        else:
            course, wind_from, wind_speed = wind
            inp.update({"airspeed": f"{speed} m/s", "course": f"{course} deg", "wind_direction": f"{wind_from} deg", "wind_speed": f"{wind_speed} m/s"})
            # The wind triangle: crab into the wind to hold the course.
            angle = math.radians(wind_from - course)
            cross, head = wind_speed * math.sin(angle), wind_speed * math.cos(angle)
            gs = math.sqrt(speed * speed - cross * cross) - head
        if usable is not None:
            inp["usable"] = usable
        if reserve is not None:
            inp["reserve"] = reserve
        if derating is not None:
            inp["derating"] = derating
        avail = wh * (100 if usable is None else usable) / 100 * (1 - (derating or 0) / 100)
        hours = avail * (1 - (reserve or 0) / 100) / power
        return pvec(i, inp, {"result.range.value": hours * gs * 3.6, "result.flight_time.value": hours * 60,
                             "result.groundspeed_used.value": gs, "result.usable_energy.value": avail}, rel=1e-9)

    out.append(case(1, 99, 180, 14, 85, 25))
    # A direct headwind and a direct tailwind change the groundspeed by the wind speed.
    out.append(case(2, 99, 180, 14, 85, 25, wind=(90, 90, 4)))
    out.append(case(3, 99, 180, 14, 85, 25, wind=(90, 270, 4)))
    while len(out) < 20:
        wh = rng.choice([43.6, 77, 99, 129.96, 274, 588])
        power = rng.choice([90, 150, 180, 240, 420, 900])
        speed = rng.choice([8, 10, 12, 15, 18, 22])
        wind = (rng.randrange(0, 360), rng.randrange(0, 360), rng.choice([2, 3, 5, 6])) if len(out) % 2 else None
        out.append(case(len(out) + 1, wh, power, speed, rng.choice([None, 80, 90]), rng.choice([None, 15, 20, 30]),
                        rng.choice([None, 10]), wind))
    base = {"energy": "99 Wh", "power": "180 W"}
    out.append(refusal(len(out) + 1, base, "INVALID_INPUT", "/groundspeed"))
    out.append(refusal(len(out) + 1, {**base, "groundspeed": "0 m/s"}, "INVALID_INPUT", "/groundspeed"))
    out.append(refusal(len(out) + 1, {**base, "airspeed": "10 m/s", "course": "0 deg", "wind_direction": "0 deg", "wind_speed": "12 m/s"}, "NO_SOLUTION", "/wind_speed"))
    out.append(refusal(len(out) + 1, {**base, "groundspeed": "10 m/s", "power": "0 W"}, "INVALID_INPUT", "/power"))
    return out


def disk_area_vectors():
    out = []
    to_m = {"in": 0.0254, "cm": 0.01, "m": 1.0, "mm": 0.001}
    cases = [(6, 15, "in", 6.2), (4, 9.4, "in", 1.4), (4, 5, "in", 0.249), (8, 28, "in", 25), (6, 21, "in", None), (4, 13, "in", 3.6),
             (1, 2.4, "m", 9.5), (2, 1.2, "m", None), (3, 30, "cm", 2.2), (4, 38.1, "cm", 4.0), (6, 533, "mm", 12.5), (8, 18, "in", 9.0),
             (12, 30, "in", 60), (4, 7, "in", None), (4, 10, "in", 1.9), (2, 60, "cm", 5.5), (16, 40, "cm", 80), (6, 17, "in", 8.4),
             (4, 24, "in", 14), (8, 15, "in", None)]
    for i, (n, d, u, mass) in enumerate(cases, 1):
        dm = d * to_m[u]
        area = n * math.pi * dm * dm / 4
        inp = {"rotors": n, "rotor_diameter": f"{d} {u}"}
        exp = {"result.disk_area.value": area, "result.area_per_rotor.value": area / n}
        if mass is not None:
            inp["mass"] = f"{mass} kg"
            exp["result.disk_loading.value"] = mass * 9.80665 / area
        out.append(pvec(i, inp, exp))
    out.append(refusal(len(out) + 1, {"rotors": 4, "rotor_diameter": "0 in"}, "INVALID_INPUT", "/rotor_diameter"))
    out.append(refusal(len(out) + 1, {"rotors": 4.5, "rotor_diameter": "10 in"}, "INVALID_INPUT", "/rotors"))
    out.append(refusal(len(out) + 1, {"rotors": 4, "rotor_diameter": "10 in", "mass": "0 kg"}, "INVALID_INPUT", "/mass"))
    return out


def c_rate_vectors():
    out = []
    cases = [(10000, None, 6, "lipo", 480), (5870, 15.4, None, None, 150.6), (5000, 14.8, None, None, 300), (2200, 11.1, None, None, 180),
             (1500, None, 4, "lipo", 600), (22000, 44.4, None, None, 2400), (16000, None, 12, "lipo", 3200), (3000, None, 6, "li-ion", 90),
             (850, None, 4, "lipo", 420), (4500, 22.2, None, None, 250), (7000, 44.4, None, None, 1000), (1300, 14.8, None, None, 700),
             (3850, 15.4, None, None, 95), (12000, 22.8, None, None, 520), (6000, None, 4, "li-ion", 60), (30000, 51.8, None, None, 1800),
             (2500, 7.4, None, None, 40), (5200, None, 3, "lipo", 130), (8000, 14.8, None, None, 450), (17500, None, 14, "li-ion", 2100)]
    for i, (mah, volts, cells, chem, power) in enumerate(cases, 1):
        inp = {"capacity": f"{mah} mAh", "power": f"{power} W"}
        if volts is None:
            inp["cells"] = cells
            if chem != "lipo":
                inp["chemistry"] = chem
            volts = cells * (3.6 if chem == "li-ion" else 3.7)
        else:
            inp["voltage"] = f"{volts} V"
        current = power / volts
        out.append(pvec(i, inp, {"result.c_rate": current / (mah / 1000), "result.current": current, "result.voltage.value": float(volts)}))
    base = {"capacity": "5000 mAh", "voltage": "14.8 V"}
    out.append(refusal(len(out) + 1, {**base, "power": "0 W"}, "INVALID_INPUT", "/power"))
    out.append(refusal(len(out) + 1, {"capacity": "5000 mAh", "power": "100 W"}, "INVALID_INPUT", "/voltage"))
    out.append(refusal(len(out) + 1, {**base, "cells": 4, "power": "100 W"}, "INVALID_INPUT", "/cells"))
    return out


CFR_107_51 = "14 CFR 107.51(b), evaluated in Python (tools/vectors/gen_drone.py)"
CFR_VER = "eCFR, as dated in data/regulations.json"


def structure_altitude_vectors():
    rng = random.Random(55301)
    out = []
    to_ft = {"ft": 1.0, "m": 1 / FT_M}

    def case(i, height, distance, unit_name="ft", ground=None):
        h, d = height * to_ft[unit_name], distance * to_ft[unit_name]
        limit = max(h + 400, 400) if d <= 400 else 400.0
        inp = {"structure_height": f"{height} {unit_name}", "structure_distance": f"{distance} {unit_name}"}
        exp = {"result.max_agl.value": float(limit)}
        if ground is not None:
            inp["ground_elevation"] = f"{ground} ft"
            exp["result.max_msl.value"] = float(ground + limit)
        v = vec(i, inp, exp, CFR_107_51, CFR_VER)
        for key in v["tolerance"]:
            v["tolerance"][key] = {"rel": 1e-11, "abs": 1e-9}
        return v

    out.append(case(1, 1150, 250))
    # The rule's edge: exactly 400 ft away is still within the radius; a foot more is not.
    out.append(case(2, 300, 400))
    out.append(case(3, 300, 401))
    # The parent page's example and its MSL form.
    out.append(case(4, 300, 200, ground=5280))
    while len(out) < 20:
        unit_name = ["ft", "m"][len(out) % 2]
        height = round(rng.uniform(20, 2000) / to_ft[unit_name], 1)
        distance = round(rng.uniform(0, 900) / to_ft[unit_name], 1)
        out.append(case(len(out) + 1, height, distance, unit_name, ground=rng.choice([None, 0, 640, 5280, 9100])))
    out.append(refusal(len(out) + 1, {"structure_height": "-10 ft", "structure_distance": "100 ft"}, "INVALID_INPUT", "/structure_height"))
    out.append(refusal(len(out) + 1, {"structure_height": "100 ft", "structure_distance": "-1 ft"}, "INVALID_INPUT", "/structure_distance"))
    return out


def rect_sides(la, lo, dla, dlo):
    """North-south and east-west sides of a small rectangle, in meters."""
    mid = la + dla / 2
    a, f = 6378137.0, 1 / 298.257223563
    e2 = f * (2 - f)
    n_rad = a / math.sqrt(1 - e2 * math.sin(math.radians(mid)) ** 2)
    return dla * meters_per_deg_lat(mid), dlo * math.pi / 180 * n_rad * math.cos(math.radians(mid))


def crosshatch_vectors():
    """Line counts of both passes over rectangles: the first across the
    narrow side, the second across the long one. Each pass centers n lines
    on its extent so that they span it: (n − 1) × spacing ≥ extent."""
    out = []
    cases = [(35.0, -100.0, 0.0018, 0.00296, 45.0), (40.0, -105.0, 0.00135, 0.00705, 52.5), (35.0, -100.0, 0.009, 0.002, 40.0),
             (51.0, 0.1, 0.004, 0.012, 60.0), (-33.0, 151.0, 0.0022, 0.0031, 25.0), (60.0, 25.0, 0.02, 0.005, 100.0),
             (10.0, 10.0, 0.003, 0.0052, 35.0), (-45.0, 170.0, 0.0061, 0.0033, 55.0), (48.0, 2.0, 0.0027, 0.0091, 70.0),
             (25.0, 55.0, 0.0047, 0.0019, 30.0), (0.5, -78.0, 0.0036, 0.0067, 48.0), (64.0, -21.0, 0.0029, 0.0152, 80.0),
             (-12.0, -77.0, 0.0078, 0.0026, 65.0), (37.5, 127.0, 0.0017, 0.0044, 22.0), (19.0, 72.8, 0.0054, 0.0092, 90.0),
             (44.0, -93.0, 0.0023, 0.0083, 38.0), (-26.0, 28.0, 0.0066, 0.0041, 75.0), (55.7, 12.5, 0.0019, 0.0077, 28.0),
             (30.0, 31.2, 0.0083, 0.0034, 58.0), (-1.3, 36.8, 0.0044, 0.0071, 42.0)]
    for i, (la, lo, dla, dlo, sp) in enumerate(cases, 1):
        ns, ew = rect_sides(la, lo, dla, dlo)
        lines = (math.ceil(min(ns, ew) / sp - 1e-6) + 1) + (math.ceil(max(ns, ew) / sp - 1e-6) + 1)
        out.append(vec(i, {"area": rect(la, lo, dla, dlo), "line_spacing": f"{sp} m", "photo_spacing": "25 m"}, {"result.lines": float(lines)}, MIS_SRC))
    area = rect(35.0, -100.0, 0.0018, 0.00296)
    out.append(refusal(len(out) + 1, {"area": area, "line_spacing": "0.1 m", "photo_spacing": "25 m"}, "OUT_OF_DOMAIN", "/line_spacing"))
    out.append(refusal(len(out) + 1, {"area": area[:2], "line_spacing": "45 m", "photo_spacing": "25 m"}, "INVALID_INPUT"))
    return out


def geofence_check_vectors():
    """Waypoints a known distance due north of a rectangle's north-west corner,
    where the nearest point of the area is the corner and the distance to it
    runs along a meridian. (Off the middle of an edge the geodesic edge bows
    a few millimeters toward the pole, which a meridian arc does not model.)"""
    rng = random.Random(55302)
    out = []

    def case(i, la, lo, dla, dlo, fence, offsets, warning=None):
        mid_lon = lo + dlo / 2
        per_deg = meters_per_deg_lat(la + dla)
        wps, flagged, outside, in_zone = [], [], 0, 0
        for off in offsets:
            if off >= 0:
                wps.append({"lat": la + dla + off / per_deg, "lon": lo})
            else:
                wps.append({"lat": la + dla / 2, "lon": mid_lon})
            if off > fence:
                outside += 1
                flagged.append(off - fence)
            elif warning is not None and off > warning:
                in_zone += 1
                flagged.append(off - warning)
        inp = {"area": rect(la, lo, dla, dlo), "distance": f"{fence} m", "waypoints": wps}
        exp = {"result.outside_count": float(outside), "result.waypoints_checked": float(len(wps))}
        if warning is not None:
            inp["warning_distance"] = f"{warning} m"
            exp["result.warning_count"] = float(in_zone)
        for j, beyond in enumerate(flagged):
            exp[f"result.flagged.{j}.beyond.value"] = beyond
        v = vec(i, inp, exp, MIS_SRC)
        for key in v["tolerance"]:
            # Counts are exact; distances carry the meridian-arc approximation.
            v["tolerance"][key] = {"abs": 0.001} if key.endswith("beyond.value") else {"abs": 0}
        return v

    out.append(case(1, 41.0, -96.0, 0.002, 0.003, 40, [-1, 22.2, 66.6, -1]))
    while len(out) < 20:
        la, lo = rng.choice([(40.0, -105.0), (51.0, 0.1), (-33.0, 151.0), (60.0, 25.0), (10.0, 10.0), (-45.0, 170.0)])
        fence = rng.choice([20, 30, 50, 75, 100])
        warning = rng.choice([None, fence * 0.5, fence * 0.6])
        offsets = [rng.choice([-1, round(rng.uniform(0, fence * 0.4), 1), round(rng.uniform(fence * 0.7, fence * 0.95), 1),
                               round(rng.uniform(fence * 1.1, fence * 1.9), 1)]) for _ in range(rng.randrange(1, 7))]
        out.append(case(len(out) + 1, la, lo, round(rng.uniform(0.001, 0.004), 4), round(rng.uniform(0.002, 0.006), 4), fence, offsets, warning))
    area = rect(41.0, -96.0, 0.002, 0.003)
    wp = [{"lat": 41.001, "lon": -95.9985}]
    out.append(refusal(len(out) + 1, {"area": area, "distance": "0 m", "waypoints": wp}, "INVALID_INPUT", "/distance"))
    out.append(refusal(len(out) + 1, {"area": area, "distance": "40 m", "warning_distance": "50 m", "waypoints": wp}, "INVALID_INPUT", "/warning_distance"))
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    selected = set(sys.argv[2:])
    files = {"drone.ops.structure-altitude": structure_altitude_vectors(), "drone.mission.crosshatch": crosshatch_vectors(),
             "drone.mission.geofence-check": geofence_check_vectors(),
             "drone.power.usable-energy": usable_energy_vectors(), "drone.power.temperature-derating": derating_vectors(),
             "drone.power.range": range_vectors(), "drone.power.disk-area": disk_area_vectors(), "drone.power.c-rate": c_rate_vectors(),
             "drone.photogrammetry.footprint": footprint_vectors(), "drone.photogrammetry.crop-factor": crop_vectors(),
             "drone.photogrammetry.trigger-distance": spacing_vectors("trigger-distance"),
             "drone.photogrammetry.line-spacing": spacing_vectors("line-spacing"),
             "drone.photogrammetry.trigger-interval": spacing_vectors("trigger-interval"),
             "drone.photogrammetry.max-shutter": shutter_vectors(),
             "drone.photogrammetry.checkpoint-requirements": checkpoint_vectors(),
             "drone.photogrammetry.gsd": gsd(), "drone.photogrammetry.altitude-for-gsd": alt(), "drone.photogrammetry.trigger": trigger(),
             "drone.photogrammetry.motion-blur": blur(), "drone.photogrammetry.asprs-accuracy": asprs(),
             "drone.power.battery-energy": battery_vectors(), "drone.power.hover-power": hover_vectors(), "drone.power.calibrate-hover": calibrate_vectors(),
             "drone.power.endurance": endurance_vectors(), "drone.power.max-payload": payload_vectors(), "drone.power.payload-impact": payload_impact_vectors(),
             "drone.power.rth-budget": rth_vectors(), "drone.ops.part107-altitude": altitude_vectors(),
             "drone.ops.speed-check": speed_vectors(), "drone.ops.kinetic-energy": ke_vectors(),
             "drone.ops.easa-subcategory": easa_vectors(), "drone.sensors.vlos": vlos_vectors(),
             "drone.mission.survey-grid": grid_vectors("grid"), "drone.photogrammetry.image-count": grid_vectors("count"),
             "drone.mission.corridor": corridor_vectors(), "drone.mission.orbit": orbit_vectors(),
             "drone.photogrammetry.oblique-gsd": oblique_vectors(),
             "drone.photogrammetry.terrain-overlap": terrain_vectors(),
             "drone.mission.facade": facade_vectors(),
             "drone.mission.geofence": geofence_vectors(),
             "drone.mission.export": export_vectors()}
    for tool, vs in files.items():
        if selected and tool not in selected:
            continue
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
