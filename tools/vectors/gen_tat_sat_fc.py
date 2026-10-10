#!/usr/bin/env python3
"""Golden vectors for aviation.airspeed.tat-sat from flightcondition, an
independent Python library (pip install flightcondition).

For a height and Mach number flightcondition gives the static temperature T,
the total temperature T0, and the temperatures a wall recovers in turbulent
and laminar flow, Tr_turb and Tr_lamr. The tool is handed a total or recovery
temperature and must return the library's static temperature, or the reverse.
For the recovery temperatures the recovery factor is the library's own,
(Tr - T) / (T0 - T): 0.888 turbulent and 0.837 laminar.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_tat_sat_fc.py   (with flightcondition installed)
"""
import json
from importlib.metadata import version
from pathlib import Path

from flightcondition import FlightCondition, unit

VER = version("flightcondition")
SRC = f"flightcondition {VER} (independent Python library): T, T0, Tr_turb, and Tr_lamr at a height and Mach number (tools/vectors/gen_tat_sat_fc.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.airspeed.tat-sat.jsonl"
TOL = {"abs": 1e-9}

# Lines from other generators are kept byte for byte.
kept = [line for line in OUT.read_text().splitlines() if "gen_tat_sat_fc.py" not in json.loads(line)["source"]]
rows = []
# height ft, Mach, which library temperature is the probe's, direction
CASES = [(35000, 0.8, "T0", "total"), (41000, 0.85, "T0", "total"), (10000, 0.4, "T0", "total"), (0, 0.2, "T0", "total"), (25000, 0.62, "T0", "total"),
         (51000, 0.92, "T0", "total"), (37000, 0.78, "T0", "static"), (5000, 0.3, "T0", "static"), (45000, 0.9, "T0", "static"),
         (33000, 0.74, "Tr_turb", "total"), (18000, 0.5, "Tr_turb", "total"), (39000, 0.82, "Tr_turb", "static"), (2000, 0.25, "Tr_turb", "static"),
         (29000, 0.7, "Tr_lamr", "total"), (43000, 0.88, "Tr_lamr", "total"), (12000, 0.45, "Tr_lamr", "static")]
for h, mach, probe, kind in CASES:
    fc = FlightCondition(h=h * unit("ft"), M=mach)
    t = float(fc.T.to("K").magnitude)
    t0 = float(fc.T0.to("K").magnitude)
    tp = float(getattr(fc, probe).to("K").magnitude)
    inp = {"temperature": f"{tp!r} K" if kind == "total" else f"{t!r} K", "mach": mach}
    if kind == "static":
        inp["temperature_kind"] = "static"
    if probe != "T0":
        inp["recovery_factor"] = (tp - t) / (t0 - t)
    exp = {"ok": True, "result.sat.value": t - 273.15, "result.tat.value": tp - 273.15, "result.ram_rise.value": tp - t}
    rows.append({"id": f"v{len(kept) + len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": VER,
                 "tolerance": {k: TOL for k in exp if k != "ok"}})
OUT.write_text("".join(line + "\n" for line in kept + [json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in rows]))
print(f"{OUT.name}: {len(kept) + len(rows)} vectors")
