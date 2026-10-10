#!/usr/bin/env python3
"""Golden vectors for the two airspeed tools from flightcondition 26.4.20, an
independent Python library (pip install flightcondition), below Mach 1.

flightcondition takes a geometric height or a pressure. A pressure altitude is
a pressure, so each case hands it the ISA pressure at that altitude and lets it
find Mach, TAS, and EAS from CAS (or CAS from Mach or TAS) at the standard
temperature for that pressure. The tools are run with temperature_source isa.

Above Mach 1 the library is no reference: it applies the subsonic relation
there (520 kt CAS at FL410 gives it Mach 1.53; the Rayleigh pitot formula, which
the tools use, gives 1.61), so every case here is subsonic.

Replaces only its own rows in each tool's vector file.

Usage: python3 tools/vectors/gen_airspeed_fc.py   (with flightcondition installed)
"""
import json
import math
from importlib.metadata import version
from pathlib import Path

from flightcondition import FlightCondition, unit

VER = version("flightcondition")
SRC = f"flightcondition {VER} (independent Python library), given the ISA pressure of the pressure altitude; subsonic (tools/vectors/gen_airspeed_fc.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors"
P0, T0, G0, R = 101325.0, 288.15, 9.80665, 287.05287


def isa_p(h):
    if h <= 11000:
        return P0 * (1 - 0.0065 * h / T0) ** (G0 / (R * 0.0065))
    p11 = P0 * (1 - 0.0065 * 11000 / T0) ** (G0 / (R * 0.0065))
    return p11 * math.exp(-G0 * (h - 11000) / (R * 216.65))


def at(ft, **speed):
    fc = FlightCondition(p=isa_p(ft * 0.3048) * unit("Pa"), **speed)
    assert float(fc.M) < 0.99, (ft, speed, float(fc.M))
    return float(fc.M), float(fc.CAS.to("kt").magnitude), float(fc.TAS.to("kt").magnitude), float(fc.EAS.to("kt").magnitude)


# The library solves for Mach by iteration: agreement is 1e-9 at low speed and
# 5e-7 in Mach near 0.95, so the tolerances sit a little above that.
TOL = {"result.mach": {"abs": 2e-6}, "result.tas.value": {"abs": 1e-3}, "result.eas.value": {"abs": 1e-3}, "result.cas.value": {"abs": 1e-3}}


def write(tool, rows):
    path = OUT / f"{tool}.jsonl"
    kept = [r for r in map(json.loads, path.read_text().splitlines()) if "flightcondition" not in r["source"]]
    for inp, exp in rows:
        kept.append({"id": f"v{len(kept) + 1:03d}", "input": inp, "expect": {"ok": True, **exp}, "source": SRC,
                     "sourceVersion": f"flightcondition {VER}", "tolerance": {k: TOL[k] for k in exp}})
    path.write_text("".join(json.dumps(r, ensure_ascii=False, separators=(",", ":")) + "\n" for r in kept))
    print(f"{path.name}: {len(kept)} vectors")


forward = []
for cas, ft in [(250, 10000), (300, 35000), (120, 5000), (65, 0), (450, 20000), (350, 28000), (180, 15000), (90, 2500),
                (280, 31000), (230, 39000), (150, 8000), (400, 12000), (210, 24000), (320, 18000)]:
    m, _, tas, eas = at(ft, CAS=cas * unit("kt"))
    forward.append(({"airspeed": f"{cas} kt", "pressure_altitude": f"{ft} ft", "temperature_source": "isa"},
                    {"result.mach": m, "result.tas.value": tas, "result.eas.value": eas}))
write("aviation.airspeed.cas-to-tas", forward)

back = []
for mach, ft in [(0.78, 35000), (0.5, 20000), (0.25, 3000), (0.85, 41000), (0.65, 30000), (0.3, 8000)]:
    m, cas, _, eas = at(ft, M=mach)
    back.append(({"mach": mach, "pressure_altitude": f"{ft} ft"}, {"result.cas.value": cas, "result.eas.value": eas}))
for tas, ft in [(288.7, 10000), (450, 35000), (130, 5000), (480, 39000), (200, 12000), (350, 25000)]:
    m, cas, _, eas = at(ft, TAS=tas * unit("kt"))
    back.append(({"tas": f"{tas} kt", "pressure_altitude": f"{ft} ft", "temperature_source": "isa"},
                 {"result.cas.value": cas, "result.mach": m, "result.eas.value": eas}))
write("aviation.airspeed.tas-to-cas", back)
