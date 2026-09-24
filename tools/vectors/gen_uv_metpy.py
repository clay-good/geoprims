#!/usr/bin/env python3
"""Wind components against MetPy (promotion of aviation.wind.uv).

MetPy is Unidata's meteorology library, written independently of geoprims.
Its wind_components takes the direction the wind blows from and returns u
(toward east) and v (toward north); wind_direction and wind_speed go back.
MetPy reports a wind from due north as 360 degrees where the tool says 0, so
directions are compared modulo 360.

Writes core/crates/gp-aviation/tests/data/uv_metpy.json (500 cases each
way) and appends vectors to core/vectors/aviation.wind.uv.jsonl (existing
lines left byte for byte), the first being the example in MetPy's own
documentation: 10 m/s from 225 degrees gives u = v = 7.07106781 m/s.
Requires metpy."""
import json
import random
from pathlib import Path

import metpy
import numpy as np
from metpy.calc import wind_components, wind_direction, wind_speed
from metpy.units import units

KT = 1852.0 / 3600.0  # m/s per knot
FIX = Path("core/crates/gp-aviation/tests/data/uv_metpy.json")
VEC = Path("core/vectors/aviation.wind.uv.jsonl")


def comps(d, s_kt):
    u, v = wind_components(s_kt * units.knot, d * units.deg)
    return float(u.to("knot").m), float(v.to("knot").m)


def back(u, v):
    uu, vv = u * units.knot, v * units.knot
    return float(wind_direction(uu, vv).to("deg").m) % 360.0, float(wind_speed(uu, vv).to("knot").m)


def main():
    rng = random.Random(8)
    fwd, rev = [], []
    for _ in range(500):
        d, s = round(rng.uniform(0, 360), 3), round(rng.uniform(0.5, 150), 3)
        fwd.append({"direction": d, "speed": s, "u": comps(d, s)[0], "v": comps(d, s)[1]})
        u, v = round(rng.uniform(-100, 100), 3), round(rng.uniform(-100, 100), 3)
        rev.append({"u": u, "v": v, "direction": back(u, v)[0], "speed": back(u, v)[1]})
    FIX.parent.mkdir(parents=True, exist_ok=True)
    FIX.write_text(json.dumps({"source": f"MetPy {metpy.__version__}", "forward": fwd, "reverse": rev}, separators=(",", ":")) + "\n")

    lines = VEC.read_text().splitlines()
    kept = [l for l in lines if "gen_uv_metpy.py" not in l]
    n, new = len(kept), []

    def add(inp, exp, src):
        e = dict(exp)
        e["ok"] = True
        tol = {k: {"rel": 1e-9, "abs": 1e-9} for k in exp}
        new.append({"id": f"v{n + len(new) + 1:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": f"MetPy {metpy.__version__}", "tolerance": tol})

    doc = 7.07106781 / KT
    add({"direction": "225 deg", "speed": "10 m/s"}, {"result.u.value": 10 / KT * np.sqrt(0.5), "result.v.value": 10 / KT * np.sqrt(0.5)},
        f"MetPy documentation, metpy.calc.wind_components example: 10 m/s from 225 deg gives u = v = 7.07106781 m/s ({doc:.6f} kt) (tools/vectors/gen_uv_metpy.py)")
    for c in fwd[:4]:
        add({"direction": f"{c['direction']} deg", "speed": f"{c['speed']} kt"}, {"result.u.value": c["u"], "result.v.value": c["v"]}, "MetPy wind_components (tools/vectors/gen_uv_metpy.py)")
    for c in rev[:4]:
        add({"u": f"{c['u']} kt", "v": f"{c['v']} kt"}, {"result.direction.value": c["direction"], "result.speed.value": c["speed"]}, "MetPy wind_direction and wind_speed (tools/vectors/gen_uv_metpy.py)")
    VEC.write_text("".join(l + "\n" for l in kept) + "".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in new))
    print(n, "->", n + len(new), "vectors")


if __name__ == "__main__":
    main()
