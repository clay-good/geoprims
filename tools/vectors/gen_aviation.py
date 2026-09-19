#!/usr/bin/env python3
"""Golden vectors for aviation slice 1 (core/vectors/aviation.*.jsonl).

Expected values come from an independent Python implementation of the
defining equations (ICAO Doc 7488/3, US 1976, the wind triangle), written
separately from the Rust core, plus published US Standard Atmosphere 1976
table rows at their printed precision. Rerunning must reproduce the files.
"""
import json
import math
import random
import sys
from pathlib import Path

P0, T0, G0, R, R0 = 101325.0, 288.15, 9.80665, 287.05287, 6356766.0
FT, INHG = 0.3048, 3386.389
LAYERS = [(0, -0.0065, 288.15), (11000, 0.0, 216.65), (20000, 0.001, 216.65), (32000, 0.0028, 228.65),
          (47000, 0.0, 270.65), (51000, -0.0028, 270.65), (71000, -0.002, 214.65)]

SRC = "Independent Python implementation of the ICAO Doc 7488/3 defining equations (tools/vectors/gen_aviation.py)"
VER = "Doc 7488/3 (1993)"
TABLE = "U.S. Standard Atmosphere 1976, Table I (geometric altitude), printed to 5 significant figures"


def bases():
    out = [(288.15, P0)]
    for i in range(1, len(LAYERS)):
        hb, l, tb = LAYERS[i - 1]
        dh = LAYERS[i][0] - hb
        pb = out[-1][1]
        p = pb * math.exp(-G0 * dh / (R * tb)) if l == 0 else pb * ((tb + l * dh) / tb) ** (-G0 / (R * l))
        out.append((LAYERS[i][2], p))
    return out


def isa(h):
    i = max(k for k in range(len(LAYERS)) if h >= LAYERS[k][0]) if h >= 0 else 0
    hb, l, tb = LAYERS[i]
    pb = bases()[i][1]
    t = tb + l * (h - hb)
    p = pb * math.exp(-G0 * (h - hb) / (R * tb)) if l == 0 else pb * (t / tb) ** (-G0 / (R * l))
    return t, p, p / (R * t)


