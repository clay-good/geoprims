#!/usr/bin/env python3
"""Golden vectors for QNH/QFE/QNE and flight levels, worked by hand with an
independent ISA: T = 288.15 - 0.0065 h to 11 km, then isothermal at 216.65
K, p from the hydrostatic equation with g0 = 9.80665 and R = 287.05287.
QFE = QNH x p(elev) / P0; the NWS formula from its published PDF; the US
lowest usable flight level from 14 CFR 91.121(b)."""
import json
import math
import sys
from pathlib import Path

SRC = "ISA altimetry worked in Python (tools/vectors/gen_qcodes.py)"
SPEC = "add-aviation-suite altimetry scenarios"
P0, T0, G0, R, L = 101325.0, 288.15, 9.80665, 287.05287, -0.0065
T11 = T0 + L * 11000
P11 = P0 * (T11 / T0) ** (-G0 / (R * L))
FT, INHG = 0.3048, 3386.389


def p_at(h):
    if h <= 11000:
        return P0 * ((T0 + L * h) / T0) ** (-G0 / (R * L))
    return P11 * math.exp(-G0 * (h - 11000) / (R * T11))


def h_for(p):
    if p >= P11:
        return (T0 * (p / P0) ** (-R * L / G0) - T0) / L
    return 11000 + R * T11 / G0 * math.log(P11 / p)


def vec(i, inp, exp, src=SRC, tol=1e-6):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": tol, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": "2026", "tolerance": t}


def nws(p_hpa, h_m):
    n = 0.190284
    ps = p_hpa - 0.3
    return ps * (1 + (1013.25 ** n * 0.0065 / 288) * (h_m / ps ** n)) ** (1 / n)


def qcodes():
    out = []
    for elev, qnh in [(1000, "1013 hPa"), (5000, "29.80 inHg"), (0, "1013.25 hPa"), (7000, "30.12 inHg"), (-200, "1020 hPa")]:
        v, u = qnh.split()
        qnh_pa = float(v) * (100 if u == "hPa" else INHG)
        qfe = qnh_pa * p_at(elev * FT) / P0
        out.append(vec(len(out) + 1, {"elevation": f"{elev} ft", "altimeter": qnh},
                       {"result.qfe.value": qfe / 100, "result.qnh.value": qnh_pa / INHG, "result.qne.value": h_for(qfe) / FT}))
    out[0]["source"] = SPEC + " (QFE from QNH 1013 hPa at 1,000 ft by the ISA pressure-height relation)"
    for elev, qfe_hpa in [(5000, 843.0), (1000, 979.3), (300, 1002.1)]:
        qfe = qfe_hpa * 100
        qnh = qfe * P0 / p_at(elev * FT)
        out.append(vec(len(out) + 1, {"elevation": f"{elev} ft", "station_pressure": f"{qfe_hpa} hPa"},
                       {"result.qfe.value": qfe_hpa, "result.qnh.value": qnh / INHG, "result.qne.value": h_for(qfe) / FT,
                        "result.qnh_nws.value": nws(qfe_hpa, elev * FT) * 100 / INHG}))
    out.append(vec(len(out) + 1, {"elevation": "1000 ft", "altimeter": "1013 hPa", "station_pressure": "979 hPa"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    return out


def lufl(inhg):
    below = max(2992 - round(inhg * 100), 0)
    k = (below + 49) // 50
    return 180 + 5 * k if k <= 6 else None


def flight_level():
    out = []
    for qnh, fl, alt in [(29.42, 185, None), (29.92, 180, None), (30.25, None, 17500), (28.50, 200, 19000), (27.00, 350, None), (29.00, 450, 12000)]:
        qnh_pa = qnh * INHG
        scale = P0 / qnh_pa
        exp = {"result.lowest_usable_fl": lufl(qnh), "result.transition_altitude.value": 18000.0}
        inp = {"altimeter": f"{qnh} inHg"}
        if fl is not None:
            inp["flight_level"] = fl
            exp["result.altitude_of_fl.value"] = h_for(p_at(fl * 100 * FT) * scale) / FT
        if alt is not None:
            inp["altitude"] = f"{alt} ft"
            exp["result.fl_of_altitude"] = h_for(p_at(alt * FT) / scale) / FT / 100
        out.append(vec(len(out) + 1, inp, exp))
    out[0]["source"] = SPEC + " (lowest usable flight level FL185 at 29.42 inHg, 14 CFR 91.121(b))"
    out.append(vec(len(out) + 1, {"altimeter": "26.80 inHg"}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}))
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.altimetry.q-codes", qcodes()), ("aviation.altimetry.flight-level", flight_level())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
