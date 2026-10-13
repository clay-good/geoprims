#!/usr/bin/env python3
"""Golden vectors for the cold-temperature correction, worked by hand: the
ICAO Doc 8168 (2020) equation dH = (-dT / L0) ln(1 + L0 H / (T0 + L0 Ha))
with L0 = -0.0019812 K/ft and T0 = 288.15 K, the 4% per 10 C rule, and AIM
Table 7-3-1 read bilinearly (transcribed from the FAA's published table)."""
import json
import math
import sys
from pathlib import Path

SRC = "ICAO 2020 equation and AIM Table 7-3-1 worked in Python (tools/vectors/gen_coldtemp.py)"
SPEC = "add-aviation-suite cold-temperature scenarios"
FAA = "FAA Aeronautical Information Manual, 7-3-1 and Table 7-3-1"
FAA_TRUE = "FAA Safety Briefing, March/April 2014, pp. 25-26"
L0, T0 = -0.0019812, 288.15
H = [200, 300, 400, 500, 600, 700, 800, 900, 1000, 1500, 2000, 3000, 4000, 5000]
T = [10, 0, -10, -20, -30, -40, -50]
TBL = [
    [10, 10, 10, 10, 20, 20, 20, 20, 20, 30, 40, 60, 80, 90],
    [20, 20, 30, 30, 40, 40, 50, 50, 60, 90, 120, 170, 230, 280],
    [20, 30, 40, 50, 60, 70, 80, 90, 100, 150, 200, 290, 390, 490],
    [30, 50, 60, 70, 90, 100, 120, 130, 140, 210, 280, 420, 570, 710],
    [40, 60, 80, 100, 120, 140, 150, 170, 190, 280, 380, 570, 760, 950],
    [50, 80, 100, 120, 150, 170, 190, 220, 240, 360, 480, 720, 970, 1210],
    [60, 90, 120, 150, 180, 210, 240, 270, 300, 450, 590, 890, 1190, 1500],
]


def table(t, h):
    if not (200 <= h <= 5000) or t < -50:
        return None
    t = min(t, 10)
    j = next((j for j in range(13) if h <= H[j + 1]), 12)
    fh = (h - H[j]) / (H[j + 1] - H[j])
    i = next((i for i in range(6) if t >= T[i + 1]), 5)
    ft = (T[i] - t) / (T[i] - T[i + 1])
    row = lambda r: TBL[r][j] + fh * (TBL[r][j + 1] - TBL[r][j])
    return row(i) + ft * (row(i + 1) - row(i))


def vec(i, inp, exp, src=SRC, version="2026", tol=1e-6):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: {"rel": 0, "abs": tol} for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": version, "tolerance": t}


def case(elev, temp, alts):
    isa = 15 + L0 * elev
    dev = temp - isa
    exp = {"result.isa_deviation.value": dev}
    for k, a in enumerate(alts):
        h = a - elev
        c = (-dev / L0) * math.log(1 + L0 * h / (T0 + L0 * elev)) if dev < 0 else 0.0
        exp[f"result.altitudes.{k}.correction.value"] = c
        exp[f"result.altitudes.{k}.corrected.value"] = a + c
        exp[f"result.altitudes.{k}.rule.value"] = 0.04 * (-dev / 10) * h if dev < 0 else 0.0
        tb = table(temp, h) if dev < 0 else 0.0
        if tb is not None:
            exp[f"result.altitudes.{k}.table.value"] = tb
        if k == 0:
            exp["result.correction.value"] = c
            exp["result.corrected.value"] = a + c
    inp = {"elevation": f"{elev} ft", "temperature": f"{temp} degC", "altitudes": [{"altitude": f"{a} ft"} for a in alts]}
    return inp, exp


