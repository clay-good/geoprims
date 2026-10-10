#!/usr/bin/env python3
"""Golden vectors for aviation.wind.aloft-interpolate from MetPy and NumPy,
neither of which the core uses.

MetPy's wind_components turns each forecast level's direction and speed into
u and v, numpy.interp carries u, v, and the temperature to the altitude, and
MetPy's wind_direction and wind_speed read the wind back. The levels are
standard winds-aloft altitudes with winds that veer, back, and swing through
north between them; no case asks for the wind exactly at a level or has a
resultant under 3 kt, where the direction means little.

Replaces only its own rows in the tool's vector file.

Usage: python3 tools/vectors/gen_aloft_metpy.py   (with metpy installed)
"""
import json
from pathlib import Path

import metpy
import numpy as np
from metpy.calc import wind_components, wind_direction, wind_speed
from metpy.units import units as u

SRC = f"MetPy {metpy.__version__} (wind_components, wind_direction, wind_speed) and NumPy {np.__version__} (interp) (tools/vectors/gen_aloft_metpy.py)"
OUT = Path(__file__).resolve().parents[2] / "core/vectors/aviation.wind.aloft-interpolate.jsonl"
TOL = {"abs": 1e-9}

# Lines from other generators are kept byte for byte.
kept = [line for line in OUT.read_text().splitlines() if "gen_aloft_metpy.py" not in json.loads(line)["source"]]
rows = []
# (levels as (altitude ft, direction deg, speed kt, temperature degC or None), altitude ft)
A = [(3000, 250, 10, 15), (6000, 260, 20, 9), (9000, 280, 35, 3), (12000, 290, 45, -4), (18000, 300, 60, -18)]
B = [(6000, 350, 15, 2), (9000, 10, 22, -5), (12000, 40, 30, -11)]
C = [(18000, 240, 60, -21), (24000, 250, 80, -33), (30000, 250, 95, -45), (34000, 260, 110, -52), (39000, 270, 120, -56)]
D = [(3000, 140, 8, None), (6000, 180, 12, None), (9000, 230, 25, None)]
E = [(12000, 20, 40, -8), (9000, 340, 25, -2), (6000, 300, 12, 5)]  # given top down
CASES = [(A, 4000), (A, 7500), (A, 10000), (A, 15500), (B, 6500), (B, 8000), (B, 11900), (C, 21000), (C, 28000), (C, 32500), (C, 38000),
         (D, 4200), (D, 7700), (E, 7000), (E, 10500), (A, 3001)]
for levels, alt in CASES:
    lv = sorted(levels)
    alts = np.array([x[0] for x in lv], dtype=float)
    uu, vv = wind_components(np.array([x[2] for x in lv], dtype=float) * u.knot, np.array([x[1] for x in lv], dtype=float) * u.deg)
    ui, vi = float(np.interp(alt, alts, uu.m)), float(np.interp(alt, alts, vv.m))
    # MetPy's u and v are where the wind blows to: u toward east, v toward north.
    spd = float(wind_speed(ui * u.knot, vi * u.knot).m)
    assert spd > 3 and alt not in alts
    exp = {"ok": True, "result.u.value": ui, "result.v.value": vi, "result.speed.value": spd,
           "result.direction.value": float(wind_direction(ui * u.knot, vi * u.knot).to("deg").m) % 360,
           "result.below.value": float(alts[alts < alt].max()), "result.above.value": float(alts[alts > alt].min())}
    if lv[0][3] is not None:
        exp["result.temperature.value"] = float(np.interp(alt, alts, [x[3] for x in lv]))
    inp = {"levels": [{"altitude": f"{a} ft", "direction": f"{d} deg", "speed": f"{s} kt", **({} if t is None else {"temperature": f"{t} degC"})} for a, d, s, t in levels],
           "altitude": f"{alt} ft"}
    rows.append({"id": f"v{len(kept) + len(rows) + 1:03d}", "input": inp, "expect": exp, "source": SRC, "sourceVersion": f"MetPy {metpy.__version__}",
                 "tolerance": {k: TOL for k in exp if k != "ok"}})
OUT.write_text("".join(line + "\n" for line in kept + [json.dumps(r, ensure_ascii=False, separators=(",", ":")) for r in rows]))
print(f"{OUT.name}: {len(kept) + len(rows)} vectors")