def alt_for(value, kind):
    """Bisection on the monotonic ISA profile: independent of the core's closed forms."""
    lo, hi = -5000.0, 84852.0
    f = (lambda h: isa(h)[1]) if kind == "p" else (lambda h: isa(h)[2])
    for _ in range(200):
        mid = (lo + hi) / 2
        if f(mid) > value:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def vec(i, inp, exp, rel, src=SRC, ver=VER):
    # A small absolute floor keeps near-zero values (a pressure altitude of -0.007 ft) meaningful.
    tol = {k: {"abs": 1e-6, "rel": rel} for k, v in exp.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    e = dict(exp)
    e.setdefault("ok", True)
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


# The ICAO standard atmosphere from ambiance 1.3.1 (airinnova), a separately
# written Python implementation, at geometric altitudes. Produced with:
#   from ambiance import Atmosphere; a = Atmosphere(zs)
# (z m, T K, p Pa, rho kg/m3, a m/s, mu Pa s, g m/s2). Pressure and density
# agree to about 2e-6 relative above 20 km, where the two implementations
# carry layer base pressures differently; both are within the standard's
# printed precision.
AMBIANCE = [
    (-2000, 301.1540914173708, 127782.8213566623, 1.4781612452746862, 347.8879198176305, 1.8514575204715214e-05, 9.812823755695224),
    (1500, 278.4023001554197, 84559.66592781676, 1.0581044626479077, 334.4886410386764, 1.7419589925374498e-05, 9.802023505965284),
    (7500, 239.45744967290807, 38299.66798370353, 0.5571918597280658, 310.2123908551758, 1.5442191260974684e-05, 9.783550230946561),
    (15000, 216.65, 12111.786132143703, 0.19475454731505212, 295.0694935090715, 1.4216130796413357e-05, 9.760531983853626),
    (25000, 221.55206472628424, 2549.2129278435896, 0.04008375667736631, 298.38903875267926, 1.4484244667793332e-05, 9.729967137756537),
    (40000, 250.34964610242113, 287.1421821481316, 0.003995656276775823, 317.18924664001145, 1.6009290415301384e-05, 9.684388360600034),
    (60000, 247.02088477279673, 21.958493710186964, 0.00030967559388573, 315.07344460230036, 1.5837189300043246e-05, 9.62411316252706),
    (78000, 202.54097785374015, 1.467355125656059, 2.5238319718083196e-05, 285.29976617538887, 1.3429642205650092e-05, 9.570345319920676),
]
KT = 1852 / 3600
PHAK = "FAA Pilot's Handbook of Aeronautical Knowledge (FAA-H-8083-25C), worked example read from the published chart or table"
PHAK_VER = "FAA-H-8083-25C (2023)"
IPH = "FAA Instrument Procedures Handbook (FAA-H-8083-16B), worked example"
IPH_VER = "FAA-H-8083-16B"


def isa_ambiance(start):
    out = []
    for i, (z, t, p, rho, a, mu, g) in enumerate(AMBIANCE, start):
        tol = {"result.temperature.value": {"abs": 1e-9}, "result.pressure.value": {"rel": 3e-6},
               "result.density.value": {"rel": 3e-6}, "result.speed_of_sound.value": {"rel": 1e-9},
               "result.dynamic_viscosity.value": {"rel": 1e-9}, "result.gravity.value": {"abs": 1e-12}}
        out.append({"id": f"v{i:03d}", "input": {"altitude": f"{z} m", "altitude_type": "geometric",
                                                 "options": {"outputUnits": {"temperature": "K", "pressure": "Pa"}}},
                    "expect": {"result.temperature.value": t, "result.pressure.value": p, "result.density.value": rho,
                               "result.speed_of_sound.value": a / KT, "result.dynamic_viscosity.value": mu,
                               "result.gravity.value": g, "ok": True},
                    "source": "ambiance (independent Python implementation of the ICAO standard atmosphere 1993)",
                    "sourceVersion": "ambiance 1.3.1", "tolerance": tol})
    return out


def isa_vectors():
    out = []
    for i, ft in enumerate([0, 5000, 10000, 20000, 36089, 40000, 65000, 100000, 150000, -1000], 1):
        h = ft * FT
        t, p, rho = isa(h)
        out.append(vec(i, {"altitude": f"{ft} ft", "options": {"outputUnits": {"temperature": "K", "pressure": "Pa"}}},
                       {"result.temperature.value": t, "result.pressure.value": p, "result.density.value": rho}, 1e-9))
    # US 1976 printed table rows (geometric altitude)
    rows = [(20000, 216.65, 5529.3), (32000, 228.49, 889.06), (50000, 270.65, 79.779), (86000, 186.87, 0.37338)]
    for k, (z, t, p) in enumerate(rows, len(out) + 1):
        out.append(vec(k, {"altitude": f"{z} m", "altitude_type": "geometric", "model": "us76",
                           "options": {"outputUnits": {"temperature": "K", "pressure": "Pa"}}},
                       {"result.temperature.value": t, "result.pressure.value": p}, 1e-4, TABLE, "NOAA-S/T 76-1562 (1976)"))
    return out + isa_ambiance(len(out) + 1)


def station(qnh_pa, elev_m):
    return qnh_pa * isa(elev_m)[1] / P0


def pa_vectors():
    cases = [(5000, 29.80), (0, 29.92126), (0, 30.50), (8000, 29.42), (1200, 28.95), (-200, 30.10),
             (2500, 30.25), (6500, 29.65), (10000, 29.92), (14000, 30.02), (330, 29.55), (4300, 28.70),
             (7200, 31.00), (900, 30.79), (12500, 28.40), (-1200, 29.98)]
    out = []
    for i, (e, a) in enumerate(cases, 1):
        pa = alt_for(station(a * INHG, e * FT), "p") / FT
        out.append(vec(i, {"elevation": f"{e} ft", "altimeter": f"{a} inHg"}, {"result.pressure_altitude.value": pa}, 1e-9))
    # hPa settings through the same independent path
    for i, (e, q) in enumerate([(0, 1009), (1500, 1030), (3000, 995)], len(out) + 1):
        pa = alt_for(station(q * 100.0, e * FT), "p") / FT
        out.append(vec(i, {"elevation": f"{e} ft", "altimeter": f"{q} hPa"}, {"result.pressure_altitude.value": pa}, 1e-9))
    # FAA-H-8083-25C figure 11-3: the printed altitude corrections at a sea-level field
    for i, (a, corr) in enumerate([(29.7, 205), (28.2, 1630), (31.0, -983)], len(out) + 1):
        out.append(vec(i, {"elevation": "0 ft", "altimeter": f"{a} inHg"}, {"result.pressure_altitude.value": corr},
                       0, PHAK, PHAK_VER))
        out[-1]["tolerance"] = {"result.pressure_altitude.value": {"abs": 1.0}}
    return out


def es(tc):
    return 610.94 * math.exp(17.625 * tc / (tc + 243.04))


def da_vectors():
    cases = [(5000, 29.80, 30, None), (5000, 29.80, 30, 20), (0, 29.92, 15, None), (6000, 30.10, 35, 10), (100, 29.70, -10, None), (4000, 29.95, 25, 24),
             (5431, 30.02, 32, None), (7000, 29.90, 28, 5), (0, 30.40, -30, None), (2000, 29.50, 40, 28),
             (8900, 30.15, 18, None), (1100, 30.00, 0, -5), (3500, 29.35, 22, None), (9900, 29.70, 10, -2),
             (500, 29.92, 45, 30), (6200, 30.30, -15, None), (12000, 29.85, 5, None), (-200, 30.05, 38, 26),
             (4500, 29.60, 33, 15), (2600, 30.20, 12, None)]
    out = []
    for i, (e, a, t, dp) in enumerate(cases, 1):
        p = station(a * INHG, e * FT)
        tk = t + 273.15
        if dp is not None:
            tk = tk / (1 - 0.378 * es(dp) / p)
        da = alt_for(p / (R * tk), "rho") / FT
        inp = {"elevation": f"{e} ft", "altimeter": f"{a} inHg", "temperature": f"{t} degC"}
        if dp is not None:
            inp["dew_point"] = f"{dp} degC"
        out.append(vec(i, inp, {"result.density_altitude.value": da}, 1e-9))
    # FAA-H-8083-25C sample problem 1 (figure 11-22): read from the chart as about 7,700 ft
    out.append(vec(len(out) + 1, {"elevation": "5883 ft", "altimeter": "30.10 inHg", "temperature": "70 degF"},
                   {"result.density_altitude.value": 7700}, 0, PHAK, PHAK_VER))
    out[-1]["tolerance"] = {"result.density_altitude.value": {"abs": 100.0}}
    return out


def isat_vectors():
    out = []
    for i, ft in enumerate([0, 10000, 30000, 36089, 41000, 60000], 1):
        t = isa(ft * FT)[0] - 273.15
        out.append(vec(i, {"pressure_altitude": f"{ft} ft"}, {"result.isa_temperature.value": t}, 1e-12))
    return out


def comps(rwy, wd, ws):
    d = math.radians(wd - rwy)
    return ws * math.cos(d), ws * math.sin(d)


def runway_vectors():
    cases = [(27, 300, 15), (9, 30, 20), (18, 200, 12), (36, 350, 25), (13, 250, 10), (4, 40, 8),
             (1, 190, 12), (22, 110, 18), (31, 310, 20), (6, 150, 9), (15, 330, 14), (33, 60, 22),
             (8, 260, 11), (27, 90, 7), (11, 200, 30), (35, 20, 16), (2, 290, 13), (24, 180, 40),
             (27, 270, 20), (18, 360, 15)]
    out = []
    for i, (r, wd, ws) in enumerate(cases, 1):
        h, x = comps((r * 10) % 360, wd, ws)
        out.append(vec(i, {"runway": f"{r:02d}", "wind_direction": wd, "wind_speed": ws},
                       {"result.headwind.value": h, "result.crosswind.value": abs(x)}, 1e-12))
    # FAA-H-8083-25C sample problem 10 (figure 11-31): read from the chart to the nearest knot as 22 kt and 13 kt
    out.append(vec(len(out) + 1, {"runway": "17", "wind_direction": 140, "wind_speed": 25},
                   {"result.headwind.value": 22, "result.crosswind.value": 13}, 0, PHAK, PHAK_VER))
    out[-1]["tolerance"] = {"result.headwind.value": {"abs": 1.0}, "result.crosswind.value": {"abs": 1.0}}
    return out


def triangle_vectors():
    cases = [(90, 120, 30, 20), (0, 100, 270, 15), (225, 150, 180, 40), (310, 95, 10, 25), (45, 200, 45, 30), (180, 80, 0, 20),
             (135, 110, 200, 18), (270, 140, 300, 35), (20, 90, 100, 12), (355, 105, 10, 28), (160, 250, 270, 60),
             (75, 65, 330, 20), (200, 180, 20, 45), (240, 120, 240, 25), (5, 130, 185, 30), (110, 85, 60, 15),
             (290, 300, 250, 80), (60, 75, 150, 30),
             (90, 120, 270, 30), (330, 160, 330, 50)]
    out = []
    for i, (tc, tas, wd, ws) in enumerate(cases, 1):
        d = math.radians(wd - tc)
        wca = math.asin(ws / tas * math.sin(d))
        gs = tas * math.cos(wca) - ws * math.cos(d)
        hd = (tc + math.degrees(wca)) % 360
        out.append(vec(i, {"course": tc, "tas": tas, "wind_direction": wd, "wind_speed": ws},
                       {"result.groundspeed.value": gs, "result.heading.value": hd}, 1e-12))
    # FAA-H-8083-25C chapter 16 (figures 16-19 to 16-22): drawn to scale as GS 88 kt, TH 076
    out.append(vec(len(out) + 1, {"course": 90, "tas": 120, "wind_direction": 45, "wind_speed": 40},
                   {"result.groundspeed.value": 88, "result.heading.value": 76}, 0, PHAK, PHAK_VER))
    out[-1]["tolerance"] = {"result.groundspeed.value": {"abs": 1.0}, "result.heading.value": {"abs": 1.0}}
    return out


def findwind_vectors():
    cases = [(81.7, 120, 90, 108.7), (0, 100, 5, 110), (270, 150, 265, 120), (135, 90, 140, 95), (10, 60, 350, 70)]
    out = []
    for i, (h, tas, tr, gs) in enumerate(cases, 1):
        we = gs * math.sin(math.radians(tr)) - tas * math.sin(math.radians(h))
        wn = gs * math.cos(math.radians(tr)) - tas * math.cos(math.radians(h))
        spd = math.hypot(we, wn)
        d = (math.degrees(math.atan2(we, wn)) + 180) % 360
        out.append(vec(i, {"heading": h, "tas": tas, "track": tr, "groundspeed": gs},
                       {"result.wind_speed.value": spd, "result.wind_direction.value": d}, 1e-11))
    return out


# ---------------------------------------------------------------- slice 2

KT = 1852 / 3600
NMI = 1852.0
A0 = math.sqrt(1.4 * R * T0)
AS_SRC = "Independent Python implementation of the Gracey (NASA RP-1046) airspeed relations, with supersonic Mach by bisection (tools/vectors/gen_aviation.py)"
AS_VER = "NASA RP-1046 (1980)"
PERF_SRC = "Independent Python implementation of the turn, gradient, glide, and pivotal-altitude relations in FAA-H-8083-3C and FAA-H-8083-16B (tools/vectors/gen_aviation.py)"
PERF_VER = "FAA-H-8083-3C (2021)"
WB_SRC = "Independent Python weight-and-balance arithmetic per FAA-H-8083-1B chapter 2 (tools/vectors/gen_aviation.py)"
WB_VER = "FAA-H-8083-1B (2016)"
WBH = "FAA Aircraft Weight and Balance Handbook (FAA-H-8083-1B), worked example"
WBH_VER = "FAA-H-8083-1B (2016)"


def qc_ratio(m):
    if m <= 1:
        return (1 + 0.2 * m * m) ** 3.5 - 1
    return 1.2 ** 3.5 * 6 ** 2.5 * m ** 7 / (7 * m * m - 1) ** 2.5 - 1


def mach_for(r):
    """Bisection on the monotonic qc/p(M): independent of the core's fixed point."""
    lo, hi = 0.0, 20.0
    for _ in range(300):
        mid = (lo + hi) / 2
        if qc_ratio(mid) < r:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def airspeed_vectors():
    out = []
    cases = [(250, 10000, -5), (300, 35000, -54.3), (120, 5000, 20), (65, 0, 15), (450, 20000, -30), (600, 40000, -56.5), (350, 28000, -40)]
    for i, (cas, ft, oat) in enumerate(cases, 1):
        p = isa(ft * FT)[1]
        qc = P0 * qc_ratio(cas * KT / A0)
        m = mach_for(qc / p)
        tas = m * math.sqrt(1.4 * R * (oat + 273.15)) / KT
        eas = m * A0 * math.sqrt(p / P0) / KT
        out.append(vec(i, {"airspeed": f"{cas} kt", "pressure_altitude": f"{ft} ft", "temperature": f"{oat} degC"},
                       {"result.mach": m, "result.tas.value": tas, "result.eas.value": eas}, 1e-9, AS_SRC, AS_VER))
    return out


def cas_from_mach(m, p):
    return A0 * mach_for(p * qc_ratio(m) / P0) / KT


def tas_to_cas_vectors():
    out = []
    cases = [(0.78, 35000, -54.3), (0.5, 20000, -25), (0.25, 3000, 10), (0.85, 41000, -56.5), (1.6, 45000, -56.5), (0.65, 30000, -44)]
    for i, (m, ft, oat) in enumerate(cases, 1):
        p = isa(ft * FT)[1]
        out.append(vec(i, {"mach": m, "pressure_altitude": f"{ft} ft", "temperature": f"{oat} degC"},
                       {"result.cas.value": cas_from_mach(m, p),
                        "result.tas.value": m * math.sqrt(1.4 * R * (oat + 273.15)) / KT}, 1e-9, AS_SRC, AS_VER))
    return out


def tat_vectors():
    out = []
    for i, (t, m, r) in enumerate([(-20, 0.8, 1.0), (-35, 0.78, 1.0), (10, 0.3, 0.95), (-10, 0.6, 0.9), (0, 0.0, 1.0)], 1):
        tk = t + 273.15
        sat = tk / (1 + 0.2 * r * m * m) - 273.15
        inp = {"temperature": f"{t} degC", "mach": m}
        if r != 1.0:
            inp["recovery_factor"] = r
        out.append(vec(i, inp, {"result.sat.value": sat, "result.ram_rise.value": t - sat}, 1e-9, AS_SRC, AS_VER))
    return out


def turn_vectors():
    out = []
    cases = [(100, None), (150, 30), (90, 45), (250, 25), (60, 60), (120, None)]
    for i, (tas, bank) in enumerate(cases, 1):
        v = tas * KT
        if bank is None:
            phi = math.atan(v * math.radians(3) / G0)
            inp = {"tas": f"{tas} kt"}
        else:
            phi = math.radians(bank)
            inp = {"tas": f"{tas} kt", "bank": f"{bank} deg"}
        r = v * v / (G0 * math.tan(phi)) / FT
        out.append(vec(i, inp, {"result.bank.value": math.degrees(phi), "result.radius.value": r,
                                "result.load_factor": 1 / math.cos(phi)}, 1e-9, PERF_SRC, PERF_VER))
    return out


def tod_vectors():
    out = []
    cases = [(35000, 3000, 420, 3), (12500, 2000, 250, 3), (8500, 3000, 140, 4), (41000, 10000, 460, 2.5), (6000, 1500, 110, 3),
             (39000, 11000, 480, 3), (24000, 4000, 380, 2.8), (17500, 2500, 300, 3.5), (9500, 1200, 160, 3), (4500, 800, 95, 4.5),
             (45000, 18000, 500, 2.2), (28000, 250, 400, 3), (14000, 6000, 220, 5), (7000, 2000, 120, 3), (33000, 9000, 440, 3.2),
             (21000, 3000, 350, 3), (11000, 2000, 200, 2.5), (5500, 1500, 90, 3), (37000, 5000, 460, 3)]
    for i, (a, b, gs, ang) in enumerate(cases, 1):
        th = math.radians(ang)
        d = (a - b) * FT / math.tan(th) / NMI
        v = gs * KT * math.tan(th) / FT * 60
        out.append(vec(i, {"from_altitude": f"{a} ft", "to_altitude": f"{b} ft", "groundspeed": f"{gs} kt", "descent_angle": f"{ang} deg"},
                       {"result.distance.value": d, "result.vertical_speed.value": v}, 1e-9, PERF_SRC, "FAA-H-8083-16B (2017)"))
    # FAA-H-8083-16B chapter 3 (descent planning): FL 310 to an approach gate at 6,000 ft is 25 x 3 = 75 NM by the 3-to-1 rule.
    out.append(vec(len(out) + 1, {"from_altitude": "31000 ft", "to_altitude": "6000 ft", "groundspeed": "420 kt"},
                   {"result.rule_3_to_1.value": 75.0}, 1e-12, IPH, IPH_VER))
    return out


def gradient_vectors():
    out = []
    for i, (g, gs) in enumerate([(200, 120), (300, 90), (425, 160), (152, 200), (500, 75)], 1):
        out.append(vec(i, {"gradient": f"{g} ft/NM", "groundspeed": f"{gs} kt"},
                       {"result.vertical_speed.value": g * gs / 60,
                        "result.angle.value": math.degrees(math.atan(g * FT / NMI))}, 1e-9, PERF_SRC, "FAA-H-8083-16B (2017)"))
    return out


def vdp_vectors():
    out = []
    for i, (hat, ang, tch) in enumerate([(400, 3, 0), (520, 3, 50), (300, 2.75, 0), (700, 3.2, 55), (450, 3.5, 40),
                                         (380, 3, 45), (610, 2.9, 50), (250, 3, 0), (820, 3.1, 60), (560, 3.3, 52), (340, 2.5, 38),
                                         (475, 3, 47), (900, 3.5, 55), (295, 3, 42), (720, 2.8, 50),
                                         (410, 3, 50), (530, 3.2, 0), (365, 2.75, 44)], 1):
        d = (hat - tch) * FT / math.tan(math.radians(ang)) / NMI
        inp = {"height_above_touchdown": f"{hat} ft", "descent_angle": f"{ang} deg"}
        if tch:
            inp["threshold_crossing_height"] = f"{tch} ft"
        out.append(vec(i, inp, {"result.distance.value": d, "result.rule_hat_300.value": hat / 300}, 1e-9, PERF_SRC, "FAA-H-8083-16B (2017)"))
    # FAA-H-8083-16B chapter 4: on a three-degree path the height above the TDZE is 300 ft per NM (450 ft at 1.5 NM, 600 ft at 2 NM).
    for hat, nm in [(450, 1.5), (600, 2.0)]:
        out.append(vec(len(out) + 1, {"height_above_touchdown": f"{hat} ft"}, {"result.rule_hat_300.value": nm}, 1e-12, IPH, IPH_VER))
    return out


def glide_vectors():
    out = []
    for i, (h, ld, tas, hw) in enumerate([(5000, 9, 70, 20), (3000, 10, 65, 0), (8000, 12, 80, -15), (10000, 25, 55, 10), (1500, 8, 68, 5)], 1):
        still = h * FT * ld / NMI
        out.append(vec(i, {"height": f"{h} ft", "glide_ratio": ld, "tas": f"{tas} kt", "headwind": f"{hw} kt"},
                       {"result.still_air_range.value": still, "result.wind_range.value": still * (tas - hw) / tas,
                        "result.sink_rate.value": tas * KT / ld / FT * 60}, 1e-9, PERF_SRC, PERF_VER))
    return out


def pivotal_vectors():
    out = []
    for i, gs in enumerate([100, 80, 120, 95, 110], 1):
        out.append(vec(i, {"groundspeed": f"{gs} kt"}, {"result.pivotal_altitude.value": (gs * KT) ** 2 / G0 / FT}, 1e-12, PERF_SRC, PERF_VER))
    return out


def fuel_vectors():
    out = []
    cases = [({"volume": "40 gal", "fuel": "100ll"}, 240.0), ({"volume": "100 gal", "fuel": "jet-a"}, 670.0),
             ({"volume": "56 gal", "fuel": "100ll"}, 336.0), ({"volume": "30 gal", "density": "5.9 lb/gal"}, 177.0),
             ({"weight": "300 lb", "fuel": "100ll"}, None)]
    for i, (inp, w) in enumerate(cases, 1):
        exp = {"result.weight.value": w} if w is not None else {"result.volume.value": 50.0}
        out.append(vec(i, inp, exp, 1e-12, WB_SRC, WB_VER))
    return out


def wb_vectors():
    out = []
    cases = [
        [(1500, 85), (340, 90), (170, 118), (240, 48)],
        [(1650, 39.0), (380, 37.0), (0, 73.0), (50, 95.0), (240, 48.0)],
        [(2100, 101.2), (400, 104.0), (300, 140.0), (100, 160.0), (480, 110.0)],
        [(1200, 32.5), (170, 34.0), (60, 50.0)],
        [(900, 12.0), (180, -10.0), (90, 25.0)],
    ]
    for i, st in enumerate(cases, 1):
        w = sum(a for a, _ in st)
        m = sum(a * b for a, b in st)
        rows = [{"name": f"S{k}", "weight": f"{a} lb", "arm": f"{b} in"} for k, (a, b) in enumerate(st, 1)]
        out.append(vec(i, {"stations": rows}, {"result.total_weight.value": w, "result.cg.value": m / w}, 1e-12, WB_SRC, WB_VER))
    rnd = random.Random(5)
    for _ in range(15):
        st = [(rnd.choice([900, 1250, 1650, 2100, 3400]) + rnd.randint(0, 99), round(rnd.uniform(-20, 120), 1)) for _ in range(rnd.randint(1, 7))]
        w = sum(a for a, _ in st)
        m = sum(a * b for a, b in st)
        rows = [{"name": f"S{k}", "weight": f"{a} lb", "arm": f"{b} in"} for k, (a, b) in enumerate(st, 1)]
        out.append(vec(len(out) + 1, {"stations": rows}, {"result.total_weight.value": w, "result.cg.value": m / w}, 1e-12, WB_SRC, WB_VER))
    # FAA-H-8083-1B figure 3-5: weighing points of a nosewheel airplane, datum at the firewall
    out.append(vec(len(out) + 1, {"stations": [{"name": "Right main", "weight": "830 lb", "arm": "46 in"}, {"name": "Left main", "weight": "836 lb", "arm": "46 in"},
                                               {"name": "Nose", "weight": "340 lb", "arm": "-32 in"}]},
                   {"result.total_weight.value": 2006, "result.total_moment": 65756, "result.cg.value": 32.8}, 0, WBH, WBH_VER))
    out[-1]["tolerance"] = {"result.total_weight.value": {"abs": 0}, "result.total_moment": {"abs": 1e-6}, "result.cg.value": {"abs": 0.05}}
    # FAA-H-8083-1B figures 6-1 to 6-4: the loaded light twin, inside the CG range (envelope closed at the empty weight)
    out.append(vec(len(out) + 1, {"stations": [
        {"name": "Airplane", "weight": "3404 lb", "arm": "35.28 in"}, {"name": "Fuel", "weight": "840 lb", "arm": "61 in"},
        {"name": "Front seat", "weight": "320 lb", "arm": "37 in"}, {"name": "Row 2 seats", "weight": "310 lb", "arm": "75 in"},
        {"name": "Forward baggage", "weight": "100 lb", "arm": "-15 in"}, {"name": "Aft baggage", "weight": "90 lb", "arm": "113 in"}],
        "envelope": [{"arm": "32 in", "weight": "3404 lb"}, {"arm": "32 in", "weight": "4300 lb"}, {"arm": "38 in", "weight": "5200 lb"},
                     {"arm": "43.1 in", "weight": "5200 lb"}, {"arm": "43.6 in", "weight": "4800 lb"}, {"arm": "43.6 in", "weight": "3404 lb"}]},
        {"result.total_weight.value": 5064, "result.total_moment": 215093, "result.cg.value": 42.47, "result.takeoff_status": "inside"}, 0, WBH, WBH_VER))
    out[-1]["tolerance"] = {"result.total_weight.value": {"abs": 0}, "result.total_moment": {"abs": 0.5}, "result.cg.value": {"abs": 0.005}}
    # FAA-H-8083-1B figure 3-19: MAC from station 144 to 206, CG at station 161, is 27.4% MAC
    out.append(vec(len(out) + 1, {"stations": [{"name": "Loaded", "weight": "1000 lb", "arm": "161 in"}], "lemac": "144 in", "mac": "62 in"},
                   {"result.cg_mac": 27.4}, 0, WBH, WBH_VER))
    out[-1]["tolerance"] = {"result.cg_mac": {"abs": 0.05}}
    return out


# ---------------------------------------------------------------- slice 3

WX_SRC = "Hand-decoded per FAA Order JO 7900.5E Change 1 (METAR body and remarks) and the FAA Aviation Weather Handbook (flight categories)"
WX_VER = "JO 7900.5E Chg 1 (2022)"
FB_SRC = "Hand-decoded per the FAA Aviation Weather Handbook FB coding rules (FAA-H-8083-28)"
FB_VER = "FAA-H-8083-28 (2022)"
HOLD_SRC = "AIM 5-3-8 figure 5-3-2 sectors and table 5-3-1 speeds, applied by hand"
HOLD_VER = "AIM (2026)"


def svec(i, inp, exp, src, ver):
    tol = {k: {"abs": 1e-9} for k, v in exp.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    e = dict(exp)
    e.setdefault("ok", True)
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": tol}


def metar_vectors():
    cases = [
        ("KDEN 181753Z 30015G25KT 10SM FEW080 SCT200 30/08 A2980 RMK AO2 SLP052 T03000083",
         {"result.wind_direction.value": 300, "result.wind_gust.value": 25, "result.sea_level_pressure.value": 1005.2,
          "result.dew_point.value": 8.3, "result.flight_category": "VFR"}),
        ("KSFO 010856Z AUTO 00000KT 1 1/2SM -RA BR VV004 12/12 A3001 RMK AO2 SLP163 T01170117",
         {"result.visibility.value": 1.5, "result.ceiling.value": 400, "result.flight_category": "LIFR",
          "result.weather": "light rain; mist", "result.sea_level_pressure.value": 1016.3}),
        ("PAFA 152253Z 22012G22KT 180V250 M1/4SM +FZRA BKN008 OVC015 M05/M07 A2992 RMK AO2 SLP987 T10501072",
         {"result.visibility.value": 0.25, "result.ceiling.value": 800, "result.temperature.value": -5.0,
          "result.dew_point.value": -7.2, "result.sea_level_pressure.value": 998.7, "result.flight_category": "LIFR"}),
        ("KJFK 181751Z 04008KT 3SM TSRA BKN015CB 22/20 A2990",
         {"result.flight_category": "MVFR", "result.weather": "thunderstorm with rain", "result.ceiling.value": 1500}),
        ("KBOS 181754Z 09010KT 2SM BR OVC007 18/17 A2998",
         {"result.flight_category": "IFR", "result.altimeter.value": 29.98}),
        ("EGLL 181750Z 24012KT 9999 FEW035 17/09 Q1013 NOSIG",
         {"result.altimeter.value": 1013, "result.visibility_text": "10 km or more", "result.flight_category": "VFR"}),
    ]
    out = [svec(i, {"report": r}, e, WX_SRC, WX_VER) for i, (r, e) in enumerate(cases, 1)]
    # FAA-H-8083-28A chapter 24 decodes this report group by group; intensity qualifies the precipitation
    # (table 24-3: heavy rain shower(s) is +SHRA), so +TSRA is a thunderstorm with heavy rain.
    out.append(svec(len(out) + 1, {"report": "METAR KOKC 011955Z AUTO 22015G25KT 180V250 3/4SM R17L/2600FT +TSRA BR OVC010CB 18/16 A2992 RMK AO2 TSB25 TS OHD MOV E SLP132"},
                    {"result.station": "KOKC", "result.modifier": "AUTO", "result.wind_direction.value": 220, "result.wind_speed.value": 15,
                     "result.wind_gust.value": 25, "result.visibility.value": 0.75, "result.weather": "thunderstorm with heavy rain; mist",
                     "result.ceiling.value": 1000, "result.temperature.value": 18, "result.dew_point.value": 16,
                     "result.altimeter.value": 29.92, "result.sea_level_pressure.value": 1013.2, "result.flight_category": "LIFR"}, AWH_A, AWH_A_VER))
    # Found by the python-metar differential on live reports: /// cloud types, trend groups, and NDV visibility.
    reg = "Live Aviation Weather Center reports (2026-09-19), as decoded by python-metar 1.11.0"
    for r, e in [("METAR EKBI 191850Z AUTO 24012KT 9999 FEW015/// SCT057/// BKN200/// 16/13 Q1009", {"result.ceiling.value": 20000, "result.clouds.2.base.value": 20000}),
                 ("METAR LFBL 191900Z AUTO 30005KT CAVOK 18/10 Q1025 BECMG 36010KT", {"result.wind_direction.value": 300, "result.wind_speed.value": 5}),
                 ("METAR LSME 191850Z AUTO 00000KT 9999NDV NCD 17/13 Q1024 RMK", {"result.visibility_text": "10 km or more"})]:
        out.append(svec(len(out) + 1, {"report": r}, e, reg, "metar 1.11.0"))
    # Every 60th knots report of the committed differential fixture, with python-metar's values.
    rows = [json.loads(l) for l in Path("core/crates/gp-aviation/tests/data/metar_diff.jsonl").read_text().splitlines()[1:]]
    for row in [r for r in rows if "KT " in r["report"]][::60]:
        e = {f"result.{k}.value": row[k] for k in ("wind_speed", "wind_direction", "wind_gust", "temperature", "dew_point", "sea_level_pressure") if k in row}
        out.append(svec(len(out) + 1, {"report": row["report"]}, e, reg, "metar 1.11.0"))
    return out


def fb_vectors():
    cases = [
        ({"report": "731960", "level": "34000 ft"}, {"result.winds.0.direction.value": 230, "result.winds.0.speed.value": 119, "result.winds.0.temperature.value": -60}),
        ({"report": "9900+05", "level": "9000 ft"}, {"result.winds.0.speed.value": 0, "result.winds.0.temperature.value": 5}),
        ({"report": "2714", "level": "3000 ft"}, {"result.winds.0.direction.value": 270, "result.winds.0.speed.value": 14}),
        ({"report": "2635-08", "level": "12000 ft"}, {"result.winds.0.direction.value": 260, "result.winds.0.speed.value": 35, "result.winds.0.temperature.value": -8}),
        ({"report": "DEN 2321-04 2532-14 2540-26 2447-38 245152 245657 245358"},
         {"result.winds.0.level.value": 9000, "result.winds.4.temperature.value": -52, "result.winds.6.direction.value": 240, "result.winds.6.speed.value": 53}),
        ({"report": "861558", "level": "39000 ft"}, {"result.winds.0.direction.value": 360, "result.winds.0.speed.value": 115, "result.winds.0.temperature.value": -58}),
    ]
    out = [svec(i, inp, e, FB_SRC, FB_VER) for i, (inp, e) in enumerate(cases, 1)]
    # An independent encoder: pick the wind and temperature, code the group by the FB rules, and expect them back.
    rnd = random.Random(27)
    for _ in range(8):
        level = rnd.choice([6000, 9000, 12000, 18000, 24000, 30000, 34000, 39000])
        d, spd = rnd.randint(1, 36) * 10, rnd.randint(5, 199)
        t = rnd.randint(-65, -1) if level > 24000 else rnd.randint(-40, 20)
        code = f"{(d // 10 + (50 if spd >= 100 else 0)) % 100:02d}{spd % 100:02d}"
        code += f"{abs(t):02d}" if level > 24000 else f"{'-' if t < 0 else '+'}{abs(t):02d}"
        out.append(svec(len(out) + 1, {"report": code, "level": f"{level} ft"},
                        {"result.winds.0.direction.value": d, "result.winds.0.speed.value": spd, "result.winds.0.temperature.value": t},
                        "Winds encoded by an independent Python implementation of the FB coding rules (tools/vectors/gen_aviation.py)", "FAA-H-8083-28A (2024)"))
    # FAA-H-8083-28A section 27.2.1.1.2 and table 27-1 (Kansas City). The coded message ends 550252, but the table
    # decodes that level as 750252 = 250 deg at 102 kt, -52 C; the table's group is used.
    table = [(3000, "9900", None, 0, None), (6000, "1709+06", 170, 9, 6), (9000, "2018+00", 200, 18, 0), (12000, "2130-06", 210, 30, -6),
             (18000, "2242-18", 220, 42, -18), (24000, "2361-30", 230, 61, -30), (30000, "247242", 240, 72, -42),
             (34000, "258848", 250, 88, -48), (39000, "750252", 250, 102, -52)]
    block = ("DATA BASED ON 010000Z\nVALID 010600Z FOR USE 0500-0900Z. TEMPS NEG ABV 24000\n"
             "FT 3000 6000 9000 12000 18000 24000 30000 34000 39000\nMKC " + " ".join(g for _, g, *_ in table))
    exp = {}
    for k, (lv, _, d, spd, t) in enumerate(table):
        exp[f"result.winds.{k}.level.value"] = lv
        exp[f"result.winds.{k}.speed.value"] = spd
        if d is not None:
            exp[f"result.winds.{k}.direction.value"] = d
        if t is not None:
            exp[f"result.winds.{k}.temperature.value"] = t
    out.append(svec(len(out) + 1, {"report": block}, dict(exp, **{"result.station": "MKC"}), AWH_A, AWH_A_VER))
    for lv, g, d, spd, t in table[1:]:
        e = {"result.winds.0.speed.value": spd, "result.winds.0.direction.value": d, "result.winds.0.temperature.value": t}
        out.append(svec(len(out) + 1, {"report": g, "level": f"{lv} ft"}, e, AWH_A, AWH_A_VER))
    return out


AWH_A = "FAA Aviation Weather Handbook (FAA-H-8083-28A), decoded example"
AWH_A_VER = "FAA-H-8083-28A (2024)"


def hold_entry_vectors():
    cases = [(360, 90, "right", "direct"), (360, 150, "right", "teardrop"), (360, 240, "right", "parallel"),
             (360, 210, "left", "teardrop"), (360, 120, "left", "parallel"), (90, 90, "left", "direct"), (270, 30, "right", "teardrop")]
    return [svec(i, {"inbound_course": f"{ic} deg", "heading": f"{h} deg", "turns": t}, {"result.entry": e}, HOLD_SRC, HOLD_VER)
            for i, (ic, h, t, e) in enumerate(cases, 1)]


def hold_speed_vectors():
    cases = [(3000, 200), (6000, 200), (6001, 230), (14000, 230), (14001, 265), (35000, 265)]
    return [svec(i, {"altitude": f"{a} ft"}, {"result.max_ias.value": v}, HOLD_SRC, HOLD_VER) for i, (a, v) in enumerate(cases, 1)]


def hold_wind_vectors():
    out = []
    cases = [(360, 120, 300, 20), (90, 150, 180, 30), (270, 100, 270, 25), (180, 140, 45, 15), (45, 90, 90, 10)]
    for i, (ic, tas, wd, ws) in enumerate(cases, 1):
        def leg(c):
            a = math.radians(wd - c)
            w = math.asin(ws / tas * math.sin(a))
            return math.degrees(w), tas * math.cos(w) - ws * math.cos(a)
        oc = (ic + 180) % 360
        w_in, gs_in = leg(ic)
        _, gs_out = leg(oc)
        v = vec(i, {"inbound_course": f"{ic} deg", "tas": f"{tas} kt", "wind_direction": f"{wd} deg", "wind_speed": f"{ws} kt"},
                {"result.inbound_wca.value": w_in, "result.outbound_heading.value": (oc - 3 * w_in) % 360,
                 "result.outbound_time.value": gs_in * 60 / gs_out}, 1e-9, PERF_SRC.replace("turn, gradient, glide, and pivotal-altitude relations in FAA-H-8083-3C and FAA-H-8083-16B", "holding wind triangle and triple-the-drift rule in FAA-H-8083-15B"), "FAA-H-8083-15B (2012)")
        out.append(v)
    return out


def taf_vectors():
    cases = [
        ("TAF KDEN 181720Z 1818/1918 30012KT P6SM SCT080 FM190200 32008KT P6SM FEW100", {"result.valid_hours": 24.0, "result.count": 2.0, "result.periods.1.from": "day 19 at 0200Z"}),
        ("TAF AMD KORD 302330Z 3100/0106 27015KT P6SM BKN025 FM010300 VRB03KT 1/2SM FG VV002", {"result.valid_hours": 30.0, "result.periods.1.flight_category": "LIFR"}),
        ("TAF KSEA 181130Z 1812/1912 18005KT 5SM BR OVC008 TEMPO 1812/1816 2SM BR OVC004", {"result.periods.0.flight_category": "IFR", "result.periods.1.flight_category": "LIFR"}),
        ("TAF KPHX 181730Z 1818/1924 VRB05KT P6SM SKC PROB40 1822/1902 VRB25G40KT 1SM TSRA", {"result.count": 2.0, "result.periods.1.change": "40% probability"}),
        ("TAF EGLL 181700Z 1818/1924 24012KT 9999 FEW035 BECMG 1822/1901 20008KT", {"result.periods.0.visibility": "10 km or more", "result.periods.1.change": "becoming"}),
    ]
    return [svec(i, {"report": r}, e, "Hand-decoded per the FAA Aviation Weather Handbook (TAF chapter)", "FAA-H-8083-28 (2022)") for i, (r, e) in enumerate(cases, 1)]


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    out.mkdir(parents=True, exist_ok=True)
    files = {
        "aviation.atmosphere.isa": isa_vectors(),
        "aviation.altimetry.pressure-altitude": pa_vectors(),
        "aviation.altimetry.density-altitude": da_vectors(),
        "aviation.altimetry.isa-temperature": isat_vectors(),
        "aviation.wind.runway-components": runway_vectors(),
        "aviation.wind.heading-groundspeed": triangle_vectors(),
        "aviation.wind.find-wind": findwind_vectors(),
        "aviation.airspeed.cas-to-tas": airspeed_vectors(),
        "aviation.airspeed.tas-to-cas": tas_to_cas_vectors(),
        "aviation.airspeed.tat-sat": tat_vectors(),
        "aviation.performance.turn": turn_vectors(),
        "aviation.performance.top-of-descent": tod_vectors(),
        "aviation.performance.climb-gradient": gradient_vectors(),
        "aviation.performance.vdp": vdp_vectors(),
        "aviation.performance.glide": glide_vectors(),
        "aviation.performance.pivotal-altitude": pivotal_vectors(),
        "aviation.loading.fuel-weight": fuel_vectors(),
        "aviation.loading.weight-balance": wb_vectors(),
        "aviation.weather.metar-decode": metar_vectors(),
        "aviation.weather.fb-winds-decode": fb_vectors(),
        "aviation.weather.taf-decode": taf_vectors(),
        "aviation.ifr.hold-entry": hold_entry_vectors(),
        "aviation.ifr.hold-speed-limit": hold_speed_vectors(),
        "aviation.ifr.hold-wind-timing": hold_wind_vectors(),
    }
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
