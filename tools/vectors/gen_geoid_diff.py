#!/usr/bin/env python3
"""EGM96 differential fixture: 2,000 seeded random points (plus poles, the
antimeridian, and grid edges) evaluated by GeographicLib's GeoidEval with the
same egm96-15 grid, cubic and bilinear. Writes
core/crates/gp-geodesy/tests/data/egm96_diff.csv. Needs GeoidEval
(brew install geographiclib)."""
import random
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GRID = ROOT / "assets/data/egm96-15/2009-08-29"


def main():
    rng = random.Random(96)
    pts = [(90, 0), (-90, 0), (90, 123.4), (-90, -77.7), (89.9, 10), (-89.9, 200), (0, 180), (0, -180), (45, 179.999), (16.776, -3.009)]
    pts += [(rng.uniform(-90, 90), rng.uniform(-180, 180)) for _ in range(2000)]
    inp = "".join(f"{la!r} {lo!r}\n" for la, lo in pts)
    run = lambda *a: subprocess.run(["GeoidEval", "-d", str(GRID), "-n", "egm96-15", *a], input=inp, capture_output=True, text=True, check=True).stdout.split()
    cub, lin = run(), run("-l")
    lines = ["lat,lon,cubic,bilinear"] + [f"{la!r},{lo!r},{c},{b}" for (la, lo), c, b in zip(pts, cub, lin)]
    (ROOT / "core/crates/gp-geodesy/tests/data/egm96_diff.csv").write_text("\n".join(lines) + "\n")
    print(len(pts), "points")
    import json
    src = "GeographicLib GeoidEval 2.7 with the egm96-15 grid (tools/vectors/gen_geoid_diff.py)"
    ver = "egm96-15 (2009-08-29)"
    sel = [(pts[i], cub[i], lin[i]) for i in (0, 1, 6, 9, 20, 400, 900, 1500)]
    gh, hc = [], []
    for k, ((la, lo), c, b) in enumerate(sel, 1):
        gh.append({"id": f"v{k:03d}", "input": {"lat": la, "lon": lo}, "expect": {"ok": True, "result.geoid_height.value": float(c)},
                   "source": src, "sourceVersion": ver, "tolerance": {"result.geoid_height.value": {"abs": 5e-5}}})
        hc.append({"id": f"v{k:03d}", "input": {"lat": la, "lon": lo, "height": "100 m"}, "expect": {"ok": True, "result.orthometric.value": 100 - float(c)},
                   "source": src, "sourceVersion": ver, "tolerance": {"result.orthometric.value": {"abs": 5e-5}}})
    k = len(sel) + 1
    gh.append({"id": f"v{k:03d}", "input": {"lat": pts[400][0], "lon": pts[400][1], "interpolation": "bilinear"}, "expect": {"ok": True, "result.geoid_height.value": float(lin[400])},
               "source": src + ", bilinear (-l)", "sourceVersion": ver, "tolerance": {"result.geoid_height.value": {"abs": 5e-5}}})
    for name, vs in [("geodesy.geoid.geoid-height", gh), ("geodesy.height.convert", hc)]:
        (ROOT / "core/vectors" / f"{name}.jsonl").write_text("".join(json.dumps(v, ensure_ascii=False, separators=(",", ":")) + "\n" for v in vs))


if __name__ == "__main__":
    main()
