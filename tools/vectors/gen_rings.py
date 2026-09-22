#!/usr/bin/env python3
"""Golden vectors for navigation.route.range-rings: each ring's points by
GeographicLib's GeodSolve at the same azimuth steps, and its area by
Planimeter (both C++, independent of the geographiclib-rs the tool uses).
Requires GeodSolve and Planimeter on PATH."""
import json
import subprocess
import sys
from pathlib import Path

SRC = "Ring points by GeodSolve and areas by Planimeter (GeographicLib, C++), via tools/vectors/gen_rings.py"
SPEC = "add-navigation-and-geometry scenarios"
VER = subprocess.run(["GeodSolve", "--version"], capture_output=True, text=True).stdout.strip() or "GeographicLib"
NM = 1852.0


def ring(lat, lon, r, n):
    lines = "".join(f"{lat} {lon} {360 - 360 * i / n} {r}\n" for i in range(n))
    out = subprocess.run(["GeodSolve", "-p", "12"], input=lines, capture_output=True, text=True, check=True).stdout.strip().splitlines()
    return [tuple(map(float, l.split()[:2])) for l in out]


def area(pts):
    out = subprocess.run(["Planimeter", "-p", "9"], input="".join(f"{a} {b}\n" for a, b in pts), capture_output=True, text=True, check=True).stdout.split()
    return abs(float(out[2]))


def vec(i, inp, exp, src=SRC, ver=VER, tol=None):
    e = dict(exp)
    e.setdefault("ok", True)
    t = {k: (tol or {}).get(k, {"rel": 1e-9, "abs": 1e-9}) for k, v in e.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
    return {"id": f"v{i:03d}", "input": inp, "expect": e, "source": src, "sourceVersion": ver, "tolerance": t}


def main():
    out = []
    cases = [(39.8617, -104.6731, [10 * NM, 25 * NM, 50 * NM], 144), (51.47, -0.4543, [5000, 20000], 90), (-33.9461, 151.1772, [100_000], 180),
             (0.0, 179.5, [150_000], 120), (85.0, 30.0, [1_500_000], 360), (-89.0, 0.0, [300_000], 72), (64.8, -147.9, [250_000, 750_000], 240)]
    for lat, lon, radii, n in cases:
        inp = {"lat": lat, "lon": lon, "radii": [{"radius": f"{r} m"} for r in radii], "points": n}
        exp = {"result.ring_count": len(radii)}
        tol = {}
        for k, r in enumerate(radii):
            key = f"result.summary.{k}.area.value"
            exp[key] = area(ring(lat, lon, r, n)) / 1e6
            tol[key] = {"rel": 1e-9, "abs": 1e-9}
        out.append(vec(len(out) + 1, inp, exp, tol=tol))
    # The spec scenario: 1,500 km around 85° N encloses the North Pole.
    v = vec(len(out) + 1, {"lat": 85, "lon": 30, "radii": [{"radius": "1500 km"}]}, {"result.summary.0.pole": "north", "meta.warnings.1.code": "POLE_ENCLOSED"}, SPEC, "2026")
    out.append(v)
    out.append(vec(len(out) + 1, {"lat": 0, "lon": 179.5, "radii": [{"radius": "150 km"}]}, {"result.summary.0.crosses_antimeridian": "yes", "result.summary.0.pole": "none"}, SPEC, "2026"))
    out.append(vec(len(out) + 1, {"lat": 40, "lon": -105, "radii": [{"radius": "0 m"}]}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}, SPEC, "2026"))
    out.append(vec(len(out) + 1, {"lat": 40, "lon": -105, "radii": [{"radius": "20000 km"}]}, {"ok": False, "error.code": "OUT_OF_DOMAIN"}, SPEC, "2026"))
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "core/vectors")
    (dest / "navigation.route.range-rings.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in out))


if __name__ == "__main__":
    main()
