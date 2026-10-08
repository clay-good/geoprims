#!/usr/bin/env python3
"""Golden vectors for aviation.altimetry.pressure-per-height from the ambiance
package, an independent implementation of the ICAO Standard Atmosphere: the
height one hPa and one inHg span is the inverse of dp/dH, taken by finite
differences of ambiance's pressure over geopotential height (converted with
ambiance's own geopotential-to-geometric function; one-sided at sea level,
where ambiance's pressure just below 0 m is slightly inconsistent). With an
outside air temperature the span scales by T / T_ISA, the hydrostatic relation
dh/dp = R T / (g0 p) at the same pressure.

Needs ambiance (pip install ambiance) in a scratch virtualenv.
Usage: python3 tools/vectors/gen_pressure_height.py
"""
import json
from pathlib import Path

import ambiance
from ambiance import Atmosphere

FT = 0.3048
INHG = 3386.388640341
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.altimetry.pressure-per-height.jsonl"


def p_at(H):
    z = Atmosphere.geop2geom_height(H)
    return float(Atmosphere(z).pressure[0]), float(Atmosphere(z).temperature[0])


def dH_dp(H):
    d = 0.25
    if H < 2 * d:
        p0, p1, p2 = p_at(H)[0], p_at(H + d)[0], p_at(H + 2 * d)[0]
        dpdh = (-3 * p0 + 4 * p1 - p2) / (2 * d)
    else:
        dpdh = (p_at(H + d)[0] - p_at(H - d)[0]) / (2 * d)
    return -1.0 / dpdh


CASES = [(0, None), (5000, None), (10000, None), (18000, None), (30000, None), (40000, None), (50000, None),
         (0, 30.0), (18000, -30.0), (5000, 0.0)]
rows = []
for i, (ft, oat) in enumerate(CASES, 1):
    H = ft * FT
    _, t_isa = p_at(H)
    k = dH_dp(H) * ((oat + 273.15) / t_isa if oat is not None else 1.0)
    inp = {"pressure_altitude": f"{ft} ft"}
    if oat is not None:
        inp["temperature"] = f"{oat} degC"
    rows.append({
        "id": f"v{i:03d}",
        "input": inp,
        "expect": {"ok": True, "result.height_per_hpa.value": k * 100 / FT, "result.height_per_inhg.value": k * INHG / FT},
        "source": "ambiance (independent ICAO Standard Atmosphere): 1 / (dp/dH) by finite differences over geopotential height"
        + (", scaled by T / T_ISA for the given temperature" if oat is not None else ""),
        "sourceVersion": f"ambiance {getattr(ambiance, '__version__', 'pip 2026')}",
        "tolerance": {"result.height_per_hpa.value": {"rel": 1e-5}, "result.height_per_inhg.value": {"rel": 1e-5}},
    })
OUT.write_text("".join(json.dumps(r, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
for r in rows:
    print(r["input"], round(r["expect"]["result.height_per_hpa.value"], 3), round(r["expect"]["result.height_per_inhg.value"], 1))
