#!/usr/bin/env python3
"""Golden vectors for aviation slice 1 (core/vectors/aviation.*.jsonl).

Expected values come from an independent Python implementation of the
defining equations (ICAO Doc 7488/3, US 1976, the wind triangle), written
separately from the Rust core, plus published US Standard Atmosphere 1976
table rows at their printed precision. Rerunning must reproduce the files.
"""
import json
import math
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
    return out


def station(qnh_pa, elev_m):
    return qnh_pa * isa(elev_m)[1] / P0


def pa_vectors():
    cases = [(5000, 29.80), (0, 29.92126), (0, 30.50), (8000, 29.42), (1200, 28.95), (-200, 30.10)]
    out = []
    for i, (e, a) in enumerate(cases, 1):
        pa = alt_for(station(a * INHG, e * FT), "p") / FT
        out.append(vec(i, {"elevation": f"{e} ft", "altimeter": f"{a} inHg"}, {"result.pressure_altitude.value": pa}, 1e-9))
    return out


def es(tc):
    return 610.94 * math.exp(17.625 * tc / (tc + 243.04))


def da_vectors():
    cases = [(5000, 29.80, 30, None), (5000, 29.80, 30, 20), (0, 29.92, 15, None), (6000, 30.10, 35, 10), (100, 29.70, -10, None), (4000, 29.95, 25, 24)]
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
    cases = [(27, 300, 15), (9, 30, 20), (18, 200, 12), (36, 350, 25), (13, 250, 10), (4, 40, 8)]
    out = []
    for i, (r, wd, ws) in enumerate(cases, 1):
        h, x = comps((r * 10) % 360, wd, ws)
        out.append(vec(i, {"runway": f"{r:02d}", "wind_direction": wd, "wind_speed": ws},
                       {"result.headwind.value": h, "result.crosswind.value": abs(x)}, 1e-12))
    return out


def triangle_vectors():
    cases = [(90, 120, 30, 20), (0, 100, 270, 15), (225, 150, 180, 40), (310, 95, 10, 25), (45, 200, 45, 30), (180, 80, 0, 20)]
    out = []
    for i, (tc, tas, wd, ws) in enumerate(cases, 1):
        d = math.radians(wd - tc)
        wca = math.asin(ws / tas * math.sin(d))
        gs = tas * math.cos(wca) - ws * math.cos(d)
        hd = (tc + math.degrees(wca)) % 360
        out.append(vec(i, {"course": tc, "tas": tas, "wind_direction": wd, "wind_speed": ws},
                       {"result.groundspeed.value": gs, "result.heading.value": hd}, 1e-12))
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
    cases = [(35000, 3000, 420, 3), (12500, 2000, 250, 3), (8500, 3000, 140, 4), (41000, 10000, 460, 2.5), (6000, 1500, 110, 3)]
    for i, (a, b, gs, ang) in enumerate(cases, 1):
        th = math.radians(ang)
        d = (a - b) * FT / math.tan(th) / NMI
        v = gs * KT * math.tan(th) / FT * 60
        out.append(vec(i, {"from_altitude": f"{a} ft", "to_altitude": f"{b} ft", "groundspeed": f"{gs} kt", "descent_angle": f"{ang} deg"},
                       {"result.distance.value": d, "result.vertical_speed.value": v}, 1e-9, PERF_SRC, "FAA-H-8083-16B (2017)"))
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
    for i, (hat, ang, tch) in enumerate([(400, 3, 0), (520, 3, 50), (300, 2.75, 0), (700, 3.2, 55), (450, 3.5, 40)], 1):
        d = (hat - tch) * FT / math.tan(math.radians(ang)) / NMI
        inp = {"height_above_touchdown": f"{hat} ft", "descent_angle": f"{ang} deg"}
        if tch:
            inp["threshold_crossing_height"] = f"{tch} ft"
        out.append(vec(i, inp, {"result.distance.value": d, "result.rule_hat_300.value": hat / 300}, 1e-9, PERF_SRC, "FAA-H-8083-16B (2017)"))
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
    return out


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
    }
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
