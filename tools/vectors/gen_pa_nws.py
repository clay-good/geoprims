#!/usr/bin/env python3
"""Golden vectors for aviation.altimetry.pressure-altitude with constants "nws":
the NWS WxCalc formulas as published (weather.gov/media/epz/wxcalc/
stationPressure.pdf and pressureAltitude.pdf, read 2026-10-09), with NWS's
33.8639 mb per inHg. Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_pa_nws.py
"""
import json
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.altimetry.pressure-altitude.jsonl"
SRC = "NWS WxCalc station pressure and pressure altitude formulas (tools/vectors/gen_pa_nws.py)"


def nws(elev_ft, alt_inhg):
    p_inhg = alt_inhg * ((288 - 0.0065 * elev_ft * 0.3048) / 288) ** 5.2561
    mb = p_inhg * 33.8639
    return p_inhg, (1 - (mb / 1013.25) ** 0.190284) * 145366.45


rows = [r for r in map(json.loads, OUT.read_text().splitlines()) if r["source"] != SRC]
for elev, alt in [(5000, 29.80), (0, 29.92), (5434, 30.12), (-200, 30.50), (9000, 28.90), (12000, 29.50)]:
    p, pa = nws(elev, alt)
    rows.append({"id": f"v{len(rows) + 1:03d}",
                 "input": {"elevation": f"{elev} ft", "altimeter": f"{alt} inHg", "constants": "nws"},
                 "expect": {"ok": True, "result.pressure_altitude.value": pa, "result.station_pressure.value": p},
                 "source": SRC, "sourceVersion": "NWS WxCalc (read 2026-10-09)",
                 "tolerance": {"result.pressure_altitude.value": {"abs": 0.01}, "result.station_pressure.value": {"abs": 1e-5}}})
OUT.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in rows))
print(f"{OUT.name}: {len(rows)} vectors")
