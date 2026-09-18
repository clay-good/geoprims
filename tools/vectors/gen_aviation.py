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
    }
    for tool, vs in files.items():
        (out / f"{tool}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
