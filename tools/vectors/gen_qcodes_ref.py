#!/usr/bin/env python3
"""Golden vectors for aviation.altimetry.q-codes from two independent
libraries. ambiance, an implementation of the ICAO standard atmosphere, gives
the pressure at a geopotential height and the height of a pressure, from which
QFE, QNH, and QNE follow by the relation that defines QNH: an altimeter set to
it reads the field elevation on the ground. MetPy, Unidata's meteorology
library, gives the altimeter setting US stations report, by inverting its
altimeter_to_station_pressure with a root finder.

MetPy writes the station formula with its own constants, so the NWS setting is
held to 5 parts in 100,000 (0.0015 inHg); the ISA figures are held to 1e-9.

Below sea level ambiance starts from the pressure ICAO tabulates at -5 km to
six figures, which leaves it 0.026 Pa under 1013.25 hPa just below zero height
(2.6 parts in 10 million, 2 mm of height). A case that passes through a
negative pressure altitude, as any setting above 1013.25 hPa does, is
therefore held to 5 parts in 10 million and 0.01 ft.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_qcodes_ref.py   (with ambiance, metpy, and scipy installed)
"""
import json
from importlib.metadata import version
from pathlib import Path

import metpy.calc as mc
from ambiance import Atmosphere
from metpy.units import units as u
from scipy.optimize import brentq

VER = f"ambiance {version('ambiance')}, MetPy {version('metpy')}"
SRC = f"ambiance {version('ambiance')} (ICAO standard atmosphere) and MetPy {version('metpy')} (altimeter_to_station_pressure) (tools/vectors/gen_qcodes_ref.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.altimetry.q-codes.jsonl"
FT, INHG = 0.3048, 33.86389  # m per ft; hPa per inHg


def height(p_hpa):
    """Geopotential height in m of a pressure."""
    return float(Atmosphere.from_pressure(p_hpa * 100).H[0])


def pressure(h_m):
    """Pressure in hPa at a geopotential height."""
    return float(Atmosphere(Atmosphere.geop2geom_height(h_m)).pressure[0]) / 100


def nws(elev_ft, station_hpa):
    return brentq(lambda a: mc.altimeter_to_station_pressure(a * u.hPa, elev_ft * u.ft).to("hPa").m - station_hpa, 700, 1200, xtol=1e-12)


ISA = {"rel": 1e-9, "abs": 1e-9}


def tolerances(*heights):
    """Tight above sea level; ambiance's own step at zero height below it."""
    low = min(heights) < 0
    return {"result.qfe.value": {"rel": 5e-7} if low else ISA, "result.qnh.value": {"rel": 5e-7} if low else ISA, "result.qne.value": {"abs": 0.01 if low else 1e-6}}

# Lines from other generators are kept byte for byte.
kept = [line for line in OUT.read_text().splitlines() if "gen_qcodes_ref.py" not in json.loads(line)["source"]]
rows = []


def add(inp, exp, tol):
    rows.append({"id": f"v{len(kept) + len(rows) + 1:03d}", "input": inp, "expect": {"ok": True, **exp}, "source": SRC, "sourceVersion": VER, "tolerance": tol})


# From an altimeter setting: elevation ft, setting, unit.
for elev, setting, unit in [(1000, 1013, "hPa"), (5000, 29.80, "inHg"), (5434, 30.42, "inHg"), (-1300, 1035, "hPa"), (13325, 1028, "hPa"),
                            (0, 28.50, "inHg"), (2500, 995, "hPa"), (9070, 29.92, "inHg")]:
    qnh = setting * (INHG if unit == "inHg" else 1)
    qfe = pressure(elev * FT + height(qnh))
    add({"elevation": f"{elev} ft", "altimeter": f"{setting} {unit}"},
        {"result.qfe.value": qfe, "result.qnh.value": qnh / INHG, "result.qne.value": height(qfe) / FT},
        tolerances(height(qnh), height(qfe)))
# From a station pressure: elevation ft, pressure hPa.
for elev, p in [(5000, 843.0), (1000, 979.3), (300, 1002.1), (7820, 752.4), (-50, 1019.6), (3200, 905.5)]:
    add({"elevation": f"{elev} ft", "station_pressure": f"{p} hPa"},
        {"result.qfe.value": p, "result.qnh.value": pressure(height(p) - elev * FT) / INHG, "result.qne.value": height(p) / FT, "result.qnh_nws.value": nws(elev, p) / INHG},
        {**tolerances(height(p), height(p) - elev * FT), "result.qnh_nws.value": {"rel": 5e-5}})
OUT.write_text("".join(line + "\n" for line in kept + [json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in rows]))
print(f"{OUT.name}: {len(kept) + len(rows)} vectors")
