#!/usr/bin/env python3
"""Golden vectors for the two directional height conversions, with the geoid
height from GeographicLib's GeoidEval on the same egm96-15 grid (not the
core's interpolator). Needs GeoidEval on the PATH."""
import json
import random
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GRID = ROOT / "assets/data/egm96-15/2009-08-29"
SRC = "GeographicLib GeoidEval with the egm96-15 grid (tools/vectors/gen_height_direction.py)"
VER = "egm96-15 (2009-08-29)"
FT = 0.3048


def geoid(points, bilinear=False):
    inp = "".join(f"{la!r} {lo!r}\n" for la, lo in points)
    args = ["GeoidEval", "-d", str(GRID), "-n", "egm96-15"] + (["-l"] if bilinear else [])
    return [float(x) for x in subprocess.run(args, input=inp, capture_output=True, text=True, check=True).stdout.split()]


def generate(direction):
    rng = random.Random({"ellipsoidal-to-orthometric": 99101, "orthometric-to-ellipsoidal": 99102}[direction])
    first = (46.452, 6.583, 448.2, "m") if direction == "ellipsoidal-to-orthometric" else (40.015, -105.2705, 1655.0, "m")
    cases = [first, (16.776, -3.009, 100.0, "m"), (90.0, 0.0, 0.0, "m"), (-90.0, 0.0, 0.0, "m")]
    while len(cases) < 20:
        unit = ["m", "ft"][len(cases) % 2]
        cases.append((round(rng.uniform(-89.9, 89.9), 5), round(rng.uniform(-180, 180), 5), round(rng.uniform(-400, 9000), 2), unit))
    bilinear = {5, 11}
    out = []
    for i, (la, lo, h, unit) in enumerate(cases, 1):
        n = geoid([(la, lo)], bilinear=i in bilinear)[0]
        scale = FT if unit == "ft" else 1.0
        h_m = h * scale
        ell, orth = (h_m, h_m - n) if direction == "ellipsoidal-to-orthometric" else (h_m + n, h_m)
        inp = {"lat": la, "lon": lo, "height": f"{h} {unit}"}
        if i in bilinear:
            inp["interpolation"] = "bilinear"
        exp = {"ok": True, "result.geoid_height.value": n, "result.ellipsoidal.value": ell / scale, "result.orthometric.value": orth / scale}
        # GeoidEval prints to 0.1 mm; the grid itself is good to about a meter.
        tol = {k: {"abs": 5e-5 / scale} for k in exp if k != "ok"}
        tol["result.geoid_height.value"] = {"abs": 5e-5}
        out.append({"id": f"v{i:03d}", "input": inp, "expect": exp, "source": SRC + (", bilinear (-l)" if i in bilinear else ""), "sourceVersion": VER, "tolerance": tol})
    for inp, field in [({"lat": 91, "lon": 0, "height": "10 m"}, "/lat"), ({"lat": 10, "lon": 0}, None)]:
        e = {"ok": False, "error.code": "INVALID_INPUT"}
        if field:
            e["error.field"] = field
        out.append({"id": f"v{len(out) + 1:03d}", "input": inp, "expect": e, "source": "add-geodesy-suite scenarios", "sourceVersion": "2026", "tolerance": {}})
    return out


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    for direction in ["ellipsoidal-to-orthometric", "orthometric-to-ellipsoidal"]:
        (out / f"geodesy.height.{direction}.jsonl").write_text("".join(json.dumps(v, separators=(",", ":")) + "\n" for v in generate(direction)))


if __name__ == "__main__":
    main()