def cases():
    # Preserve the seven originally published vectors byte for byte.
    out = []
    for elev, temp, alts in [(2000, -30, [3500, 2500]), (0, 15, [1500]), (5000, -20, [6500, 7000, 9000]), (500, -45, [2500, 1200]), (1000, -10, [6500]), (4300, 3, [4800])]:
        inp, exp = case(elev, temp, alts)
        out.append(vec(len(out) + 1, inp, exp))
    out[0]["source"] = SPEC + " (1,500 ft above a 2,000 ft airport at -30 °C: about +218 ft, the 4% rule about +246 ft)"
    out[1]["source"] = SPEC + " (no correction when warmer than ISA)"
    out.append(vec(len(out) + 1, {"elevation": "2000 ft", "temperature": "-30 degC", "altitudes": [{"altitude": "1500 ft"}]}, {"ok": False, "error.code": "INVALID_INPUT"}))
    inp, exp = case(0, -12, [1314])
    out.append(vec(8, inp, {"result.altitudes.0.table.value": 150.0}, FAA, "FAA AIM effective 2025-08-07", tol=10.0))
    for elev, temp, alts in [
        (0, -50, [200, 5000]),
        (0, -25, [1500]),
        (2000, -30, [2200]),
        (1000, 20, [2000]),
        (-500, -20, [0]),
        (10000, -40, [11000]),
        (0, -60, [1000]),
        (0, -10, [6000]),
        (3000, -5, [3500, 4500, 7000]),
    ]:
        inp, exp = case(elev, temp, alts)
        out.append(vec(len(out) + 1, inp, exp))
    out.extend([
        vec(18, {"elevation": "15001 ft", "temperature": "-20 degC", "altitudes": [{"altitude": "16000 ft"}]}, {"ok": False, "error.code": "INVALID_INPUT"}),
        vec(19, {"elevation": "0 ft", "temperature": "-81 degC", "altitudes": [{"altitude": "1000 ft"}]}, {"ok": False, "error.code": "INVALID_INPUT"}),
        vec(20, {"elevation": "0 ft", "temperature": "61 degC", "altitudes": [{"altitude": "1000 ft"}]}, {"ok": False, "error.code": "INVALID_INPUT"}),
    ])
    assert len(out) == 20
    return out


def true_altitude():
    # Preserve the seven originally published vectors byte for byte.
    out = []
    for ind, dev, stn in [(8000, -20, None), (8000, 20, None), (12000, -10, 3000), (5500, 0, 1500), (35000, -15, None), (3000, -35, 2500)]:
        h = ind - (stn or 0)
        err = (dev / L0) * math.log(1 + L0 * h / (T0 + L0 * (stn or 0)))
        inp = {"indicated": f"{ind} ft", "isa_deviation": f"{dev} degC"}
        if stn is not None:
            inp["station_elevation"] = f"{stn} ft"
        out.append(vec(len(out) + 1, inp, {"result.true_altitude.value": ind + err, "result.error.value": err, "result.rule_error.value": 0.04 * dev / 10 * h}))
    out[0]["source"] = SPEC + " (ISA -20 °C at 8,000 ft indicated: true altitude below indicated)"
    out.append(vec(len(out) + 1, {"indicated": "1000 ft", "isa_deviation": "-10 degC", "station_elevation": "2000 ft"}, {"ok": False, "error.code": "INVALID_INPUT"}))
    out.append(vec(8, {"indicated": "240 ft", "isa_deviation": "-45 degC"}, {"result.true_altitude.value": 200.0}, FAA_TRUE, "March/April 2014", tol=10.0))
    for ind, dev, stn in [
        (0, -80, 0),
        (60000, 60, 0),
        (-2000, -20, -2000),
        (60000, 12, 15000),
        (10000, -1, 5000),
        (5000, 1, -500),
        (25000, -40, 10000),
        (1000, 30, -2000),
    ]:
        h = ind - stn
        err = (dev / L0) * math.log(1 + L0 * h / (T0 + L0 * stn))
        inp = {"indicated": f"{ind} ft", "isa_deviation": f"{dev} degC", "station_elevation": f"{stn} ft"}
        out.append(vec(len(out) + 1, inp, {"result.true_altitude.value": ind + err, "result.error.value": err, "result.rule_error.value": 0.04 * dev / 10 * h}))
    out.extend([
        vec(17, {"indicated": "1000 ft", "isa_deviation": "-81 degC"}, {"ok": False, "error.code": "INVALID_INPUT"}),
        vec(18, {"indicated": "1000 ft", "isa_deviation": "61 degC"}, {"ok": False, "error.code": "INVALID_INPUT"}),
        vec(19, {"indicated": "0 ft", "isa_deviation": "-10 degC", "station_elevation": "-2001 ft"}, {"ok": False, "error.code": "INVALID_INPUT"}),
        vec(20, {"indicated": "59000 ft", "isa_deviation": "-10 degC", "station_elevation": "-2000 ft"}, {"ok": False, "error.code": "INVALID_INPUT"}),
    ])
    assert len(out) == 20
    return out


def main():
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for name, vs in [("aviation.altimetry.cold-temperature", cases()), ("aviation.altimetry.true-altitude", true_altitude())]:
        (dest / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
